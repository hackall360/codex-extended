use codex_core::{Config, LlmTier, Message};
use reqwest::StatusCode;
use thiserror::Error;

/// Client for interacting with an Ollama HTTP API.
pub struct OllamaClient {
    http: reqwest::Client,
    base_url: String,
    config: Config,
}

impl OllamaClient {
    /// Create a new client with the given base URL and configuration.
    pub fn new(base_url: impl Into<String>, config: Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            config,
        }
    }

    /// Fetch embeddings for the provided texts using the configured embedding model.
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Error> {
        #[derive(serde::Serialize)]
        struct EmbedRequest<'a> {
            model: &'a str,
            input: &'a [String],
        }

        #[derive(serde::Deserialize)]
        struct EmbedResponse {
            embeddings: Vec<Vec<f32>>,
        }

        let req = EmbedRequest {
            model: self.config.embedding_model(),
            input: texts,
        };
        let url = format!("{}/api/embed", self.base_url);
        let resp = self.http.post(url).json(&req).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Http { status, text });
        }
        let body = resp.json::<EmbedResponse>().await?;
        Ok(body.embeddings)
    }

    /// Send a chat conversation and return the assistant's response.
    pub async fn chat(&self, tier: LlmTier, messages: &[Message]) -> Result<String, Error> {
        #[derive(serde::Serialize)]
        struct ChatRequest<'a> {
            model: &'a str,
            messages: &'a [Message],
        }

        #[derive(serde::Deserialize)]
        struct ChatResponse {
            message: ChatMessage,
        }

        #[derive(serde::Deserialize)]
        struct ChatMessage {
            content: String,
        }

        let req = ChatRequest {
            model: self.config.model_for_tier(tier),
            messages,
        };
        let url = format!("{}/api/chat", self.base_url);
        let resp = self.http.post(url).json(&req).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Http { status, text });
        }
        let body = resp.json::<ChatResponse>().await?;
        Ok(body.message.content)
    }
}

/// Errors produced by [`OllamaClient`].
#[derive(Debug, Error)]
pub enum Error {
    /// Error from the underlying HTTP client.
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    /// Non-successful HTTP status returned by the server.
    #[error("HTTP {status}: {text}")]
    Http { status: StatusCode, text: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::{Message, Role};
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn embed_sends_texts_and_returns_vectors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                serde_json::json!({ "embeddings": [[1.0, 2.0]] }).to_string(),
                "application/json",
            ))
            .mount(&server)
            .await;

        let client = OllamaClient::new(server.uri(), Config::default());
        let res = client.embed(&["hello".to_string()]).await.expect("embed");
        assert_eq!(res, vec![vec![1.0, 2.0]]);
    }

    #[tokio::test]
    async fn embed_propagates_http_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/embed"))
            .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
            .mount(&server)
            .await;

        let client = OllamaClient::new(server.uri(), Config::default());
        let err = client
            .embed(&["bad".to_string()])
            .await
            .expect_err("embed should fail");
        match err {
            Error::Http { status, text } => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "oops");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn chat_uses_tier_model_and_returns_content() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .and(body_string_contains("chat-low"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "message": {"content": "hi"}
                    })
                    .to_string(),
                    "application/json",
                ),
            )
            .mount(&server)
            .await;

        let mut config = Config::default();
        config.models.low = "chat-low".into();
        let client = OllamaClient::new(server.uri(), config);
        let msgs = vec![Message {
            role: Role::User,
            content: "Hello".into(),
        }];
        let res = client.chat(LlmTier::Low, &msgs).await.expect("chat");
        assert_eq!(res, "hi");
    }

    #[tokio::test]
    async fn chat_propagates_http_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(404).set_body_string("missing"))
            .mount(&server)
            .await;

        let client = OllamaClient::new(server.uri(), Config::default());
        let err = client
            .chat(LlmTier::Low, &[])
            .await
            .expect_err("chat should fail");
        match err {
            Error::Http { status, text } => {
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(text, "missing");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
