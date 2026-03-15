use std::sync::Arc;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::{ModelInfo, ModelSource, ProviderConfig, ProviderType};
use crate::state::AppState;

#[tauri::command]
pub async fn add_provider(
    state: State<'_, Arc<AppState>>,
    name: String,
    provider_type: ProviderType,
    api_key: Option<String>,
    api_base: String,
    enabled: bool,
) -> AppResult<ProviderConfig> {
    let id = uuid::Uuid::new_v4().to_string();
    let type_str = provider_type_to_db(&provider_type);
    validate_provider_config(&provider_type, api_key.as_deref(), enabled)?;

    // Insert into DB via raw SQL (no db::providers module)
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO providers (id, name, type, api_key, base_url, is_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, name, type_str, api_key, api_base, if enabled { 1 } else { 0 }],
        )?;
        Ok(())
    })?;

    if enabled {
        // Register in AppState
        state
            .register_provider(
                &id,
                &provider_type,
                api_key.as_deref(),
                Some(api_base.as_str()),
            )
            .await?;
    }

    Ok(ProviderConfig {
        id,
        name,
        provider_type,
        api_base,
        api_key,
        models: vec![],
        enabled,
    })
}

#[tauri::command]
pub async fn list_providers(
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<ProviderConfig>> {
    state.db.with_conn(|conn| {
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
            let provider_type = parse_provider_type(&type_str);

            // Load models for this provider
            let models = load_models_for_provider(conn, &id)?;

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
    })
}

#[tauri::command]
pub async fn update_provider(
    state: State<'_, Arc<AppState>>,
    id: String,
    name: String,
    provider_type: ProviderType,
    api_key: Option<String>,
    api_base: String,
    enabled: bool,
) -> AppResult<ProviderConfig> {
    validate_provider_config(&provider_type, api_key.as_deref(), enabled)?;

    let type_str = provider_type_to_db(&provider_type);
    let rows = state.db.with_conn(|conn| {
        Ok(conn.execute(
            "UPDATE providers
             SET name = ?1, type = ?2, api_key = ?3, base_url = ?4, is_enabled = ?5, updated_at = datetime('now')
             WHERE id = ?6",
            rusqlite::params![
                name,
                type_str,
                api_key,
                api_base,
                if enabled { 1 } else { 0 },
                id
            ],
        )?)
    })?;

    if rows == 0 {
        return Err(AppError::NotFound(format!("Provider {id}")));
    }

    if enabled {
        state
            .register_provider(
                &id,
                &provider_type,
                api_key.as_deref(),
                Some(api_base.as_str()),
            )
            .await?;
    } else {
        state.unregister_provider(&id).await;
    }

    let models = state.db.with_conn(|conn| load_models_for_provider(conn, &id))?;
    Ok(ProviderConfig {
        id,
        name,
        provider_type,
        api_base,
        api_key,
        models,
        enabled,
    })
}

#[tauri::command]
pub async fn delete_provider(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
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
    Ok(())
}

#[tauri::command]
pub async fn fetch_models(
    state: State<'_, Arc<AppState>>,
    provider_id: String,
) -> AppResult<Vec<ModelInfo>> {
    let provider = state
        .get_provider(&provider_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Provider {provider_id}")))?;

    let mut models = provider.list_models().await?;

    // Set provider_id on each model
    for m in &mut models {
        m.provider_id = provider_id.clone();
        m.source = ModelSource::Synced;
    }

    state
        .db
        .with_conn(|conn| replace_synced_models_for_provider(conn, &provider_id, &models))?;

    // Return all models (synced + manual) so the frontend stays in sync
    state.db.with_conn(|conn| load_models_for_provider(conn, &provider_id))
}

#[tauri::command]
pub async fn update_model_visibility(
    state: State<'_, Arc<AppState>>,
    model_id: String,
    enabled: bool,
) -> AppResult<()> {
    let rows = state.db.with_conn(|conn| update_model_visibility_in_db(conn, &model_id, enabled))?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Model {model_id}")));
    }
    Ok(())
}

#[tauri::command]
pub async fn update_provider_models_visibility(
    state: State<'_, Arc<AppState>>,
    provider_id: String,
    enabled: bool,
) -> AppResult<usize> {
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
        .with_conn(|conn| update_provider_models_visibility_in_db(conn, &provider_id, enabled))
}

#[tauri::command]
pub async fn create_manual_model(
    state: State<'_, Arc<AppState>>,
    provider_id: String,
    request_name: String,
    display_name: Option<String>,
    enabled: bool,
) -> AppResult<ModelInfo> {
    state.db.with_conn(|conn| {
        create_manual_model_in_db(
            conn,
            &provider_id,
            &request_name,
            display_name.as_deref(),
            enabled,
        )
    })
}

#[tauri::command]
pub async fn update_manual_model(
    state: State<'_, Arc<AppState>>,
    model_id: String,
    request_name: String,
    display_name: Option<String>,
    enabled: bool,
) -> AppResult<ModelInfo> {
    state.db.with_conn(|conn| {
        update_manual_model_in_db(
            conn,
            &model_id,
            &request_name,
            display_name.as_deref(),
            enabled,
        )
    })
}

#[tauri::command]
pub async fn delete_manual_model(
    state: State<'_, Arc<AppState>>,
    model_id: String,
) -> AppResult<()> {
    state
        .db
        .with_conn(|conn| delete_manual_model_in_db(conn, &model_id))
}

fn provider_type_to_db(provider_type: &ProviderType) -> &'static str {
    match provider_type {
        ProviderType::OpenaiCompat => "openai_compat",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::Ollama => "ollama",
    }
}

fn parse_provider_type(s: &str) -> ProviderType {
    match s {
        "anthropic" => ProviderType::Anthropic,
        "gemini" => ProviderType::Gemini,
        "ollama" => ProviderType::Ollama,
        _ => ProviderType::OpenaiCompat,
    }
}

fn validate_provider_config(
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

fn load_models_for_provider(
    conn: &rusqlite::Connection,
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

fn replace_synced_models_for_provider(
    conn: &rusqlite::Connection,
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

fn update_model_visibility_in_db(
    conn: &rusqlite::Connection,
    model_id: &str,
    enabled: bool,
) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE models SET is_enabled = ?1 WHERE id = ?2",
        rusqlite::params![if enabled { 1 } else { 0 }, model_id],
    )?)
}

fn update_provider_models_visibility_in_db(
    conn: &rusqlite::Connection,
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

fn ensure_provider_exists(conn: &rusqlite::Connection, provider_id: &str) -> AppResult<()> {
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

fn load_model_by_id(conn: &rusqlite::Connection, model_id: &str) -> AppResult<ModelInfo> {
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

fn ensure_manual_model(conn: &rusqlite::Connection, model_id: &str) -> AppResult<()> {
    let source: String = conn
        .query_row("SELECT source FROM models WHERE id = ?1", [model_id], |row| row.get(0))
        .map_err(|_| AppError::NotFound(format!("Model {model_id}")))?;

    if parse_model_source(&source) == ModelSource::Manual {
        Ok(())
    } else {
        Err(AppError::Provider(format!("Model {model_id} is not a manual model")))
    }
}

fn create_manual_model_in_db(
    conn: &rusqlite::Connection,
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

    // Reject duplicate model name within the same provider
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

fn update_manual_model_in_db(
    conn: &rusqlite::Connection,
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

fn delete_manual_model_in_db(conn: &rusqlite::Connection, model_id: &str) -> AppResult<()> {
    ensure_manual_model(conn, model_id)?;
    let deleted = conn.execute("DELETE FROM models WHERE id = ?1", [model_id])?;
    if deleted == 0 {
        Err(AppError::NotFound(format!("Model {model_id}")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_allows_disabled_without_key() {
        let result = validate_provider_config(&ProviderType::OpenaiCompat, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_requires_key_for_enabled_openai() {
        let result = validate_provider_config(&ProviderType::OpenaiCompat, None, true);
        assert!(result.is_err());
    }

    #[test]
    fn validate_allows_enabled_ollama_without_key() {
        let result = validate_provider_config(&ProviderType::Ollama, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn update_model_visibility_updates_single_model() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1
            );
            INSERT INTO models (id, provider_id, name, is_enabled) VALUES
              ('m1', 'p1', 'model-1', 1),
              ('m2', 'p1', 'model-2', 1),
              ('m3', 'p2', 'model-3', 1);
            ",
        )
        .unwrap();

        let updated = update_model_visibility_in_db(&conn, "m2", false).unwrap();
        assert_eq!(updated, 1);

        let enabled: i32 = conn
            .query_row("SELECT is_enabled FROM models WHERE id = 'm2'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 0);
    }

    #[test]
    fn update_provider_models_visibility_updates_all_provider_models() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1
            );
            INSERT INTO models (id, provider_id, name, is_enabled) VALUES
              ('m1', 'p1', 'model-1', 1),
              ('m2', 'p1', 'model-2', 1),
              ('m3', 'p2', 'model-3', 1);
            ",
        )
        .unwrap();

        let updated = update_provider_models_visibility_in_db(&conn, "p1", false).unwrap();
        assert_eq!(updated, 2);

        let p1_hidden: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM models WHERE provider_id = 'p1' AND is_enabled = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(p1_hidden, 2);

        let p2_enabled: i32 = conn
            .query_row(
                "SELECT is_enabled FROM models WHERE id = 'm3'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(p2_enabled, 1);
    }

    #[test]
    fn load_models_for_provider_uses_display_name_when_present() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1,
              source TEXT NOT NULL DEFAULT 'synced'
            );
            INSERT INTO models (id, provider_id, name, display_name, max_tokens, is_vision, is_enabled, source) VALUES
              ('m1', 'p1', 'gpt-4.1', 'Friendly GPT', 128000, 1, 1, 'manual');
            ",
        )
        .unwrap();

        let models = load_models_for_provider(&conn, "p1").unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "Friendly GPT");
        assert_eq!(models[0].request_name, "gpt-4.1");
        assert_eq!(models[0].display_name.as_deref(), Some("Friendly GPT"));
        assert_eq!(models[0].source, ModelSource::Manual);
    }

    #[test]
    fn load_models_for_provider_falls_back_to_request_name_when_display_name_missing_or_blank() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1,
              source TEXT NOT NULL DEFAULT 'synced'
            );
            INSERT INTO models (id, provider_id, name, display_name, max_tokens, is_vision, is_enabled, source) VALUES
              ('m1', 'p1', 'gpt-4.1', NULL, 128000, 1, 1, 'synced'),
              ('m2', 'p1', 'gpt-4.1-mini', '', 64000, 0, 1, 'synced');
            ",
        )
        .unwrap();

        let models = load_models_for_provider(&conn, "p1").unwrap();
        assert_eq!(models.len(), 2);

        assert_eq!(models[0].name, "gpt-4.1");
        assert_eq!(models[0].request_name, "gpt-4.1");
        assert_eq!(models[0].display_name, None);
        assert_eq!(models[0].source, ModelSource::Synced);

        assert_eq!(models[1].name, "gpt-4.1-mini");
        assert_eq!(models[1].request_name, "gpt-4.1-mini");
        assert_eq!(models[1].display_name.as_deref(), Some(""));
        assert_eq!(models[1].source, ModelSource::Synced);
    }

    #[test]
    fn create_manual_model_persists_request_name_and_source() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE providers (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              type TEXT NOT NULL,
              api_key TEXT,
              base_url TEXT,
              is_enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1,
              source TEXT NOT NULL DEFAULT 'synced'
            );
            INSERT INTO providers (id, name, type, base_url, is_enabled)
            VALUES ('p1', 'Test Provider', 'openai_compat', 'https://example.com/v1', 1);
            ",
        )
        .unwrap();

        let model = create_manual_model_in_db(&conn, "p1", "gpt-4.1", Some("生产模型"), true).unwrap();
        assert_eq!(model.request_name, "gpt-4.1");
        assert_eq!(model.display_name.as_deref(), Some("生产模型"));
        assert_eq!(model.source, ModelSource::Manual);
        assert_eq!(model.name, "生产模型");
        assert!(model.enabled);
    }

    #[test]
    fn update_and_delete_manual_model_only_apply_to_manual_rows() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE providers (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              type TEXT NOT NULL,
              api_key TEXT,
              base_url TEXT,
              is_enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1,
              source TEXT NOT NULL DEFAULT 'synced'
            );
            INSERT INTO providers (id, name, type, base_url, is_enabled)
            VALUES ('p1', 'Test Provider', 'openai_compat', 'https://example.com/v1', 1);
            INSERT INTO models (id, provider_id, name, display_name, is_enabled, source) VALUES
              ('manual-1', 'p1', 'gpt-4.1', 'Friendly GPT', 1, 'manual'),
              ('synced-1', 'p1', 'gpt-4o', 'GPT-4o', 1, 'synced');
            ",
        )
        .unwrap();

        let updated = update_manual_model_in_db(&conn, "manual-1", "gpt-4.1-mini", Some("轻量模型"), false).unwrap();
        assert_eq!(updated.request_name, "gpt-4.1-mini");
        assert_eq!(updated.display_name.as_deref(), Some("轻量模型"));
        assert_eq!(updated.name, "轻量模型");
        assert!(!updated.enabled);

        let update_synced = update_manual_model_in_db(&conn, "synced-1", "gpt-4.1", Some("Nope"), true);
        assert!(update_synced.is_err());

        delete_manual_model_in_db(&conn, "manual-1").unwrap();
        let remaining: i64 = conn
            .query_row("SELECT COUNT(1) FROM models WHERE id = 'manual-1'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, 0);

        let delete_synced = delete_manual_model_in_db(&conn, "synced-1");
        assert!(delete_synced.is_err());
    }

    #[test]
    fn replace_synced_models_removes_stale_unreferenced_rows_and_keeps_manual() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE providers (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              type TEXT NOT NULL,
              api_key TEXT,
              base_url TEXT,
              is_enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1,
              source TEXT NOT NULL DEFAULT 'synced'
            );
            CREATE TABLE assistants (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              model_id TEXT
            );
            CREATE TABLE conversations (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL,
              model_id TEXT
            );
            INSERT INTO providers (id, name, type, base_url, is_enabled)
            VALUES ('p1', 'Test Provider', 'openai_compat', 'https://example.com/v1', 1);
            INSERT INTO models (id, provider_id, name, display_name, is_enabled, source) VALUES
              ('stale-synced', 'p1', 'old-model', 'Old Model', 1, 'synced'),
              ('manual-1', 'p1', 'manual-model', 'Manual Model', 1, 'manual');
            ",
        )
        .unwrap();

        let fetched = vec![ModelInfo {
            id: "fresh-synced".into(),
            name: "Fresh Model".into(),
            request_name: "fresh-model".into(),
            display_name: Some("Fresh Model".into()),
            provider_id: "p1".into(),
            context_length: Some(128000),
            supports_vision: true,
            supports_streaming: true,
            enabled: true,
            source: ModelSource::Synced,
        }];

        replace_synced_models_for_provider(&conn, "p1", &fetched).unwrap();

        let rows: Vec<(String, String)> = conn
            .prepare("SELECT id, source FROM models WHERE provider_id = 'p1' ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert_eq!(
            rows,
            vec![
                ("fresh-synced".to_string(), "synced".to_string()),
                ("manual-1".to_string(), "manual".to_string()),
            ]
        );
    }

    #[test]
    fn replace_synced_models_keeps_stale_rows_when_referenced() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE providers (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              type TEXT NOT NULL,
              api_key TEXT,
              base_url TEXT,
              is_enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE models (
              id TEXT PRIMARY KEY,
              provider_id TEXT NOT NULL,
              name TEXT NOT NULL,
              display_name TEXT,
              max_tokens INTEGER,
              is_vision INTEGER NOT NULL DEFAULT 0,
              is_enabled INTEGER NOT NULL DEFAULT 1,
              source TEXT NOT NULL DEFAULT 'synced'
            );
            CREATE TABLE assistants (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              model_id TEXT
            );
            CREATE TABLE conversations (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL,
              model_id TEXT
            );
            INSERT INTO providers (id, name, type, base_url, is_enabled)
            VALUES ('p1', 'Test Provider', 'openai_compat', 'https://example.com/v1', 1);
            INSERT INTO models (id, provider_id, name, display_name, is_enabled, source) VALUES
              ('stale-conv', 'p1', 'old-conv-model', 'Old Conv Model', 1, 'synced'),
              ('stale-assistant', 'p1', 'old-assistant-model', 'Old Assistant Model', 1, 'synced');
            INSERT INTO conversations (id, title, model_id) VALUES
              ('conv-1', 'Conversation', 'stale-conv');
            INSERT INTO assistants (id, name, model_id) VALUES
              ('assistant-1', 'Assistant', 'stale-assistant');
            ",
        )
        .unwrap();

        let fetched = vec![ModelInfo {
            id: "fresh-synced".into(),
            name: "Fresh Model".into(),
            request_name: "fresh-model".into(),
            display_name: Some("Fresh Model".into()),
            provider_id: "p1".into(),
            context_length: Some(128000),
            supports_vision: true,
            supports_streaming: true,
            enabled: true,
            source: ModelSource::Synced,
        }];

        replace_synced_models_for_provider(&conn, "p1", &fetched).unwrap();

        let rows: Vec<String> = conn
            .prepare("SELECT id FROM models WHERE provider_id = 'p1' ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert_eq!(
            rows,
            vec![
                "fresh-synced".to_string(),
                "stale-assistant".to_string(),
                "stale-conv".to_string(),
            ]
        );
    }
}
