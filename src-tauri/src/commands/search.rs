use std::sync::Arc;

use tauri::State;

use std::collections::HashSet;

use crate::db;
use crate::error::AppResult;
use crate::models::{Message, Role};
use crate::state::AppState;

#[tauri::command]
pub async fn search_messages(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> AppResult<Vec<Message>> {
    state.db.with_conn(|conn| {
        let mut results = db::messages::search(conn, &query)?;
        let mut seen = results.iter().map(|message| message.id.clone()).collect::<HashSet<_>>();
        for message_id in db::paste_blobs::search_message_ids(conn, &query)? {
            let message = conn.query_row(
                "SELECT id, conversation_id, content, role, model_id, reasoning, token_completion, created_at, status, version_group_id, version_number,
                   CASE WHEN version_group_id IS NULL THEN 1
                   ELSE (SELECT COUNT(*) FROM messages m2 WHERE m2.version_group_id = messages.version_group_id AND m2.deleted_at IS NULL)
                   END as total_versions
                 FROM messages
                 WHERE id = ?1 AND deleted_at IS NULL AND is_active_version = 1",
                [&message_id],
                |row| {
                    Ok(Message {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        content: row.get(2)?,
                        role: match row.get::<_, String>(3)?.as_str() {
                            "assistant" => Role::Assistant,
                            "system" => Role::System,
                            _ => Role::User,
                        },
                        model_id: row.get(4)?,
                        reasoning: row.get(5)?,
                        token_count: row.get(6)?,
                        created_at: row.get(7)?,
                        status: match row.get::<_, String>(8)?.as_str() {
                            "streaming" => crate::models::MessageStatus::Streaming,
                            "error" => crate::models::MessageStatus::Error,
                            _ => crate::models::MessageStatus::Done,
                        },
                        version_group_id: row.get(9)?,
                        version_number: row.get::<_, u32>(10).unwrap_or(1),
                        total_versions: row.get::<_, u32>(11).unwrap_or(1),
                    })
                },
            )?;
            if seen.insert(message.id.clone()) {
                results.push(message);
            }
        }
        Ok(results)
    })
}
