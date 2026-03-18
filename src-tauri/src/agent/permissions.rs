use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;
use yoagent::{AgentTool, ToolContext, ToolError, ToolResult};

use crate::models::{AuthAction, ChatEvent, PermissionLevel, ToolPermissions};
use crate::state::AppState;

pub type ChatEventEmitter = Arc<dyn Fn(ChatEvent) + Send + Sync>;

pub struct PermissionedTool {
    inner: Box<dyn AgentTool>,
    state: Arc<AppState>,
    conversation_id: String,
    permissions: ToolPermissions,
    emit_event: ChatEventEmitter,
    auth_timeout: Duration,
}

impl PermissionedTool {
    pub fn new(
        inner: Box<dyn AgentTool>,
        state: Arc<AppState>,
        conversation_id: impl Into<String>,
        permissions: ToolPermissions,
        emit_event: ChatEventEmitter,
    ) -> Self {
        Self {
            inner,
            state,
            conversation_id: conversation_id.into(),
            permissions,
            emit_event,
            auth_timeout: Duration::from_secs(300),
        }
    }

    #[cfg(test)]
    fn with_timeout(mut self, auth_timeout: Duration) -> Self {
        self.auth_timeout = auth_timeout;
        self
    }

    async fn authorization_level(&self, tool_name: &str) -> PermissionLevel {
        self.state
            .get_session_override(&self.conversation_id, tool_name)
            .await
            .unwrap_or_else(|| {
                self.permissions
                    .0
                    .get(tool_name)
                    .cloned()
                    .unwrap_or(PermissionLevel::Ask)
            })
    }

    async fn await_authorization(
        &self,
        ctx: &ToolContext,
        params: &serde_json::Value,
    ) -> Result<(), ToolError> {
        let tool_call_id = ctx.tool_call_id.clone();
        let tool_name = self.name().to_string();
        let args = params.to_string();
        let (tx, rx) = oneshot::channel();

        self.state
            .register_pending_auth(&self.conversation_id, &tool_call_id, tx)
            .await;

        (self.emit_event)(ChatEvent::ToolAuthRequest {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            args,
        });

        match tokio::time::timeout(self.auth_timeout, rx).await {
            Ok(Ok(AuthAction::Allow)) => Ok(()),
            Ok(Ok(AuthAction::AllowSession)) => {
                self.state
                    .set_session_override(
                        &self.conversation_id,
                        &tool_name,
                        PermissionLevel::Auto,
                    )
                    .await;
                Ok(())
            }
            Ok(Ok(AuthAction::Deny)) => Err(ToolError::Failed("User denied tool execution".into())),
            Ok(Err(_)) => Err(ToolError::Failed("Authorization channel closed".into())),
            Err(_) => {
                self.state.pending_auth.lock().await.remove(&tool_call_id);
                self.state
                    .pending_auth_conversations
                    .lock()
                    .await
                    .remove(&tool_call_id);
                Err(ToolError::Failed("Tool authorization timed out".into()))
            }
        }
    }
}

#[async_trait::async_trait]
impl AgentTool for PermissionedTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        match self.authorization_level(self.name()).await {
            PermissionLevel::Auto => {}
            PermissionLevel::Deny => {
                return Err(ToolError::Failed(format!(
                    "Tool '{}' is blocked by permission policy",
                    self.name()
                )));
            }
            PermissionLevel::Ask => self.await_authorization(&ctx, &params).await?,
        }

        self.inner.execute(params, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use yoagent::tools::ReadFileTool;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState::new(":memory:", "/tmp").unwrap())
    }

    fn test_emitter(events: Arc<StdMutex<Vec<ChatEvent>>>) -> ChatEventEmitter {
        Arc::new(move |event| {
            events.lock().unwrap().push(event);
        })
    }

    fn test_ctx() -> ToolContext {
        ToolContext {
            tool_call_id: "tool-call-1".into(),
            tool_name: "read_file".into(),
            cancel: tokio_util::sync::CancellationToken::new(),
            on_update: None,
            on_progress: None,
        }
    }

    async fn write_temp_file(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("orion-agent-test-{}.txt", uuid::Uuid::new_v4()));
        tokio::fs::write(&path, contents).await.unwrap();
        path.display().to_string()
    }

    #[tokio::test]
    async fn test_auto_permission_executes_inner_tool() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let tool = PermissionedTool::new(
            Box::new(ReadFileTool::new()),
            test_state(),
            "conv-1",
            ToolPermissions::with_defaults(),
            test_emitter(events.clone()),
        );

        let path = write_temp_file("hello").await;

        let result = tool
            .execute(serde_json::json!({ "path": path }), test_ctx())
            .await
            .unwrap();

        assert!(matches!(&result.content[0], yoagent::Content::Text { text } if text.contains("hello")));
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_deny_permission_blocks_execution() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let mut permissions = ToolPermissions::with_defaults();
        permissions
            .0
            .insert("read_file".to_string(), PermissionLevel::Deny);

        let tool = PermissionedTool::new(
            Box::new(ReadFileTool::new()),
            test_state(),
            "conv-1",
            permissions,
            test_emitter(events),
        );

        let err = tool
            .execute(serde_json::json!({ "path": "missing.txt" }), test_ctx())
            .await
            .unwrap_err();

        assert!(err.to_string().contains("blocked by permission policy"));
    }

    #[tokio::test]
    async fn test_allow_session_persists_session_override() {
        let state = test_state();
        let events = Arc::new(StdMutex::new(Vec::new()));
        let mut permissions = ToolPermissions::with_defaults();
        permissions
            .0
            .insert("read_file".to_string(), PermissionLevel::Ask);

        let tool = PermissionedTool::new(
            Box::new(ReadFileTool::new()),
            state.clone(),
            "conv-1",
            permissions,
            test_emitter(events.clone()),
        )
        .with_timeout(Duration::from_secs(1));

        let path = write_temp_file("hello").await;
        let task = tokio::spawn({
            let path = path.clone();
            async move {
                tool.execute(serde_json::json!({ "path": path }), test_ctx())
                    .await
                    .unwrap()
            }
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(state
            .resolve_auth("tool-call-1", AuthAction::AllowSession)
            .await);

        let _ = task.await.unwrap();
        assert_eq!(
            state.get_session_override("conv-1", "read_file").await,
            Some(PermissionLevel::Auto)
        );
        assert!(matches!(
            &events.lock().unwrap()[0],
            ChatEvent::ToolAuthRequest { tool_name, .. } if tool_name == "read_file"
        ));
    }
}
