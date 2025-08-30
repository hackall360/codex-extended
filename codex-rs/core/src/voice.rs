use crate::client::ModelClient;
use crate::error::CodexErr;
use crate::error::Result;
use reqwest::multipart;
use serde::Deserialize;
use serde_json::json;

#[cfg(feature = "offline-voice")]
pub use crate::local_voice::LocalVoice;

/// Selects between the default OpenAI voice stack and an optional local stack.
pub enum VoiceBackend<'a> {
    /// Use OpenAI's hosted models for speech.
    OpenAI(&'a ModelClient),
    /// Use local, offline speech engines.
    #[cfg(feature = "offline-voice")]
    Local(LocalVoice),
}

impl<'a> VoiceBackend<'a> {
    /// Transcribe audio bytes to text using the selected backend.
    pub async fn transcribe_audio(&self, audio: &[u8], mime_type: &str) -> Result<String> {
        match self {
            VoiceBackend::OpenAI(client) => client.transcribe_audio(audio, mime_type).await,
            #[cfg(feature = "offline-voice")]
            VoiceBackend::Local(local) => local.transcribe_audio(audio, mime_type).await,
        }
    }

    /// Convert text to spoken audio using the selected backend.
    pub async fn synthesize_speech(&self, text: &str, voice: &str) -> Result<Vec<u8>> {
        match self {
            VoiceBackend::OpenAI(client) => client.synthesize_speech(text, voice).await,
            #[cfg(feature = "offline-voice")]
            VoiceBackend::Local(local) => local.synthesize_speech(text, voice).await,
        }
    }
}

impl ModelClient {
    /// Transcribe audio bytes to text using OpenAI's Whisper model.
    pub async fn transcribe_audio(&self, audio: &[u8], mime_type: &str) -> Result<String> {
        let auth_mode = self.auth_manager.as_ref().and_then(|m| m.auth());
        let provider = self
            .provider
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let builder = provider
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
        let provider = self
            .provider
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let builder = provider
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
