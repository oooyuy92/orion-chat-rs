use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;

#[tauri::command]
pub async fn send_message(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    content: String,
    model_id: String,
    channel: Channel<ChatEvent>,
) -> AppResult<Message> {
    // Look up which provider owns this model
    let provider_id = state.db.with_conn(|conn| {
        let pid: String = conn
            .query_row(
                "SELECT provider_id FROM models WHERE id = ?1",
                [&model_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::NotFound(format!("Model {model_id}")))?;
        Ok(pid)
    })?;

    let provider = state
        .get_provider(&provider_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Provider {provider_id}")))?;

    // Determine provider type for building ProviderParams
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

    let now = chrono_now();

    // Save user message
    let user_msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.clone(),
        role: Role::User,
        content: content.clone(),
        reasoning: None,
        model_id: None,
        status: MessageStatus::Done,
        token_count: None,
        created_at: now.clone(),
    };
    state.db.with_conn(|conn| db::messages::create(conn, &user_msg))?;

    // Create assistant placeholder
    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    let assistant_msg = Message {
        id: assistant_msg_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        model_id: Some(model_id.clone()),
        status: MessageStatus::Streaming,
        token_count: None,
        created_at: now,
    };
    state
        .db
        .with_conn(|conn| db::messages::create(conn, &assistant_msg))?;

    // Load conversation history
    let history = state
        .db
        .with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;

    let messages: Vec<ChatMessage> = history
        .iter()
        .filter(|m| m.status != MessageStatus::Error && m.status != MessageStatus::Streaming)
        .map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    // Build default provider params based on type
    let provider_params = match provider_type.as_str() {
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
    };

    let request = ChatRequest {
        model: model_id,
        messages,
        common: CommonParams {
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: true,
        },
        provider_params,
    };

    // Send Started event
    let _ = channel.send(ChatEvent::Started {
        message_id: assistant_msg_id.clone(),
    });

    // Reset cancel signal
    let _ = state.cancel_sender.send(false);
    let cancel_rx = state.cancel_receiver.clone();

    // Stream
    let result = provider.stream_chat(request, channel.clone(), cancel_rx).await;

    match result {
        Ok(stream_result) => {
            // Persist accumulated content to DB
            let _ = state.db.with_conn(|conn| {
                db::messages::update_content(
                    conn,
                    &assistant_msg_id,
                    &stream_result.content,
                    stream_result.reasoning.as_deref(),
                    Some(stream_result.prompt_tokens),
                    Some(stream_result.completion_tokens),
                )
            });
            let _ = channel.send(ChatEvent::Finished {
                message_id: assistant_msg_id.clone(),
            });
        }
        Err(AppError::Cancelled) => {
            // On cancel, still persist whatever content was accumulated
            let _ = channel.send(ChatEvent::Finished {
                message_id: assistant_msg_id.clone(),
            });
            // Update status to done with partial content (empty is fine)
            let _ = state.db.with_conn(|conn| {
                db::messages::update_content(
                    conn,
                    &assistant_msg_id,
                    "",
                    None,
                    None,
                    None,
                )
            });
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = channel.send(ChatEvent::Error {
                message: err_msg.clone(),
            });
            let _ = state
                .db
                .with_conn(|conn| db::messages::set_error(conn, &assistant_msg_id, &err_msg));
        }
    }

    // Touch conversation
    let _ = state
        .db
        .with_conn(|conn| db::conversations::touch(conn, &conversation_id));

    // Return the updated message from DB (not the stale placeholder)
    let final_msg = state
        .db
        .with_conn(|conn| db::messages::get(conn, &assistant_msg_id))?;
    Ok(final_msg)
}

#[tauri::command]
pub async fn stop_generation(state: State<'_, Arc<AppState>>) -> AppResult<()> {
    let _ = state.cancel_sender.send(true);
    Ok(())
}

pub(crate) fn chrono_now() -> String {
    // Simple ISO-8601 timestamp using std
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Format as basic datetime — good enough for SQLite ordering
    let secs_per_day = 86400u64;
    let days = now / secs_per_day;
    let rem = now % secs_per_day;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;

    // Days since 1970-01-01
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
