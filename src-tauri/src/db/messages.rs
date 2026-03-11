use rusqlite::Connection;

use crate::error::AppResult;
use crate::models::{Message, MessageStatus, Role, SearchSidebarResult};

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

const SELECT_COLS: &str =
    "id, conversation_id, content, role, model_id, reasoning, token_completion, created_at, status, version_group_id, version_number";

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
        version_group_id: row.get(9)?,
        version_number: row.get::<_, u32>(10).unwrap_or(1),
        total_versions: 1,
    })
}

fn row_to_message_with_total(row: &rusqlite::Row) -> rusqlite::Result<Message> {
    let mut msg = row_to_message(row)?;
    msg.total_versions = row.get::<_, u32>(11).unwrap_or(1);
    Ok(msg)
}

pub fn create(conn: &Connection, msg: &Message) -> AppResult<()> {
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, model_id, reasoning, token_completion, status, created_at, version_group_id, version_number, is_active_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
            msg.version_group_id,
            msg.version_number,
            1,
        ],
    )?;
    Ok(())
}

/// Like create(), but allows controlling is_active_version (used by group send).
pub fn create_version(conn: &Connection, msg: &Message, is_active: bool) -> AppResult<()> {
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, model_id, reasoning, token_completion, status, created_at, version_group_id, version_number, is_active_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
            msg.version_group_id,
            msg.version_number,
            if is_active { 1 } else { 0 },
        ],
    )?;
    Ok(())
}

pub struct MessagePage {
    pub messages: Vec<Message>,
    pub has_more: bool,
}

pub fn list_by_conversation(conn: &Connection, conversation_id: &str) -> AppResult<Vec<Message>> {
    let sql = format!(
        "SELECT {SELECT_COLS},
           CASE WHEN version_group_id IS NULL THEN 1
           ELSE (SELECT COUNT(*) FROM messages m2 WHERE m2.version_group_id = messages.version_group_id AND m2.deleted_at IS NULL)
           END as total_versions
         FROM messages
         WHERE conversation_id = ?1 AND deleted_at IS NULL AND is_active_version = 1
         ORDER BY created_at ASC, rowid ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([conversation_id], |row| row_to_message_with_total(row))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn list_page_by_conversation(
    conn: &Connection,
    conversation_id: &str,
    limit: usize,
    before_message_id: Option<&str>,
) -> AppResult<MessagePage> {
    if limit == 0 {
        return Ok(MessagePage {
            messages: Vec::new(),
            has_more: false,
        });
    }

    let page_sql = match before_message_id {
        Some(_) => format!(
            "SELECT {SELECT_COLS}, total_versions FROM (
                SELECT {SELECT_COLS},
                    CASE WHEN version_group_id IS NULL THEN 1
                    ELSE (SELECT COUNT(*) FROM messages m2 WHERE m2.version_group_id = messages.version_group_id AND m2.deleted_at IS NULL)
                    END as total_versions,
                    rowid as sort_rowid
                FROM messages
                WHERE conversation_id = ?1
                  AND deleted_at IS NULL
                  AND is_active_version = 1
                  AND rowid < (SELECT rowid FROM messages WHERE id = ?2)
                ORDER BY created_at DESC, rowid DESC
                LIMIT ?3
             ) page
             ORDER BY created_at ASC, sort_rowid ASC"
        ),
        None => format!(
            "SELECT {SELECT_COLS}, total_versions FROM (
                SELECT {SELECT_COLS},
                    CASE WHEN version_group_id IS NULL THEN 1
                    ELSE (SELECT COUNT(*) FROM messages m2 WHERE m2.version_group_id = messages.version_group_id AND m2.deleted_at IS NULL)
                    END as total_versions,
                    rowid as sort_rowid
                FROM messages
                WHERE conversation_id = ?1
                  AND deleted_at IS NULL
                  AND is_active_version = 1
                ORDER BY created_at DESC, rowid DESC
                LIMIT ?2
             ) page
             ORDER BY created_at ASC, sort_rowid ASC"
        ),
    };

    let mut stmt = conn.prepare(&page_sql)?;
    let mut messages = Vec::new();
    match before_message_id {
        Some(before_id) => {
            let rows = stmt.query_map(rusqlite::params![conversation_id, before_id, limit], row_to_message_with_total)?;
            for row in rows {
                messages.push(row?);
            }
        }
        None => {
            let rows = stmt.query_map(rusqlite::params![conversation_id, limit], row_to_message_with_total)?;
            for row in rows {
                messages.push(row?);
            }
        }
    }

    let Some(first_message) = messages.first() else {
        return Ok(MessagePage {
            messages,
            has_more: false,
        });
    };

    let has_more: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM messages
            WHERE conversation_id = ?1
              AND deleted_at IS NULL
              AND is_active_version = 1
              AND rowid < (SELECT rowid FROM messages WHERE id = ?2)
        )",
        rusqlite::params![conversation_id, first_message.id],
        |row| row.get(0),
    )?;

    Ok(MessagePage { messages, has_more })
}

/// Load messages before a given message (by rowid). Used to build context for version generation.
pub fn list_before_message(
    conn: &Connection,
    conversation_id: &str,
    before_message_id: &str,
) -> AppResult<Vec<Message>> {
    let sql = format!(
        "SELECT {SELECT_COLS}, 1 as total_versions
         FROM messages
         WHERE conversation_id = ?1
           AND deleted_at IS NULL
           AND is_active_version = 1
           AND rowid < (SELECT MIN(rowid) FROM messages WHERE (version_group_id = ?2 OR id = ?2) AND deleted_at IS NULL)
         ORDER BY created_at ASC, rowid ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params![conversation_id, before_message_id], |row| {
        row_to_message_with_total(row)
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Message> {
    let sql = format!(
        "SELECT {SELECT_COLS},
           CASE WHEN version_group_id IS NULL THEN 1
           ELSE (SELECT COUNT(*) FROM messages m2 WHERE m2.version_group_id = messages.version_group_id AND m2.deleted_at IS NULL)
           END as total_versions
         FROM messages WHERE id = ?1"
    );
    let msg = conn.query_row(&sql, [id], |row| row_to_message_with_total(row))?;
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

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM messages WHERE id = ?1", [id])?;
    Ok(())
}

pub fn soft_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET deleted_at = datetime('now') WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

pub fn restore(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET deleted_at = NULL WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

pub fn delete_after(conn: &Connection, conversation_id: &str, message_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM messages WHERE conversation_id = ?1 AND rowid > (SELECT rowid FROM messages WHERE id = ?2)",
        rusqlite::params![conversation_id, message_id],
    )?;
    Ok(())
}

pub fn delete_from(conn: &Connection, conversation_id: &str, message_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM messages WHERE conversation_id = ?1 AND rowid >= (SELECT rowid FROM messages WHERE id = ?2)",
        rusqlite::params![conversation_id, message_id],
    )?;
    Ok(())
}

/// Delete all messages after a version group (by max rowid in the group).
pub fn delete_after_version_group(
    conn: &Connection,
    conversation_id: &str,
    version_group_id: &str,
) -> AppResult<()> {
    conn.execute(
        "DELETE FROM messages WHERE conversation_id = ?1 AND rowid > (SELECT MAX(rowid) FROM messages WHERE version_group_id = ?2)",
        rusqlite::params![conversation_id, version_group_id],
    )?;
    Ok(())
}

pub fn update_text(conn: &Connection, id: &str, content: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET content = ?1 WHERE id = ?2",
        rusqlite::params![content, id],
    )?;
    Ok(())
}

// ---------- Version management ----------

/// Initialize a version group on an existing message (called when +1 is first used).
pub fn init_version_group(conn: &Connection, message_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET version_group_id = ?1, version_number = 1 WHERE id = ?1 AND version_group_id IS NULL",
        [message_id],
    )?;
    Ok(())
}

/// Deactivate all versions in a group.
pub fn deactivate_versions(conn: &Connection, version_group_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET is_active_version = 0 WHERE version_group_id = ?1",
        [version_group_id],
    )?;
    Ok(())
}

/// Get the next version number in a group.
pub fn next_version_number(conn: &Connection, version_group_id: &str) -> AppResult<u32> {
    let max: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version_number), 0) FROM messages WHERE version_group_id = ?1 AND deleted_at IS NULL",
        [version_group_id],
        |row| row.get(0),
    )?;
    Ok(max + 1)
}

/// Switch the active version in a group.
pub fn switch_active_version(
    conn: &Connection,
    version_group_id: &str,
    target_version: u32,
) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET is_active_version = 0 WHERE version_group_id = ?1",
        [version_group_id],
    )?;
    conn.execute(
        "UPDATE messages SET is_active_version = 1 WHERE version_group_id = ?1 AND version_number = ?2 AND deleted_at IS NULL",
        rusqlite::params![version_group_id, target_version],
    )?;
    Ok(())
}

/// List all versions in a group (for version tabs).
pub fn list_versions(conn: &Connection, version_group_id: &str) -> AppResult<Vec<Message>> {
    let sql = format!(
        "SELECT {SELECT_COLS}, 1 as total_versions
         FROM messages
         WHERE version_group_id = ?1 AND deleted_at IS NULL
         ORDER BY version_number ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([version_group_id], |row| row_to_message_with_total(row))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Get model IDs for all versions in a group (for resend-all).
pub fn get_version_models(conn: &Connection, version_group_id: &str) -> AppResult<Vec<(u32, String)>> {
    let mut stmt = conn.prepare(
        "SELECT version_number, model_id FROM messages WHERE version_group_id = ?1 AND deleted_at IS NULL ORDER BY version_number ASC",
    )?;
    let rows = stmt.query_map([version_group_id], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Clear a message's content for regeneration.
pub fn clear_for_regenerate(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET content = '', reasoning = NULL, token_prompt = NULL, token_completion = NULL, status = 'streaming' WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

/// Soft-delete one version. If it's the last version, soft-delete the group marker.
/// Returns the new active version's message if one was auto-activated, or None.
pub fn soft_delete_version(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    // Get version info
    let (version_group_id, version_number): (Option<String>, u32) = conn.query_row(
        "SELECT version_group_id, version_number FROM messages WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // Soft delete this version
    soft_delete(conn, id)?;

    if let Some(gid) = version_group_id {
        // Check remaining versions
        let remaining: u32 = conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE version_group_id = ?1 AND deleted_at IS NULL",
            [&gid],
            |row| row.get(0),
        )?;

        if remaining == 0 {
            return Ok(None);
        }

        // Check if we deleted the active version — need to activate another
        let active_count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE version_group_id = ?1 AND deleted_at IS NULL AND is_active_version = 1",
            [&gid],
            |row| row.get(0),
        )?;

        if active_count == 0 {
            // Activate the nearest version (prefer lower version number)
            let new_active_id: String = conn.query_row(
                "SELECT id FROM messages WHERE version_group_id = ?1 AND deleted_at IS NULL ORDER BY ABS(version_number - ?2), version_number ASC LIMIT 1",
                rusqlite::params![gid, version_number],
                |row| row.get(0),
            )?;
            conn.execute(
                "UPDATE messages SET is_active_version = 1 WHERE id = ?1",
                [&new_active_id],
            )?;
            return Ok(Some(new_active_id));
        }
    }

    Ok(None)
}

pub fn search(conn: &Connection, query: &str) -> AppResult<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.conversation_id, m.content, m.role, m.model_id, m.reasoning, m.token_completion, m.created_at, m.status, m.version_group_id, m.version_number, 1 as total_versions
         FROM messages m
         JOIN messages_fts fts ON m.rowid = fts.rowid
         WHERE messages_fts MATCH ?1 AND m.deleted_at IS NULL AND m.is_active_version = 1
         ORDER BY fts.rank",
    )?;
    let rows = stmt.query_map([query], |row| row_to_message_with_total(row))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn search_sidebar_results(conn: &Connection, query: &str) -> AppResult<Vec<SearchSidebarResult>> {
    let mut stmt = conn.prepare(
        "SELECT m.conversation_id, m.id, snippet(messages_fts, 0, '', '', ' … ', 12), m.created_at
         FROM messages m
         JOIN messages_fts ON m.rowid = messages_fts.rowid
         WHERE messages_fts MATCH ?1 AND m.deleted_at IS NULL AND m.is_active_version = 1
         ORDER BY bm25(messages_fts), m.created_at DESC",
    )?;
    let rows = stmt.query_map([query], |row| {
        Ok(SearchSidebarResult {
            conversation_id: row.get(0)?,
            message_id: Some(row.get(1)?),
            snippet: row.get::<_, String>(2)?.trim().to_string(),
            created_at: row.get(3)?,
        })
    })?;
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
            version_group_id: None,
            version_number: 1,
            total_versions: 1,
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

    #[test]
    fn test_list_by_conversation_page() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            make_conv(conn);
            for idx in 1..=6 {
                create(conn, &make_msg(&format!("m{idx}"), &format!("msg-{idx}")))?;
            }

            soft_delete(conn, "m2")?;
            conn.execute(
                "UPDATE messages SET is_active_version = 0 WHERE id = ?1",
                ["m5"],
            )?;

            let latest = list_page_by_conversation(conn, "conv-1", 3, None)?;
            assert_eq!(latest.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["m3", "m4", "m6"]);
            assert!(latest.has_more);

            let older = list_page_by_conversation(conn, "conv-1", 3, Some("m3"))?;
            assert_eq!(older.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["m1"]);
            assert!(!older.has_more);

            Ok(())
        })
        .unwrap();
    }
}
