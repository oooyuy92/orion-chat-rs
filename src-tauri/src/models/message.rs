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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
    #[default]
    Text,
    ToolCall,
    ToolResult,
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
    pub version_group_id: Option<String>,
    pub version_number: u32,
    pub total_versions: u32,
    #[serde(default)]
    pub message_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<String>,
    #[serde(default)]
    pub tool_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedMessages {
    pub messages: Vec<Message>,
    pub has_more: bool,
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
    Delta { message_id: String, content: String },
    Reasoning { message_id: String, content: String },
    Usage { message_id: String, prompt_tokens: u32, completion_tokens: u32 },
    Finished { message_id: String },
    Error { message_id: String, message: String },
    ToolCallStart {
        message_id: String,
        tool_call_id: String,
        tool_name: String,
        args: String,
    },
    ToolCallUpdate {
        message_id: String,
        tool_call_id: String,
        partial_result: String,
    },
    ToolCallEnd {
        message_id: String,
        tool_call_id: String,
        result: String,
        is_error: bool,
    },
    ToolAuthRequest {
        tool_call_id: String,
        tool_name: String,
        args: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub common: CommonParams,
    pub provider_params: ProviderParams,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_defaults_to_text_when_missing() {
        let message: Message = serde_json::from_value(serde_json::json!({
            "id": "m1",
            "conversationId": "c1",
            "role": "assistant",
            "content": "hello",
            "reasoning": null,
            "modelId": null,
            "status": "done",
            "tokenCount": null,
            "createdAt": "2026-03-18T00:00:00Z",
            "versionGroupId": null,
            "versionNumber": 1,
            "totalVersions": 1
        }))
        .unwrap();

        assert_eq!(message.message_type, MessageType::Text);
        assert_eq!(message.tool_call_id, None);
        assert_eq!(message.tool_name, None);
        assert_eq!(message.tool_input, None);
        assert!(!message.tool_error);
    }
}
