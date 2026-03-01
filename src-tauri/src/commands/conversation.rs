use std::sync::Arc;

use tauri::State;

use crate::db;
use crate::error::AppResult;
use crate::models::{Conversation, Message};
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
pub async fn get_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<Vec<Message>> {
    state.db.with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))
}
