use std::sync::Arc;

use tauri::State;

use crate::db;
use crate::error::AppResult;
use crate::paste_storage;
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
        let content = paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &msg.content, &|paste_id| {
            state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
        })?;
        let role_label = match msg.role {
            crate::models::Role::User => "User",
            crate::models::Role::Assistant => "Assistant",
            crate::models::Role::System => "System",
        };
        md.push_str(&format!("## {}\n\n{}\n\n", role_label, content));
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

    let expanded_messages = messages
        .into_iter()
        .map(|mut message| -> AppResult<_> {
            message.content = paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &message.content, &|paste_id| {
                state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
            })?;
            Ok(message)
        })
        .collect::<AppResult<Vec<_>>>()?;

    let export = serde_json::json!({
        "conversation": conv,
        "messages": expanded_messages,
    });
    let json = serde_json::to_string_pretty(&export)?;
    Ok(json)
}
