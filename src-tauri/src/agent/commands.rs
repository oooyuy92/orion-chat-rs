use std::collections::HashMap;
use std::sync::Arc;

use tauri::{ipc::Channel, State};
use tokio::sync::Mutex;
use yoagent::agent_loop::AgentLoopConfig;
use yoagent::tools::{BashTool, EditFileTool, ListFilesTool, ReadFileTool, SearchTool, WriteFileTool};
use yoagent::{
    agent_loop, now_ms, AgentContext, AgentMessage, CacheConfig, Content,
    Message as AgentLlmMessage, StopReason, ThinkingLevel, ToolExecutionStrategy, Usage,
};

use crate::agent::config;
use crate::agent::events::handle_agent_event;
use crate::agent::permissions::{ChatEventEmitter, PermissionedTool};
use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{AuthAction, ChatEvent, Message, MessageStatus, MessageType, Role, ToolPermissions};
use crate::state::AppState;

#[tauri::command]
pub async fn agent_chat(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    message: String,
    model_id: String,
    channel: Channel<ChatEvent>,
) -> AppResult<Message> {
    let app_state = Arc::clone(state.inner());
    app_state
        .db
        .with_conn(|conn| db::conversations::get(conn, &conversation_id).map(|_| ()))?;

    let provider_config = config::build_provider_config(&app_state.db, &model_id).await?;
    let permissions = load_tool_permissions(&app_state)?;
    let working_dir = load_working_dir(&app_state)?;
    let history = load_conversation_messages(&app_state.db, &conversation_id)?;

    let user_message = insert_user_message(&app_state.db, &conversation_id, &message)?;
    let assistant_message_id = uuid::Uuid::new_v4().to_string();
    let emit_event: ChatEventEmitter = Arc::new({
        let channel = channel.clone();
        move |event| {
            let _ = channel.send(event);
        }
    });

    emit_event(ChatEvent::Started {
        message_id: assistant_message_id.clone(),
    });

    let tool_message_ids = Arc::new(Mutex::new(HashMap::new()));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let event_state = app_state.clone();
    let event_emit = emit_event.clone();
    let event_conversation_id = conversation_id.clone();
    let event_assistant_message_id = assistant_message_id.clone();
    let event_tool_message_ids = tool_message_ids.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            handle_agent_event(
                event,
                &event_emit,
                &event_state.db,
                &event_conversation_id,
                &event_assistant_message_id,
                &event_tool_message_ids,
            )
            .await;
        }
    });

    let prompts = vec![AgentMessage::Llm(AgentLlmMessage::user(message))];
    let mut context = AgentContext {
        system_prompt: "You are a helpful assistant with access to tools.".to_string(),
        messages: history,
        tools: build_tools(
            app_state.clone(),
            &conversation_id,
            permissions,
            emit_event.clone(),
            &working_dir,
        ),
    };

    let cancel_rx = app_state.create_cancel_token(&conversation_id).await;
    let cancel_token = watch_to_cancellation_token(cancel_rx);
    let model_max_tokens = provider_config.model_config.max_tokens;
    let config = AgentLoopConfig {
        provider: provider_config.provider,
        model: provider_config.model,
        api_key: provider_config.api_key,
        thinking_level: ThinkingLevel::Off,
        max_tokens: Some(model_max_tokens),
        temperature: None,
        model_config: Some(provider_config.model_config),
        convert_to_llm: None,
        transform_context: None,
        get_steering_messages: None,
        get_follow_up_messages: None,
        context_config: None,
        compaction_strategy: None,
        execution_limits: None,
        cache_config: CacheConfig::default(),
        tool_execution: ToolExecutionStrategy::default(),
        retry_config: yoagent::RetryConfig::default(),
        before_turn: None,
        after_turn: None,
        on_error: None,
        input_filters: Vec::new(),
    };

    let new_messages = agent_loop(prompts, &mut context, &config, tx, cancel_token).await;
    let _ = event_task.await;

    app_state.remove_cancel_token(&conversation_id).await;

    let final_text = extract_final_text(&new_messages);
    let assistant_message = insert_assistant_message(
        &app_state.db,
        &conversation_id,
        &assistant_message_id,
        &final_text,
        &model_id,
    )?;

    emit_event(ChatEvent::Finished {
        message_id: assistant_message_id,
    });

    let _ = user_message;
    Ok(assistant_message)
}

#[tauri::command]
pub async fn agent_stop(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<()> {
    state.cancel_conversation(&conversation_id).await;
    state.clear_pending_auth_for_conversation(&conversation_id).await;
    Ok(())
}

#[tauri::command]
pub async fn agent_authorize_tool(
    state: State<'_, Arc<AppState>>,
    tool_call_id: String,
    action: AuthAction,
) -> AppResult<()> {
    if state.resolve_auth(&tool_call_id, action).await {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "No pending auth for {tool_call_id}"
        )))
    }
}

#[tauri::command]
pub async fn get_tool_permissions(
    state: State<'_, Arc<AppState>>,
) -> AppResult<ToolPermissions> {
    load_tool_permissions(&state)
}

#[tauri::command]
pub async fn set_tool_permissions(
    state: State<'_, Arc<AppState>>,
    permissions: ToolPermissions,
) -> AppResult<()> {
    let json = serde_json::to_string(&permissions)?;
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO agent_settings (key, value) VALUES ('tool_permissions', ?1)",
            [json],
        )?;
        Ok(())
    })
}

fn load_tool_permissions(state: &AppState) -> AppResult<ToolPermissions> {
    let permissions_json = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT value FROM agent_settings WHERE key = 'tool_permissions'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(Into::into)
    })?;

    serde_json::from_str(&permissions_json).map_err(Into::into)
}

fn load_working_dir(state: &AppState) -> AppResult<String> {
    let working_dir = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT value FROM agent_settings WHERE key = 'working_dir'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(Into::into)
    })?;

    if working_dir.trim().is_empty() {
        Ok(std::env::current_dir()
            .map_err(AppError::Io)?
            .display()
            .to_string())
    } else {
        Ok(working_dir)
    }
}

fn build_tools(
    state: Arc<AppState>,
    conversation_id: &str,
    permissions: ToolPermissions,
    emit_event: ChatEventEmitter,
    working_dir: &str,
) -> Vec<Box<dyn yoagent::AgentTool>> {
    let make_tool = |tool: Box<dyn yoagent::AgentTool>| {
        Box::new(PermissionedTool::new(
            tool,
            state.clone(),
            conversation_id.to_string(),
            permissions.clone(),
            emit_event.clone(),
        )) as Box<dyn yoagent::AgentTool>
    };

    vec![
        make_tool(Box::new(BashTool::new().with_cwd(working_dir.to_string()))),
        make_tool(Box::new(ReadFileTool::new())),
        make_tool(Box::new(WriteFileTool::new())),
        make_tool(Box::new(EditFileTool::new())),
        make_tool(Box::new(ListFilesTool::new())),
        make_tool(Box::new(SearchTool::new().with_root(working_dir.to_string()))),
    ]
}

fn load_conversation_messages(db: &crate::db::Database, conversation_id: &str) -> AppResult<Vec<AgentMessage>> {
    let messages = db.with_conn(|conn| db::messages::list_by_conversation(conn, conversation_id))?;
    Ok(messages
        .iter()
        .filter_map(orion_message_to_agent_message)
        .collect())
}

fn orion_message_to_agent_message(message: &Message) -> Option<AgentMessage> {
    match message.message_type {
        MessageType::Text => match message.role {
            Role::User => Some(AgentMessage::Llm(AgentLlmMessage::User {
                content: vec![Content::Text {
                    text: message.content.clone(),
                }],
                timestamp: now_ms(),
            })),
            Role::Assistant => {
                let mut content = Vec::new();
                if !message.content.is_empty() {
                    content.push(Content::Text {
                        text: message.content.clone(),
                    });
                }
                if let Some(reasoning) = message.reasoning.clone().filter(|s| !s.is_empty()) {
                    content.push(Content::Thinking {
                        thinking: reasoning,
                        signature: None,
                    });
                }
                Some(AgentMessage::Llm(AgentLlmMessage::Assistant {
                    content,
                    stop_reason: if matches!(message.status, MessageStatus::Error) {
                        StopReason::Error
                    } else {
                        StopReason::Stop
                    },
                    model: message.model_id.clone().unwrap_or_default(),
                    provider: "orion".into(),
                    usage: Usage::default(),
                    timestamp: now_ms(),
                    error_message: if matches!(message.status, MessageStatus::Error) {
                        Some(message.content.clone())
                    } else {
                        None
                    },
                }))
            }
            Role::System => None,
        },
        MessageType::ToolCall => Some(AgentMessage::Llm(AgentLlmMessage::Assistant {
            content: vec![Content::ToolCall {
                id: message.tool_call_id.clone().unwrap_or_default(),
                name: message.tool_name.clone().unwrap_or_default(),
                arguments: serde_json::from_str(
                    message.tool_input.as_deref().unwrap_or("{}"),
                )
                .unwrap_or_else(|_| serde_json::json!({})),
            }],
            stop_reason: StopReason::ToolUse,
            model: message.model_id.clone().unwrap_or_default(),
            provider: "orion".into(),
            usage: Usage::default(),
            timestamp: now_ms(),
            error_message: None,
        })),
        MessageType::ToolResult => Some(AgentMessage::Llm(AgentLlmMessage::ToolResult {
            tool_call_id: message.tool_call_id.clone().unwrap_or_default(),
            tool_name: message.tool_name.clone().unwrap_or_default(),
            content: vec![Content::Text {
                text: message.content.clone(),
            }],
            is_error: message.tool_error,
            timestamp: now_ms(),
        })),
    }
}

fn insert_user_message(db: &crate::db::Database, conversation_id: &str, content: &str) -> AppResult<Message> {
    let message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        role: Role::User,
        content: content.to_string(),
        reasoning: None,
        model_id: None,
        status: MessageStatus::Done,
        token_count: None,
        created_at: crate::commands::chat::chrono_now(),
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
        message_type: MessageType::Text,
        tool_call_id: None,
        tool_name: None,
        tool_input: None,
        tool_error: false,
    };
    db.with_conn(|conn| db::messages::create(conn, &message))?;
    db.with_conn(|conn| db::conversations::touch(conn, conversation_id))?;
    Ok(message)
}

fn insert_assistant_message(
    db: &crate::db::Database,
    conversation_id: &str,
    message_id: &str,
    content: &str,
    model_id: &str,
) -> AppResult<Message> {
    let message = Message {
        id: message_id.to_string(),
        conversation_id: conversation_id.to_string(),
        role: Role::Assistant,
        content: content.to_string(),
        reasoning: None,
        model_id: Some(model_id.to_string()),
        status: MessageStatus::Done,
        token_count: None,
        created_at: crate::commands::chat::chrono_now(),
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
        message_type: MessageType::Text,
        tool_call_id: None,
        tool_name: None,
        tool_input: None,
        tool_error: false,
    };
    db.with_conn(|conn| db::messages::create(conn, &message))?;
    db.with_conn(|conn| db::conversations::touch(conn, conversation_id))?;
    Ok(message)
}

fn extract_final_text(messages: &[AgentMessage]) -> String {
    messages
        .iter()
        .rev()
        .find_map(|message| match message {
            AgentMessage::Llm(AgentLlmMessage::Assistant { content, .. }) => Some(
                content
                    .iter()
                    .filter_map(|item| match item {
                        Content::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn watch_to_cancellation_token(
    mut rx: tokio::sync::watch::Receiver<bool>,
) -> tokio_util::sync::CancellationToken {
    let token = tokio_util::sync::CancellationToken::new();
    let child = token.clone();
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            if *rx.borrow() {
                child.cancel();
                break;
            }
        }
    });
    token
}
