use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Configuration for selecting models by tier and for embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub low: String,
    pub medium: String,
    pub high: String,
    pub embedding: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            low: "tinyllama".into(),
            medium: "llama3".into(),
            high: "gpt4".into(),
            embedding: "nomic-embed-text".into(),
        }
    }
}

/// Application configuration wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub models: ModelConfig,
}

/// Logical tiers for language models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmTier {
    Low,
    Medium,
    High,
}

impl Config {
    /// Return the model name associated with a tier.
    pub fn model_for_tier(&self, tier: LlmTier) -> &str {
        match tier {
            LlmTier::Low => &self.models.low,
            LlmTier::Medium => &self.models.medium,
            LlmTier::High => &self.models.high,
        }
    }

    /// Return the model name used for embeddings.
    pub fn embedding_model(&self) -> &str {
        &self.models.embedding
    }

    /// Parse configuration from TOML.
    pub fn from_toml(src: &str) -> Result<Self, ConfigError> {
        toml::from_str(src).map_err(ConfigError::from)
    }
}

/// Role for chat messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Chat message sent to or received from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Errors that can occur when working with configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_config_from_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            file,
            "[models]\nlow='a'\nmedium='b'\nhigh='c'\nembedding='d'"
        )
        .unwrap();
        let contents = std::fs::read_to_string(file.path()).unwrap();
        let cfg = Config::from_toml(&contents).unwrap();
        assert_eq!(cfg.model_for_tier(LlmTier::Low), "a");
        assert_eq!(cfg.embedding_model(), "d");
    }
}
