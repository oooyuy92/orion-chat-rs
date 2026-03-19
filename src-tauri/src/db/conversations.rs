use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::models::Conversation;

pub fn create(conn: &Connection, conv: &Conversation) -> AppResult<()> {
    conn.execute(
        "INSERT INTO conversations (id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at, working_dirs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            conv.id,
            conv.title,
            conv.assistant_id,
            conv.model_id,
            conv.is_pinned as i32,
            conv.sort_order,
            conv.created_at,
            conv.updated_at,
            serde_json::to_string(&conv.working_dirs).unwrap_or_else(|_| "[]".to_string()),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Conversation> {
    conn.query_row(
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at, working_dirs
         FROM conversations WHERE id = ?1",
        [id],
        |row| {
            let working_dirs_json: String = row.get(8)?;
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                assistant_id: row.get(2)?,
                model_id: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                working_dirs: serde_json::from_str(&working_dirs_json).unwrap_or_default(),
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
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at, working_dirs
         FROM conversations ORDER BY is_pinned DESC, updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let working_dirs_json: String = row.get(8)?;
        Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            assistant_id: row.get(2)?,
            model_id: row.get(3)?,
            is_pinned: row.get::<_, i32>(4)? != 0,
            sort_order: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            working_dirs: serde_json::from_str(&working_dirs_json).unwrap_or_default(),
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

pub fn update_assistant(
    conn: &Connection,
    id: &str,
    assistant_id: Option<&str>,
) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE conversations SET assistant_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![assistant_id, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Conversation {id}")));
    }
    Ok(())
}

pub fn update_model(
    conn: &Connection,
    id: &str,
    model_id: Option<&str>,
) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE conversations SET model_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![model_id, id],
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

pub fn set_working_dirs(conn: &Connection, id: &str, dirs: &[String]) -> AppResult<()> {
    let json = serde_json::to_string(dirs).unwrap_or_else(|_| "[]".to_string());
    let changed = conn.execute(
        "UPDATE conversations SET working_dirs = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![json, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Conversation {id}")));
    }
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
                working_dirs: vec![],
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

    #[test]
    fn test_update_assistant_binding() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO assistants (id, name, extra_params, created_at) VALUES (?1, ?2, '{}', ?3)",
                rusqlite::params!["assistant-1", "Helper", "2025-01-01T00:00:00"],
            )?;

            let conv = Conversation {
                id: "conv-1".into(),
                title: "Hello".into(),
                assistant_id: None,
                model_id: None,
                is_pinned: false,
                sort_order: 0,
                created_at: "2025-01-01T00:00:00".into(),
                updated_at: "2025-01-01T00:00:00".into(),
                working_dirs: vec![],
            };
            create(conn, &conv)?;

            update_assistant(conn, "conv-1", Some("assistant-1"))?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.assistant_id.as_deref(), Some("assistant-1"));

            update_assistant(conn, "conv-1", None)?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.assistant_id, None);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_update_model_binding() {
        let db = Database::new(":memory:").unwrap();

        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO providers (id, name, type, is_enabled) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params!["provider-1", "OpenAI", "openai_compat", 1],
            )?;
            conn.execute(
                "INSERT INTO models (id, provider_id, name, display_name) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params!["model-1", "provider-1", "gpt-4.1", "GPT-4.1"],
            )?;

            let conv = Conversation {
                id: "conv-1".into(),
                title: "Hello".into(),
                assistant_id: None,
                model_id: None,
                is_pinned: false,
                sort_order: 0,
                created_at: "2025-01-01T00:00:00".into(),
                updated_at: "2025-01-01T00:00:00".into(),
                working_dirs: vec![],
            };
            create(conn, &conv)?;

            update_model(conn, "conv-1", Some("model-1"))?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.model_id.as_deref(), Some("model-1"));

            update_model(conn, "conv-1", None)?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.model_id, None);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_set_working_dirs() {
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
                working_dirs: vec![],
            };
            create(conn, &conv)?;
            let fetched = get(conn, "conv-1")?;
            assert!(fetched.working_dirs.is_empty());

            set_working_dirs(
                conn,
                "conv-1",
                &["/tmp/project-a".into(), "/tmp/project-b".into()],
            )?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.working_dirs, vec!["/tmp/project-a", "/tmp/project-b"]);

            let all = list(conn)?;
            assert_eq!(all[0].working_dirs, vec!["/tmp/project-a", "/tmp/project-b"]);

            set_working_dirs(conn, "conv-1", &[])?;
            let fetched = get(conn, "conv-1")?;
            assert!(fetched.working_dirs.is_empty());
            Ok(())
        })
        .unwrap();
    }
}
