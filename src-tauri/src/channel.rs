use std::sync::Arc;

use crate::models::ChatEvent;

/// A channel for sending chat events from providers to callers.
/// Wraps a plain callback so providers don't depend on Tauri's IPC types.
#[derive(Clone)]
pub struct ChatEventSender {
    inner: Arc<dyn Fn(ChatEvent) + Send + Sync>,
}

impl ChatEventSender {
    pub fn new<F: Fn(ChatEvent) + Send + Sync + 'static>(f: F) -> Self {
        Self {
            inner: Arc::new(f),
        }
    }

    pub fn send(&self, event: ChatEvent) {
        (self.inner)(event);
    }
}

/// In desktop builds, convert from Tauri's Channel<ChatEvent> into ChatEventSender.
#[cfg(feature = "desktop")]
impl From<tauri::ipc::Channel<ChatEvent>> for ChatEventSender {
    fn from(channel: tauri::ipc::Channel<ChatEvent>) -> Self {
        Self::new(move |event| {
            let _ = channel.send(event);
        })
    }
}
