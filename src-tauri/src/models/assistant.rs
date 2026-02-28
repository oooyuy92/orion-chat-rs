use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub extra_params: serde_json::Value,
    pub sort_order: i32,
    pub created_at: String,
}
