use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::models::Conversation;

pub fn create(conn: &Connection, conv: &Conversation) -> AppResult<()> {
    conn.execute(
        "INSERT INTO conversations (id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            conv.id,
            conv.title,
            conv.assistant_id,
            conv.model_id,
            conv.is_pinned as i32,
            conv.sort_order,
            conv.created_at,
            conv.updated_at,
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Conversation> {
    conn.query_row(
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at
         FROM conversations WHERE id = ?1",
        [id],
        |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                assistant_id: row.get(2)?,
                model_id: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Conversation {id}"))
        }
        other => AppError::Database(other),
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at
         FROM conversations ORDER BY is_pinned DESC, updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            assistant_id: row.get(2)?,
            model_id: row.get(3)?,
            is_pinned: row.get::<_, i32>(4)? != 0,
            sort_order: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn update_title(conn: &Connection, id: &str, title: &str) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE conversations SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![title, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Conversation {id}")));
    }
    Ok(())
}

pub fn update_pin(conn: &Connection, id: &str, is_pinned: bool) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE conversations SET is_pinned = ?1 WHERE id = ?2",
        rusqlite::params![is_pinned as i32, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Conversation {id}")));
    }
    Ok(())
}

pub fn touch(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE conversations SET updated_at = datetime('now') WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM conversations WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Conversation {id}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_conversation_crud() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            let conv = Conversation {
                id: "conv-1".into(),
                title: "Hello".into(),
                assistant_id: None,
                model_id: None,
                is_pinned: false,
                sort_order: 0,
                created_at: "2025-01-01T00:00:00".into(),
                updated_at: "2025-01-01T00:00:00".into(),
            };
            create(conn, &conv)?;

            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.title, "Hello");

            update_title(conn, "conv-1", "Updated")?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.title, "Updated");

            let all = list(conn)?;
            assert_eq!(all.len(), 1);

            delete(conn, "conv-1")?;
            let all = list(conn)?;
            assert_eq!(all.len(), 0);

            Ok(())
        })
        .unwrap();
    }
}
