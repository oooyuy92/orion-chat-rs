use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::core;
use crate::db;
use crate::error::AppError;
use crate::models::*;
use crate::state::AppState;

// Error response wrapper for HTTP
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Cancelled => StatusCode::REQUEST_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(serde_json::json!({ "error": self.to_string() }))).into_response()
    }
}

// Request/Response types
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub assistant_id: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConversationTitleRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub content: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesQuery {
    pub limit: Option<u32>,
    pub before_message_id: Option<String>,
}

// Conversation handlers
pub async fn list_conversations(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Conversation>>, AppError> {
    let conversations = state.db.with_conn(|conn| db::conversations::list(conn))?;
    Ok(Json(conversations))
}

pub async fn create_conversation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<Conversation>, AppError> {
    let conversation_id = uuid::Uuid::new_v4().to_string();
    let title = req.title.unwrap_or_else(|| "New Conversation".to_string());
    let now = chrono::Utc::now().to_rfc3339();

    state.db.with_conn(|conn| {
        core::conversation::ensure_assistant_exists(conn, req.assistant_id.as_deref())?;
        core::conversation::ensure_model_exists(conn, req.model_id.as_deref())?;

        let conversation = Conversation {
            id: conversation_id.clone(),
            title: title.clone(),
            assistant_id: req.assistant_id.clone(),
            model_id: req.model_id.clone(),
            is_pinned: false,
            sort_order: 0,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        db::conversations::create(conn, &conversation)
    })?;

    let conversation = state.db.with_conn(|conn| db::conversations::get(conn, &conversation_id))?;
    Ok(Json(conversation))
}

pub async fn update_conversation_title(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateConversationTitleRequest>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::conversations::update_title(conn, &id, &req.title))?;
    Ok(StatusCode::OK)
}

pub async fn delete_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    core::conversation::delete_conversation_pastes(&state, &id)?;
    state.db.with_conn(|conn| db::conversations::delete(conn, &id))?;
    Ok(StatusCode::OK)
}

// Message handlers
pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<PagedMessages>, AppError> {
    let messages = state.db.with_conn(|conn| {
        core::conversation::load_messages_page(
            conn,
            &conversation_id,
            query.limit,
            query.before_message_id.as_deref(),
        )
    })?;
    Ok(Json(messages))
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<Message>, AppError> {
    let message_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Create user message
    let message = Message {
        id: message_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::User,
        content: req.content.clone(),
        reasoning: None,
        model_id: req.model_id.clone(),
        status: MessageStatus::Done,
        token_count: None,
        created_at: now.clone(),
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
    };

    state.db.with_conn(|conn| db::messages::create(conn, &message))?;

    let message = state.db.with_conn(|conn| db::messages::get(conn, &message_id))?;

    // TODO: Trigger background streaming task here
    // For now, just return the user message
    // The streaming will be implemented in the chat module integration

    Ok(Json(message))
}

// Provider handlers
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProviderConfig>>, AppError> {
    let providers = state.db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, type, api_key, base_url, is_enabled FROM providers ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let type_str: String = row.get(2)?;
            let api_key: Option<String> = row.get(3)?;
            let api_base: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
            let enabled: bool = row.get::<_, i32>(5)? != 0;
            Ok((id, name, type_str, api_key, api_base, enabled))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (id, name, type_str, api_key, api_base, enabled) = row?;
            let provider_type = core::provider::parse_provider_type(&type_str);

            // Load models for this provider
            let models = core::provider::load_models_for_provider(conn, &id)?;

            result.push(ProviderConfig {
                id,
                name,
                provider_type,
                api_key,
                api_base,
                models,
                enabled,
            });
        }
        Ok(result)
    })?;
    Ok(Json(providers))
}

// Assistant handlers
pub async fn list_assistants(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Assistant>>, AppError> {
    let assistants = state.db.with_conn(|conn| core::assistant::list_assistants(conn))?;
    Ok(Json(assistants))
}

// Model handlers
pub async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ModelInfo>>, AppError> {
    let mut all_models = Vec::new();

    // Query providers directly
    let providers = state.db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT id FROM providers")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok::<Vec<String>, AppError>(result)
    })?;

    for provider_id in providers {
        let models = state.db.with_conn(|conn| {
            core::provider::load_models_for_provider(conn, &provider_id)
        })?;
        all_models.extend(models);
    }

    Ok(Json(all_models))
}

