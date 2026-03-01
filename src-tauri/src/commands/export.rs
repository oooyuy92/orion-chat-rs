use std::sync::Arc;

use tauri::State;

use crate::db;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn export_conversation_markdown(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<String> {
    let conv = state
        .db
        .with_conn(|conn| db::conversations::get(conn, &conversation_id))?;
    let messages = state
        .db
        .with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;

    let mut md = format!("# {}\n\n", conv.title);
    for msg in &messages {
        let role_label = match msg.role {
            crate::models::Role::User => "User",
            crate::models::Role::Assistant => "Assistant",
            crate::models::Role::System => "System",
        };
        md.push_str(&format!("## {}\n\n{}\n\n", role_label, msg.content));
    }
    Ok(md)
}

#[tauri::command]
pub async fn export_conversation_json(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<String> {
    let conv = state
        .db
        .with_conn(|conn| db::conversations::get(conn, &conversation_id))?;
    let messages = state
        .db
        .with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;

    let export = serde_json::json!({
        "conversation": conv,
        "messages": messages,
    });
    let json = serde_json::to_string_pretty(&export)?;
    Ok(json)
}
