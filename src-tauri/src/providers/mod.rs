pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;

use async_trait::async_trait;
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::AppResult;
use crate::models::{ChatEvent, ChatRequest, ModelInfo};

#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        cancel: watch::Receiver<bool>,
    ) -> AppResult<()>;

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>>;

    async fn validate(&self) -> AppResult<bool>;
}
