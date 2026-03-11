use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::paste_storage;
use crate::providers::Provider;
use crate::state::AppState;

/// Resolve provider instance and type from a model_id.
async fn resolve_provider(
    state: &AppState,
    model_id: &str,
) -> AppResult<(Arc<dyn Provider>, String)> {
    let provider_id = state.db.with_conn(|conn| {
        let pid: String = conn
            .query_row(
                "SELECT provider_id FROM models WHERE id = ?1",
                [model_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::NotFound(format!("Model {model_id}")))?;
        Ok(pid)
    })?;

    let provider = state
        .get_provider(&provider_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Provider {provider_id}")))?;

    let provider_type: String = state.db.with_conn(|conn| {
        let pt: String = conn
            .query_row(
                "SELECT type FROM providers WHERE id = ?1",
                [&provider_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::NotFound(format!("Provider {provider_id}")))?;
        Ok(pt)
    })?;

    Ok((provider, provider_type))
}

/// Build default ProviderParams from provider type string.
fn default_provider_params(provider_type: &str) -> ProviderParams {
    match provider_type {
        "anthropic" => ProviderParams::Anthropic {
            top_k: None,
            thinking: None,
            effort: None,
        },
        "gemini" => ProviderParams::Gemini {
            thinking_budget: None,
            thinking_level: None,
        },
        "ollama" => ProviderParams::Ollama {
            think: None,
            num_ctx: None,
            repeat_penalty: None,
            min_p: None,
            keep_alive: None,
        },
        _ => ProviderParams::OpenaiCompat {
            frequency_penalty: None,
            presence_penalty: None,
            reasoning_effort: None,
            seed: None,
            max_completion_tokens: None,
        },
    }
}

fn strip_paste_markers(text: &str) -> String {
    paste_storage::expand_legacy_inline_pastes(text)
}

fn resolve_paste_blob_path(state: &AppState, paste_id: &str) -> AppResult<String> {
    state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
}

fn expand_content_for_model(state: &AppState, text: &str) -> AppResult<String> {
    let legacy_expanded = strip_paste_markers(text);
    paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &legacy_expanded, &|paste_id| {
        resolve_paste_blob_path(state, paste_id)
    })
}

fn persist_external_pastes(
    state: &AppState,
    conversation_id: &str,
    message_id: &str,
    content: &str,
    created_at: &str,
) -> AppResult<String> {
    paste_storage::externalize_legacy_inline_pastes(content, |text, _count| {
        let paste_id = uuid::Uuid::new_v4().to_string();
        let persisted = paste_storage::persist_paste_blob(&state.data_dir, &paste_id, text)?;
        state.db.with_conn(|conn| {
            db::paste_blobs::create(
                conn,
                &paste_id,
                conversation_id,
                message_id,
                persisted.char_count,
                &persisted.file_path,
                text,
                created_at,
            )
        })?;
        Ok(format!("<<paste-ref:{paste_id}:{}>>", persisted.char_count))
    })
}

fn build_request_messages(
    history: &[Message],
    assistant_prompt: Option<&str>,
) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    if let Some(prompt) = assistant_prompt.map(str::trim).filter(|prompt| !prompt.is_empty()) {
        messages.push(ChatMessage {
            role: Role::System,
            content: prompt.to_string(),
        });
    }

    messages.extend(
        history
            .iter()
            .filter(|m| m.status != MessageStatus::Error && m.status != MessageStatus::Streaming)
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }),
    );

    messages
}

fn load_assistant_system_prompt(
    state: &AppState,
    conversation_id: &str,
) -> AppResult<Option<String>> {
    state.db.with_conn(|conn| {
        let conversation = db::conversations::get(conn, conversation_id)?;
        let Some(assistant_id) = conversation.assistant_id else {
            return Ok(None);
        };

        let assistant = db::assistants::get(conn, &assistant_id)?;
        Ok(assistant.system_prompt)
    })
}

/// Core streaming logic: send events, stream, persist result.
/// The message must already exist in DB. `history` is the conversation context.
async fn run_stream(
    state: &AppState,
    provider: Arc<dyn Provider>,
    provider_type: &str,
    model_id: &str,
    msg_id: &str,
    conversation_id: &str,
    history: Vec<Message>,
    channel: &Channel<ChatEvent>,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> AppResult<Message> {
    let assistant_prompt = load_assistant_system_prompt(state, conversation_id)?;
    let expanded_history = history
        .iter()
        .map(|message| -> AppResult<Message> {
            let mut cloned = message.clone();
            cloned.content = expand_content_for_model(state, &message.content)?;
            Ok(cloned)
        })
        .collect::<AppResult<Vec<_>>>()?;
    let messages = build_request_messages(&expanded_history, assistant_prompt.as_deref());

    let mut common = common_params.unwrap_or(CommonParams {
        temperature: None,
        top_p: None,
        max_tokens: None,
        stream: true,
    });
    common.stream = true;

    let request = ChatRequest {
        model: model_id.to_string(),
        messages,
        common,
        provider_params: provider_params.unwrap_or_else(|| default_provider_params(provider_type)),
    };

    let _ = channel.send(ChatEvent::Started {
        message_id: msg_id.to_string(),
    });

    let _ = state.cancel_sender.send(false);
    let cancel_rx = state.cancel_receiver.clone();

    let result = provider
        .stream_chat(request, msg_id.to_string(), channel.clone(), cancel_rx)
        .await;

    match result {
        Ok(stream_result) => {
            let _ = state.db.with_conn(|conn| {
                db::messages::update_content(
                    conn,
                    msg_id,
                    &stream_result.content,
                    stream_result.reasoning.as_deref(),
                    Some(stream_result.prompt_tokens),
                    Some(stream_result.completion_tokens),
                )
            });
            let _ = channel.send(ChatEvent::Finished {
                message_id: msg_id.to_string(),
            });
        }
        Err(AppError::Cancelled) => {
            let _ = channel.send(ChatEvent::Finished {
                message_id: msg_id.to_string(),
            });
            let _ = state.db.with_conn(|conn| {
                db::messages::update_content(conn, msg_id, "", None, None, None)
            });
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = channel.send(ChatEvent::Error {
                message_id: msg_id.to_string(),
                message: err_msg.clone(),
            });
            let _ = state
                .db
                .with_conn(|conn| db::messages::set_error(conn, msg_id, &err_msg));
        }
    }

    let _ = state
        .db
        .with_conn(|conn| db::conversations::touch(conn, conversation_id));

    let final_msg = state
        .db
        .with_conn(|conn| db::messages::get(conn, msg_id))?;
    Ok(final_msg)
}

/// Create assistant placeholder and stream response.
async fn do_stream(
    state: &AppState,
    provider: Arc<dyn Provider>,
    provider_type: &str,
    conversation_id: &str,
    model_id: &str,
    channel: &Channel<ChatEvent>,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> AppResult<Message> {
    let now = chrono_now();

    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    let assistant_msg = Message {
        id: assistant_msg_id.clone(),
        conversation_id: conversation_id.to_string(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        model_id: Some(model_id.to_string()),
        status: MessageStatus::Streaming,
        token_count: None,
        created_at: now,
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
    };
    state
        .db
        .with_conn(|conn| db::messages::create(conn, &assistant_msg))?;

    let history = state
        .db
        .with_conn(|conn| db::messages::list_by_conversation(conn, conversation_id))?;

    run_stream(
        state,
        provider,
        provider_type,
        model_id,
        &assistant_msg_id,
        conversation_id,
        history,
        channel,
        common_params,
        provider_params,
    )
    .await
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    content: String,
    model_id: String,
    channel: Channel<ChatEvent>,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> AppResult<Message> {
    let (provider, provider_type) = resolve_provider(&state, &model_id).await?;

    let now = chrono_now();

    let user_message_id = uuid::Uuid::new_v4().to_string();
    let user_msg = Message {
        id: user_message_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::User,
        content: content.clone(),
        reasoning: None,
        model_id: None,
        status: MessageStatus::Done,
        token_count: None,
        created_at: now.clone(),
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
    };
    state
        .db
        .with_conn(|conn| db::messages::create(conn, &user_msg))?;

    let persisted_content = persist_external_pastes(&state, &conversation_id, &user_message_id, &content, &now)?;
    if persisted_content != content {
        state
            .db
            .with_conn(|conn| db::messages::update_text(conn, &user_message_id, &persisted_content))?;
    }

    do_stream(&state, provider, &provider_type, &conversation_id, &model_id, &channel, common_params, provider_params).await
}

#[tauri::command]
pub async fn resend_message(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    model_id: String,
    channel: Channel<ChatEvent>,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> AppResult<Message> {
    let (provider, provider_type) = resolve_provider(&state, &model_id).await?;

    do_stream(&state, provider, &provider_type, &conversation_id, &model_id, &channel, common_params, provider_params).await
}

/// Generate a new version of an AI response (+1 button).
#[tauri::command]
pub async fn generate_version(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    message_id: String,
    model_id: String,
    channel: Channel<ChatEvent>,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> AppResult<Message> {
    let (provider, provider_type) = resolve_provider(&state, &model_id).await?;

    // 1. Initialize version group if needed
    state
        .db
        .with_conn(|conn| db::messages::init_version_group(conn, &message_id))?;

    // The version_group_id is the original message's ID
    let version_group_id: String = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT COALESCE(version_group_id, id) FROM messages WHERE id = ?1",
            [&message_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound(format!("Message {message_id}")))
    })?;

    // 2. Check version limit
    let next_version = state
        .db
        .with_conn(|conn| db::messages::next_version_number(conn, &version_group_id))?;
    if next_version > 50 {
        return Err(AppError::Provider("Version limit (50) reached".into()));
    }

    // 3. Deactivate all versions
    state
        .db
        .with_conn(|conn| db::messages::deactivate_versions(conn, &version_group_id))?;

    // 4. Get created_at of v1 for consistent ordering
    let created_at: String = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT created_at FROM messages WHERE id = ?1",
            [&version_group_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound("Version group".into()))
    })?;

    // 5. Create new version message
    let new_msg_id = uuid::Uuid::new_v4().to_string();
    let new_msg = Message {
        id: new_msg_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        model_id: Some(model_id.clone()),
        status: MessageStatus::Streaming,
        token_count: None,
        created_at,
        version_group_id: Some(version_group_id.clone()),
        version_number: next_version,
        total_versions: 0,
    };
    state
        .db
        .with_conn(|conn| db::messages::create(conn, &new_msg))?;

    // 6. Delete messages after this version group
    state.db.with_conn(|conn| {
        db::messages::delete_after_version_group(conn, &conversation_id, &version_group_id)
    })?;

    // 7. Load history before the version group
    let history = state
        .db
        .with_conn(|conn| db::messages::list_before_message(conn, &conversation_id, &version_group_id))?;

    // 8. Stream
    run_stream(
        &state,
        provider,
        &provider_type,
        &model_id,
        &new_msg_id,
        &conversation_id,
        history,
        &channel,
        common_params,
        provider_params,
    )
    .await
}

/// Regenerate an existing message in-place (for versioned or non-versioned messages).
#[tauri::command]
pub async fn regenerate_message(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    message_id: String,
    model_id: String,
    channel: Channel<ChatEvent>,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> AppResult<Message> {
    let (provider, provider_type) = resolve_provider(&state, &model_id).await?;

    // Get version info
    let version_group_id: Option<String> = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT version_group_id FROM messages WHERE id = ?1",
            [&message_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound(format!("Message {message_id}")))
    })?;

    // Delete subsequent messages
    if let Some(ref gid) = version_group_id {
        state.db.with_conn(|conn| {
            db::messages::delete_after_version_group(conn, &conversation_id, gid)
        })?;
    } else {
        state
            .db
            .with_conn(|conn| db::messages::delete_after(conn, &conversation_id, &message_id))?;
    }

    // Clear message for regeneration
    state
        .db
        .with_conn(|conn| db::messages::clear_for_regenerate(conn, &message_id))?;

    // Load history before this message (or version group)
    let before_id = version_group_id.as_deref().unwrap_or(&message_id);
    let history = state
        .db
        .with_conn(|conn| db::messages::list_before_message(conn, &conversation_id, before_id))?;

    run_stream(
        &state,
        provider,
        &provider_type,
        &model_id,
        &message_id,
        &conversation_id,
        history,
        &channel,
        common_params,
        provider_params,
    )
    .await
}

/// Compress conversation: summarize all messages, replace with a single summary message.
#[tauri::command]
pub async fn compress_conversation(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    model_id: String,
    channel: Channel<ChatEvent>,
) -> AppResult<Vec<Message>> {
    let (provider, provider_type) = resolve_provider(&state, &model_id).await?;

    // Load all messages
    let history = state
        .db
        .with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;

    if history.is_empty() {
        return Ok(vec![]);
    }

    // Build summarization request
    let mut chat_messages = vec![ChatMessage {
        role: Role::System,
        content: "You are a conversation summarizer. Summarize the following conversation concisely but comprehensively. Include all key topics, decisions, code snippets, and important details. The summary should allow the conversation to continue naturally. Respond with the summary only, in the same language as the conversation.".to_string(),
    }];

    for msg in &history {
        if msg.status != MessageStatus::Error {
            chat_messages.push(ChatMessage {
                role: msg.role.clone(),
                content: expand_content_for_model(&state, &msg.content)?,
            });
        }
    }

    let request = ChatRequest {
        model: model_id.to_string(),
        messages: chat_messages,
        common: CommonParams {
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: true,
        },
        provider_params: default_provider_params(&provider_type),
    };

    let _ = channel.send(ChatEvent::Started {
        message_id: "compress".to_string(),
    });

    let _ = state.cancel_sender.send(false);
    let cancel_rx = state.cancel_receiver.clone();

    let result = provider
        .stream_chat(request, "compress".to_string(), channel.clone(), cancel_rx)
        .await;

    match result {
        Ok(stream_result) => {
            // Delete all messages in conversation (hard delete)
            state.db.with_conn(|conn| {
                conn.execute(
                    "DELETE FROM messages WHERE conversation_id = ?1",
                    [&conversation_id],
                )
                .map_err(AppError::Database)?;
                Ok(())
            })?;

            // Create summary assistant message
            let now = chrono_now();
            let summary_msg = Message {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: conversation_id.clone(),
                role: Role::Assistant,
                content: stream_result.content,
                reasoning: None,
                model_id: Some(model_id),
                status: MessageStatus::Done,
                token_count: Some(stream_result.prompt_tokens + stream_result.completion_tokens),
                created_at: now,
                version_group_id: None,
                version_number: 1,
                total_versions: 1,
            };
            state
                .db
                .with_conn(|conn| db::messages::create(conn, &summary_msg))?;

            let _ = state
                .db
                .with_conn(|conn| db::conversations::touch(conn, &conversation_id));

            let _ = channel.send(ChatEvent::Finished {
                message_id: "compress".to_string(),
            });

            let messages = state
                .db
                .with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;
            Ok(messages)
        }
        Err(e) => {
            let _ = channel.send(ChatEvent::Error {
                message_id: "compress".to_string(),
                message: e.to_string(),
            });
            Err(e)
        }
    }
}

/// Send a message to multiple models simultaneously (group/combo send).
/// Creates one user message + N assistant version messages, streams all in parallel.
#[tauri::command]
pub async fn send_message_group(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    content: String,
    model_ids: Vec<String>,
    channel: Channel<ChatEvent>,
) -> AppResult<Vec<Message>> {
    if model_ids.is_empty() {
        return Err(AppError::Provider("No models specified".into()));
    }

    // 1. Resolve all providers upfront (fail fast if any model is invalid)
    let mut resolved: Vec<(Arc<dyn Provider>, String, String)> = Vec::new(); // (provider, provider_type, model_id)
    for mid in &model_ids {
        let (provider, provider_type) = resolve_provider(&state, mid).await?;
        resolved.push((provider, provider_type, mid.clone()));
    }

    let now = chrono_now();

    // 2. Create user message
    let user_message_id = uuid::Uuid::new_v4().to_string();
    let user_msg = Message {
        id: user_message_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::User,
        content: content.clone(),
        reasoning: None,
        model_id: None,
        status: MessageStatus::Done,
        token_count: None,
        created_at: now.clone(),
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
    };
    state
        .db
        .with_conn(|conn| db::messages::create(conn, &user_msg))?;

    // Persist external pastes
    let persisted_content =
        persist_external_pastes(&state, &conversation_id, &user_message_id, &content, &now)?;
    if persisted_content != content {
        state.db.with_conn(|conn| {
            db::messages::update_text(conn, &user_message_id, &persisted_content)
        })?;
    }

    // 3. Create assistant messages as versions
    let mut assistant_msg_ids: Vec<String> = Vec::new();
    let first_msg_id = uuid::Uuid::new_v4().to_string();

    // First message: is_active=1, no version_group_id yet
    let first_msg = Message {
        id: first_msg_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        model_id: Some(model_ids[0].clone()),
        status: MessageStatus::Streaming,
        token_count: None,
        created_at: now.clone(),
        version_group_id: None,
        version_number: 1,
        total_versions: model_ids.len() as u32,
    };
    state
        .db
        .with_conn(|conn| db::messages::create(conn, &first_msg))?;
    assistant_msg_ids.push(first_msg_id.clone());

    // Initialize version group on first message
    state
        .db
        .with_conn(|conn| db::messages::init_version_group(conn, &first_msg_id))?;

    // Create remaining version messages
    for (idx, mid) in model_ids.iter().enumerate().skip(1) {
        let msg_id = uuid::Uuid::new_v4().to_string();
        let msg = Message {
            id: msg_id.clone(),
            conversation_id: conversation_id.clone(),
            role: Role::Assistant,
            content: String::new(),
            reasoning: None,
            model_id: Some(mid.clone()),
            status: MessageStatus::Streaming,
            token_count: None,
            created_at: now.clone(),
            version_group_id: Some(first_msg_id.clone()),
            version_number: (idx + 1) as u32,
            total_versions: model_ids.len() as u32,
        };
        state
            .db
            .with_conn(|conn| db::messages::create_version(conn, &msg, false))?;
        assistant_msg_ids.push(msg_id);
    }

    // 4. Load conversation history once
    let history = state
        .db
        .with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;

    // 5. Reset cancel once
    let _ = state.cancel_sender.send(false);

    // 6. Load params for all models
    let assistant_prompt = load_assistant_system_prompt(&state, &conversation_id)?;
    let expanded_history = history
        .iter()
        .map(|message| -> AppResult<Message> {
            let mut cloned = message.clone();
            cloned.content = expand_content_for_model(&state, &message.content)?;
            Ok(cloned)
        })
        .collect::<AppResult<Vec<_>>>()?;
    let request_messages = build_request_messages(&expanded_history, assistant_prompt.as_deref());

    // 7. Spawn concurrent tasks via JoinSet
    let mut join_set = tokio::task::JoinSet::new();
    let first_finished = std::sync::Arc::new(std::sync::Mutex::new(Option::<String>::None));

    for (i, (provider, provider_type, model_id)) in resolved.into_iter().enumerate() {
        let msg_id = assistant_msg_ids[i].clone();
        let state_clone = state.inner().clone();
        let channel_clone = channel.clone();
        let conv_id = conversation_id.clone();
        let request_messages = request_messages.clone();
        let first_finished = first_finished.clone();

        let common = CommonParams {
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: true,
        };

        let provider_params = default_provider_params(&provider_type);

        join_set.spawn(async move {
            let request = ChatRequest {
                model: model_id.clone(),
                messages: request_messages,
                common,
                provider_params,
            };

            let _ = channel_clone.send(ChatEvent::Started {
                message_id: msg_id.clone(),
            });

            let cancel_rx = state_clone.cancel_receiver.clone();

            let result = provider
                .stream_chat(request, msg_id.clone(), channel_clone.clone(), cancel_rx)
                .await;

            match result {
                Ok(stream_result) => {
                    let _ = state_clone.db.with_conn(|conn| {
                        db::messages::update_content(
                            conn,
                            &msg_id,
                            &stream_result.content,
                            stream_result.reasoning.as_deref(),
                            Some(stream_result.prompt_tokens),
                            Some(stream_result.completion_tokens),
                        )
                    });
                    let _ = channel_clone.send(ChatEvent::Finished {
                        message_id: msg_id.clone(),
                    });

                    // Track first to finish
                    let mut lock = first_finished.lock().unwrap();
                    if lock.is_none() {
                        *lock = Some(msg_id.clone());
                    }
                }
                Err(AppError::Cancelled) => {
                    let _ = channel_clone.send(ChatEvent::Finished {
                        message_id: msg_id.clone(),
                    });
                    let _ = state_clone.db.with_conn(|conn| {
                        db::messages::update_content(conn, &msg_id, "", None, None, None)
                    });
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    let _ = channel_clone.send(ChatEvent::Error {
                        message_id: msg_id.clone(),
                        message: err_msg.clone(),
                    });
                    let _ = state_clone
                        .db
                        .with_conn(|conn| db::messages::set_error(conn, &msg_id, &err_msg));
                }
            }

            let _ = state_clone
                .db
                .with_conn(|conn| db::conversations::touch(conn, &conv_id));
        });
    }

    // 8. Wait for all tasks to complete
    while let Some(_) = join_set.join_next().await {}

    // 9. Activate the first version that finished successfully
    if let Some(winner_id) = first_finished.lock().unwrap().as_ref() {
        // Find its version_number
        let winner = state.db.with_conn(|conn| db::messages::get(conn, winner_id));
        if let Ok(msg) = winner {
            if msg.status == MessageStatus::Done {
                let _ = state.db.with_conn(|conn| {
                    db::messages::switch_active_version(
                        conn,
                        &first_msg_id,
                        msg.version_number,
                    )
                });
            }
        }
    }

    // 10. Return all final messages (versions)
    let all_versions = state
        .db
        .with_conn(|conn| db::messages::list_versions(conn, &first_msg_id))?;
    Ok(all_versions)
}

#[tauri::command]
pub async fn stop_generation(state: State<'_, Arc<AppState>>) -> AppResult<()> {
    let _ = state.cancel_sender.send(true);
    Ok(())
}

pub(crate) fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let secs_per_day = 86400u64;
    let days = now / secs_per_day;
    let rem = now % secs_per_day;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;

    let mut y = 1970i64;
    let mut d = days as i64;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if d < year_days {
            break;
        }
        d -= year_days;
        y += 1;
    }
    let month_days: [i64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0usize;
    for (i, &md) in month_days.iter().enumerate() {
        if d < md {
            m = i;
            break;
        }
        d -= md;
    }
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        y,
        m + 1,
        d + 1,
        hours,
        minutes,
        seconds
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(role: Role, content: &str, status: MessageStatus) -> Message {
        Message {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: "conv-1".into(),
            role,
            content: content.into(),
            reasoning: None,
            model_id: None,
            status,
            token_count: None,
            created_at: "2025-01-01T00:00:00".into(),
            version_group_id: None,
            version_number: 1,
            total_versions: 1,
        }
    }

    #[test]
    fn test_build_request_messages_prepends_assistant_prompt() {
        let history = vec![
            make_message(Role::User, "hi", MessageStatus::Done),
            make_message(Role::Assistant, "hello", MessageStatus::Done),
        ];

        let messages = build_request_messages(&history, Some("You are a coding assistant."));

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[0].content, "You are a coding assistant.");
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_build_request_messages_skips_blank_prompt_and_invalid_status() {
        let history = vec![
            make_message(Role::User, "hi", MessageStatus::Done),
            make_message(Role::Assistant, "streaming", MessageStatus::Streaming),
            make_message(Role::Assistant, "oops", MessageStatus::Error),
        ];

        let messages = build_request_messages(&history, Some("   "));

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[0].content, "hi");
    }

    #[test]
    fn test_build_request_messages_keeps_summary_after_assistant_prompt() {
        let history = vec![
            make_message(Role::Assistant, "summary: we discussed refactoring", MessageStatus::Done),
            make_message(Role::User, "continue from that plan", MessageStatus::Done),
        ];

        let messages = build_request_messages(&history, Some("You are Orion assistant."));

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[0].content, "You are Orion assistant.");
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[1].content, "summary: we discussed refactoring");
        assert_eq!(messages[2].role, Role::User);
        assert_eq!(messages[2].content, "continue from that plan");
    }
}
