use futures::StreamExt;
use ollama_rs::Ollama;
use ollama_rs::error::OllamaError;
use ollama_rs::generation::completion::GenerationResponseStream;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::embeddings::request::GenerateEmbeddingsRequest;

/// Thin wrapper around the `ollama-rs` client exposing the few operations Codex
/// needs when talking to a local Ollama instance.
pub struct OllamaBackend {
    client: Ollama,
}

impl OllamaBackend {
    /// Construct a backend pointing at the given base URL, e.g.
    /// `http://localhost:11434`.
    pub fn new(base_url: &str) -> Result<Self, url::ParseError> {
        Ok(Self {
            client: Ollama::try_new(base_url)?,
        })
    }

    /// Perform a blocking chat completion request and return the final text.
    pub async fn chat(&self, model: &str, prompt: &str) -> Result<String, OllamaError> {
        let req = GenerationRequest::new(model.to_string(), prompt);
        let resp = self.client.generate(req).await?;
        Ok(resp.response)
    }

    /// Stream a chat completion, invoking `on_chunk` for each text fragment.
    pub async fn chat_stream<F>(
        &self,
        model: &str,
        prompt: &str,
        mut on_chunk: F,
    ) -> Result<(), OllamaError>
    where
        F: FnMut(String),
    {
        let req = GenerationRequest::new(model.to_string(), prompt);
        let mut stream: GenerationResponseStream = self.client.generate_stream(req).await?;
        while let Some(chunk) = stream.next().await {
            for part in chunk? {
                on_chunk(part.response);
            }
        }
        Ok(())
    }

    /// Generate an embedding vector for the given input text.
    pub async fn embed(&self, model: &str, input: &str) -> Result<Vec<f32>, OllamaError> {
        let req = GenerateEmbeddingsRequest::new(model.to_string(), input.into());
        let resp = self.client.generate_embeddings(req).await?;
        Ok(resp.embeddings.into_iter().next().unwrap_or_default())
    }
}
