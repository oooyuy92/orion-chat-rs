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
use crate::agent::mcp;
use crate::agent::permissions::{ChatEventEmitter, PermissionedTool};
use crate::agent::skills::{self, SkillInfo};
use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{
    AuthAction,
    ChatEvent,
    McpServerConfig,
    McpServerStatus,
    Message,
    MessageStatus,
    MessageType,
    Role,
    ToolPermissions,
};
use crate::state::AppState;

#[derive(Debug, PartialEq)]
struct AssistantOutcome {
    status: MessageStatus,
    content: String,
    emit_finished: bool,
}

#[tauri::command]
pub async fn agent_chat(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    message: String,
    model_id: String,
    channel: Channel<ChatEvent>,
) -> AppResult<Message> {
    let app_state = Arc::clone(state.inner());
    let conv = app_state
        .db
        .with_conn(|conn| db::conversations::get(conn, &conversation_id))?;
    let working_dirs = conv.working_dirs;

    let provider_config = config::build_provider_config(&app_state.db, &model_id).await?;
    let permissions = load_tool_permissions(&app_state)?;
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
            &working_dirs,
        )
        .await?,
    };

    if working_dirs.is_empty() {
        context.system_prompt.push_str(
            "\n\n注意：当前未设置工作目录，文件操作功能不可用。请用户先设置工作目录。"
        );
    }

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

    let outcome = final_assistant_outcome(&new_messages).ok_or(AppError::Cancelled)?;
    let assistant_message = insert_assistant_message(
        &app_state.db,
        &conversation_id,
        &assistant_message_id,
        &outcome.content,
        &model_id,
        outcome.status,
    )?;

    if outcome.emit_finished {
        emit_event(ChatEvent::Finished {
            message_id: assistant_message_id,
        });
    }

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

#[tauri::command]
pub async fn get_skills_dir(state: State<'_, Arc<AppState>>) -> AppResult<String> {
    state.db.with_conn(|conn| {
        Ok(conn
            .query_row(
                "SELECT value FROM agent_settings WHERE key = 'skills_dir'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default())
    })
}

#[tauri::command]
pub async fn set_skills_dir(state: State<'_, Arc<AppState>>, dir: String) -> AppResult<()> {
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO agent_settings (key, value) VALUES ('skills_dir', ?1)",
            rusqlite::params![dir],
        )?;
        Ok(())
    })
}

#[tauri::command]
pub async fn get_conversation_working_dirs(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<Vec<String>> {
    state.db.with_conn(|conn| {
        let conv = db::conversations::get(conn, &conversation_id)?;
        Ok(conv.working_dirs)
    })
}

#[tauri::command]
pub async fn set_conversation_working_dirs(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    dirs: Vec<String>,
) -> AppResult<()> {
    let dirs: Vec<String> = dirs
        .into_iter()
        .map(|d| d.trim_end_matches('/').to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    state.db.with_conn(|conn| db::conversations::set_working_dirs(conn, &conversation_id, &dirs))
}

#[tauri::command]
pub async fn scan_skills(state: State<'_, Arc<AppState>>) -> AppResult<Vec<SkillInfo>> {
    let dir = state.db.with_conn(|conn| {
        Ok(conn
            .query_row(
                "SELECT value FROM agent_settings WHERE key = 'skills_dir'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_default())
    })?;

    if dir.is_empty() {
        return Ok(vec![]);
    }

    skills::scan_skills_dir(&dir)
}

#[tauri::command]
pub async fn add_mcp_server(
    state: State<'_, Arc<AppState>>,
    config: McpServerConfig,
) -> AppResult<Vec<String>> {
    let mut configs = load_mcp_server_configs(&state)?;
    if configs.iter().any(|existing| existing.name == config.name) {
        return Err(AppError::Mcp(format!(
            "MCP server {} already exists",
            config.name
        )));
    }

    configs.push(config.clone());
    save_mcp_server_configs(&state, &configs)?;

    mcp::connect_server(&state, config).await
}

#[tauri::command]
pub async fn remove_mcp_server(
    state: State<'_, Arc<AppState>>,
    name: String,
) -> AppResult<()> {
    let mut configs = load_mcp_server_configs(&state)?;
    if !configs.iter().any(|config| config.name == name) {
        return Err(AppError::NotFound(format!("MCP server {name}")));
    }

    mcp::disconnect_server(&state, &name).await?;

    configs.retain(|config| config.name != name);
    save_mcp_server_configs(&state, &configs)
}

#[tauri::command]
pub async fn list_mcp_servers(
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<McpServerStatus>> {
    let configs = load_mcp_server_configs(&state)?;
    Ok(mcp::get_server_statuses(&state, &configs).await)
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

fn load_mcp_server_configs(state: &AppState) -> AppResult<Vec<McpServerConfig>> {
    let configs_json = state.db.with_conn(|conn| {
        Ok(conn
            .query_row(
                "SELECT value FROM agent_settings WHERE key = 'mcp_servers'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "[]".to_string()))
    })?;

    serde_json::from_str(&configs_json).map_err(Into::into)
}

fn save_mcp_server_configs(state: &AppState, configs: &[McpServerConfig]) -> AppResult<()> {
    let configs_json = serde_json::to_string(configs)?;
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO agent_settings (key, value) VALUES ('mcp_servers', ?1)",
            [configs_json],
        )?;
        Ok(())
    })
}

async fn build_tools(
    state: Arc<AppState>,
    conversation_id: &str,
    permissions: ToolPermissions,
    emit_event: ChatEventEmitter,
    working_dirs: &[String],
) -> AppResult<Vec<Box<dyn yoagent::AgentTool>>> {
    let make_tool = |tool: Box<dyn yoagent::AgentTool>| {
        Box::new(PermissionedTool::new(
            tool,
            state.clone(),
            conversation_id.to_string(),
            permissions.clone(),
            emit_event.clone(),
        )) as Box<dyn yoagent::AgentTool>
    };

    let bash_cwd = if let Some(first) = working_dirs.first() {
        first.clone()
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/"))
            .display()
            .to_string()
    };

    let mut tools = vec![
        make_tool(Box::new(BashTool::new().with_cwd(bash_cwd))),
    ];

    if !working_dirs.is_empty() {
        let paths: Vec<String> = working_dirs.to_vec();
        tools.extend(vec![
            make_tool(Box::new(ReadFileTool::new().with_allowed_paths(paths.clone()))),
            make_tool(Box::new(WriteFileTool::new().with_allowed_paths(paths.clone()))),
            make_tool(Box::new(EditFileTool::new().with_allowed_paths(paths.clone()))),
            make_tool(Box::new(ListFilesTool::new().with_root(working_dirs[0].clone()))),
            make_tool(Box::new(SearchTool::new().with_root(working_dirs[0].clone()))),
        ]);
    }

    let mcp_tools = mcp::get_mcp_tools(&state).await?;
    tools.extend(mcp_tools);

    Ok(tools)
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
    status: MessageStatus,
) -> AppResult<Message> {
    let message = Message {
        id: message_id.to_string(),
        conversation_id: conversation_id.to_string(),
        role: Role::Assistant,
        content: content.to_string(),
        reasoning: None,
        model_id: Some(model_id.to_string()),
        status,
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

fn final_assistant_outcome(messages: &[AgentMessage]) -> Option<AssistantOutcome> {
    messages
        .iter()
        .rev()
        .find_map(|message| match message {
            AgentMessage::Llm(AgentLlmMessage::Assistant {
                content,
                stop_reason,
                error_message,
                ..
            }) => {
                let text = content
                    .iter()
                    .filter_map(|item| match item {
                        Content::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                match stop_reason {
                    StopReason::Stop => Some(AssistantOutcome {
                        status: MessageStatus::Done,
                        content: text,
                        emit_finished: true,
                    }),
                    StopReason::Error => Some(AssistantOutcome {
                        status: MessageStatus::Error,
                        content: error_message
                            .clone()
                            .filter(|message| !message.trim().is_empty())
                            .unwrap_or_else(|| {
                                if text.trim().is_empty() {
                                    "Agent error".to_string()
                                } else {
                                    text
                                }
                            }),
                        emit_finished: false,
                    }),
                    StopReason::Aborted => None,
                    _ => Some(AssistantOutcome {
                        status: MessageStatus::Done,
                        content: text,
                        emit_finished: false,
                    }),
                }
            }
            _ => None,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    fn assistant_message(
        stop_reason: StopReason,
        text: &str,
        error_message: Option<&str>,
    ) -> AgentMessage {
        AgentMessage::Llm(AgentLlmMessage::Assistant {
            content: vec![Content::Text {
                text: text.to_string(),
            }],
            stop_reason,
            model: "test-model".into(),
            provider: "test-provider".into(),
            usage: Usage::default(),
            timestamp: now_ms(),
            error_message: error_message.map(str::to_string),
        })
    }

    fn test_ctx(name: &str) -> yoagent::ToolContext {
        yoagent::ToolContext {
            tool_call_id: "tool-call-1".into(),
            tool_name: name.into(),
            cancel: CancellationToken::new(),
            on_update: None,
            on_progress: None,
        }
    }

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orion-agent-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_agent_run_error_uses_error_status_and_no_finished_event() {
        let outcome = final_assistant_outcome(&[assistant_message(
            StopReason::Error,
            "",
            Some("provider exploded"),
        )]);

        assert_eq!(
            outcome,
            Some(AssistantOutcome {
                status: MessageStatus::Error,
                content: "provider exploded".into(),
                emit_finished: false,
            })
        );
    }

    #[test]
    fn test_agent_run_aborted_without_content_persists_nothing() {
        let outcome = final_assistant_outcome(&[assistant_message(
            StopReason::Aborted,
            "",
            None,
        )]);

        assert_eq!(outcome, None);
    }

    #[tokio::test]
    async fn test_build_tools_scopes_read_file_to_working_dir() {
        let state = Arc::new(AppState::new(":memory:", "/tmp").unwrap());
        let working_dir = unique_temp_dir("working-dir");
        let outside_dir = unique_temp_dir("outside-dir");
        let outside_file = outside_dir.join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();

        let tools = build_tools(
            state,
            "conv-1",
            ToolPermissions::with_defaults(),
            Arc::new(|_| {}),
            &[working_dir.display().to_string()],
        )
        .await
        .unwrap();
        let read_tool = tools
            .into_iter()
            .find(|tool| tool.name() == "read_file")
            .unwrap();

        let result = read_tool
            .execute(
                serde_json::json!({ "path": outside_file.display().to_string() }),
                test_ctx("read_file"),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("outside"));

        let _ = std::fs::remove_dir_all(working_dir);
        let _ = std::fs::remove_dir_all(outside_dir);
    }

    #[tokio::test]
    async fn test_build_tools_scopes_list_files_to_working_dir() {
        let state = Arc::new(AppState::new(":memory:", "/tmp").unwrap());
        let working_dir = unique_temp_dir("working-dir-list");
        let outside_dir = unique_temp_dir("outside-dir-list");
        std::fs::write(outside_dir.join("secret.txt"), "secret").unwrap();

        let tools = build_tools(
            state,
            "conv-1",
            ToolPermissions::with_defaults(),
            Arc::new(|_| {}),
            &[working_dir.display().to_string()],
        )
        .await
        .unwrap();
        let list_tool = tools
            .into_iter()
            .find(|tool| tool.name() == "list_files")
            .unwrap();

        let result = list_tool
            .execute(
                serde_json::json!({ "path": outside_dir.display().to_string() }),
                test_ctx("list_files"),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("outside"));

        let _ = std::fs::remove_dir_all(working_dir);
        let _ = std::fs::remove_dir_all(outside_dir);
    }

    #[tokio::test]
    async fn test_build_tools_empty_working_dirs_excludes_file_tools() {
        let state = Arc::new(AppState::new(":memory:", "/tmp").unwrap());
        let tools = build_tools(
            state,
            "conv-1",
            ToolPermissions::with_defaults(),
            Arc::new(|_| {}),
            &[],
        )
        .await
        .unwrap();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();

        assert!(tool_names.contains(&"bash"));
        assert!(!tool_names.contains(&"read_file"));
        assert!(!tool_names.contains(&"write_file"));
        assert!(!tool_names.contains(&"edit_file"));
        assert!(!tool_names.contains(&"list_files"));
        assert!(!tool_names.contains(&"search"));
    }
}
