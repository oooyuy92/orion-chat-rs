use uuid::Uuid;

use crate::db::Database;
use crate::error::AppResult;

pub struct ToolCallRecord {
    pub message_id: String,
    pub conversation_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_input: String,
}

pub fn insert_tool_call_start(
    db: &Database,
    conversation_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    tool_input: &str,
) -> AppResult<String> {
    let message_id = Uuid::new_v4().to_string();

    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO messages
                (id, conversation_id, role, content, message_type, tool_call_id, tool_name, tool_input, tool_error, status, created_at, version_group_id, version_number, is_active_version)
             VALUES
                (?1, ?2, 'assistant', '', 'tool_call', ?3, ?4, ?5, 0, 'done', datetime('now'), ?1, 1, 1)",
            rusqlite::params![message_id, conversation_id, tool_call_id, tool_name, tool_input],
        )?;
        Ok(())
    })?;

    Ok(message_id)
}

pub fn insert_tool_call_result(
    db: &Database,
    conversation_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    result: &str,
    is_error: bool,
) -> AppResult<String> {
    let message_id = Uuid::new_v4().to_string();

    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO messages
                (id, conversation_id, role, content, message_type, tool_call_id, tool_name, tool_error, status, created_at, version_group_id, version_number, is_active_version)
             VALUES
                (?1, ?2, 'assistant', ?3, 'tool_result', ?4, ?5, ?6, 'done', datetime('now'), ?1, 1, 1)",
            rusqlite::params![
                message_id,
                conversation_id,
                result,
                tool_call_id,
                tool_name,
                if is_error { 1 } else { 0 }
            ],
        )?;
        Ok(())
    })?;

    Ok(message_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::models::Conversation;

    fn setup_db() -> Database {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            db::conversations::create(
                conn,
                &Conversation {
                    id: "test-conv".into(),
                    title: "Test".into(),
                    assistant_id: None,
                    model_id: None,
                    is_pinned: false,
                    sort_order: 0,
                    created_at: "2026-03-18T00:00:00Z".into(),
                    updated_at: "2026-03-18T00:00:00Z".into(),
                },
            )?;
            Ok(())
        })
        .unwrap();

        db
    }

    #[test]
    fn test_insert_tool_call_start_returns_id() {
        let db = setup_db();

        let message_id = insert_tool_call_start(
            &db,
            "test-conv",
            "call-123",
            "read_file",
            r#"{"path":"src/main.rs"}"#,
        )
        .unwrap();

        assert!(!message_id.is_empty());

        let count: i64 = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM messages WHERE tool_call_id = 'call-123' AND message_type = 'tool_call'",
                    [],
                    |row| row.get(0),
                )
                .map_err(Into::into)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_insert_tool_result() {
        let db = setup_db();

        insert_tool_call_start(&db, "test-conv", "call-456", "bash", "{}").unwrap();
        let result_id = insert_tool_call_result(
            &db,
            "test-conv",
            "call-456",
            "bash",
            "exit 0",
            false,
        )
        .unwrap();

        let is_error: i64 = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT tool_error FROM messages WHERE id = ?1",
                    [result_id.clone()],
                    |row| row.get(0),
                )
                .map_err(Into::into)
            })
            .unwrap();
        assert_eq!(is_error, 0);

        let tool_name: String = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT tool_name FROM messages WHERE id = ?1",
                    [result_id],
                    |row| row.get(0),
                )
                .map_err(Into::into)
            })
            .unwrap();
        assert_eq!(tool_name, "bash");
    }

    #[test]
    fn test_tool_result_not_indexed_in_fts() {
        let db = setup_db();

        insert_tool_call_start(&db, "test-conv", "call-789", "bash", "{}").unwrap();
        insert_tool_call_result(&db, "test-conv", "call-789", "bash", "some output", false)
            .unwrap();

        let count: i64 = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'output'",
                    [],
                    |row| row.get(0),
                )
                .map_err(Into::into)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
