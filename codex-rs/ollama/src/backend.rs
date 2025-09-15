use codex_core::error::{CodexErr, Result};
use codex_core::{ContentItem, ResponseEvent, ResponseItem, TOOLING_SCHEMA, ToolingBridge};
use futures::StreamExt;
use ollama_rs::Ollama;
use ollama_rs::error::OllamaError;
use ollama_rs::generation::completion::GenerationResponseStream;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::embeddings::request::GenerateEmbeddingsRequest;
use std::io;
use std::sync::Arc;

/// Thin wrapper around the `ollama-rs` client exposing the few operations Codex
/// needs when talking to a local Ollama instance.
pub struct OllamaBackend {
    client: Ollama,
    tool_bridge: Option<Arc<dyn ToolingBridge>>,
}

/// Events produced while streaming chat completions.
#[derive(Debug, Clone)]
pub enum ChatStreamEvent {
    /// A standard response event from the model.
    Response(ResponseEvent),
    /// Error encountered while parsing a streamed chunk.
    Error(String),
}

impl OllamaBackend {
    /// Construct a backend pointing at the given base URL, e.g.
    /// `http://localhost:11434`.
    pub fn new(base_url: &str) -> std::result::Result<Self, url::ParseError> {
        Ok(Self {
            client: Ollama::try_new(base_url)?,
            tool_bridge: None,
        })
    }

    pub fn with_tool_bridge(
        base_url: &str,
        bridge: Arc<dyn ToolingBridge>,
    ) -> std::result::Result<Self, url::ParseError> {
        Ok(Self {
            client: Ollama::try_new(base_url)?,
            tool_bridge: Some(bridge),
        })
    }

    pub fn set_tool_bridge(&mut self, bridge: Option<Arc<dyn ToolingBridge>>) {
        self.tool_bridge = bridge;
    }

    /// Perform a blocking chat completion request and return the final text.
    pub async fn chat(&self, model: &str, prompt: &str) -> Result<Vec<ResponseEvent>> {
        let mut full_prompt = prompt.to_string();
        if self.tool_bridge.is_some() {
            full_prompt = format!(
                "Respond only with JSON following this schema:\n{TOOLING_SCHEMA}\nDo not include any prose outside of the JSON.\n\n{prompt}"
            );
        }
        let req = GenerationRequest::new(model.to_string(), full_prompt);
        let resp = self
            .client
            .generate(req)
            .await
            .map_err(|e| CodexErr::Io(io::Error::other(e.to_string())))?;
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText {
                text: resp.response,
            }],
        };
        if let Some(bridge) = &self.tool_bridge {
            bridge.parse_event(ResponseEvent::OutputItemDone(item))
        } else {
            Ok(vec![ResponseEvent::OutputItemDone(item)])
        }
    }

    /// Stream a chat completion, invoking `on_event` for each emitted
    /// [`ChatStreamEvent`].
    pub async fn chat_stream<F>(&self, model: &str, prompt: &str, mut on_event: F) -> Result<()>
    where
        F: FnMut(ChatStreamEvent),
    {
        let mut full_prompt = prompt.to_string();
        if self.tool_bridge.is_some() {
            full_prompt = format!(
                "Respond only with JSON following this schema:\n{TOOLING_SCHEMA}\nDo not include any prose outside of the JSON.\n\n{prompt}"
            );
        }
        let req = GenerationRequest::new(model.to_string(), full_prompt);
        let mut stream: GenerationResponseStream = self
            .client
            .generate_stream(req)
            .await
            .map_err(|e| CodexErr::Io(io::Error::other(e.to_string())))?;

        let mut full_buffer = String::new();
        let mut json_buffer = String::new();
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escape = false;

        while let Some(chunk) = stream.next().await {
            let parts = chunk.map_err(|e| CodexErr::Io(io::Error::other(e.to_string())))?;
            for part in parts {
                full_buffer.push_str(&part.response);
                if self.tool_bridge.is_none() {
                    on_event(ChatStreamEvent::Response(ResponseEvent::OutputTextDelta(
                        part.response,
                    )));
                    continue;
                }

                for ch in part.response.chars() {
                    json_buffer.push(ch);
                    if in_string {
                        if escape {
                            escape = false;
                            continue;
                        }
                        match ch {
                            '\\' => escape = true,
                            '"' => in_string = false,
                            _ => {}
                        }
                        continue;
                    }
                    match ch {
                        '"' => in_string = true,
                        '{' => depth += 1,
                        '}' => {
                            if depth > 0 {
                                depth -= 1;
                            }
                            if depth == 0 {
                                let text = std::mem::take(&mut json_buffer);
                                let item = ResponseItem::Message {
                                    id: None,
                                    role: "assistant".into(),
                                    content: vec![ContentItem::OutputText { text: text.clone() }],
                                };
                                if let Some(bridge) = &self.tool_bridge {
                                    match bridge.parse_event(ResponseEvent::OutputItemDone(item)) {
                                        Ok(events) => {
                                            for ev in events {
                                                on_event(ChatStreamEvent::Response(ev));
                                            }
                                        }
                                        Err(err) => {
                                            on_event(ChatStreamEvent::Error(err.to_string()));
                                            return Err(err);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if self.tool_bridge.is_some() {
            if depth != 0 {
                let msg = "incomplete JSON object".to_string();
                on_event(ChatStreamEvent::Error(msg.clone()));
                return Err(CodexErr::Json(serde_json::Error::custom(msg)));
            }
            if !json_buffer.trim().is_empty() {
                let text = std::mem::take(&mut json_buffer);
                let item = ResponseItem::Message {
                    id: None,
                    role: "assistant".into(),
                    content: vec![ContentItem::OutputText { text: text.clone() }],
                };
                if let Some(bridge) = &self.tool_bridge {
                    match bridge.parse_event(ResponseEvent::OutputItemDone(item)) {
                        Ok(events) => {
                            for ev in events {
                                on_event(ChatStreamEvent::Response(ev));
                            }
                        }
                        Err(err) => {
                            on_event(ChatStreamEvent::Error(err.to_string()));
                            return Err(err);
                        }
                    }
                }
            }
        } else {
            let item = ResponseItem::Message {
                id: None,
                role: "assistant".into(),
                content: vec![ContentItem::OutputText { text: full_buffer }],
            };
            on_event(ChatStreamEvent::Response(ResponseEvent::OutputItemDone(
                item,
            )));
        }
        Ok(())
    }

    /// Generate an embedding vector for the given input text.
    pub async fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> std::result::Result<Vec<f32>, OllamaError> {
        let req = GenerateEmbeddingsRequest::new(model.to_string(), input.into());
        let resp = self.client.generate_embeddings(req).await?;
        Ok(resp.embeddings.into_iter().next().unwrap_or_default())
    }
}
