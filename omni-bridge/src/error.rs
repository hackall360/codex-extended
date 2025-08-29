use thiserror::Error;

#[derive(Debug, Error)]
pub enum OmniError {
    #[error("auth: {0}")]
    Auth(String),
    #[error("rate_limited: {0}")]
    RateLimited(String),
    #[error("not_found: {0}")]
    NotFound(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("bad_request: {0}")]
    BadRequest(String),
    #[error("io: {0}")]
    Io(String),
    #[error("network: {0}")]
    Network(String),
    #[error("provider: {provider} error: {message}")]
    Provider {
        provider: &'static str,
        message: String,
    },
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}
