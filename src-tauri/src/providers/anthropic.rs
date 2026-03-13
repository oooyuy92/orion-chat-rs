use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;

use super::{Provider, StreamResult};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let base_url = base_url
            .unwrap_or_else(|| "https://api.anthropic.com".to_string())
            .trim_end_matches('/')
            .to_string();
        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }

    fn build_body(&self, request: &ChatRequest) -> Value {
        // Extract system prompt from messages
        let system_text: Option<String> = request
            .messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.clone())
            .reduce(|a, b| format!("{}\n{}", a, b));

        // Build messages array without system messages
        let messages: Vec<Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::User => "user",
                        _ => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        let obj = body.as_object_mut().unwrap();

        if let Some(system) = system_text {
            obj.insert("system".into(), json!(system));
        }

        if let Some(max_tokens) = request.common.max_tokens {
            obj.insert("max_tokens".into(), json!(max_tokens));
        } else {
            // Anthropic requires max_tokens
            obj.insert("max_tokens".into(), json!(4096));
        }

        if let Some(top_p) = request.common.top_p {
            obj.insert("top_p".into(), json!(top_p));
        }

        // Anthropic-specific params
        let mut has_thinking = false;
        if let ProviderParams::Anthropic {
            top_k,
            thinking,
            effort,
        } = &request.provider_params
        {
            if let Some(k) = top_k {
                obj.insert("top_k".into(), json!(k));
            }

            if let Some(thinking_config) = thinking {
                match thinking_config {
                    AnthropicThinking::Enabled { budget_tokens } => {
                        has_thinking = true;
                        obj.insert(
                            "thinking".into(),
                            json!({
                                "type": "enabled",
                                "budget_tokens": budget_tokens,
                            }),
                        );
                    }
                    AnthropicThinking::Adaptive => {
                        has_thinking = true;
                        let mut thinking_obj = json!({
                            "type": "enabled",
                            "budget_tokens": 10000,
                        });
                        if let Some(eff) = effort {
                            let effort_str = match eff {
                                AnthropicEffort::Low => "low",
                                AnthropicEffort::Medium => "medium",
                                AnthropicEffort::High => "high",
                            };
                            thinking_obj = json!({
                                "type": "enabled",
                                "budget_tokens": match eff {
                                    AnthropicEffort::Low => 2000,
                                    AnthropicEffort::Medium => 5000,
                                    AnthropicEffort::High => 10000,
                                },
                            });
                            let _ = effort_str; // effort mapped via budget
                        }
                        obj.insert("thinking".into(), thinking_obj);
                    }
                    AnthropicThinking::Disabled => {}
                }
            }
        }

        // When thinking is enabled, temperature must not be set
        if !has_thinking {
            if let Some(temp) = request.common.temperature {
                obj.insert("temperature".into(), json!(temp));
            }
        }

        body
    }

    fn handle_sse_event(
        &self,
        message_id: &str,
        event_type: &str,
        data: &str,
        channel: &Channel<ChatEvent>,
        acc: &mut StreamResult,
    ) -> AppResult<()> {
        let json: Value = serde_json::from_str(data)?;

        match event_type {
            "content_block_delta" => {
                if let Some(delta) = json.get("delta") {
                    match delta["type"].as_str() {
                        Some("text_delta") => {
                            if let Some(text) = delta["text"].as_str() {
                                if !text.is_empty() {
                                    acc.content.push_str(text);
                                    let _ = channel.send(ChatEvent::Delta {
                                        message_id: message_id.to_string(),
                                        content: text.to_string(),
                                    });
                                }
                            }
                        }
                        Some("thinking_delta") => {
                            if let Some(thinking) = delta["thinking"].as_str() {
                                if !thinking.is_empty() {
                                    acc.reasoning
                                        .get_or_insert_with(String::new)
                                        .push_str(thinking);
                                    let _ = channel.send(ChatEvent::Reasoning {
                                        message_id: message_id.to_string(),
                                        content: thinking.to_string(),
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            "message_start" => {
                if let Some(usage) = json.get("message").and_then(|m| m.get("usage")) {
                    if let Some(input) = usage["input_tokens"].as_u64() {
                        acc.prompt_tokens = input as u32;
                        let _ = channel.send(ChatEvent::Usage {
                            message_id: message_id.to_string(),
                            prompt_tokens: input as u32,
                            completion_tokens: 0,
                        });
                    }
                }
            }
            "message_delta" => {
                if let Some(usage) = json.get("usage") {
                    if let Some(output) = usage["output_tokens"].as_u64() {
                        acc.completion_tokens = output as u32;
                        let _ = channel.send(ChatEvent::Usage {
                            message_id: message_id.to_string(),
                            prompt_tokens: 0,
                            completion_tokens: output as u32,
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        message_id: String,
        channel: Channel<ChatEvent>,
        mut cancel: watch::Receiver<bool>,
    ) -> AppResult<StreamResult> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_body(&request);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2025-04-14")
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
        let mut current_event = String::new();
        let mut acc = StreamResult::default();

        loop {
            tokio::select! {
                maybe_chunk = stream.next() => {
                    match maybe_chunk {
                        Some(Ok(chunk)) => {
                            let text = String::from_utf8_lossy(&chunk).replace('\r', "");
                            buf.push_str(&text);

                            while let Some(pos) = buf.find("\n\n") {
                                let event_block = buf[..pos].to_string();
                                buf = buf[pos + 2..].to_string();

                                for line in event_block.lines() {
                                    let line = line.trim();
                                    if let Some(evt) = line.strip_prefix("event: ") {
                                        current_event = evt.trim().to_string();
                                    } else if let Some(data) = line.strip_prefix("data: ") {
                                        if !current_event.is_empty() {
                                            self.handle_sse_event(&message_id, &current_event, data, &channel, &mut acc)?;
                                        }
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => return Err(AppError::from(e)),
                        None => break,
                    }
                }
                _ = cancel.changed() => {
                    if *cancel.borrow() {
                        let _ = channel.send(ChatEvent::Error {
                            message_id: message_id.clone(),
                            message: "Cancelled".into(),
                        });
                        return Err(AppError::Cancelled);
                    }
                }
            }
        }

        Ok(acc)
    }

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>> {
        let models = vec![
            ("claude-opus-4-6-20260205", "Claude Opus 4.6"),
            ("claude-sonnet-4-6-20260205", "Claude Sonnet 4.6"),
            ("claude-haiku-4-5-20251001", "Claude Haiku 4.5"),
        ];

        Ok(models
            .into_iter()
            .map(|(id, name)| ModelInfo {
                id: id.to_string(),
                name: name.to_string(),
                request_name: id.to_string(),
                display_name: Some(name.to_string()),
                provider_id: String::new(),
                context_length: Some(200000),
                supports_vision: true,
                supports_streaming: true,
                enabled: true,
                source: crate::models::ModelSource::Synced,
            })
            .collect())
    }

    async fn validate(&self) -> AppResult<bool> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}],
        });

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2025-04-14")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}
