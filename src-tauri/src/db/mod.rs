pub mod assistants;
pub mod conversations;
pub mod messages;
pub mod paste_blobs;
pub mod migrations;

use rusqlite::Connection;
use std::sync::Mutex;

use crate::error::AppResult;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }

    fn run_migrations(&self) -> AppResult<()> {
        self.with_conn(|conn| {
            migrations::run(conn)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_init() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='messages'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_fts5_table_created() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='messages_fts'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_migrations_add_model_source_with_synced_default() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE models (
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                name TEXT NOT NULL,
                display_name TEXT,
                max_tokens INTEGER,
                is_vision INTEGER NOT NULL DEFAULT 0,
                supports_thinking INTEGER NOT NULL DEFAULT 0,
                is_enabled INTEGER NOT NULL DEFAULT 1
            );
            ",
        )
        .unwrap();

        migrations::run(&conn).unwrap();

        let source_exists = conn
            .prepare("PRAGMA table_info(models)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .flatten()
            .any(|name| name == "source");
        assert!(source_exists);

        conn.execute(
            "INSERT INTO models (id, provider_id, name) VALUES (?1, ?2, ?3)",
            ("m1", "p1", "gpt-4.1"),
        )
        .unwrap();
        let source: String = conn
            .query_row("SELECT source FROM models WHERE id = 'm1'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(source, "synced");
    }

    #[test]
    fn test_migrations_backfill_model_display_name_from_name() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE models (
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                name TEXT NOT NULL,
                display_name TEXT,
                max_tokens INTEGER,
                is_vision INTEGER NOT NULL DEFAULT 0,
                supports_thinking INTEGER NOT NULL DEFAULT 0,
                is_enabled INTEGER NOT NULL DEFAULT 1
            );
            INSERT INTO models (id, provider_id, name, display_name) VALUES
                ('m1', 'p1', 'gpt-4.1', NULL),
                ('m2', 'p1', 'gpt-4.1-mini', '');
            ",
        )
        .unwrap();

        migrations::run(&conn).unwrap();

        let display_names: Vec<(String, String)> = conn
            .prepare("SELECT id, display_name FROM models ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert_eq!(
            display_names,
            vec![
                ("m1".to_string(), "gpt-4.1".to_string()),
                ("m2".to_string(), "gpt-4.1-mini".to_string()),
            ]
        );
    }
}
