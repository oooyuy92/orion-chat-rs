use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;

use super::Provider;

pub struct OpenAICompatProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAICompatProvider {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    fn build_body(&self, request: &ChatRequest) -> Value {
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": request.common.stream,
        });

        let obj = body.as_object_mut().unwrap();

        // Common params
        if let Some(temp) = request.common.temperature {
            obj.insert("temperature".into(), json!(temp));
        }
        if let Some(top_p) = request.common.top_p {
            obj.insert("top_p".into(), json!(top_p));
        }
        if let Some(max_tokens) = request.common.max_tokens {
            obj.insert("max_tokens".into(), json!(max_tokens));
        }

        // OpenAI-specific params
        if let ProviderParams::OpenaiCompat {
            frequency_penalty,
            presence_penalty,
            reasoning_effort,
            seed,
            max_completion_tokens,
        } = &request.provider_params
        {
            if let Some(fp) = frequency_penalty {
                obj.insert("frequency_penalty".into(), json!(fp));
            }
            if let Some(pp) = presence_penalty {
                obj.insert("presence_penalty".into(), json!(pp));
            }
            if let Some(effort) = reasoning_effort {
                let effort_str = match effort {
                    ReasoningEffort::Low => "low",
                    ReasoningEffort::Medium => "medium",
                    ReasoningEffort::High => "high",
                };
                obj.insert("reasoning_effort".into(), json!(effort_str));
            }
            if let Some(s) = seed {
                obj.insert("seed".into(), json!(s));
            }
            if let Some(mct) = max_completion_tokens {
                obj.insert("max_completion_tokens".into(), json!(mct));
            }
        }

        body
    }

    fn handle_sse_data(&self, data: &str, channel: &Channel<ChatEvent>) -> AppResult<()> {
        let json: Value = serde_json::from_str(data)?;

        if let Some(choices) = json["choices"].as_array() {
            for choice in choices {
                let delta = &choice["delta"];

                // Content delta
                if let Some(content) = delta["content"].as_str() {
                    if !content.is_empty() {
                        let _ = channel.send(ChatEvent::Delta {
                            content: content.to_string(),
                        });
                    }
                }

                // Reasoning content (e.g. o1/o3 models)
                if let Some(reasoning) = delta["reasoning_content"].as_str() {
                    if !reasoning.is_empty() {
                        let _ = channel.send(ChatEvent::Reasoning {
                            content: reasoning.to_string(),
                        });
                    }
                }
            }
        }

        // Usage in final chunk
        if let Some(usage) = json.get("usage") {
            if let (Some(prompt), Some(completion)) = (
                usage["prompt_tokens"].as_u64(),
                usage["completion_tokens"].as_u64(),
            ) {
                let _ = channel.send(ChatEvent::Usage {
                    prompt_tokens: prompt as u32,
                    completion_tokens: completion as u32,
                });
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Provider for OpenAICompatProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        mut cancel: watch::Receiver<bool>,
    ) -> AppResult<()> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = self.build_body(&request);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
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

        // Buffer for incomplete SSE lines across chunks
        let mut buf = String::new();

        loop {
            tokio::select! {
                maybe_chunk = stream.next() => {
                    match maybe_chunk {
                        Some(Ok(chunk)) => {
                            buf.push_str(&String::from_utf8_lossy(&chunk));

                            // Process complete SSE lines
                            while let Some(pos) = buf.find("\n\n") {
                                let event_block = buf[..pos].to_string();
                                buf = buf[pos + 2..].to_string();

                                for line in event_block.lines() {
                                    let line = line.trim();
                                    if line == "data: [DONE]" {
                                        return Ok(());
                                    }
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        self.handle_sse_data(data, &channel)?;
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => return Err(AppError::Http(e)),
                        None => break, // Stream ended
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
        let url = format!("{}/v1/models", self.base_url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
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

        let body: Value = response.json().await?;
        let models = body["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| {
                let id = m["id"].as_str()?.to_string();
                Some(ModelInfo {
                    name: id.clone(),
                    id,
                    provider_id: String::new(),
                    context_length: None,
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
