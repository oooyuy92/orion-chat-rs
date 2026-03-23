use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::{Event, Sse}},
    Json,
};
use futures::stream::Stream;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProviderRequest {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub api_base: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProviderRequest {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub api_base: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateManualModelRequest {
    pub request_name: String,
    pub display_name: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManualModelRequest {
    pub request_name: String,
    pub display_name: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVisibilityRequest {
    pub enabled: bool,
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

    // Create assistant message with streaming status
    let assistant_message_id = uuid::Uuid::new_v4().to_string();
    let assistant_message = Message {
        id: assistant_message_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        model_id: req.model_id.clone(),
        status: MessageStatus::Streaming,
        token_count: None,
        created_at: now.clone(),
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
    };

    state.db.with_conn(|conn| db::messages::create(conn, &assistant_message))?;

    // Spawn background generation task
    let model_id = req.model_id.ok_or_else(|| AppError::Provider("Model ID required".into()))?;

    super::background_tasks::spawn_generation_task(
        state.clone(),
        conversation_id,
        assistant_message_id.clone(),
        model_id,
        None, // common_params
        None, // provider_params
    )
    .await?;

    // Return the assistant message
    let message = state.db.with_conn(|conn| db::messages::get(conn, &assistant_message_id))?;

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

// Provider CRUD handlers
pub async fn add_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddProviderRequest>,
) -> Result<Json<ProviderConfig>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let type_str = core::provider::provider_type_to_db(&req.provider_type);
    core::provider::validate_provider_config(&req.provider_type, req.api_key.as_deref(), req.enabled)?;

    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO providers (id, name, type, api_key, base_url, is_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, req.name, type_str, req.api_key, req.api_base, if req.enabled { 1 } else { 0 }],
        )?;
        Ok(())
    })?;

    if req.enabled {
        state
            .register_provider(
                &id,
                &req.provider_type,
                req.api_key.as_deref(),
                Some(req.api_base.as_str()),
            )
            .await?;
    }

    Ok(Json(ProviderConfig {
        id,
        name: req.name,
        provider_type: req.provider_type,
        api_base: req.api_base,
        api_key: req.api_key,
        models: vec![],
        enabled: req.enabled,
    }))
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<ProviderConfig>, AppError> {
    core::provider::validate_provider_config(&req.provider_type, req.api_key.as_deref(), req.enabled)?;

    let type_str = core::provider::provider_type_to_db(&req.provider_type);
    let rows = state.db.with_conn(|conn| {
        Ok(conn.execute(
            "UPDATE providers
             SET name = ?1, type = ?2, api_key = ?3, base_url = ?4, is_enabled = ?5, updated_at = datetime('now')
             WHERE id = ?6",
            rusqlite::params![
                req.name,
                type_str,
                req.api_key,
                req.api_base,
                if req.enabled { 1 } else { 0 },
                id
            ],
        )?)
    })?;

    if rows == 0 {
        return Err(AppError::NotFound(format!("Provider {id}")));
    }

    if req.enabled {
        state
            .register_provider(
                &id,
                &req.provider_type,
                req.api_key.as_deref(),
                Some(req.api_base.as_str()),
            )
            .await?;
    } else {
        state.unregister_provider(&id).await;
    }

    let models = state.db.with_conn(|conn| core::provider::load_models_for_provider(conn, &id))?;
    Ok(Json(ProviderConfig {
        id,
        name: req.name,
        provider_type: req.provider_type,
        api_base: req.api_base,
        api_key: req.api_key,
        models,
        enabled: req.enabled,
    }))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let rows = state.db.with_conn(|conn| {
        Ok(conn.execute(
            "DELETE FROM providers WHERE id = ?1",
            rusqlite::params![id],
        )?)
    })?;

    if rows == 0 {
        return Err(AppError::NotFound(format!("Provider {id}")));
    }

    state.unregister_provider(&id).await;
    Ok(StatusCode::OK)
}

pub async fn fetch_models_for_provider(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> Result<Json<Vec<ModelInfo>>, AppError> {
    let provider = state
        .get_provider(&provider_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Provider {provider_id}")))?;

    let mut models = provider.list_models().await?;

    for m in &mut models {
        m.provider_id = provider_id.clone();
        m.source = ModelSource::Synced;
    }

    state
        .db
        .with_conn(|conn| core::provider::replace_synced_models_for_provider(conn, &provider_id, &models))?;

    state.db.with_conn(|conn| core::provider::load_models_for_provider(conn, &provider_id))
        .map(Json)
}

pub async fn create_manual_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<CreateManualModelRequest>,
) -> Result<Json<ModelInfo>, AppError> {
    state.db.with_conn(|conn| {
        core::provider::create_manual_model_in_db(
            conn,
            &provider_id,
            &req.request_name,
            req.display_name.as_deref(),
            req.enabled,
        )
    }).map(Json)
}

pub async fn update_manual_model(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
    Json(req): Json<UpdateManualModelRequest>,
) -> Result<Json<ModelInfo>, AppError> {
    state.db.with_conn(|conn| {
        core::provider::update_manual_model_in_db(
            conn,
            &model_id,
            &req.request_name,
            req.display_name.as_deref(),
            req.enabled,
        )
    }).map(Json)
}

pub async fn delete_manual_model(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .db
        .with_conn(|conn| core::provider::delete_manual_model_in_db(conn, &model_id))?;
    Ok(StatusCode::OK)
}

pub async fn update_model_visibility(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
    Json(req): Json<UpdateVisibilityRequest>,
) -> Result<StatusCode, AppError> {
    let rows = state.db.with_conn(|conn| core::provider::update_model_visibility_in_db(conn, &model_id, req.enabled))?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Model {model_id}")));
    }
    Ok(StatusCode::OK)
}

pub async fn update_provider_models_visibility(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<UpdateVisibilityRequest>,
) -> Result<Json<usize>, AppError> {
    let provider_exists = state.db.with_conn(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(1) FROM providers WHERE id = ?1",
            rusqlite::params![&provider_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })?;

    if !provider_exists {
        return Err(AppError::NotFound(format!("Provider {provider_id}")));
    }

    state
        .db
        .with_conn(|conn| core::provider::update_provider_models_visibility_in_db(conn, &provider_id, req.enabled))
        .map(Json)
}


// Conversation extended handlers
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinConversationRequest {
    pub is_pinned: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConversationAssistantRequest {
    pub assistant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConversationModelRequest {
    pub model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateTitleRequest {
    pub model_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkConversationRequest {
    pub up_to_message_id: String,
}

pub async fn pin_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<PinConversationRequest>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::conversations::update_pin(conn, &id, req.is_pinned))?;
    Ok(StatusCode::OK)
}

pub async fn update_conversation_assistant(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateConversationAssistantRequest>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| {
        db::conversations::get(conn, &id)?;
        core::conversation::ensure_assistant_exists(conn, req.assistant_id.as_deref())?;
        core::conversation::ensure_conversation_assistant_can_change(conn, &id)?;
        db::conversations::update_assistant(conn, &id, req.assistant_id.as_deref())
    })?;
    Ok(StatusCode::OK)
}

pub async fn update_conversation_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateConversationModelRequest>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| {
        db::conversations::get(conn, &id)?;
        core::conversation::ensure_model_exists(conn, req.model_id.as_deref())?;
        db::conversations::update_model(conn, &id, req.model_id.as_deref())
    })?;
    Ok(StatusCode::OK)
}

pub async fn generate_conversation_title(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    Json(req): Json<GenerateTitleRequest>,
) -> Result<Json<String>, AppError> {
    use reqwest::Client;
    use serde_json::json;

    let messages = state.db.with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;
    if messages.is_empty() {
        return Ok(Json(String::new()));
    }

    let context = messages
        .iter()
        .filter(|m| !matches!(m.status, MessageStatus::Error | MessageStatus::Streaming))
        .take(6)
        .map(|m| {
            let role = if matches!(m.role, Role::User) { "用户" } else { "助手" };
            let content: String = m.content.chars().take(200).collect();
            format!("{role}: {content}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "根据以下对话，用不超过10个字给对话起一个简洁的标题，只输出标题文字，不加标点符号或解释。\n\n{context}"
    );

    let (provider_type, api_key, base_url) = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT p.type, p.api_key, p.base_url \
             FROM models m JOIN providers p ON m.provider_id = p.id WHERE m.id = ?1",
            [&req.model_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .map_err(|_| AppError::NotFound(format!("Model {}", req.model_id)))
    })?;

    let client = Client::new();

    let raw = match provider_type.as_str() {
        "anthropic" => {
            let key = api_key.ok_or_else(|| AppError::Provider("No API key".into()))?;
            let base = base_url.as_deref().unwrap_or("https://api.anthropic.com");
            let resp = client
                .post(format!("{base}/v1/messages"))
                .header("x-api-key", &key)
                .header("anthropic-version", "2023-06-01")
                .json(&json!({
                    "model": req.model_id,
                    "max_tokens": 30,
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value = resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["content"][0]["text"].as_str().unwrap_or("").trim().to_string()
        }
        "gemini" => {
            let key = api_key.ok_or_else(|| AppError::Provider("No API key".into()))?;
            let base = base_url.as_deref().unwrap_or("https://generativelanguage.googleapis.com");
            let url = format!("{base}/v1beta/models/{}:generateContent?key={key}", req.model_id);
            let resp = client
                .post(url)
                .json(&json!({
                    "contents": [{"role": "user", "parts": [{"text": prompt}]}],
                    "generationConfig": {"maxOutputTokens": 30}
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value = resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string()
        }
        "ollama" => {
            let base = base_url.as_deref().unwrap_or("http://127.0.0.1:11434");
            let resp = client
                .post(format!("{base}/api/chat"))
                .json(&json!({
                    "model": req.model_id,
                    "stream": false,
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value = resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["message"]["content"].as_str().unwrap_or("").trim().to_string()
        }
        _ => {
            let key = api_key.ok_or_else(|| AppError::Provider("No API key".into()))?;
            let base = base_url.as_deref().unwrap_or("https://api.openai.com");
            let resp = client
                .post(format!("{base}/chat/completions"))
                .bearer_auth(&key)
                .json(&json!({
                    "model": req.model_id,
                    "stream": false,
                    "max_tokens": 30,
                    "temperature": 0.3,
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
                .map_err(|e| AppError::Provider(e.to_string()))?;
            let val: serde_json::Value = resp.json().await.map_err(|e| AppError::Provider(e.to_string()))?;
            val["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string()
        }
    };

    let title: String = raw
        .trim_matches(|c| matches!(c, '"' | '\'' | '\n'))
        .chars()
        .take(15)
        .collect();
    Ok(Json(title))
}

pub async fn fork_conversation(
    State(state): State<Arc<AppState>>,
    Path(source_conversation_id): Path<String>,
    Json(req): Json<ForkConversationRequest>,
) -> Result<Json<Conversation>, AppError> {
    let now = chrono::Utc::now().to_rfc3339();

    let source = state.db.with_conn(|conn| db::conversations::get(conn, &source_conversation_id))?;

    let new_conv = Conversation {
        id: uuid::Uuid::new_v4().to_string(),
        title: format!("{} 副本", source.title),
        assistant_id: source.assistant_id.clone(),
        model_id: source.model_id.clone(),
        is_pinned: false,
        sort_order: 0,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    state.db.with_conn(|conn| db::conversations::create(conn, &new_conv))?;

    let all_messages = state.db.with_conn(|conn| db::messages::list_by_conversation(conn, &source_conversation_id))?;

    let cut_idx = all_messages
        .iter()
        .position(|m| m.id == req.up_to_message_id)
        .ok_or_else(|| AppError::NotFound(format!("Message {}", req.up_to_message_id)))?;

    let messages_to_copy = &all_messages[..=cut_idx];

    state.db.with_conn(|conn| {
        for msg in messages_to_copy {
            let new_msg = Message {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: new_conv.id.clone(),
                role: msg.role.clone(),
                content: msg.content.clone(),
                model_id: msg.model_id.clone(),
                reasoning: msg.reasoning.clone(),
                token_count: msg.token_count,
                status: MessageStatus::Done,
                created_at: msg.created_at.clone(),
                version_group_id: None,
                version_number: 1,
                total_versions: 1,
            };
            db::messages::create(conn, &new_msg)?;
        }
        Ok(())
    })?;

    Ok(Json(new_conv))
}

// Message operation handlers
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMessageContentRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMessagesAfterRequest {
    pub message_id: String,
}

pub async fn delete_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| {
        let _ = db::messages::soft_delete_version(conn, &id)?;
        Ok(())
    })?;
    Ok(StatusCode::OK)
}

pub async fn restore_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::messages::restore(conn, &id))?;
    Ok(StatusCode::OK)
}

pub async fn delete_messages_after(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    Json(req): Json<DeleteMessagesAfterRequest>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::messages::delete_after(conn, &conversation_id, &req.message_id))?;
    Ok(StatusCode::OK)
}

pub async fn delete_messages_from(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    Json(req): Json<DeleteMessagesAfterRequest>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::messages::delete_from(conn, &conversation_id, &req.message_id))?;
    Ok(StatusCode::OK)
}

pub async fn update_message_content(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMessageContentRequest>,
) -> Result<StatusCode, AppError> {
    let (conversation_id, created_at): (String, String) = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT conversation_id, created_at FROM messages WHERE id = ?1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::NotFound(format!("Message {id}")))
    })?;

    core::conversation::delete_message_pastes(&state, &id)?;
    let persisted_content = core::conversation::persist_external_pastes(&state, &conversation_id, &id, &req.content, &created_at)?;
    state.db.with_conn(|conn| db::messages::update_text(conn, &id, &persisted_content))?;
    Ok(StatusCode::OK)
}

// Assistant CRUD handlers
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssistantRequest {
    pub name: String,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub async fn create_assistant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAssistantRequest>,
) -> Result<Json<Assistant>, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let assistant = Assistant {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        icon: None,
        system_prompt: req.system_prompt,
        model_id: req.model_id,
        temperature: req.temperature,
        top_p: req.top_p,
        max_tokens: req.max_tokens,
        extra_params: serde_json::json!({}),
        sort_order: 0,
        created_at: now,
    };
    state.db.with_conn(|conn| db::assistants::create(conn, &assistant))?;
    Ok(Json(assistant))
}

pub async fn update_assistant(
    State(state): State<Arc<AppState>>,
    Path(_id): Path<String>,
    Json(assistant): Json<Assistant>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::assistants::update(conn, &assistant))?;
    Ok(StatusCode::OK)
}

pub async fn delete_assistant(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::assistants::delete(conn, &id))?;
    Ok(StatusCode::OK)
}

// Paste handlers
pub async fn get_paste_blob_content(
    State(state): State<Arc<AppState>>,
    Path(paste_id): Path<String>,
) -> Result<Json<String>, AppError> {
    let relative_path = core::conversation::resolve_paste_blob_path(&state, &paste_id)?;
    let content = crate::paste_storage::read_paste_blob(&state.data_dir, &relative_path)?;
    Ok(Json(content))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteContentRequest {
    pub content: String,
}

pub async fn hydrate_paste_content(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PasteContentRequest>,
) -> Result<Json<String>, AppError> {
    let result = crate::paste_storage::hydrate_paste_refs_to_legacy_markers(&state.data_dir, &req.content, &|paste_id| {
        core::conversation::resolve_paste_blob_path(&state, paste_id)
    })?;
    Ok(Json(result))
}

pub async fn expand_paste_content(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PasteContentRequest>,
) -> Result<Json<String>, AppError> {
    let result = crate::paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &req.content, &|paste_id| {
        core::conversation::resolve_paste_blob_path(&state, paste_id)
    })?;
    Ok(Json(result))
}

// Version handlers
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchVersionRequest {
    pub version_number: u32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version_number: u32,
    pub model_id: Option<String>,
    pub id: String,
}

pub async fn switch_version(
    State(state): State<Arc<AppState>>,
    Path(version_group_id): Path<String>,
    Json(req): Json<SwitchVersionRequest>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| db::messages::switch_active_version(conn, &version_group_id, req.version_number))?;
    Ok(StatusCode::OK)
}

pub async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(version_group_id): Path<String>,
) -> Result<Json<Vec<VersionInfo>>, AppError> {
    let msgs = state.db.with_conn(|conn| db::messages::list_versions(conn, &version_group_id))?;
    let versions = msgs
        .into_iter()
        .map(|m| VersionInfo {
            version_number: m.version_number,
            model_id: m.model_id,
            id: m.id,
        })
        .collect();
    Ok(Json(versions))
}

pub async fn list_version_messages(
    State(state): State<Arc<AppState>>,
    Path(version_group_id): Path<String>,
) -> Result<Json<Vec<Message>>, AppError> {
    let msgs = state.db.with_conn(|conn| db::messages::list_versions(conn, &version_group_id))?;
    Ok(Json(msgs))
}

pub async fn get_version_models(
    State(state): State<Arc<AppState>>,
    Path(version_group_id): Path<String>,
) -> Result<Json<Vec<(u32, String)>>, AppError> {
    let models = state.db.with_conn(|conn| db::messages::get_version_models(conn, &version_group_id))?;
    Ok(Json(models))
}

// Search handlers
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub query: String,
}

pub async fn search_messages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<Message>>, AppError> {
    use std::collections::HashSet;

    let results = state.db.with_conn(|conn| {
        let mut results = db::messages::search(conn, &params.query)?;
        let mut seen = results.iter().map(|message| message.id.clone()).collect::<HashSet<_>>();

        for message_id in db::paste_blobs::search_message_ids(conn, &params.query)? {
            let message = conn.query_row(
                "SELECT id, conversation_id, content, role, model_id, reasoning, token_completion, created_at, status, version_group_id, version_number,
                   CASE WHEN version_group_id IS NULL THEN 1
                   ELSE (SELECT COUNT(*) FROM messages m2 WHERE m2.version_group_id = messages.version_group_id AND m2.deleted_at IS NULL)
                   END as total_versions
                 FROM messages
                 WHERE id = ?1 AND deleted_at IS NULL AND is_active_version = 1",
                [&message_id],
                |row| {
                    Ok(Message {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        content: row.get(2)?,
                        role: match row.get::<_, String>(3)?.as_str() {
                            "assistant" => Role::Assistant,
                            "system" => Role::System,
                            _ => Role::User,
                        },
                        model_id: row.get(4)?,
                        reasoning: row.get(5)?,
                        token_count: row.get(6)?,
                        created_at: row.get(7)?,
                        status: match row.get::<_, String>(8)?.as_str() {
                            "streaming" => MessageStatus::Streaming,
                            "error" => MessageStatus::Error,
                            _ => MessageStatus::Done,
                        },
                        version_group_id: row.get(9)?,
                        version_number: row.get::<_, u32>(10).unwrap_or(1),
                        total_versions: row.get::<_, u32>(11).unwrap_or(1),
                    })
                },
            )?;
            if seen.insert(message.id.clone()) {
                results.push(message);
            }
        }
        Ok(results)
    })?;
    Ok(Json(results))
}

pub async fn search_sidebar_results(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchSidebarResult>>, AppError> {
    use std::collections::HashSet;

    let results = state.db.with_conn(|conn| {
        let mut seen = HashSet::new();
        let mut results = Vec::new();

        for result in db::messages::search_sidebar_results(conn, &params.query)? {
            let key = (
                result.conversation_id.clone(),
                result.message_id.clone().unwrap_or_default(),
            );
            if seen.insert(key) {
                results.push(result);
            }
        }

        for result in db::paste_blobs::search_sidebar_results(conn, &params.query)? {
            let key = (
                result.conversation_id.clone(),
                result.message_id.clone().unwrap_or_default(),
            );
            if seen.insert(key) {
                results.push(result);
            }
        }

        results.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(results)
    })?;
    Ok(Json(results))
}

// Export handlers
pub async fn export_conversation_markdown(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<String>, AppError> {
    let conv = state.db.with_conn(|conn| db::conversations::get(conn, &conversation_id))?;
    let messages = state.db.with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;

    let mut md = format!("# {}\n\n", conv.title);
    for msg in &messages {
        let content = crate::paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &msg.content, &|paste_id| {
            state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
        })?;
        let role_label = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };
        md.push_str(&format!("## {}\n\n{}\n\n", role_label, content));
    }
    Ok(Json(md))
}

pub async fn export_conversation_json(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<String>, AppError> {
    let conv = state.db.with_conn(|conn| db::conversations::get(conn, &conversation_id))?;
    let messages = state.db.with_conn(|conn| db::messages::list_by_conversation(conn, &conversation_id))?;

    let expanded_messages = messages
        .into_iter()
        .map(|mut message| -> Result<_, AppError> {
            message.content = crate::paste_storage::expand_paste_refs_to_plain_text(&state.data_dir, &message.content, &|paste_id| {
                state.db.with_conn(|conn| Ok(db::paste_blobs::get(conn, paste_id)?.file_path))
            })?;
            Ok(message)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let export = serde_json::json!({
        "conversation": conv,
        "messages": expanded_messages,
    });
    let json = serde_json::to_string_pretty(&export)?;
    Ok(Json(json))
}

// Settings handlers
pub async fn get_proxy_mode(
    State(state): State<Arc<AppState>>,
) -> Result<Json<String>, AppError> {
    Ok(Json(state.proxy_mode.lock().await.clone()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProxyModeRequest {
    pub mode: String,
}

pub async fn set_proxy_mode(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetProxyModeRequest>,
) -> Result<StatusCode, AppError> {
    *state.proxy_mode.lock().await = req.mode;
    Ok(StatusCode::OK)
}

pub async fn reset_app_data(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    state.db.with_conn(|conn| {
        conn.execute_batch(
            "DELETE FROM messages;
             DELETE FROM conversations;
             DELETE FROM assistants;",
        )?;
        Ok(())
    })?;
    Ok(StatusCode::OK)
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub async fn get_cache_size(
    State(state): State<Arc<AppState>>,
) -> Result<Json<String>, AppError> {
    let cache_dir = state.data_dir.join("cache");

    if !cache_dir.exists() {
        return Ok(Json("0 KB".to_string()));
    }

    let total = dir_size(&cache_dir);
    Ok(Json(format_size(total)))
}

pub async fn clear_cache(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    let cache_dir = state.data_dir.join("cache");

    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir)
            .map_err(|e| AppError::Provider(e.to_string()))?;
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| AppError::Provider(e.to_string()))?;
    }
    Ok(StatusCode::OK)
}

/// SSE stream handler for message generation progress
pub async fn stream_message(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    use crate::state::StreamChunk;

    // Create SSE stream
    let stream = async_stream::stream! {
        // Check if generation task exists
        let progress_rx = match state.subscribe_to_generation(&message_id).await {
            Some(rx) => rx,
            None => {
                // Task not found or already completed
                yield Ok(Event::default()
                    .event("error")
                    .data("Task not found or already completed"));
                return;
            }
        };

        let mut rx = progress_rx;

        // Send started event
        yield Ok(Event::default()
            .event("started")
            .data(serde_json::json!({ "messageId": message_id }).to_string()));

        // Continuously receive and push chunks
        while rx.changed().await.is_ok() {
            let chunk = rx.borrow().clone();

            match chunk {
                StreamChunk::Content { content } => {
                    yield Ok(Event::default()
                        .event("chunk")
                        .data(serde_json::json!({
                            "messageId": message_id,
                            "content": content
                        }).to_string()));
                }
                StreamChunk::Done { prompt_tokens, completion_tokens } => {
                    yield Ok(Event::default()
                        .event("done")
                        .data(serde_json::json!({
                            "messageId": message_id,
                            "promptTokens": prompt_tokens,
                            "completionTokens": completion_tokens
                        }).to_string()));
                    break;
                }
                StreamChunk::Error { message } => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(serde_json::json!({
                            "messageId": message_id,
                            "message": message
                        }).to_string()));
                    break;
                }
            }
        }
    };

    Sse::new(stream)
}

/// Stop ongoing generation for a conversation
pub async fn stop_generation(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.cancel_conversation(&conversation_id).await;
    Ok(StatusCode::OK)
}

/// Regenerate an assistant message with streaming
pub async fn regenerate_message(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<String>,
) -> Result<Json<Message>, AppError> {
    // Get the message to regenerate
    let message = state.db.with_conn(|conn| db::messages::get(conn, &message_id))?;

    if message.role != Role::Assistant {
        return Err(AppError::Provider("Can only regenerate assistant messages".into()));
    }

    // Clear the message content and set to streaming
    state.db.with_conn(|conn| {
        db::messages::clear_for_regenerate(conn, &message_id)
    })?;

    // Get model_id from the message
    let model_id = message.model_id.ok_or_else(|| AppError::Provider("Message has no model_id".into()))?;

    // Spawn background generation task
    super::background_tasks::spawn_generation_task(
        state.clone(),
        message.conversation_id.clone(),
        message_id.clone(),
        model_id,
        None,
        None,
    )
    .await?;

    // Return the updated message
    let updated_message = state.db.with_conn(|conn| db::messages::get(conn, &message_id))?;
    Ok(Json(updated_message))
}

/// Generate a new version of an assistant message with streaming
pub async fn generate_message_version(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<String>,
) -> Result<Json<Message>, AppError> {
    // Get the original message
    let original = state.db.with_conn(|conn| db::messages::get(conn, &message_id))?;

    if original.role != Role::Assistant {
        return Err(AppError::Provider("Can only create versions of assistant messages".into()));
    }

    let model_id = original.model_id.ok_or_else(|| AppError::Provider("Message has no model_id".into()))?;

    // Initialize version group if needed
    state.db.with_conn(|conn| {
        db::messages::init_version_group(conn, &message_id)
    })?;

    // Get version group id
    let version_group_id = state.db.with_conn(|conn| {
        db::messages::get(conn, &message_id).map(|m| m.version_group_id)
    })?.ok_or_else(|| AppError::Provider("Failed to get version group".into()))?;

    // Deactivate all versions in this group
    state.db.with_conn(|conn| {
        db::messages::deactivate_versions(conn, &version_group_id)
    })?;

    // Get next version number
    let version_number = state.db.with_conn(|conn| {
        db::messages::next_version_number(conn, &version_group_id)
    })?;

    // Create new version message
    let new_message_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let new_message = Message {
        id: new_message_id.clone(),
        conversation_id: original.conversation_id.clone(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        model_id: Some(model_id.clone()),
        status: MessageStatus::Streaming,
        token_count: None,
        created_at: now,
        version_group_id: Some(version_group_id.clone()),
        version_number,
        total_versions: version_number,
    };

    state.db.with_conn(|conn| db::messages::create_version(conn, &new_message, true))?;

    // Spawn background generation task
    super::background_tasks::spawn_generation_task(
        state.clone(),
        original.conversation_id,
        new_message_id.clone(),
        model_id,
        None,
        None,
    )
    .await?;

    // Return the new message
    let message = state.db.with_conn(|conn| db::messages::get(conn, &new_message_id))?;
    Ok(Json(message))
}

/// Resend last user message and generate new assistant response
pub async fn resend_message(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Message>, AppError> {
    // Get conversation messages
    let messages = state.db.with_conn(|conn| {
        db::messages::list_by_conversation(conn, &conversation_id)
    })?;

    // Find the last user message
    let _last_user_msg = messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User)
        .ok_or_else(|| AppError::NotFound("No user message found".into()))?;

    // Get model_id from last assistant message or conversation default
    let model_id = messages
        .iter()
        .rev()
        .find(|m| m.role == Role::Assistant && m.model_id.is_some())
        .and_then(|m| m.model_id.clone())
        .ok_or_else(|| AppError::Provider("No model_id found".into()))?;

    // Create new assistant message
    let assistant_message_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let assistant_message = Message {
        id: assistant_message_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::Assistant,
        content: String::new(),
        reasoning: None,
        model_id: Some(model_id.clone()),
        status: MessageStatus::Streaming,
        token_count: None,
        created_at: now,
        version_group_id: None,
        version_number: 1,
        total_versions: 1,
    };

    state.db.with_conn(|conn| db::messages::create(conn, &assistant_message))?;

    // Spawn background generation task
    super::background_tasks::spawn_generation_task(
        state.clone(),
        conversation_id,
        assistant_message_id.clone(),
        model_id,
        None,
        None,
    )
    .await?;

    // Return the assistant message
    let message = state.db.with_conn(|conn| db::messages::get(conn, &assistant_message_id))?;
    Ok(Json(message))
}
