pub mod assistants;
pub mod conversations;
pub mod messages;
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
}
