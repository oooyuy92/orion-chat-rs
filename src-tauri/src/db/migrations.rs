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
            is_enabled INTEGER NOT NULL DEFAULT 1
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

        CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
        END;

        CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE OF content ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        ",
    )?;

    // Add deleted_at column for existing databases (idempotent)
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN deleted_at TEXT", []);

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

    // Purge soft-deleted messages older than 3 days
    conn.execute(
        "DELETE FROM messages WHERE deleted_at IS NOT NULL AND deleted_at < datetime('now', '-3 days')",
        [],
    )?;

    Ok(())
}
