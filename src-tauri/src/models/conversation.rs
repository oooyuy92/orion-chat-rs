use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub assistant_id: Option<String>,
    pub model_id: Option<String>,
    pub is_pinned: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}
