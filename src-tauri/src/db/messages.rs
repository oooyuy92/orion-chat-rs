use rusqlite::Connection;

use crate::error::AppResult;
use crate::models::{Message, MessageStatus, Role};

fn parse_role(s: &str) -> Role {
    match s {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        _ => Role::User,
    }
}

fn role_to_str(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
    }
}

fn parse_status(s: &str) -> MessageStatus {
    match s {
        "streaming" => MessageStatus::Streaming,
        "error" => MessageStatus::Error,
        _ => MessageStatus::Done,
    }
}

fn status_to_str(status: &MessageStatus) -> &'static str {
    match status {
        MessageStatus::Streaming => "streaming",
        MessageStatus::Done => "done",
        MessageStatus::Error => "error",
    }
}

pub fn create(conn: &Connection, msg: &Message) -> AppResult<()> {
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, model_id, reasoning, token_completion, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            msg.id,
            msg.conversation_id,
            role_to_str(&msg.role),
            msg.content,
            msg.model_id,
            msg.reasoning,
            msg.token_count,
            status_to_str(&msg.status),
            msg.created_at,
        ],
    )?;
    Ok(())
}

fn row_to_message(row: &rusqlite::Row) -> rusqlite::Result<Message> {
    let role_str: String = row.get(3)?;
    let status_str: String = row.get(8)?;
    Ok(Message {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        role: parse_role(&role_str),
        content: row.get(2)?,
        model_id: row.get(4)?,
        reasoning: row.get(5)?,
        token_count: row.get(6)?,
        status: parse_status(&status_str),
        created_at: row.get(7)?,
    })
}

const SELECT_COLS: &str =
    "id, conversation_id, content, role, model_id, reasoning, token_completion, created_at, status";

pub fn list_by_conversation(conn: &Connection, conversation_id: &str) -> AppResult<Vec<Message>> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([conversation_id], |row| row_to_message(row))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Message> {
    let sql = format!("SELECT {SELECT_COLS} FROM messages WHERE id = ?1");
    let msg = conn.query_row(&sql, [id], |row| row_to_message(row))?;
    Ok(msg)
}

pub fn update_content(
    conn: &Connection,
    id: &str,
    content: &str,
    reasoning: Option<&str>,
    token_prompt: Option<u32>,
    token_completion: Option<u32>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET content = ?1, reasoning = ?2, token_prompt = ?3, token_completion = ?4, status = 'done' WHERE id = ?5",
        rusqlite::params![content, reasoning, token_prompt, token_completion, id],
    )?;
    Ok(())
}

pub fn set_error(conn: &Connection, id: &str, error_message: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET content = ?1, status = 'error' WHERE id = ?2",
        rusqlite::params![error_message, id],
    )?;
    Ok(())
}

pub fn search(conn: &Connection, query: &str) -> AppResult<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.conversation_id, m.content, m.role, m.model_id, m.reasoning, m.token_completion, m.created_at, m.status
         FROM messages m
         JOIN messages_fts fts ON m.rowid = fts.rowid
         WHERE messages_fts MATCH ?1
         ORDER BY fts.rank",
    )?;
    let rows = stmt.query_map([query], |row| row_to_message(row))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::db::conversations;
    use crate::models::Conversation;

    fn make_conv(conn: &Connection) {
        let conv = Conversation {
            id: "conv-1".into(),
            title: "Test".into(),
            assistant_id: None,
            model_id: None,
            is_pinned: false,
            sort_order: 0,
            created_at: "2025-01-01T00:00:00".into(),
            updated_at: "2025-01-01T00:00:00".into(),
        };
        conversations::create(conn, &conv).unwrap();
    }

    fn make_msg(id: &str, content: &str) -> Message {
        Message {
            id: id.into(),
            conversation_id: "conv-1".into(),
            role: Role::User,
            content: content.into(),
            model_id: None,
            reasoning: None,
            token_count: None,
            status: MessageStatus::Done,
            created_at: "2025-01-01T00:00:00".into(),
        }
    }

    #[test]
    fn test_message_create_and_list() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            make_conv(conn);
            create(conn, &make_msg("m1", "hello"))?;
            create(conn, &make_msg("m2", "world"))?;
            let msgs = list_by_conversation(conn, "conv-1")?;
            assert_eq!(msgs.len(), 2);
            assert_eq!(msgs[0].content, "hello");
            assert_eq!(msgs[1].content, "world");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_fts_search() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            make_conv(conn);
            create(conn, &make_msg("m1", "rust programming language"))?;
            create(conn, &make_msg("m2", "python scripting"))?;
            let results = search(conn, "rust")?;
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].id, "m1");
            Ok(())
        })
        .unwrap();
    }
}
