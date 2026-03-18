use std::sync::Arc;

use yoagent::provider::{
    AnthropicProvider, GoogleProvider, ModelConfig, OpenAiCompat, OpenAiCompatProvider,
    StreamProvider,
};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::ProviderType;

pub struct AgentProviderConfig {
    pub provider: Arc<dyn StreamProvider>,
    pub api_key: String,
    pub model: String,
    pub model_config: ModelConfig,
}

pub async fn build_provider_config(
    db: &Database,
    model_id: &str,
) -> AppResult<AgentProviderConfig> {
    let (provider_type, api_key, base_url, model_name, display_name, max_tokens, supports_thinking) =
        db.with_conn(|conn| {
            conn.query_row(
                "SELECT p.type,
                        COALESCE(p.api_key, ''),
                        COALESCE(p.base_url, ''),
                        m.name,
                        COALESCE(NULLIF(m.display_name, ''), m.name),
                        COALESCE(m.max_tokens, 4096),
                        COALESCE(m.supports_thinking, 0)
                 FROM models m
                 JOIN providers p ON p.id = m.provider_id
                 WHERE m.id = ?1
                   AND m.is_enabled = 1
                   AND p.is_enabled = 1",
                [model_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, u32>(5)?,
                        row.get::<_, i64>(6)? != 0,
                    ))
                },
            )
            .map_err(|_| AppError::NotFound(format!("Model {model_id}")))
        })?;

    let provider_type = match provider_type.as_str() {
        "anthropic" => ProviderType::Anthropic,
        "gemini" => ProviderType::Gemini,
        "ollama" => ProviderType::Ollama,
        _ => ProviderType::OpenaiCompat,
    };

    let config = match provider_type {
        ProviderType::Anthropic => {
            let mut model_config = ModelConfig::anthropic(&model_name, &display_name);
            if !base_url.trim().is_empty() {
                model_config.base_url = base_url.clone();
            }
            model_config.reasoning = supports_thinking;
            model_config.max_tokens = max_tokens;
            AgentProviderConfig {
                provider: Arc::new(AnthropicProvider),
                api_key,
                model: model_name,
                model_config,
            }
        }
        ProviderType::Gemini => {
            let mut model_config = ModelConfig::google(&model_name, &display_name);
            if !base_url.trim().is_empty() {
                model_config.base_url = base_url.clone();
            }
            model_config.reasoning = supports_thinking;
            model_config.max_tokens = max_tokens;
            AgentProviderConfig {
                provider: Arc::new(GoogleProvider),
                api_key,
                model: model_name,
                model_config,
            }
        }
        ProviderType::Ollama => {
            let base = if base_url.trim().is_empty() {
                "http://localhost:11434/v1".to_string()
            } else {
                base_url.clone()
            };
            let mut model_config = ModelConfig::local(base, &model_name);
            model_config.name = display_name;
            model_config.reasoning = supports_thinking;
            model_config.max_tokens = max_tokens;
            AgentProviderConfig {
                provider: Arc::new(OpenAiCompatProvider),
                api_key,
                model: model_name,
                model_config,
            }
        }
        ProviderType::OpenaiCompat => {
            let mut model_config = ModelConfig::openai(&model_name, &display_name);
            if !base_url.trim().is_empty() {
                model_config.base_url = base_url.clone();
                model_config.compat = Some(OpenAiCompat::default());
            }
            model_config.reasoning = supports_thinking;
            model_config.max_tokens = max_tokens;
            AgentProviderConfig {
                provider: Arc::new(OpenAiCompatProvider),
                api_key,
                model: model_name,
                model_config,
            }
        }
    };

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoagent::provider::ApiProtocol;

    fn setup_db() -> Database {
        Database::new(":memory:").unwrap()
    }

    fn seed_model(
        db: &Database,
        provider_type: &str,
        api_key: Option<&str>,
        base_url: Option<&str>,
        model_id: &str,
        model_name: &str,
        display_name: &str,
    ) {
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO providers (id, name, type, api_key, base_url, is_enabled)
                 VALUES ('provider-1', 'Provider', ?1, ?2, ?3, 1)",
                rusqlite::params![provider_type, api_key, base_url],
            )?;
            conn.execute(
                "INSERT INTO models (id, provider_id, name, display_name, max_tokens, supports_thinking, is_enabled, source)
                 VALUES (?1, 'provider-1', ?2, ?3, 8192, 1, 1, 'manual')",
                rusqlite::params![model_id, model_name, display_name],
            )?;
            Ok(())
        })
        .unwrap();
    }

    #[tokio::test]
    async fn test_build_provider_config_for_openai_compat() {
        let db = setup_db();
        seed_model(
            &db,
            "openai_compat",
            Some("sk-test"),
            Some("https://example.test/v1"),
            "model-1",
            "gpt-test",
            "GPT Test",
        );

        let result = build_provider_config(&db, "model-1").await.unwrap();

        assert_eq!(result.api_key, "sk-test");
        assert_eq!(result.model, "gpt-test");
        assert_eq!(result.model_config.api, ApiProtocol::OpenAiCompletions);
        assert_eq!(result.model_config.base_url, "https://example.test/v1");
        assert_eq!(result.model_config.name, "GPT Test");
        assert_eq!(result.model_config.max_tokens, 8192);
        assert!(result.model_config.reasoning);
    }

    #[tokio::test]
    async fn test_build_provider_config_for_gemini_uses_default_base_url() {
        let db = setup_db();
        seed_model(
            &db,
            "gemini",
            Some("gem-key"),
            None,
            "model-2",
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
        );

        let result = build_provider_config(&db, "model-2").await.unwrap();

        assert_eq!(result.api_key, "gem-key");
        assert_eq!(result.model_config.api, ApiProtocol::GoogleGenerativeAi);
        assert_eq!(
            result.model_config.base_url,
            "https://generativelanguage.googleapis.com"
        );
    }
}
