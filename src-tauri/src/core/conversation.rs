use rusqlite::Connection;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::PagedMessages;
use crate::paste_storage;
use crate::state::AppState;

pub fn resolve_paste_blob_path(state: &AppState, paste_id: &str) -> AppResult<String> {
    state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
}

pub fn persist_external_pastes(
    state: &AppState,
    conversation_id: &str,
    message_id: &str,
    content: &str,
    created_at: &str,
) -> AppResult<String> {
    paste_storage::externalize_legacy_inline_pastes(content, |text, _count| {
        let paste_id = uuid::Uuid::new_v4().to_string();
        let persisted = paste_storage::persist_paste_blob(&state.data_dir, &paste_id, text)?;
        state.db.with_conn(|conn| {
            db::paste_blobs::create(
                conn,
                &paste_id,
                conversation_id,
                message_id,
                persisted.char_count,
                &persisted.file_path,
                text,
                created_at,
            )
        })?;
        Ok(format!("<<paste-ref:{paste_id}:{}>>", persisted.char_count))
    })
}

pub fn delete_message_pastes(state: &AppState, message_id: &str) -> AppResult<()> {
    let blobs = state.db.with_conn(|conn| db::paste_blobs::list_by_message(conn, message_id))?;
    for blob in &blobs {
        paste_storage::delete_paste_blob_file(&state.data_dir, &blob.file_path)?;
    }
    state.db.with_conn(|conn| db::paste_blobs::delete_by_message(conn, message_id))?;
    Ok(())
}

pub fn delete_conversation_pastes(state: &AppState, conversation_id: &str) -> AppResult<()> {
    let blobs = state.db.with_conn(|conn| db::paste_blobs::list_by_conversation(conn, conversation_id))?;
    for blob in &blobs {
        paste_storage::delete_paste_blob_file(&state.data_dir, &blob.file_path)?;
    }
    state.db.with_conn(|conn| db::paste_blobs::delete_by_conversation(conn, conversation_id))?;
    Ok(())
}

pub fn ensure_assistant_exists(
    conn: &Connection,
    assistant_id: Option<&str>,
) -> AppResult<()> {
    if let Some(assistant_id) = assistant_id {
        db::assistants::get(conn, assistant_id)?;
    }
    Ok(())
}

pub fn ensure_model_exists(
    conn: &Connection,
    model_id: Option<&str>,
) -> AppResult<()> {
    if let Some(model_id) = model_id {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM models WHERE id = ?1)",
            [model_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(AppError::NotFound(format!("Model {model_id}")));
        }
    }
    Ok(())
}

pub fn ensure_conversation_assistant_can_change(
    conn: &Connection,
    conversation_id: &str,
) -> AppResult<()> {
    let has_user_messages: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM messages
            WHERE conversation_id = ?1 AND role = 'user' AND deleted_at IS NULL
        )",
        [conversation_id],
        |row| row.get(0),
    )?;

    if has_user_messages {
        return Err(AppError::Provider(
            "Conversation assistant is locked after the first user message".into(),
        ));
    }

    Ok(())
}

pub fn load_messages_page(
    conn: &Connection,
    conversation_id: &str,
    limit: Option<u32>,
    before_message_id: Option<&str>,
) -> AppResult<PagedMessages> {
    let page = db::messages::list_page_by_conversation(
        conn,
        conversation_id,
        limit.unwrap_or(100) as usize,
        before_message_id,
    )?;

    Ok(PagedMessages {
        messages: page.messages,
        has_more: page.has_more,
    })
}
