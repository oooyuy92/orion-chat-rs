use rusqlite::Connection;

use crate::error::AppResult;

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS providers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            api_key TEXT,
            base_url TEXT,
            proxy TEXT,
            is_enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS models (
            id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            display_name TEXT,
            max_tokens INTEGER,
            is_vision INTEGER NOT NULL DEFAULT 0,
            supports_thinking INTEGER NOT NULL DEFAULT 0,
            is_enabled INTEGER NOT NULL DEFAULT 1,
            source TEXT NOT NULL DEFAULT 'synced'
        );

        CREATE TABLE IF NOT EXISTS assistants (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            icon TEXT,
            system_prompt TEXT,
            model_id TEXT REFERENCES models(id),
            temperature REAL,
            top_p REAL,
            max_tokens INTEGER,
            extra_params TEXT NOT NULL DEFAULT '{}',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            assistant_id TEXT REFERENCES assistants(id),
            model_id TEXT REFERENCES models(id),
            agent_mode INTEGER NOT NULL DEFAULT 1,
            is_pinned INTEGER NOT NULL DEFAULT 0,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            model_id TEXT,
            reasoning TEXT,
            message_type TEXT NOT NULL DEFAULT 'text',
            tool_call_id TEXT,
            tool_name TEXT,
            tool_input TEXT,
            tool_error INTEGER NOT NULL DEFAULT 0,
            token_prompt INTEGER,
            token_completion INTEGER,
            status TEXT NOT NULL DEFAULT 'done',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT,
            version_group_id TEXT,
            version_number INTEGER NOT NULL DEFAULT 1,
            is_active_version INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS paste_blobs (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            char_count INTEGER NOT NULL,
            file_path TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS paste_blobs_fts USING fts5(
            paste_id UNINDEXED,
            message_id UNINDEXED,
            content
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content,
            content=messages,
            content_rowid=rowid
        );

        CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
        WHEN new.message_type = 'text' BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages
        WHEN old.message_type = 'text' BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
        END;

        CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE OF content, message_type ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content)
            SELECT 'delete', old.rowid, old.content
            WHERE old.message_type = 'text';
            INSERT INTO messages_fts(rowid, content)
            SELECT new.rowid, new.content
            WHERE new.message_type = 'text';
        END;
        ",
    )?;

    // Add deleted_at column for existing databases (idempotent)
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN deleted_at TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN message_type TEXT NOT NULL DEFAULT 'text'",
        [],
    );
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN tool_call_id TEXT", []);
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN tool_name TEXT", []);
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN tool_input TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN tool_error INTEGER NOT NULL DEFAULT 0",
        [],
    );

    // Add version columns for existing databases (idempotent)
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN version_group_id TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN version_number INTEGER NOT NULL DEFAULT 1",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN is_active_version INTEGER NOT NULL DEFAULT 1",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE models ADD COLUMN source TEXT NOT NULL DEFAULT 'synced'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE conversations ADD COLUMN agent_mode INTEGER NOT NULL DEFAULT 1",
        [],
    );
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_settings (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );",
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO agent_settings (key, value) VALUES ('tool_permissions', ?1)",
        rusqlite::params![serde_json::json!({
            "read_file": "auto",
            "list_files": "auto",
            "search": "auto",
            "edit_file": "ask",
            "write_file": "ask",
            "bash": "ask"
        })
        .to_string()],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO agent_settings (key, value) VALUES ('mcp_servers', '[]')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO agent_settings (key, value) VALUES ('skills_dir', '')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO agent_settings (key, value) VALUES ('working_dir', '')",
        [],
    )?;
    conn.execute(
        "UPDATE models
         SET display_name = name
         WHERE display_name IS NULL OR trim(display_name) = ''",
        [],
    )?;
    conn.execute_batch(
        "
        DROP TRIGGER IF EXISTS messages_ai;
        CREATE TRIGGER messages_ai AFTER INSERT ON messages
        WHEN new.message_type = 'text' BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        DROP TRIGGER IF EXISTS messages_ad;
        CREATE TRIGGER messages_ad AFTER DELETE ON messages
        WHEN old.message_type = 'text' BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
        END;

        DROP TRIGGER IF EXISTS messages_au;
        CREATE TRIGGER messages_au AFTER UPDATE OF content, message_type ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content)
            SELECT 'delete', old.rowid, old.content
            WHERE old.message_type = 'text';
            INSERT INTO messages_fts(rowid, content)
            SELECT new.rowid, new.content
            WHERE new.message_type = 'text';
        END;
        ",
    )?;

    // Purge soft-deleted messages older than 3 days
    conn.execute(
        "DELETE FROM messages WHERE deleted_at IS NOT NULL AND deleted_at < datetime('now', '-3 days')",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_value(conn: &Connection, key: &str) -> String {
        conn.query_row(
            "SELECT value FROM agent_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn test_migrations_run_idempotently() {
        let conn = Connection::open_in_memory().unwrap();

        run(&conn).unwrap();
        run(&conn).unwrap();
    }

    #[test]
    fn test_agent_schema_and_defaults_exist() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();

        let message_cols: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(messages)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        assert!(message_cols.contains(&"message_type".to_string()));
        assert!(message_cols.contains(&"tool_call_id".to_string()));
        assert!(message_cols.contains(&"tool_name".to_string()));
        assert!(message_cols.contains(&"tool_input".to_string()));
        assert!(message_cols.contains(&"tool_error".to_string()));

        let conversation_cols: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(conversations)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        assert!(conversation_cols.contains(&"agent_mode".to_string()));

        let agent_settings_exists: bool = conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1
                    FROM sqlite_master
                    WHERE type = 'table' AND name = 'agent_settings'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(agent_settings_exists);

        assert!(settings_value(&conn, "tool_permissions").contains("\"bash\":\"ask\""));
        assert_eq!(settings_value(&conn, "mcp_servers"), "[]");
        assert_eq!(settings_value(&conn, "skills_dir"), "");
        assert_eq!(settings_value(&conn, "working_dir"), "");
    }

    #[test]
    fn test_messages_fts_excludes_tool_messages_across_insert_update_delete() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-1', 'Test')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, message_type)
             VALUES ('msg-1', 'conv-1', 'assistant', 'hello world', 'text')",
            [],
        )
        .unwrap();

        let initial_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'hello'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(initial_count, 1);

        conn.execute(
            "UPDATE messages
             SET content = 'tool output', message_type = 'tool_result'
             WHERE id = 'msg-1'",
            [],
        )
        .unwrap();

        let after_tool_update: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'tool'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(after_tool_update, 0);

        conn.execute(
            "UPDATE messages
             SET content = 'hello again', message_type = 'text'
             WHERE id = 'msg-1'",
            [],
        )
        .unwrap();

        let after_text_restore: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'again'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(after_text_restore, 1);

        conn.execute("DELETE FROM messages WHERE id = 'msg-1'", []).unwrap();

        let after_delete: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'again'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(after_delete, 0);
    }
}
