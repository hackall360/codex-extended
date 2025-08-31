use serde::{Deserialize, Serialize};

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
