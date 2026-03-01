use std::sync::Arc;

use tauri::State;

use crate::db;
use crate::error::AppResult;
use crate::models::Assistant;
use crate::state::AppState;

#[tauri::command]
pub async fn create_assistant(
    state: State<'_, Arc<AppState>>,
    name: String,
    system_prompt: Option<String>,
    model_id: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
) -> AppResult<Assistant> {
    let now = super::chat::chrono_now();
    let assistant = Assistant {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        icon: None,
        system_prompt,
        model_id,
        temperature,
        top_p,
        max_tokens,
        extra_params: serde_json::json!({}),
        sort_order: 0,
        created_at: now,
    };
    state.db.with_conn(|conn| db::assistants::create(conn, &assistant))?;
    Ok(assistant)
}

#[tauri::command]
pub async fn list_assistants(
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<Assistant>> {
    state.db.with_conn(|conn| db::assistants::list(conn))
}

#[tauri::command]
pub async fn update_assistant(
    state: State<'_, Arc<AppState>>,
    assistant: Assistant,
) -> AppResult<()> {
    state.db.with_conn(|conn| db::assistants::update(conn, &assistant))
}

#[tauri::command]
pub async fn delete_assistant(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
    state.db.with_conn(|conn| db::assistants::delete(conn, &id))
}
