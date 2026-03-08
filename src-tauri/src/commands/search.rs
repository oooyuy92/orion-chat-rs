use std::collections::HashSet;
use std::sync::Arc;

use rusqlite::Connection;
use tauri::State;

use crate::db;
use crate::error::AppResult;
use crate::models::{Message, Role, SearchSidebarResult};
use crate::state::AppState;

fn search_sidebar_results_with_conn(
    conn: &Connection,
    query: &str,
) -> AppResult<Vec<SearchSidebarResult>> {
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for result in db::messages::search_sidebar_results(conn, query)? {
        let key = (
            result.conversation_id.clone(),
            result.message_id.clone().unwrap_or_default(),
        );
        if seen.insert(key) {
            results.push(result);
        }
    }

    for result in db::paste_blobs::search_sidebar_results(conn, query)? {
        let key = (
            result.conversation_id.clone(),
            result.message_id.clone().unwrap_or_default(),
        );
        if seen.insert(key) {
            results.push(result);
        }
    }

    results.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(results)
}

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

#[tauri::command]
pub async fn search_sidebar_results(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> AppResult<Vec<SearchSidebarResult>> {
    state
        .db
        .with_conn(|conn| search_sidebar_results_with_conn(conn, &query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::db::conversations;
    use crate::db::messages;
    use crate::db::paste_blobs;
    use crate::models::{Conversation, MessageStatus};

    fn make_conversation(id: &str) -> Conversation {
        Conversation {
            id: id.into(),
            title: "Search Test".into(),
            assistant_id: None,
            model_id: None,
            is_pinned: false,
            sort_order: 0,
            created_at: "2025-01-01T00:00:00".into(),
            updated_at: "2025-01-01T00:00:00".into(),
        }
    }

    fn make_message(id: &str, content: &str, created_at: &str) -> Message {
        Message {
            id: id.into(),
            conversation_id: "conv-1".into(),
            role: Role::User,
            content: content.into(),
            reasoning: None,
            model_id: None,
            status: MessageStatus::Done,
            token_count: None,
            created_at: created_at.into(),
            version_group_id: None,
            version_number: 1,
            total_versions: 1,
        }
    }

    #[test]
    fn sidebar_search_returns_message_snippet_for_message_match() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            conversations::create(conn, &make_conversation("conv-1"))?;
            messages::create(
                conn,
                &make_message(
                    "msg-1",
                    "hello rust programming language in chat history",
                    "2025-01-01T00:00:00",
                ),
            )?;

            let results = search_sidebar_results_with_conn(conn, "rust")?;
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].conversation_id, "conv-1");
            assert_eq!(results[0].message_id.as_deref(), Some("msg-1"));
            assert!(results[0].snippet.contains("rust"));
            Ok::<(), crate::error::AppError>(())
        })
        .unwrap();
    }

    #[test]
    fn sidebar_search_returns_paste_snippet_for_paste_match() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            conversations::create(conn, &make_conversation("conv-1"))?;
            messages::create(
                conn,
                &make_message(
                    "msg-2",
                    "see attached long paste placeholder",
                    "2025-01-01T00:00:01",
                ),
            )?;
            paste_blobs::create(
                conn,
                "paste-1",
                "conv-1",
                "msg-2",
                24,
                "pastes/paste-1.txt",
                "alpha beta gamma delta epsilon",
                "2025-01-01T00:00:01",
            )?;

            let results = search_sidebar_results_with_conn(conn, "gamma")?;
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].message_id.as_deref(), Some("msg-2"));
            assert!(results[0].snippet.contains("gamma"));
            assert!(!results[0].snippet.contains("placeholder"));
            Ok::<(), crate::error::AppError>(())
        })
        .unwrap();
    }
}
