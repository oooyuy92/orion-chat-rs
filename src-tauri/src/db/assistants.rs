use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::models::Assistant;

pub fn create(conn: &Connection, a: &Assistant) -> AppResult<()> {
    let extra = serde_json::to_string(&a.extra_params)?;
    conn.execute(
        "INSERT INTO assistants (id, name, icon, system_prompt, model_id, temperature, top_p, max_tokens, extra_params, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            a.id, a.name, a.icon, a.system_prompt, a.model_id,
            a.temperature, a.top_p, a.max_tokens, extra,
            a.sort_order, a.created_at,
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Assistant> {
    conn.query_row(
        "SELECT id, name, icon, system_prompt, model_id, temperature, top_p, max_tokens, extra_params, sort_order, created_at
         FROM assistants WHERE id = ?1",
        [id],
        |row| {
            let extra_str: String = row.get(8)?;
            let extra: serde_json::Value =
                serde_json::from_str(&extra_str).unwrap_or_default();
            Ok(Assistant {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                system_prompt: row.get(3)?,
                model_id: row.get(4)?,
                temperature: row.get(5)?,
                top_p: row.get(6)?,
                max_tokens: row.get(7)?,
                extra_params: extra,
                sort_order: row.get(9)?,
                created_at: row.get(10)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Assistant {id}"))
        }
        other => AppError::Database(other),
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<Assistant>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, system_prompt, model_id, temperature, top_p, max_tokens, extra_params, sort_order, created_at
         FROM assistants ORDER BY sort_order ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let extra_str: String = row.get(8)?;
        let extra: serde_json::Value =
            serde_json::from_str(&extra_str).unwrap_or_default();
        Ok(Assistant {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            system_prompt: row.get(3)?,
            model_id: row.get(4)?,
            temperature: row.get(5)?,
            top_p: row.get(6)?,
            max_tokens: row.get(7)?,
            extra_params: extra,
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn update(conn: &Connection, a: &Assistant) -> AppResult<()> {
    let extra = serde_json::to_string(&a.extra_params)?;
    let changed = conn.execute(
        "UPDATE assistants SET name=?1, icon=?2, system_prompt=?3, model_id=?4, temperature=?5, top_p=?6, max_tokens=?7, extra_params=?8, sort_order=?9 WHERE id=?10",
        rusqlite::params![
            a.name, a.icon, a.system_prompt, a.model_id,
            a.temperature, a.top_p, a.max_tokens, extra,
            a.sort_order, a.id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Assistant {}", a.id)));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM assistants WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Assistant {id}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn make_assistant(id: &str, name: &str) -> Assistant {
        Assistant {
            id: id.into(),
            name: name.into(),
            icon: None,
            system_prompt: Some("You are helpful.".into()),
            model_id: None,
            temperature: Some(0.7),
            top_p: None,
            max_tokens: None,
            extra_params: serde_json::json!({}),
            sort_order: 0,
            created_at: "2025-01-01T00:00:00".into(),
        }
    }

    #[test]
    fn test_assistant_crud() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            create(conn, &make_assistant("a1", "Coder"))?;

            let fetched = get(conn, "a1")?;
            assert_eq!(fetched.name, "Coder");
            assert_eq!(fetched.temperature, Some(0.7));

            let all = list(conn)?;
            assert_eq!(all.len(), 1);

            delete(conn, "a1")?;
            let all = list(conn)?;
            assert_eq!(all.len(), 0);

            Ok(())
        })
        .unwrap();
    }
}
