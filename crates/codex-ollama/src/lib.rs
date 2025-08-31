use codex_core::Config;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OllamaError {
    #[error("http error: {0}")]
    Http(String),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Config(#[from] codex_core::config::ConfigError),
}

#[derive(Debug, Clone, Copy)]
pub enum LlmTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

pub struct OllamaClient {
    base_url: String,
    http: reqwest::Client,
    cfg: Config,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, OllamaError> {
        Ok(Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
            cfg: Config::load()?,
        })
    }

    fn model_for_tier(&self, tier: LlmTier) -> String {
        match tier {
            LlmTier::Low => format!("{}-low", self.cfg.chat_model),
            LlmTier::Medium => format!("{}-medium", self.cfg.chat_model),
            LlmTier::High => self.cfg.chat_model.clone(),
        }
    }

    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, OllamaError> {
        #[derive(Serialize)]
        struct EmbedRequest<'a> {
            model: &'a str,
            input: &'a [String],
        }
        #[derive(Deserialize)]
        struct EmbedResponse {
            embeddings: Vec<Vec<f32>>,
        }
        let url = format!("{}/api/embeddings", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&EmbedRequest {
                model: &self.cfg.embedding_model,
                input: texts,
            })
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http(format!("status {}: {}", status, text)));
        }
        let body: EmbedResponse = resp.json().await?;
        Ok(body.embeddings)
    }

    pub async fn chat(&self, tier: LlmTier, messages: &[Message]) -> Result<String, OllamaError> {
        #[derive(Serialize)]
        struct ChatRequest<'a> {
            model: String,
            messages: &'a [Message],
        }
        #[derive(Deserialize)]
        struct ChatResponse {
            message: Message,
        }
        let url = format!("{}/api/chat", self.base_url);
        let model = self.model_for_tier(tier);
        let resp = self
            .http
            .post(&url)
            .json(&ChatRequest { model, messages })
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http(format!("status {}: {}", status, text)));
        }
        let body: ChatResponse = resp.json().await?;
        Ok(body.message.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;

    #[tokio::test]
    async fn embed_works() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/embeddings");
            then.status(200).json_body(json!({
                "embeddings": [[1.0,2.0],[3.0,4.0]]
            }));
        });
        let client = OllamaClient::new(server.base_url()).unwrap();
        let texts = vec!["a".to_string(), "b".to_string()];
        let res = client.embed(&texts).await.unwrap();
        assert_eq!(res, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[tokio::test]
    async fn chat_works() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200).json_body(json!({
                "message": {"role": "assistant", "content": "hi"}
            }));
        });
        let client = OllamaClient::new(server.base_url()).unwrap();
        let msgs = vec![Message {
            role: Role::User,
            content: "hello".into(),
        }];
        let reply = client.chat(LlmTier::Low, &msgs).await.unwrap();
        assert_eq!(reply, "hi");
    }
}
