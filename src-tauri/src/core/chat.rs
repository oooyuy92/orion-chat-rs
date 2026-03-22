use std::sync::Arc;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::paste_storage;
use crate::providers::Provider;
use crate::state::AppState;

/// Resolved model information from database
pub struct ResolvedModelRequest {
    pub provider_id: String,
    pub provider_type: String,
    pub request_model: String,
}

/// Resolve model request from database
pub fn resolve_model_request(
    conn: &rusqlite::Connection,
    model_id: &str,
) -> AppResult<ResolvedModelRequest> {
    conn.query_row(
        "SELECT m.provider_id, p.type, m.name
         FROM models m
         JOIN providers p ON p.id = m.provider_id
         WHERE m.id = ?1",
        [model_id],
        |row| {
            Ok(ResolvedModelRequest {
                provider_id: row.get(0)?,
                provider_type: row.get(1)?,
                request_model: row.get(2)?,
            })
        },
    )
    .map_err(|_| AppError::NotFound(format!("Model {model_id}")))
}

/// Resolve provider instance and type from a model_id
pub async fn resolve_provider(
    state: &AppState,
    model_id: &str,
) -> AppResult<(Arc<dyn Provider>, String, String)> {
    let resolved = state
        .db
        .with_conn(|conn| resolve_model_request(conn, model_id))?;

    let provider = state
        .get_provider(&resolved.provider_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Provider {}", resolved.provider_id)))?;

    Ok((provider, resolved.provider_type, resolved.request_model))
}

/// Build default ProviderParams from provider type string
pub fn default_provider_params(provider_type: &str) -> ProviderParams {
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

/// Strip paste markers from text (legacy inline pastes)
pub fn strip_paste_markers(text: &str) -> String {
    paste_storage::expand_legacy_inline_pastes(text)
}

/// Resolve paste blob path from database
pub fn resolve_paste_blob_path(state: &AppState, paste_id: &str) -> AppResult<String> {
    state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
}

/// Expand content for model by resolving paste references
pub fn expand_content_for_model(state: &AppState, text: &str) -> AppResult<String> {
    let legacy_expanded = strip_paste_markers(text);
    paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &legacy_expanded, &|paste_id| {
        resolve_paste_blob_path(state, paste_id)
    })
}

/// Persist external pastes to storage
pub fn persist_external_pastes(
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

/// Build request messages from history and optional assistant prompt
pub fn build_request_messages(
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

/// Load assistant system prompt for a conversation
pub fn load_assistant_system_prompt(
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

/// Generate current timestamp in ISO 8601 format
pub fn chrono_now() -> String {
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
}
