pub mod config;

pub use config::Config;

/// Trait representing an embedding service.
pub trait Embedder {
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// Trait representing a chat-oriented language model.
pub trait ChatModel {
    fn chat(&self, prompt: &str) -> String;
}

/// Trait for a vector store that can persist embeddings and perform search.
pub trait VectorStore {
    fn add_embedding(&self, embedding: Vec<f32>) -> Result<(), Box<dyn std::error::Error>>;
    fn search(&self, embedding: Vec<f32>, top_k: usize) -> Vec<usize>;
}

/// Trait for a generic storage engine.
pub trait StorageEngine {
    fn save(&self, key: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn load(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>>;
}
