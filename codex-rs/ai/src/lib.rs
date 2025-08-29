use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;
use genai::{
    Client as GenaiClient,
    chat::{ChatMessage, ChatRequest, ChatStreamEvent},
};
use groqai::{AudioTranscriptionRequest, GroqClient};

/// Thin facade over GenAI and Groq clients.
#[derive(Clone)]
pub struct Ai {
    /// Unified chat across many providers.
    genai: GenaiClient,
    /// Full Groq coverage for audio, files, batches, and more.
    groq: Arc<GroqClient>,
}

impl Ai {
    /// Construct a new [`Ai`] client using environment configuration.
    pub fn new() -> Result<Self> {
        Ok(Self {
            genai: GenaiClient::default(),
            groq: Arc::new(GroqClient::from_env()?),
        })
    }

    /// Cross‑provider chat (routes by model name; `genai` auto‑maps to provider).
    pub async fn chat(&self, model: &str, system: Option<&str>, user: &str) -> Result<String> {
        let mut msgs = Vec::new();
        if let Some(sys) = system {
            msgs.push(ChatMessage::system(sys));
        }
        msgs.push(ChatMessage::user(user));
        let req = ChatRequest::new(msgs);
        let res = self.genai.exec_chat(model, req, None).await?;
        Ok(res.content_text_as_str().unwrap_or_default().to_string())
    }

    /// Streaming chat (Server‑Sent Events normalized by `genai`).
    pub async fn chat_stream(&self, model: &str, user: &str) -> Result<()> {
        let req = ChatRequest::new(vec![ChatMessage::user(user)]);
        let res = self.genai.exec_chat_stream(model, req, None).await?;
        let mut stream = res.stream;
        while let Some(event) = stream.next().await {
            if let Ok(ChatStreamEvent::Chunk(chunk)) = event {
                print!("{}", chunk.content);
            }
        }
        println!();
        Ok(())
    }

    /// Groq Speech‑to‑Text (Whisper Large V3) – ultra‑low latency.
    pub async fn transcribe(&self, file_path: &str) -> Result<String> {
        let request = AudioTranscriptionRequest::new("whisper-large-v3".to_string());
        let tr = self.groq.audio_transcription(request, file_path).await?;
        Ok(tr.text)
    }

    /// Groq Text‑to‑Speech (Create speech) – use an available TTS model when exposed in your org.
    pub async fn tts(&self, text: &str, out_path: &str) -> Result<()> {
        let bytes = self
            .groq
            .audio_speech("tts-1", text, "alloy", None, None, None)
            .await?;
        tokio::fs::write(out_path, bytes).await?;
        Ok(())
    }

    /// Files (upload, list) — useful for Batches or tool context.
    pub async fn upload_file(&self, purpose: &str, path: &str) -> Result<String> {
        let f = self.groq.upload_file(path, purpose).await?;
        Ok(f.id)
    }

    /// Batches — bulk chat completions via JSONL + `/batches`.
    pub async fn create_batch(&self, input_file_id: &str, window: &str) -> Result<String> {
        let b = self.groq.create_batch(input_file_id, window).await?;
        Ok(b.id)
    }
}
