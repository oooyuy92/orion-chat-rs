use rusqlite::Connection;

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasteBlob {
    pub id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub char_count: usize,
    pub file_path: String,
    pub created_at: String,
}

fn row_to_paste_blob(row: &rusqlite::Row) -> rusqlite::Result<PasteBlob> {
    Ok(PasteBlob {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        message_id: row.get(2)?,
        char_count: row.get(3)?,
        file_path: row.get(4)?,
        created_at: row.get(5)?,
    })
}

pub fn create(
    conn: &Connection,
    id: &str,
    conversation_id: &str,
    message_id: &str,
    char_count: usize,
    file_path: &str,
    search_content: &str,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO paste_blobs (id, conversation_id, message_id, char_count, file_path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, conversation_id, message_id, char_count, file_path, created_at],
    )?;
    conn.execute(
        "INSERT INTO paste_blobs_fts (paste_id, message_id, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, message_id, search_content],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<PasteBlob> {
    conn.query_row(
        "SELECT id, conversation_id, message_id, char_count, file_path, created_at
         FROM paste_blobs WHERE id = ?1",
        [id],
        row_to_paste_blob,
    )
    .map_err(Into::into)
}

pub fn list_by_message(conn: &Connection, message_id: &str) -> AppResult<Vec<PasteBlob>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, message_id, char_count, file_path, created_at
         FROM paste_blobs WHERE message_id = ?1 ORDER BY created_at ASC, rowid ASC",
    )?;
    let rows = stmt.query_map([message_id], row_to_paste_blob)?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn list_by_conversation(conn: &Connection, conversation_id: &str) -> AppResult<Vec<PasteBlob>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, message_id, char_count, file_path, created_at
         FROM paste_blobs WHERE conversation_id = ?1 ORDER BY created_at ASC, rowid ASC",
    )?;
    let rows = stmt.query_map([conversation_id], row_to_paste_blob)?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete_by_message(conn: &Connection, message_id: &str) -> AppResult<()> {
    let ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM paste_blobs WHERE message_id = ?1")?;
        let rows = stmt.query_map([message_id], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        result
    };

    for id in &ids {
        conn.execute("DELETE FROM paste_blobs_fts WHERE paste_id = ?1", [id])?;
    }
    conn.execute("DELETE FROM paste_blobs WHERE message_id = ?1", [message_id])?;
    Ok(())
}

pub fn delete_by_conversation(conn: &Connection, conversation_id: &str) -> AppResult<()> {
    let ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM paste_blobs WHERE conversation_id = ?1")?;
        let rows = stmt.query_map([conversation_id], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        result
    };

    for id in &ids {
        conn.execute("DELETE FROM paste_blobs_fts WHERE paste_id = ?1", [id])?;
    }
    conn.execute("DELETE FROM paste_blobs WHERE conversation_id = ?1", [conversation_id])?;
    Ok(())
}

pub fn search_message_ids(conn: &Connection, query: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT p.message_id
         FROM paste_blobs_fts f
         JOIN paste_blobs p ON p.id = f.paste_id
         WHERE paste_blobs_fts MATCH ?1
         ORDER BY p.message_id ASC",
    )?;
    let rows = stmt.query_map([query], |row| row.get::<_, String>(0))?;
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
    use crate::db::messages;
    use crate::models::{Conversation, Message, MessageStatus, Role};

    fn make_conversation(id: &str) -> Conversation {
        Conversation {
            id: id.into(),
            title: "Test".into(),
            assistant_id: None,
            model_id: None,
            is_pinned: false,
            sort_order: 0,
            created_at: "2025-01-01T00:00:00".into(),
            updated_at: "2025-01-01T00:00:00".into(),
        }
    }

    fn make_message(id: &str) -> Message {
        Message {
            id: id.into(),
            conversation_id: "conv-1".into(),
            role: Role::User,
            content: "body".into(),
            reasoning: None,
            model_id: None,
            status: MessageStatus::Done,
            token_count: None,
            created_at: "2025-01-01T00:00:00".into(),
            version_group_id: None,
            version_number: 1,
            total_versions: 1,
        }
    }

    #[test]
    fn test_paste_blob_crud_and_search() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            conversations::create(conn, &make_conversation("conv-1"))?;
            messages::create(conn, &make_message("msg-1"))?;
            messages::create(conn, &make_message("msg-2"))?;

            create(
                conn,
                "paste-1",
                "conv-1",
                "msg-1",
                12,
                "pastes/paste-1.txt",
                "alpha beta gamma",
                "2025-01-01T00:00:00",
            )?;
            create(
                conn,
                "paste-2",
                "conv-1",
                "msg-2",
                8,
                "pastes/paste-2.txt",
                "delta epsilon",
                "2025-01-01T00:00:01",
            )?;

            let blob = get(conn, "paste-1")?;
            assert_eq!(blob.message_id, "msg-1");

            let blobs = list_by_message(conn, "msg-1")?;
            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].id, "paste-1");

            let matches = search_message_ids(conn, "gamma")?;
            assert_eq!(matches, vec!["msg-1"]);

            delete_by_message(conn, "msg-1")?;
            assert!(list_by_message(conn, "msg-1")?.is_empty());
            assert!(search_message_ids(conn, "gamma")?.is_empty());

            delete_by_conversation(conn, "conv-1")?;
            assert!(list_by_conversation(conn, "conv-1")?.is_empty());

            Ok(())
        })
        .unwrap();
    }
}
