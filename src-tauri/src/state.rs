use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{watch, Mutex};
use serde::Serialize;

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::ProviderType;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::gemini::GeminiProvider;
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::Provider;

/// Stream chunk types for SSE
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamChunk {
    Content { content: String },
    Done { prompt_tokens: u32, completion_tokens: u32 },
    Error { message: String },
}

/// Background generation task metadata
pub struct GenerationTask {
    pub message_id: String,
    pub conversation_id: String,
    pub task_handle: tokio::task::JoinHandle<()>,
    pub progress_tx: watch::Sender<StreamChunk>,
    pub started_at: String,
}

pub struct AppState {
    pub db: Database,
    pub data_dir: PathBuf,
    pub providers: Mutex<HashMap<String, Arc<dyn Provider>>>,
    /// Per-conversation cancel tokens: conversation_id -> Sender
    pub cancel_tokens: Mutex<HashMap<String, watch::Sender<bool>>>,
    /// "system" | "none"
    pub proxy_mode: Mutex<String>,
    /// Background generation tasks: message_id -> GenerationTask
    pub generation_tasks: Mutex<HashMap<String, GenerationTask>>,
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
            generation_tasks: Mutex::new(HashMap::new()),
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

    /// Get a progress receiver for a generation task (for SSE subscription)
    pub async fn subscribe_to_generation(&self, message_id: &str) -> Option<watch::Receiver<StreamChunk>> {
        self.generation_tasks
            .lock()
            .await
            .get(message_id)
            .map(|task| task.progress_tx.subscribe())
    }

    /// Register a new generation task
    pub async fn register_generation_task(
        &self,
        message_id: String,
        conversation_id: String,
        task_handle: tokio::task::JoinHandle<()>,
        progress_tx: watch::Sender<StreamChunk>,
        started_at: String,
    ) {
        let task = GenerationTask {
            message_id: message_id.clone(),
            conversation_id,
            task_handle,
            progress_tx,
            started_at,
        };
        self.generation_tasks.lock().await.insert(message_id, task);
    }

    /// Remove a generation task (called when task completes)
    pub async fn remove_generation_task(&self, message_id: &str) {
        self.generation_tasks.lock().await.remove(message_id);
    }

    /// Check if a generation task exists
    pub async fn has_generation_task(&self, message_id: &str) -> bool {
        self.generation_tasks.lock().await.contains_key(message_id)
    }
}
