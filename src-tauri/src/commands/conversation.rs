use std::sync::Arc;

use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{Conversation, Message, MessageStatus, Role};

/// Lightweight version info for version tabs.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version_number: u32,
    pub model_id: Option<String>,
    pub id: String,
}
use crate::state::AppState;

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

#[tauri::command]
pub async fn get_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<Vec<Message>> {
    state.db.with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))
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
    state.db.with_conn(|conn| db::messages::update_text(conn, &id, &content))
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
pub async fn get_version_models(
    state: State<'_, Arc<AppState>>,
    version_group_id: String,
) -> AppResult<Vec<(u32, String)>> {
    state
        .db
        .with_conn(|conn| db::messages::get_version_models(conn, &version_group_id))
}
