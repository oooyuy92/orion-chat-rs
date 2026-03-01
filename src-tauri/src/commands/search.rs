use std::sync::Arc;

use tauri::State;

use crate::db;
use crate::error::AppResult;
use crate::models::Message;
use crate::state::AppState;

#[tauri::command]
pub async fn search_messages(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> AppResult<Vec<Message>> {
    state.db.with_conn(|conn| db::messages::search(conn, &query))
}
