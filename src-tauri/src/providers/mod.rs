pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;

use async_trait::async_trait;
use tokio::sync::watch;

use crate::channel::ChatEventSender;
use crate::error::AppResult;
use crate::models::{ChatRequest, ModelInfo};

/// Accumulated result from a streaming chat completion.
#[derive(Debug, Clone, Default)]
pub struct StreamResult {
    pub content: String,
    pub reasoning: Option<String>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        message_id: String,
        channel: ChatEventSender,
        cancel: watch::Receiver<bool>,
    ) -> AppResult<StreamResult>;

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>>;

    async fn validate(&self) -> AppResult<bool>;
}
