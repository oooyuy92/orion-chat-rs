use serde::{Deserialize, Serialize};

use super::provider::{CommonParams, ProviderParams};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MessageStatus {
    Streaming,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: Role,
    pub content: String,
    pub reasoning: Option<String>,
    pub model_id: Option<String>,
    pub status: MessageStatus,
    pub token_count: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ChatEvent {
    Started { message_id: String },
    Delta { content: String },
    Reasoning { content: String },
    Usage { prompt_tokens: u32, completion_tokens: u32 },
    Finished { message_id: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub common: CommonParams,
    pub provider_params: ProviderParams,
}
