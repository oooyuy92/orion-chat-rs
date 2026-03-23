use std::sync::Arc;
use tokio::sync::watch;

use crate::channel::ChatEventSender;
use crate::db;
use crate::error::AppError;
use crate::models::*;
use crate::providers::Provider;
use crate::state::{AppState, StreamChunk};

/// Spawn a background generation task that runs independently of SSE connections
pub async fn spawn_generation_task(
    state: Arc<AppState>,
    conversation_id: String,
    message_id: String,
    model_id: String,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> Result<(), AppError> {
    // Create progress channel for SSE subscription
    let (progress_tx, _) = watch::channel(StreamChunk::Content {
        content: String::new(),
    });

    let state_clone = state.clone();
    let progress_tx_clone = progress_tx.clone();
    let message_id_clone = message_id.clone();
    let conversation_id_clone = conversation_id.clone();

    // Spawn independent background task
    let task_handle = tokio::spawn(async move {
        if let Err(e) = run_generation_task(
            state_clone,
            conversation_id_clone,
            message_id_clone,
            model_id,
            common_params,
            provider_params,
            progress_tx_clone,
        )
        .await
        {
            eprintln!("Generation task error: {}", e);
        }
    });

    // Register task
    let now = chrono::Utc::now().to_rfc3339();
    state
        .register_generation_task(
            message_id.clone(),
            conversation_id,
            task_handle,
            progress_tx,
            now,
        )
        .await;

    Ok(())
}

/// Core generation logic that runs in background
async fn run_generation_task(
    state: Arc<AppState>,
    conversation_id: String,
    message_id: String,
    model_id: String,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
    progress_tx: watch::Sender<StreamChunk>,
) -> Result<(), AppError> {
    // 1. Resolve provider from model_id
    let (provider, _provider_type, request_model) = resolve_provider(&state, &model_id).await?;

    // 2. Load conversation history
    let history = state.db.with_conn(|conn| {
        db::messages::list_by_conversation(conn, &conversation_id)
    })?;

    // 3. Build request messages
    let messages = build_request_messages(&history);

    // 4. Build chat request
    let common = common_params.unwrap_or(CommonParams {
        temperature: None,
        top_p: None,
        max_tokens: None,
        stream: true,
    });
    let provider_params = provider_params.unwrap_or_else(|| ProviderParams::OpenaiCompat {
        frequency_penalty: None,
        presence_penalty: None,
        reasoning_effort: None,
        seed: None,
        max_completion_tokens: None,
    });

    let request = ChatRequest {
        model: request_model,
        messages,
        common,
        provider_params,
    };

    // 5. Create cancel token
    let cancel_rx = state.create_cancel_token(&conversation_id).await;

    // 6. Create ChatEventSender that converts events to StreamChunks
    let message_id_clone = message_id.clone();
    let state_clone = state.clone();
    let progress_tx_clone = progress_tx.clone();

    let channel = ChatEventSender::new(move |event| {
        match event {
            ChatEvent::Delta { content, .. } => {
                // Send content chunk
                let _ = progress_tx_clone.send(StreamChunk::Content {
                    content: content.clone(),
                });

                // Update database with accumulated content
                let _ = state_clone.db.with_conn(|conn| {
                    // Get current content and append
                    if let Ok(msg) = db::messages::get(conn, &message_id_clone) {
                        let new_content = format!("{}{}", msg.content, content);
                        db::messages::update_content(conn, &message_id_clone, &new_content, None, None, None)
                    } else {
                        Ok(())
                    }
                });
            }
            ChatEvent::Usage { prompt_tokens, completion_tokens, .. } => {
                // Send done event
                let _ = progress_tx_clone.send(StreamChunk::Done {
                    prompt_tokens,
                    completion_tokens,
                });
            }
            ChatEvent::Finished { .. } => {
                // Mark as done in database
                let _ = state_clone.db.with_conn(|conn| -> Result<(), AppError> {
                    conn.execute(
                        "UPDATE messages SET status = 'done' WHERE id = ?1",
                        [&message_id_clone],
                    )?;
                    Ok(())
                });
            }
            ChatEvent::Error { message, .. } => {
                // Send error event
                let _ = progress_tx_clone.send(StreamChunk::Error {
                    message: message.clone(),
                });

                // Mark as error in database
                let _ = state_clone.db.with_conn(|conn| {
                    db::messages::set_error(conn, &message_id_clone, &message)
                });
            }
            _ => {}
        }
    });

    // 7. Stream chat
    let result = provider
        .stream_chat(
            request,
            message_id.clone(),
            channel,
            cancel_rx,
        )
        .await;

    // 8. Handle final result
    if let Err(e) = result {
        let error_msg = e.to_string();
        let _ = progress_tx.send(StreamChunk::Error {
            message: error_msg.clone(),
        });
        let _ = state.db.with_conn(|conn| {
            db::messages::set_error(conn, &message_id, &error_msg)
        });
    }

    // 9. Cleanup
    state.remove_cancel_token(&conversation_id).await;
    state.remove_generation_task(&message_id).await;

    Ok(())
}

// Helper functions
async fn resolve_provider(
    state: &AppState,
    model_id: &str,
) -> Result<(Arc<dyn Provider>, String, String), AppError> {
    let resolved = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT m.provider_id, p.type, m.name
             FROM models m
             JOIN providers p ON p.id = m.provider_id
             WHERE m.id = ?1",
            [model_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(|_| AppError::NotFound(format!("Model {model_id}")))
    })?;

    let provider = state
        .get_provider(&resolved.0)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Provider {}", resolved.0)))?;

    Ok((provider, resolved.1, resolved.2))
}

fn build_request_messages(history: &[Message]) -> Vec<ChatMessage> {
    history
        .iter()
        .filter(|m| m.status != MessageStatus::Error && m.status != MessageStatus::Streaming)
        .map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect()
}
