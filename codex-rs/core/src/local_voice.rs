use std::path::PathBuf;

use crate::error::Result;

/// Offline voice engine leveraging local models.
///
/// Placeholder implementation that wires up the local speech stack:
/// - TTS via `kokoro_tts`, `piper-rs`, or `bark.cpp`.
/// - STT via `faster-whisper` or `whisper-rs`.
/// - Audio I/O and DSP via `cpal`, `rodio`, `dasp`, and `rubato`.
#[derive(Debug, Clone)]
pub struct LocalVoice {
    /// Path to the text-to-speech model.
    pub tts_model: PathBuf,
    /// Path to the speech-to-text model.
    pub stt_model: PathBuf,
}

impl LocalVoice {
    pub fn new(tts_model: PathBuf, stt_model: PathBuf) -> Self {
        Self {
            tts_model,
            stt_model,
        }
    }

    /// Transcribe audio bytes to text using local whisper models.
    pub async fn transcribe_audio(&self, _audio: &[u8], _mime_type: &str) -> Result<String> {
        // TODO: Integrate `faster-whisper` or `whisper-rs` for offline transcription.
        Ok(String::new())
    }

    /// Synthesize speech audio bytes from text using local TTS models.
    pub async fn synthesize_speech(&self, _text: &str, _voice: &str) -> Result<Vec<u8>> {
        // TODO: Integrate `kokoro_tts`, `piper-rs`, or `bark.cpp` for offline TTS.
        Ok(Vec::new())
    }
}
