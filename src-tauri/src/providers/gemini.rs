use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;

use super::Provider;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    fn build_body(&self, request: &ChatRequest) -> Value {
        // Extract system instruction
        let system_text: Option<String> = request
            .messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.clone())
            .reduce(|a, b| format!("{}\n{}", a, b));

        // Build contents array (no system messages, assistant -> "model")
        let contents: Vec<Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::User => "user",
                        Role::Assistant => "model",
                        _ => unreachable!(),
                    },
                    "parts": [{"text": m.content}],
                })
            })
            .collect();

        let mut body = json!({ "contents": contents });
        let obj = body.as_object_mut().unwrap();

        if let Some(system) = system_text {
            obj.insert(
                "systemInstruction".into(),
                json!({ "parts": [{"text": system}] }),
            );
        }

        // generationConfig
        let mut gen_config = json!({});
        let gc = gen_config.as_object_mut().unwrap();

        if let Some(temp) = request.common.temperature {
            gc.insert("temperature".into(), json!(temp));
        }
        if let Some(top_p) = request.common.top_p {
            gc.insert("topP".into(), json!(top_p));
        }
        if let Some(max_tokens) = request.common.max_tokens {
            gc.insert("maxOutputTokens".into(), json!(max_tokens));
        }

        // Gemini-specific params
        if let ProviderParams::Gemini {
            thinking_budget,
            thinking_level,
        } = &request.provider_params
        {
            if thinking_budget.is_some() || thinking_level.is_some() {
                let mut thinking_config = json!({});
                let tc = thinking_config.as_object_mut().unwrap();

                if let Some(budget) = thinking_budget {
                    tc.insert("thinkingBudget".into(), json!(budget));
                }
                if let Some(level) = thinking_level {
                    let level_str = match level {
                        GeminiThinkingLevel::Low => "LOW",
                        GeminiThinkingLevel::Medium => "MEDIUM",
                        GeminiThinkingLevel::High => "HIGH",
                    };
                    tc.insert("thinkingLevel".into(), json!(level_str));
                }

                gc.insert("thinkingConfig".into(), thinking_config);
            }
        }

        if !gc.is_empty() {
            obj.insert("generationConfig".into(), gen_config);
        }

        body
    }

    fn handle_sse_data(&self, data: &str, channel: &Channel<ChatEvent>) -> AppResult<()> {
        let json: Value = serde_json::from_str(data)?;

        // Extract text and reasoning from candidates
        if let Some(candidates) = json["candidates"].as_array() {
            for candidate in candidates {
                if let Some(parts) = candidate["content"]["parts"].as_array() {
                    for part in parts {
                        if let Some(text) = part["text"].as_str() {
                            if !text.is_empty() {
                                // thought:true means reasoning
                                if part.get("thought") == Some(&json!(true)) {
                                    let _ = channel.send(ChatEvent::Reasoning {
                                        content: text.to_string(),
                                    });
                                } else {
                                    let _ = channel.send(ChatEvent::Delta {
                                        content: text.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Usage metadata
        if let Some(usage) = json.get("usageMetadata") {
            let prompt = usage["promptTokenCount"].as_u64().unwrap_or(0) as u32;
            let completion = usage["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;
            if prompt > 0 || completion > 0 {
                let _ = channel.send(ChatEvent::Usage {
                    prompt_tokens: prompt,
                    completion_tokens: completion,
                });
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        mut cancel: watch::Receiver<bool>,
    ) -> AppResult<()> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            request.model, self.api_key
        );
        let body = self.build_body(&request);

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!(
                "API returned {}: {}",
                status, text
            )));
        }

        let mut stream = response.bytes_stream();
        let mut buf = String::new();

        loop {
            tokio::select! {
                maybe_chunk = stream.next() => {
                    match maybe_chunk {
                        Some(Ok(chunk)) => {
                            buf.push_str(&String::from_utf8_lossy(&chunk));

                            while let Some(pos) = buf.find("\n\n") {
                                let event_block = buf[..pos].to_string();
                                buf = buf[pos + 2..].to_string();

                                for line in event_block.lines() {
                                    let line = line.trim();
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        self.handle_sse_data(data, &channel)?;
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => return Err(AppError::Http(e)),
                        None => break,
                    }
                }
                _ = cancel.changed() => {
                    if *cancel.borrow() {
                        let _ = channel.send(ChatEvent::Error {
                            message: "Cancelled".into(),
                        });
                        return Err(AppError::Cancelled);
                    }
                }
            }
        }

        Ok(())
    }

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            self.api_key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!(
                "API returned {}: {}",
                status, text
            )));
        }

        let body: Value = response.json().await?;
        let models = body["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| {
                let name = m["name"].as_str()?;
                // Strip "models/" prefix
                let id = name.strip_prefix("models/").unwrap_or(name);
                // Only include gemini models
                if !id.contains("gemini") {
                    return None;
                }
                Some(ModelInfo {
                    name: m["displayName"]
                        .as_str()
                        .unwrap_or(id)
                        .to_string(),
                    id: id.to_string(),
                    provider_id: String::new(),
                    context_length: m["inputTokenLimit"].as_u64().map(|v| v as u32),
                    supports_vision: false,
                    supports_streaming: true,
                })
            })
            .collect();

        Ok(models)
    }

    async fn validate(&self) -> AppResult<bool> {
        self.list_models().await?;
        Ok(true)
    }
}