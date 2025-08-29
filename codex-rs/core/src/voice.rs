use crate::client::ModelClient;
use crate::error::{CodexErr, Result};
use reqwest::multipart;
use serde::Deserialize;
use serde_json::json;

impl ModelClient {
    /// Transcribe audio bytes to text using OpenAI's Whisper model.
    pub async fn transcribe_audio(&self, audio: &[u8], mime_type: &str) -> Result<String> {
        let auth_mode = self.auth_manager.as_ref().and_then(|m| m.auth());
        let builder = self
            .provider
            .create_request_builder_for_path(&self.client, &auth_mode, "/audio/transcriptions")
            .await?;

        let part = multipart::Part::bytes(audio.to_vec())
            .file_name("audio")
            .mime_str(mime_type)?;
        let form = multipart::Form::new()
            .text("model", "whisper-1")
            .part("file", part);
        let resp = builder.multipart(form).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CodexErr::UnexpectedStatus(status, body));
        }
        #[derive(Deserialize)]
        struct Transcription {
            text: String,
        }
        let t: Transcription = resp.json().await?;
        Ok(t.text)
    }

    /// Convert text to spoken audio using GPT-4o TTS models.
    pub async fn synthesize_speech(&self, text: &str, voice: &str) -> Result<Vec<u8>> {
        let auth_mode = self.auth_manager.as_ref().and_then(|m| m.auth());
        let builder = self
            .provider
            .create_request_builder_for_path(&self.client, &auth_mode, "/audio/speech")
            .await?;
        let payload = json!({
            "model": "gpt-4o-mini-tts",
            "input": text,
            "voice": voice,
        });
        let resp = builder
            .header("Accept", "audio/mpeg")
            .json(&payload)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CodexErr::UnexpectedStatus(status, body));
        }
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}
