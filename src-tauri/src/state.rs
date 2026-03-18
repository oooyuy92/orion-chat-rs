use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{oneshot, watch, Mutex};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{AuthAction, PermissionLevel, ProviderType};
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
    /// tool_call_id -> authorization response sender
    pub pending_auth: Mutex<HashMap<String, oneshot::Sender<AuthAction>>>,
    /// tool_call_id -> conversation_id for scoped cleanup
    pub pending_auth_conversations: Mutex<HashMap<String, String>>,
    /// (conversation_id, tool_name) -> session-level permission override
    pub session_tool_overrides: Mutex<HashMap<(String, String), PermissionLevel>>,
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
            pending_auth: Mutex::new(HashMap::new()),
            pending_auth_conversations: Mutex::new(HashMap::new()),
            session_tool_overrides: Mutex::new(HashMap::new()),
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

    pub async fn resolve_auth(&self, tool_call_id: &str, action: AuthAction) -> bool {
        self.pending_auth_conversations
            .lock()
            .await
            .remove(tool_call_id);
        if let Some(tx) = self.pending_auth.lock().await.remove(tool_call_id) {
            tx.send(action).is_ok()
        } else {
            false
        }
    }

    pub async fn register_pending_auth(
        &self,
        conversation_id: &str,
        tool_call_id: &str,
        tx: oneshot::Sender<AuthAction>,
    ) {
        self.pending_auth
            .lock()
            .await
            .insert(tool_call_id.to_string(), tx);
        self.pending_auth_conversations
            .lock()
            .await
            .insert(tool_call_id.to_string(), conversation_id.to_string());
    }

    pub async fn clear_pending_auth_for_conversation(&self, conversation_id: &str) {
        let tool_call_ids = {
            let owners = self.pending_auth_conversations.lock().await;
            owners
                .iter()
                .filter(|(_, owner_conv_id)| *owner_conv_id == conversation_id)
                .map(|(tool_call_id, _)| tool_call_id.clone())
                .collect::<Vec<_>>()
        };

        let mut pending = self.pending_auth.lock().await;
        let mut owners = self.pending_auth_conversations.lock().await;
        for tool_call_id in tool_call_ids {
            pending.remove(&tool_call_id);
            owners.remove(&tool_call_id);
        }
    }

    pub async fn set_session_override(
        &self,
        conversation_id: &str,
        tool_name: &str,
        level: PermissionLevel,
    ) {
        self.session_tool_overrides.lock().await.insert(
            (conversation_id.to_string(), tool_name.to_string()),
            level,
        );
    }

    pub async fn get_session_override(
        &self,
        conversation_id: &str,
        tool_name: &str,
    ) -> Option<PermissionLevel> {
        self.session_tool_overrides
            .lock()
            .await
            .get(&(conversation_id.to_string(), tool_name.to_string()))
            .cloned()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        AppState::new(":memory:", "/tmp").unwrap()
    }

    #[tokio::test]
    async fn test_session_override_round_trip() {
        let state = test_state();

        assert_eq!(state.get_session_override("conv-1", "bash").await, None);

        state
            .set_session_override("conv-1", "bash", PermissionLevel::Auto)
            .await;

        assert_eq!(
            state.get_session_override("conv-1", "bash").await,
            Some(PermissionLevel::Auto)
        );
    }

    #[tokio::test]
    async fn test_resolve_auth_sends_response_and_clears_pending_entry() {
        let state = test_state();
        let (tx, rx) = oneshot::channel();

        state
            .pending_auth
            .lock()
            .await
            .insert("tool-1".to_string(), tx);
        state
            .pending_auth_conversations
            .lock()
            .await
            .insert("tool-1".to_string(), "conv-1".to_string());

        assert!(state.resolve_auth("tool-1", AuthAction::Allow).await);
        assert_eq!(rx.await.unwrap(), AuthAction::Allow);
        assert!(state.pending_auth.lock().await.is_empty());
        assert!(state.pending_auth_conversations.lock().await.is_empty());
        assert!(!state.resolve_auth("tool-1", AuthAction::Deny).await);
    }
}
