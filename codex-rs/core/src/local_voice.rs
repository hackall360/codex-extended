use std::io::Cursor;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::path::PathBuf;

use crate::error::CodexErr;
use crate::error::Result;
use hound::SampleFormat;
use hound::WavSpec;
use hound::WavWriter;
use kokoro_tts::KokoroTts;
use kokoro_tts::Voice;
use rodio::Decoder;
use rodio::Source;
use whisper_rs::FullParams;
use whisper_rs::SamplingStrategy;
use whisper_rs::WhisperContext;
use whisper_rs::WhisperContextParameters;

/// Offline voice engine leveraging local models for speech-to-text and text-to-speech.
#[derive(Debug, Clone)]
pub struct LocalVoice {
    /// Path to the Kokoro TTS model (.onnx).
    pub tts_model: PathBuf,
    /// Path to the Kokoro voice database (.bin).
    pub tts_voice: PathBuf,
    /// Path to the Whisper model file used for transcription.
    pub stt_model: PathBuf,
}

impl LocalVoice {
    pub fn new(tts_model: PathBuf, tts_voice: PathBuf, stt_model: PathBuf) -> Self {
        Self {
            tts_model,
            tts_voice,
            stt_model,
        }
    }

    /// Transcribe audio bytes to text using local whisper models.
    pub async fn transcribe_audio(&self, audio: &[u8], _mime_type: &str) -> Result<String> {
        // Decode using rodio; expect 16kHz mono PCM.
        let cursor = Cursor::new(audio.to_vec());
        let decoder = Decoder::new(cursor)
            .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        if sample_rate != 16_000 || channels != 1 {
            return Err(CodexErr::Io(IoError::new(
                ErrorKind::InvalidInput,
                "audio must be 16kHz mono",
            )));
        }
        let samples: Vec<f32> = decoder.convert_samples().collect();

        let model_path = self
            .stt_model
            .to_str()
            .ok_or_else(|| CodexErr::Io(IoError::new(ErrorKind::Other, "invalid model path")))?;
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
        let mut state = ctx
            .create_state()
            .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        state
            .full(params, &samples)
            .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
        let num_segments = state.full_n_segments();
        let mut out = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let text = segment
                    .to_str()
                    .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
                out.push_str(text);
            }
        }
        Ok(out)
    }

    /// Synthesize speech audio bytes from text using Kokoro TTS.
    pub async fn synthesize_speech(&self, text: &str, _voice: &str) -> Result<Vec<u8>> {
        let model_path = self.tts_model.to_str().ok_or_else(|| {
            CodexErr::Io(IoError::new(ErrorKind::Other, "invalid TTS model path"))
        })?;
        let voice_path = self
            .tts_voice
            .to_str()
            .ok_or_else(|| CodexErr::Io(IoError::new(ErrorKind::Other, "invalid voice path")))?;
        let tts = KokoroTts::new(model_path, voice_path)
            .await
            .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
        let (audio, _took) = tts
            .synth(text, Voice::AfAlloy(1.0))
            .await
            .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;

        // Convert f32 samples to WAV bytes (16-bit PCM at 24kHz).
        let pcm: Vec<i16> = audio
            .iter()
            .map(|s| (*s * i16::MAX as f32) as i16)
            .collect();
        let mut out = Vec::new();
        {
            let spec = WavSpec {
                channels: 1,
                sample_rate: 24_000,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };
            let mut writer = WavWriter::new(Cursor::new(&mut out), spec)
                .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
            for s in pcm {
                writer
                    .write_sample(s)
                    .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
            }
            writer
                .finalize()
                .map_err(|e| CodexErr::Io(IoError::new(ErrorKind::Other, e.to_string())))?;
        }
        Ok(out)
    }
}
