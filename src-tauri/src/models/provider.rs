use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProviderType {
    OpenaiCompat,
    Anthropic,
    Gemini,
    Ollama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub api_base: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelInfo>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub request_name: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub context_length: Option<u32>,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    pub enabled: bool,
    pub source: ModelSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelSource {
    Synced,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonParams {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default = "default_stream")]
    pub stream: bool,
}

fn default_stream() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnthropicEffort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AnthropicThinking {
    Adaptive,
    Enabled { budget_tokens: u32 },
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GeminiThinkingLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaThink {
    Bool(bool),
    Level(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider_type", rename_all = "camelCase")]
pub enum ProviderParams {
    OpenaiCompat {
        frequency_penalty: Option<f32>,
        presence_penalty: Option<f32>,
        reasoning_effort: Option<ReasoningEffort>,
        seed: Option<u64>,
        max_completion_tokens: Option<u32>,
    },
    Anthropic {
        top_k: Option<u32>,
        thinking: Option<AnthropicThinking>,
        effort: Option<AnthropicEffort>,
    },
    Gemini {
        thinking_budget: Option<u32>,
        thinking_level: Option<GeminiThinkingLevel>,
    },
    Ollama {
        think: Option<OllamaThink>,
        num_ctx: Option<u32>,
        repeat_penalty: Option<f32>,
        min_p: Option<f32>,
        keep_alive: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_params_serde_openai() {
        let params = ProviderParams::OpenaiCompat {
            frequency_penalty: Some(0.5),
            presence_penalty: Some(0.3),
            reasoning_effort: Some(ReasoningEffort::High),
            seed: Some(42),
            max_completion_tokens: Some(1024),
        };
        let json = serde_json::to_string(&params).unwrap();
        let deserialized: ProviderParams = serde_json::from_str(&json).unwrap();
        match deserialized {
            ProviderParams::OpenaiCompat {
                frequency_penalty,
                seed,
                ..
            } => {
                assert_eq!(frequency_penalty, Some(0.5));
                assert_eq!(seed, Some(42));
            }
            _ => panic!("Expected OpenaiCompat variant"),
        }
    }

    #[test]
    fn test_anthropic_thinking_adaptive() {
        let thinking = AnthropicThinking::Adaptive;
        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains("adaptive"));
        let deserialized: AnthropicThinking = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, AnthropicThinking::Adaptive));
    }

    #[test]
    fn test_ollama_think_untagged() {
        let bool_json = "true";
        let think: OllamaThink = serde_json::from_str(bool_json).unwrap();
        assert!(matches!(think, OllamaThink::Bool(true)));

        let str_json = "\"detailed\"";
        let think: OllamaThink = serde_json::from_str(str_json).unwrap();
        match think {
            OllamaThink::Level(s) => assert_eq!(s, "detailed"),
            _ => panic!("Expected Level variant"),
        }
    }
}
