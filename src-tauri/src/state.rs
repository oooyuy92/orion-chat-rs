use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{watch, Mutex};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::ProviderType;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::gemini::GeminiProvider;
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::Provider;

pub struct AppState {
    pub db: Database,
    pub providers: Mutex<HashMap<String, Arc<dyn Provider>>>,
    pub cancel_sender: watch::Sender<bool>,
    pub cancel_receiver: watch::Receiver<bool>,
    /// "system" | "none"
    pub proxy_mode: Mutex<String>,
}

impl AppState {
    pub fn new(db_path: &str) -> AppResult<Self> {
        let db = Database::new(db_path)?;
        let (cancel_sender, cancel_receiver) = watch::channel(false);
        Ok(Self {
            db,
            providers: Mutex::new(HashMap::new()),
            cancel_sender,
            cancel_receiver,
            proxy_mode: Mutex::new("system".to_string()),
        })
    }

    pub async fn register_provider(
        &self,
        id: &str,
        provider_type: &ProviderType,
        api_key: Option<&str>,
        api_base: Option<&str>,
    ) -> AppResult<()> {
        let provider: Arc<dyn Provider> = match provider_type {
            ProviderType::OpenaiCompat => {
                let key = api_key
                    .ok_or_else(|| AppError::Provider("API key required for OpenAI".into()))?;
                let base = api_base.unwrap_or("https://api.openai.com");
                Arc::new(OpenAICompatProvider::new(
                    key.to_string(),
                    base.to_string(),
                ))
            }
            ProviderType::Anthropic => {
                let key = api_key
                    .ok_or_else(|| AppError::Provider("API key required for Anthropic".into()))?;
                Arc::new(AnthropicProvider::new(
                    key.to_string(),
                    api_base.map(|s| s.to_string()),
                ))
            }
            ProviderType::Gemini => {
                let key = api_key
                    .ok_or_else(|| AppError::Provider("API key required for Gemini".into()))?;
                Arc::new(GeminiProvider::new(
                    key.to_string(),
                    api_base.map(|s| s.to_string()),
                ))
            }
            ProviderType::Ollama => Arc::new(OllamaProvider::new(
                api_base.map(|s| s.to_string()),
            )),
        };

        self.providers
            .lock()
            .await
            .insert(id.to_string(), provider);
        Ok(())
    }

    pub async fn get_provider(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.lock().await.get(id).cloned()
    }

    pub async fn unregister_provider(&self, id: &str) {
        self.providers.lock().await.remove(id);
    }
}
