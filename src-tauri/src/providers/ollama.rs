use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;

use super::{Provider, StreamResult};

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        let base_url = base_url
            .unwrap_or_else(|| "http://localhost:11434".to_string())
            .trim_end_matches('/')
            .to_string();
        Self {
            client: Client::new(),
            base_url,
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
            "stream": true,
        });

        let obj = body.as_object_mut().unwrap();

        // Build options object for common params
        let mut options = json!({});
        let opts = options.as_object_mut().unwrap();

        if let Some(temp) = request.common.temperature {
            opts.insert("temperature".into(), json!(temp));
        }
        if let Some(top_p) = request.common.top_p {
            opts.insert("top_p".into(), json!(top_p));
        }
        if let Some(max_tokens) = request.common.max_tokens {
            opts.insert("num_predict".into(), json!(max_tokens));
        }

        // Ollama-specific params
        if let ProviderParams::Ollama {
            think,
            num_ctx,
            repeat_penalty,
            min_p,
            keep_alive,
        } = &request.provider_params
        {
            // think goes at top level
            if let Some(t) = think {
                match t {
                    OllamaThink::Bool(b) => {
                        obj.insert("think".into(), json!(b));
                    }
                    OllamaThink::Level(l) => {
                        obj.insert("think".into(), json!(l));
                    }
                }
            }

            // These go in options
            if let Some(ctx) = num_ctx {
                opts.insert("num_ctx".into(), json!(ctx));
            }
            if let Some(rp) = repeat_penalty {
                opts.insert("repeat_penalty".into(), json!(rp));
            }
            if let Some(mp) = min_p {
                opts.insert("min_p".into(), json!(mp));
            }

            // keep_alive at top level
            if let Some(ka) = keep_alive {
                obj.insert("keep_alive".into(), json!(ka));
            }
        }

        if !opts.is_empty() {
            obj.insert("options".into(), options);
        }

        body
    }

    fn handle_ndjson_line(
        &self,
        message_id: &str,
        line: &str,
        channel: &Channel<ChatEvent>,
        acc: &mut StreamResult,
    ) -> AppResult<bool> {
        let json: Value = serde_json::from_str(line)?;

        let done = json["done"].as_bool().unwrap_or(false);

        if done {
            let prompt = json["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
            let completion = json["eval_count"].as_u64().unwrap_or(0) as u32;
            acc.prompt_tokens = prompt;
            acc.completion_tokens = completion;
            if prompt > 0 || completion > 0 {
                let _ = channel.send(ChatEvent::Usage {
                    message_id: message_id.to_string(),
                    prompt_tokens: prompt,
                    completion_tokens: completion,
                });
            }
            return Ok(true);
        }

        if let Some(message) = json.get("message") {
            if let Some(content) = message["content"].as_str() {
                if !content.is_empty() {
                    acc.content.push_str(content);
                    let _ = channel.send(ChatEvent::Delta {
                        message_id: message_id.to_string(),
                        content: content.to_string(),
                    });
                }
            }
            if let Some(thinking) = message["thinking"].as_str() {
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

        Ok(false)
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        message_id: String,
        channel: Channel<ChatEvent>,
        mut cancel: watch::Receiver<bool>,
    ) -> AppResult<StreamResult> {
        let url = format!("{}/api/chat", self.base_url);
        let body = self.build_body(&request);

        let response = self
            .client
            .post(&url)
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
        let mut acc = StreamResult::default();

        loop {
            tokio::select! {
                maybe_chunk = stream.next() => {
                    match maybe_chunk {
                        Some(Ok(chunk)) => {
                            let text = String::from_utf8_lossy(&chunk).replace('\r', "");
                            buf.push_str(&text);

                            while let Some(pos) = buf.find('\n') {
                                let line = buf[..pos].trim().to_string();
                                buf = buf[pos + 1..].to_string();

                                if line.is_empty() {
                                    continue;
                                }

                                let done = self.handle_ndjson_line(&message_id, &line, &channel, &mut acc)?;
                                if done {
                                    return Ok(acc);
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
        let url = format!("{}/api/tags", self.base_url);

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
                let name = m["name"].as_str()?.to_string();
                Some(ModelInfo {
                    id: name.clone(),
                    name,
                    provider_id: String::new(),
                    context_length: None,
                    supports_vision: false,
                    supports_streaming: true,
                    enabled: true,
                })
            })
            .collect();

        Ok(models)
    }

    async fn validate(&self) -> AppResult<bool> {
        let url = format!("{}/api/version", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}
