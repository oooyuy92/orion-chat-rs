use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchSidebarResult {
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub snippet: String,
    pub created_at: String,
}
