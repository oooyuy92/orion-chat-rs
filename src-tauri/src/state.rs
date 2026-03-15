use std::collections::HashMap;
use std::path::PathBuf;
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
    pub data_dir: PathBuf,
    pub providers: Mutex<HashMap<String, Arc<dyn Provider>>>,
    /// Per-conversation cancel tokens: conversation_id -> Sender
    pub cancel_tokens: Mutex<HashMap<String, watch::Sender<bool>>>,
    /// "system" | "none"
    pub proxy_mode: Mutex<String>,
}

impl AppState {
    pub fn new(db_path: &str, data_dir: impl Into<PathBuf>) -> AppResult<Self> {
        let db = Database::new(db_path)?;
        let data_dir = data_dir.into();
        Ok(Self {
            db,
            data_dir,
            providers: Mutex::new(HashMap::new()),
            cancel_tokens: Mutex::new(HashMap::new()),
            proxy_mode: Mutex::new("system".to_string()),
        })
    }

    /// Create a cancel token for a conversation, returning the receiver.
    pub async fn create_cancel_token(&self, conversation_id: &str) -> watch::Receiver<bool> {
        let (tx, rx) = watch::channel(false);
        self.cancel_tokens
            .lock()
            .await
            .insert(conversation_id.to_string(), tx);
        rx
    }

    /// Cancel a specific conversation's stream.
    pub async fn cancel_conversation(&self, conversation_id: &str) {
        if let Some(tx) = self.cancel_tokens.lock().await.get(conversation_id) {
            let _ = tx.send(true);
        }
    }

    /// Remove the cancel token for a conversation (call after stream ends).
    pub async fn remove_cancel_token(&self, conversation_id: &str) {
        self.cancel_tokens
            .lock()
            .await
            .remove(conversation_id);
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
