use thiserror::Error;

/// Result type used within the mul crate.
pub type Result<T> = std::result::Result<T, MulError>;

/// Errors that can occur while working with `MulProgram`s.
#[derive(Debug, Error)]
pub enum MulError {
    /// Error originating from JSON serialization or deserialization.
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}
