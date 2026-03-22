use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::models::{ModelInfo, ModelSource, ProviderType};

pub fn provider_type_to_db(provider_type: &ProviderType) -> &'static str {
    match provider_type {
        ProviderType::OpenaiCompat => "openai_compat",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::Ollama => "ollama",
    }
}

pub fn parse_provider_type(s: &str) -> ProviderType {
    match s {
        "anthropic" => ProviderType::Anthropic,
        "gemini" => ProviderType::Gemini,
        "ollama" => ProviderType::Ollama,
        _ => ProviderType::OpenaiCompat,
    }
}

pub fn validate_provider_config(
    provider_type: &ProviderType,
    api_key: Option<&str>,
    enabled: bool,
) -> AppResult<()> {
    if !enabled {
        return Ok(());
    }

    let needs_key = !matches!(provider_type, ProviderType::Ollama);
    if needs_key && api_key.unwrap_or("").trim().is_empty() {
        return Err(AppError::Provider("API key is required when provider is enabled".into()));
    }
    Ok(())
}

pub fn load_models_for_provider(
    conn: &Connection,
    provider_id: &str,
) -> AppResult<Vec<ModelInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, display_name, provider_id, max_tokens, is_vision, is_enabled, COALESCE(source, 'synced') FROM models WHERE provider_id = ?1",
    )?;
    let rows = stmt.query_map([provider_id], |row| {
        let request_name: String = row.get(1)?;
        let display_name: Option<String> = row.get(2)?;
        let source: String = row.get(7)?;
        let resolved_name = display_name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| request_name.clone());
        Ok(ModelInfo {
            id: row.get(0)?,
            name: resolved_name,
            request_name,
            display_name,
            provider_id: row.get(3)?,
            context_length: row.get(4)?,
            supports_vision: row.get::<_, i32>(5)? != 0,
            supports_streaming: true,
            enabled: row.get::<_, i32>(6)? != 0,
            source: parse_model_source(&source),
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn replace_synced_models_for_provider(
    conn: &Connection,
    provider_id: &str,
    models: &[ModelInfo],
) -> AppResult<()> {
    for m in models {
        conn.execute(
            "INSERT INTO models (id, provider_id, name, display_name, max_tokens, is_vision, is_enabled, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 'synced')
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                display_name = excluded.display_name,
                max_tokens = excluded.max_tokens,
                is_vision = excluded.is_vision,
                source = 'synced'",
            rusqlite::params![
                m.id,
                m.provider_id,
                m.request_name,
                m.display_name.clone().unwrap_or_else(|| m.name.clone()),
                m.context_length,
                m.supports_vision as i32,
            ],
        )?;
    }

    let mut sql = String::from(
        "DELETE FROM models
         WHERE provider_id = ?1
           AND source = 'synced'
           AND id NOT IN (
             SELECT model_id FROM assistants WHERE model_id IS NOT NULL
           )
           AND id NOT IN (
             SELECT model_id FROM conversations WHERE model_id IS NOT NULL
           )",
    );

    if !models.is_empty() {
        let placeholders = std::iter::repeat_n("?", models.len())
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(&format!(" AND id NOT IN ({placeholders})"));
    }

    let mut params = vec![rusqlite::types::Value::from(provider_id.to_string())];
    params.extend(
        models
            .iter()
            .map(|model| rusqlite::types::Value::from(model.id.clone())),
    );

    conn.execute(&sql, rusqlite::params_from_iter(params))?;
    Ok(())
}

pub fn update_model_visibility_in_db(
    conn: &Connection,
    model_id: &str,
    enabled: bool,
) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE models SET is_enabled = ?1 WHERE id = ?2",
        rusqlite::params![if enabled { 1 } else { 0 }, model_id],
    )?)
}

pub fn update_provider_models_visibility_in_db(
    conn: &Connection,
    provider_id: &str,
    enabled: bool,
) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE models SET is_enabled = ?1 WHERE provider_id = ?2",
        rusqlite::params![if enabled { 1 } else { 0 }, provider_id],
    )?)
}

fn parse_model_source(value: &str) -> ModelSource {
    match value {
        "manual" => ModelSource::Manual,
        _ => ModelSource::Synced,
    }
}

fn model_source_to_db(source: &ModelSource) -> &'static str {
    match source {
        ModelSource::Manual => "manual",
        ModelSource::Synced => "synced",
    }
}

fn normalize_display_name(display_name: Option<&str>) -> Option<String> {
    display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn ensure_provider_exists(conn: &Connection, provider_id: &str) -> AppResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM providers WHERE id = ?1)",
        [provider_id],
        |row| row.get(0),
    )?;

    if exists {
        Ok(())
    } else {
        Err(AppError::NotFound(format!("Provider {provider_id}")))
    }
}

fn load_model_by_id(conn: &Connection, model_id: &str) -> AppResult<ModelInfo> {
    let provider_id: String = conn
        .query_row(
            "SELECT provider_id FROM models WHERE id = ?1",
            [model_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound(format!("Model {model_id}")))?;

    load_models_for_provider(conn, &provider_id)?
        .into_iter()
        .find(|model| model.id == model_id)
        .ok_or_else(|| AppError::NotFound(format!("Model {model_id}")))
}

fn ensure_manual_model(conn: &Connection, model_id: &str) -> AppResult<()> {
    let source: String = conn
        .query_row("SELECT source FROM models WHERE id = ?1", [model_id], |row| row.get(0))
        .map_err(|_| AppError::NotFound(format!("Model {model_id}")))?;

    if parse_model_source(&source) == ModelSource::Manual {
        Ok(())
    } else {
        Err(AppError::Provider(format!("Model {model_id} is not a manual model")))
    }
}

pub fn create_manual_model_in_db(
    conn: &Connection,
    provider_id: &str,
    request_name: &str,
    display_name: Option<&str>,
    enabled: bool,
) -> AppResult<ModelInfo> {
    ensure_provider_exists(conn, provider_id)?;

    let request_name = request_name.trim();
    if request_name.is_empty() {
        return Err(AppError::Provider("Request model name is required".into()));
    }

    let dup_count: i64 = conn.query_row(
        "SELECT COUNT(1) FROM models WHERE provider_id = ?1 AND name = ?2",
        rusqlite::params![provider_id, request_name],
        |row| row.get(0),
    )?;
    if dup_count > 0 {
        return Err(AppError::Provider(format!(
            "Model '{}' already exists for this provider",
            request_name
        )));
    }

    let model_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO models (id, provider_id, name, display_name, is_enabled, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &model_id,
            provider_id,
            request_name,
            normalize_display_name(display_name),
            if enabled { 1 } else { 0 },
            model_source_to_db(&ModelSource::Manual),
        ],
    )?;

    load_model_by_id(conn, &model_id)
}

pub fn update_manual_model_in_db(
    conn: &Connection,
    model_id: &str,
    request_name: &str,
    display_name: Option<&str>,
    enabled: bool,
) -> AppResult<ModelInfo> {
    ensure_manual_model(conn, model_id)?;

    let request_name = request_name.trim();
    if request_name.is_empty() {
        return Err(AppError::Provider("Request model name is required".into()));
    }

    conn.execute(
        "UPDATE models
         SET name = ?1, display_name = ?2, is_enabled = ?3
         WHERE id = ?4",
        rusqlite::params![
            request_name,
            normalize_display_name(display_name),
            if enabled { 1 } else { 0 },
            model_id,
        ],
    )?;

    load_model_by_id(conn, model_id)
}

pub fn delete_manual_model_in_db(conn: &Connection, model_id: &str) -> AppResult<()> {
    ensure_manual_model(conn, model_id)?;
    let deleted = conn.execute("DELETE FROM models WHERE id = ?1", [model_id])?;
    if deleted == 0 {
        Err(AppError::NotFound(format!("Model {model_id}")))
    } else {
        Ok(())
    }
}

