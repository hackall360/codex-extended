use codex_core::error::{CodexErr, Result};
use codex_core::{ContentItem, ResponseEvent, ResponseItem, ToolingBridge, TOOLING_SCHEMA};
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
    /// [`ResponseEvent`].
    pub async fn chat_stream<F>(&self, model: &str, prompt: &str, mut on_event: F) -> Result<()>
    where
        F: FnMut(ResponseEvent),
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

        let mut buffer = String::new();
        while let Some(chunk) = stream.next().await {
            let parts =
                chunk.map_err(|e| CodexErr::Io(io::Error::other(e.to_string())))?;
            for part in parts {
                buffer.push_str(&part.response);
                if self.tool_bridge.is_none() {
                    on_event(ResponseEvent::OutputTextDelta(part.response));
                }
            }
        }

        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText { text: buffer }],
        };
        if let Some(bridge) = &self.tool_bridge {
            for ev in bridge.parse_event(ResponseEvent::OutputItemDone(item))? {
                on_event(ev);
            }
        } else {
            on_event(ResponseEvent::OutputItemDone(item));
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
