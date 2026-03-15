use std::sync::Arc;

use rusqlite::Connection;
use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{Conversation, Message, MessageStatus, PagedMessages, Role};
use crate::paste_storage;

/// Lightweight version info for version tabs.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version_number: u32,
    pub model_id: Option<String>,
    pub id: String,
}
use crate::state::AppState;

fn resolve_paste_blob_path(state: &AppState, paste_id: &str) -> AppResult<String> {
    state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
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

fn delete_message_pastes(state: &AppState, message_id: &str) -> AppResult<()> {
    let blobs = state.db.with_conn(|conn| db::paste_blobs::list_by_message(conn, message_id))?;
    for blob in &blobs {
        paste_storage::delete_paste_blob_file(&state.data_dir, &blob.file_path)?;
    }
    state.db.with_conn(|conn| db::paste_blobs::delete_by_message(conn, message_id))?;
    Ok(())
}

fn delete_conversation_pastes(state: &AppState, conversation_id: &str) -> AppResult<()> {
    let blobs = state.db.with_conn(|conn| db::paste_blobs::list_by_conversation(conn, conversation_id))?;
    for blob in &blobs {
        paste_storage::delete_paste_blob_file(&state.data_dir, &blob.file_path)?;
    }
    state.db.with_conn(|conn| db::paste_blobs::delete_by_conversation(conn, conversation_id))?;
    Ok(())
}

fn ensure_assistant_exists(
    conn: &Connection,
    assistant_id: Option<&str>,
) -> AppResult<()> {
    if let Some(assistant_id) = assistant_id {
        db::assistants::get(conn, assistant_id)?;
    }
    Ok(())
}

fn ensure_model_exists(
    conn: &Connection,
    model_id: Option<&str>,
) -> AppResult<()> {
    if let Some(model_id) = model_id {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM models WHERE id = ?1)",
            [model_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(AppError::NotFound(format!("Model {model_id}")));
        }
    }
    Ok(())
}

fn ensure_conversation_assistant_can_change(
    conn: &Connection,
    conversation_id: &str,
) -> AppResult<()> {
    let has_user_messages: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM messages
            WHERE conversation_id = ?1 AND role = 'user' AND deleted_at IS NULL
        )",
        [conversation_id],
        |row| row.get(0),
    )?;

    if has_user_messages {
        return Err(AppError::Provider(
            "Conversation assistant is locked after the first user message".into(),
        ));
    }

    Ok(())
}

#[tauri::command]
pub async fn create_conversation(
    state: State<'_, Arc<AppState>>,
    title: String,
    assistant_id: Option<String>,
    model_id: Option<String>,
) -> AppResult<Conversation> {
    let now = super::chat::chrono_now();
    let conv = Conversation {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        assistant_id,
        model_id,
        is_pinned: false,
        sort_order: 0,
        created_at: now.clone(),
        updated_at: now,
    };
    state.db.with_conn(|conn| db::conversations::create(conn, &conv))?;
    Ok(conv)
}

#[tauri::command]
pub async fn list_conversations(
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<Conversation>> {
    state.db.with_conn(|conn| db::conversations::list(conn))
}

#[tauri::command]
pub async fn update_conversation_title(
    state: State<'_, Arc<AppState>>,
    id: String,
    title: String,
) -> AppResult<()> {
    state.db.with_conn(|conn| db::conversations::update_title(conn, &id, &title))
}

#[tauri::command]
pub async fn delete_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
    delete_conversation_pastes(&state, &id)?;
    state.db.with_conn(|conn| db::conversations::delete(conn, &id))
}

#[tauri::command]
pub async fn pin_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
    is_pinned: bool,
) -> AppResult<()> {
    state.db.with_conn(|conn| db::conversations::update_pin(conn, &id, is_pinned))
}

#[tauri::command]
pub async fn update_conversation_assistant(
    state: State<'_, Arc<AppState>>,
    id: String,
    assistant_id: Option<String>,
) -> AppResult<()> {
    state.db.with_conn(|conn| {
        db::conversations::get(conn, &id)?;
        ensure_assistant_exists(conn, assistant_id.as_deref())?;
        ensure_conversation_assistant_can_change(conn, &id)?;
        db::conversations::update_assistant(conn, &id, assistant_id.as_deref())
    })
}

#[tauri::command]
pub async fn update_conversation_model(
    state: State<'_, Arc<AppState>>,
    id: String,
    model_id: Option<String>,
) -> AppResult<()> {
    state.db.with_conn(|conn| {
        db::conversations::get(conn, &id)?;
        ensure_model_exists(conn, model_id.as_deref())?;
        db::conversations::update_model(conn, &id, model_id.as_deref())
    })
}

#[tauri::command]
pub async fn generate_conversation_title(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    model_id: String,
) -> AppResult<String> {
    use reqwest::Client;
    use serde_json::json;

    let messages =
        state.db.with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;
    if messages.is_empty() {
        return Ok(String::new());
    }

    let context = messages
        .iter()
        .filter(|m| {
            !matches!(m.status, MessageStatus::Error | MessageStatus::Streaming)
        })
        .take(6)
        .map(|m| {
            let role = if matches!(m.role, Role::User) { "用户" } else { "助手" };
            let content: String = m.content.chars().take(200).collect();
            format!("{role}: {content}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "根据以下对话，用不超过10个字给对话起一个简洁的标题，只输出标题文字，不加标点符号或解释。\n\n{context}"
    );

    let (provider_type, api_key, base_url) = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT p.type, p.api_key, p.base_url \
             FROM models m JOIN providers p ON m.provider_id = p.id WHERE m.id = ?1",
            [&model_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .map_err(|_| AppError::NotFound(format!("Model {model_id}")))
    })?;

    let client = Client::new();

    let raw = match provider_type.as_str() {
        "anthropic" => {
            let key = api_key.ok_or_else(|| AppError::Provider("No API key".into()))?;
            let base = base_url.as_deref().unwrap_or("https://api.anthropic.com");
            let resp = client
                .post(format!("{base}/v1/messages"))
                .header("x-api-key", &key)
                .header("anthropic-version", "2023-06-01")
                .json(&json!({
                    "model": model_id,
                    "max_tokens": 30,
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value =
                resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["content"][0]["text"].as_str().unwrap_or("").trim().to_string()
        }
        "gemini" => {
            let key = api_key.ok_or_else(|| AppError::Provider("No API key".into()))?;
            let base =
                base_url.as_deref().unwrap_or("https://generativelanguage.googleapis.com");
            let url = format!("{base}/v1beta/models/{model_id}:generateContent?key={key}");
            let resp = client
                .post(url)
                .json(&json!({
                    "contents": [{"role": "user", "parts": [{"text": prompt}]}],
                    "generationConfig": {"maxOutputTokens": 30}
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value =
                resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string()
        }
        "ollama" => {
            let base = base_url.as_deref().unwrap_or("http://127.0.0.1:11434");
            let resp = client
                .post(format!("{base}/api/chat"))
                .json(&json!({
                    "model": model_id,
                    "stream": false,
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value =
                resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["message"]["content"].as_str().unwrap_or("").trim().to_string()
        }
        _ => {
            let key = api_key.ok_or_else(|| AppError::Provider("No API key".into()))?;
            let base = base_url.as_deref().unwrap_or("https://api.openai.com");
            let resp = client
                .post(format!("{base}/chat/completions"))
                .bearer_auth(&key)
                .json(&json!({
                    "model": model_id,
                    "stream": false,
                    "max_tokens": 30,
                    "temperature": 0.3,
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value =
                resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string()
        }
    };

    // Strip quotes and limit to 15 chars
    let title: String = raw
        .trim_matches(|c| matches!(c, '"' | '\'' | '\n'))
        .chars()
        .take(15)
        .collect();
    Ok(title)
}

fn load_messages_page(
    conn: &Connection,
    conversation_id: &str,
    limit: Option<u32>,
    before_message_id: Option<&str>,
) -> AppResult<PagedMessages> {
    let page = db::messages::list_page_by_conversation(
        conn,
        conversation_id,
        limit.unwrap_or(100) as usize,
        before_message_id,
    )?;

    Ok(PagedMessages {
        messages: page.messages,
        has_more: page.has_more,
    })
}

#[tauri::command]
pub async fn get_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    limit: Option<u32>,
    before_message_id: Option<String>,
) -> AppResult<PagedMessages> {
    state.db.with_conn(|conn| {
        load_messages_page(conn, &conversation_id, limit, before_message_id.as_deref())
    })
}

#[tauri::command]
pub async fn delete_message(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
    state.db.with_conn(|conn| {
        let _ = db::messages::soft_delete_version(conn, &id)?;
        Ok(())
    })
}

#[tauri::command]
pub async fn restore_message(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
    state.db.with_conn(|conn| db::messages::restore(conn, &id))
}

#[tauri::command]
pub async fn delete_messages_after(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    message_id: String,
) -> AppResult<()> {
    state.db.with_conn(|conn| db::messages::delete_after(conn, &conversation_id, &message_id))
}

#[tauri::command]
pub async fn delete_messages_from(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    message_id: String,
) -> AppResult<()> {
    state.db.with_conn(|conn| db::messages::delete_from(conn, &conversation_id, &message_id))
}

#[tauri::command]
pub async fn update_message_content(
    state: State<'_, Arc<AppState>>,
    id: String,
    content: String,
) -> AppResult<()> {
    let (conversation_id, created_at): (String, String) = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT conversation_id, created_at FROM messages WHERE id = ?1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::NotFound(format!("Message {id}")))
    })?;
    delete_message_pastes(&state, &id)?;
    let persisted_content = persist_external_pastes(&state, &conversation_id, &id, &content, &created_at)?;
    state.db.with_conn(|conn| db::messages::update_text(conn, &id, &persisted_content))
}

#[tauri::command]
pub async fn get_paste_blob_content(
    state: State<'_, Arc<AppState>>,
    paste_id: String,
) -> AppResult<String> {
    let relative_path = resolve_paste_blob_path(&state, &paste_id)?;
    paste_storage::read_paste_blob(&state.data_dir, &relative_path)
}

#[tauri::command]
pub async fn hydrate_paste_content(
    state: State<'_, Arc<AppState>>,
    content: String,
) -> AppResult<String> {
    paste_storage::hydrate_paste_refs_to_legacy_markers(&state.data_dir, &content, &|paste_id| {
        resolve_paste_blob_path(&state, paste_id)
    })
}

#[tauri::command]
pub async fn expand_paste_content(
    state: State<'_, Arc<AppState>>,
    content: String,
) -> AppResult<String> {
    paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &content, &|paste_id| {
        resolve_paste_blob_path(&state, paste_id)
    })
}

#[tauri::command]
pub async fn switch_version(
    state: State<'_, Arc<AppState>>,
    version_group_id: String,
    version_number: u32,
) -> AppResult<()> {
    state
        .db
        .with_conn(|conn| db::messages::switch_active_version(conn, &version_group_id, version_number))
}

#[tauri::command]
pub async fn list_versions(
    state: State<'_, Arc<AppState>>,
    version_group_id: String,
) -> AppResult<Vec<VersionInfo>> {
    let msgs = state
        .db
        .with_conn(|conn| db::messages::list_versions(conn, &version_group_id))?;
    Ok(msgs
        .into_iter()
        .map(|m| VersionInfo {
            version_number: m.version_number,
            model_id: m.model_id,
            id: m.id,
        })
        .collect())
}

#[tauri::command]
pub async fn list_version_messages(
    state: State<'_, Arc<AppState>>,
    version_group_id: String,
) -> AppResult<Vec<Message>> {
    state
        .db
        .with_conn(|conn| db::messages::list_versions(conn, &version_group_id))
}

#[tauri::command]
pub async fn get_version_models(
    state: State<'_, Arc<AppState>>,
    version_group_id: String,
) -> AppResult<Vec<(u32, String)>> {
    state
        .db
        .with_conn(|conn| db::messages::get_version_models(conn, &version_group_id))
}

#[tauri::command]
pub async fn fork_conversation(
    state: State<'_, Arc<AppState>>,
    source_conversation_id: String,
    up_to_message_id: String,
) -> AppResult<Conversation> {
    let now = super::chat::chrono_now();

    // 1. Get source conversation
    let source = state
        .db
        .with_conn(|conn| db::conversations::get(conn, &source_conversation_id))?;

    // 2. Create new conversation with " 副本" suffix
    let new_conv = Conversation {
        id: uuid::Uuid::new_v4().to_string(),
        title: format!("{} 副本", source.title),
        assistant_id: source.assistant_id.clone(),
        model_id: source.model_id.clone(),
        is_pinned: false,
        sort_order: 0,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    state
        .db
        .with_conn(|conn| db::conversations::create(conn, &new_conv))?;

    // 3. Get all messages from source conversation
    let all_messages = state
        .db
        .with_conn(|conn| db::messages::list_by_conversation(conn, &source_conversation_id))?;

    // 4. Find up_to_message_id position and copy messages up to (and including) it
    let cut_idx = all_messages
        .iter()
        .position(|m| m.id == up_to_message_id)
        .ok_or_else(|| {
            AppError::NotFound(format!("Message {up_to_message_id}"))
        })?;

    let messages_to_copy = &all_messages[..=cut_idx];

    // 5. Insert copies into new conversation
    state.db.with_conn(|conn| {
        for msg in messages_to_copy {
            let new_msg = Message {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: new_conv.id.clone(),
                role: msg.role.clone(),
                content: msg.content.clone(),
                model_id: msg.model_id.clone(),
                reasoning: msg.reasoning.clone(),
                token_count: msg.token_count,
                status: MessageStatus::Done,
                created_at: msg.created_at.clone(),
                version_group_id: None,
                version_number: 1,
                total_versions: 1,
            };
            db::messages::create(conn, &new_msg)?;
        }
        Ok(())
    })?;

    Ok(new_conv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{Assistant, Message};

    fn make_conversation(id: &str) -> Conversation {
        Conversation {
            id: id.into(),
            title: "Test".into(),
            assistant_id: None,
            model_id: None,
            is_pinned: false,
            sort_order: 0,
            created_at: "2025-01-01T00:00:00".into(),
            updated_at: "2025-01-01T00:00:00".into(),
        }
    }

    fn make_assistant(id: &str) -> Assistant {
        Assistant {
            id: id.into(),
            name: "Helper".into(),
            icon: None,
            system_prompt: Some("You are helpful.".into()),
            model_id: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            extra_params: serde_json::json!({}),
            sort_order: 0,
            created_at: "2025-01-01T00:00:00".into(),
        }
    }

    fn make_user_message(id: &str, conversation_id: &str) -> Message {
        Message {
            id: id.into(),
            conversation_id: conversation_id.into(),
            role: Role::User,
            content: "hello".into(),
            reasoning: None,
            model_id: None,
            status: MessageStatus::Done,
            token_count: None,
            created_at: "2025-01-01T00:00:00".into(),
            version_group_id: None,
            version_number: 1,
            total_versions: 1,
        }
    }

    #[test]
    fn test_binding_allowed_without_user_messages() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            db::conversations::create(conn, &make_conversation("conv-1"))?;
            db::assistants::create(conn, &make_assistant("assistant-1"))?;

            ensure_conversation_assistant_can_change(conn, "conv-1")?;
            ensure_assistant_exists(conn, Some("assistant-1"))?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_binding_rejected_after_first_user_message() {
        let db = Database::new(":memory:").unwrap();

        let err = db
            .with_conn(|conn| {
                db::conversations::create(conn, &make_conversation("conv-1"))?;
                db::messages::create(conn, &make_user_message("msg-1", "conv-1"))?;
                ensure_conversation_assistant_can_change(conn, "conv-1")
            })
            .unwrap_err();

        assert!(matches!(err, AppError::Provider(_)));
    }

    #[test]
    fn test_binding_rejected_for_unknown_assistant() {
        let db = Database::new(":memory:").unwrap();

        let err = db
            .with_conn(|conn| {
                db::conversations::create(conn, &make_conversation("conv-1"))?;
                ensure_assistant_exists(conn, Some("missing-assistant"))
            })
            .unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn test_binding_rejected_for_unknown_model() {
        let db = Database::new(":memory:").unwrap();

        let err = db
            .with_conn(|conn| {
                db::conversations::create(conn, &make_conversation("conv-1"))?;
                ensure_model_exists(conn, Some("missing-model"))
            })
            .unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn test_binding_allowed_when_only_deleted_user_messages_exist() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            db::conversations::create(conn, &make_conversation("conv-1"))?;
            db::messages::create(conn, &make_user_message("msg-1", "conv-1"))?;
            db::messages::soft_delete(conn, "msg-1")?;

            ensure_conversation_assistant_can_change(conn, "conv-1")
        })
        .unwrap();
    }

    #[test]
    fn test_get_messages_page() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            db::conversations::create(conn, &make_conversation("conv-1"))?;
            for idx in 1..=5 {
                db::messages::create(conn, &make_user_message(&format!("msg-{idx}"), "conv-1"))?;
                db::messages::update_text(conn, &format!("msg-{idx}"), &format!("message-{idx}"))?;
            }

            let latest = load_messages_page(conn, "conv-1", Some(2), None)?;
            assert!(latest.has_more);
            assert_eq!(latest.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["msg-4", "msg-5"]);

            let older = load_messages_page(conn, "conv-1", Some(2), Some("msg-4"))?;
            assert!(older.has_more);
            assert_eq!(older.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["msg-2", "msg-3"]);

            let oldest = load_messages_page(conn, "conv-1", Some(2), Some("msg-2"))?;
            assert!(!oldest.has_more);
            assert_eq!(oldest.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["msg-1"]);

            Ok(())
        })
        .unwrap();
    }
}
