use rusqlite::Connection;

use crate::error::AppResult;
use crate::models::Assistant;
use crate::db;

pub fn create_assistant(
    conn: &Connection,
    assistant: &Assistant,
) -> AppResult<()> {
    db::assistants::create(conn, assistant)
}

pub fn list_assistants(conn: &Connection) -> AppResult<Vec<Assistant>> {
    db::assistants::list(conn)
}

pub fn update_assistant(
    conn: &Connection,
    assistant: &Assistant,
) -> AppResult<()> {
    db::assistants::update(conn, assistant)
}

pub fn delete_assistant(conn: &Connection, id: &str) -> AppResult<()> {
    db::assistants::delete(conn, id)
}
