use std::sync::Arc;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::{ModelInfo, ProviderConfig, ProviderType};
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
    }

    // Save to DB (upsert)
    state.db.with_conn(|conn| {
        for m in &models {
            conn.execute(
                "INSERT INTO models (id, provider_id, name, display_name, max_tokens, is_vision, is_enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    display_name = excluded.display_name,
                    max_tokens = excluded.max_tokens,
                    is_vision = excluded.is_vision",
                rusqlite::params![
                    m.id,
                    m.provider_id,
                    m.name,
                    m.name,
                    m.context_length,
                    m.supports_vision as i32,
                ],
            )?;
        }
        Ok(())
    })?;

    Ok(models)
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
        "SELECT id, name, provider_id, max_tokens, is_vision FROM models WHERE provider_id = ?1",
    )?;
    let rows = stmt.query_map([provider_id], |row| {
        Ok(ModelInfo {
            id: row.get(0)?,
            name: row.get(1)?,
            provider_id: row.get(2)?,
            context_length: row.get(3)?,
            supports_vision: row.get::<_, i32>(4)? != 0,
            supports_streaming: true,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
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
}
