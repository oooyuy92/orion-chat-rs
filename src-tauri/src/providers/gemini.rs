use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;

use super::{Provider, StreamResult};

const DEFAULT_BASE: &str = "https://generativelanguage.googleapis.com";

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let base = base_url
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE.to_string());
        Self {
            client: Client::new(),
            api_key,
            base_url: base.trim_end_matches('/').to_string(),
        }
    }

    fn build_body(&self, request: &ChatRequest) -> Value {
        let system_text: Option<String> = request
            .messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.clone())
            .reduce(|a, b| format!("{}\n{}", a, b));

        let contents: Vec<Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::User => "user",
                        _ => "model",
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

        if let ProviderParams::Gemini {
            thinking_budget,
            thinking_level,
        } = &request.provider_params
        {
            // Always request thought summaries so we can display CoT reasoning
            let mut thinking_config = json!({ "includeThoughts": true });
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

        if !gc.is_empty() {
            obj.insert("generationConfig".into(), gen_config);
        }

        body
    }

    fn handle_sse_data(
        &self,
        message_id: &str,
        data: &str,
        channel: &Channel<ChatEvent>,
        acc: &mut StreamResult,
    ) -> AppResult<()> {
        // Skip non-JSON lines (e.g. empty, "[DONE]")
        let json: Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        // Check for API-level error embedded in the SSE stream
        if let Some(err) = json.get("error") {
            let msg = err["message"].as_str().unwrap_or("Unknown Gemini error");
            return Err(AppError::Provider(msg.to_string()));
        }

        if let Some(candidates) = json["candidates"].as_array() {
            for candidate in candidates {
                if let Some(parts) = candidate["content"]["parts"].as_array() {
                    for part in parts {
                        if let Some(text) = part["text"].as_str() {
                            if !text.is_empty() {
                                if part.get("thought") == Some(&json!(true)) {
                                    acc.reasoning
                                        .get_or_insert_with(String::new)
                                        .push_str(text);
                                    let _ = channel.send(ChatEvent::Reasoning {
                                        message_id: message_id.to_string(),
                                        content: text.to_string(),
                                    });
                                } else {
                                    acc.content.push_str(text);
                                    let _ = channel.send(ChatEvent::Delta {
                                        message_id: message_id.to_string(),
                                        content: text.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(usage) = json.get("usageMetadata") {
            let prompt = usage["promptTokenCount"].as_u64().unwrap_or(0) as u32;
            let completion = usage["candidatesTokenCount"]
                .as_u64()
                .or_else(|| usage["totalTokenCount"].as_u64())
                .unwrap_or(0) as u32;
            if prompt > 0 || completion > 0 {
                acc.prompt_tokens = prompt;
                acc.completion_tokens = completion;
                let _ = channel.send(ChatEvent::Usage {
                    message_id: message_id.to_string(),
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
        message_id: String,
        channel: Channel<ChatEvent>,
        mut cancel: watch::Receiver<bool>,
    ) -> AppResult<StreamResult> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, request.model, self.api_key
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
                "Gemini API error {}: {}",
                status, text
            )));
        }

        let mut stream = response.bytes_stream();
        let mut buf = String::new();
        let mut acc = StreamResult::default();

        loop {
            tokio::select! {
                maybe_chunk = stream.next() => {
                    match maybe_chunk {
                        Some(Ok(chunk)) => {
                            // Strip \r to normalize CRLF → LF (Gemini sends \r\n line endings)
                            let text = String::from_utf8_lossy(&chunk).replace('\r', "");
                            buf.push_str(&text);

                            while let Some(pos) = buf.find("\n\n") {
                                let event_block = buf[..pos].to_string();
                                buf = buf[pos + 2..].to_string();

                                for line in event_block.lines() {
                                    let line = line.trim();
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        self.handle_sse_data(&message_id, data, &channel, &mut acc)?;
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
                        return Err(AppError::Cancelled);
                    }
                }
            }
        }

        Ok(acc)
    }

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>> {
        let url = format!(
            "{}/v1beta/models?key={}&pageSize=200",
            self.base_url, self.api_key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!(
                "Gemini API error {}: {}",
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
                let id = name.strip_prefix("models/").unwrap_or(name);
                // Only include generative models (skip embedding/aqa models)
                let supported: Vec<&str> = m["supportedGenerationMethods"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();
                if !supported.contains(&"generateContent") {
                    return None;
                }
                let display_name = m["displayName"].as_str().unwrap_or(id).to_string();
                Some(ModelInfo {
                    name: display_name.clone(),
                    request_name: id.to_string(),
                    display_name: Some(display_name),
                    id: id.to_string(),
                    provider_id: String::new(),
                    context_length: m["inputTokenLimit"].as_u64().map(|v| v as u32),
                    supports_vision: false,
                    supports_streaming: true,
                    enabled: true,
                    source: crate::models::ModelSource::Synced,
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
