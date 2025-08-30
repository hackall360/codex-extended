#![allow(clippy::print_stdout)]

use std::io::Cursor;
use std::io::Error as IoError;
use std::io::Write;
use std::path::Path;
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

use tokio::fs as tokio_fs;

/// Metadata for a downloadable speech model.
#[derive(Debug, Clone)]
struct ModelInfo {
    /// Human readable name.
    name: &'static str,
    /// License or attribution URL.
    license: &'static str,
    /// Direct download URL for the model file.
    url: &'static str,
    /// Default filename when stored locally.
    filename: &'static str,
}

/// Available Whisper models for transcription.
const STT_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "Whisper Tiny EN",
        license: "https://github.com/openai/whisper/blob/main/LICENSE",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        filename: "ggml-tiny.en.bin",
    },
    ModelInfo {
        name: "Whisper Base EN",
        license: "https://github.com/openai/whisper/blob/main/LICENSE",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        filename: "ggml-base.en.bin",
    },
];

/// Available Kokoro ONNX models.
const TTS_MODELS: &[ModelInfo] = &[ModelInfo {
    name: "Kokoro v1.1 English",
    license: "https://huggingface.co/hexgrad/kokoro-tts-onnx",
    url: "https://huggingface.co/hexgrad/kokoro-tts-onnx/resolve/main/kokoro-v1_1.onnx",
    filename: "kokoro-v1_1.onnx",
}];

/// Kokoro voice databases.
const TTS_VOICES: &[ModelInfo] = &[
    ModelInfo {
        name: "af_alloy",
        license: "https://huggingface.co/hexgrad/kokoro-tts-onnx",
        url: "https://huggingface.co/hexgrad/kokoro-tts-onnx/resolve/main/af_alloy.bin",
        filename: "af_alloy.bin",
    },
    ModelInfo {
        name: "af_bella",
        license: "https://huggingface.co/hexgrad/kokoro-tts-onnx",
        url: "https://huggingface.co/hexgrad/kokoro-tts-onnx/resolve/main/af_bella.bin",
        filename: "af_bella.bin",
    },
];

fn prompt(msg: &str) -> std::io::Result<String> {
    print!("{}", msg);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn select_model<'a>(models: &'a [ModelInfo], kind: &str) -> Result<&'a ModelInfo> {
    println!("Available {kind} models:");
    for (i, m) in models.iter().enumerate() {
        println!("  {}: {}", i + 1, m.name);
    }
    loop {
        let ans = prompt("Choose a model number: ")
            .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
        if let Ok(idx) = ans.parse::<usize>()
            && idx > 0
            && idx <= models.len()
        {
            return Ok(&models[idx - 1]);
        }
        println!("Invalid selection. Please try again.");
    }
}

async fn ensure_file(path: &Path, model: &ModelInfo) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    println!("Model '{}' is licensed under {}", model.name, model.license);
    println!(
        "By downloading, you agree to the model's license terms provided by the third-party author."
    );
    let ans = prompt("Download this model now? [y/N]: ")
        .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
    if !matches!(ans.to_lowercase().as_str(), "y" | "yes") {
        return Err(CodexErr::Io(IoError::other("model download declined")));
    }
    println!("Downloading {}...", model.url);
    let bytes = reqwest::get(model.url)
        .await
        .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?
        .bytes()
        .await
        .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
    if let Some(parent) = path.parent() {
        tokio_fs::create_dir_all(parent)
            .await
            .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
    }
    tokio_fs::write(path, &bytes)
        .await
        .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
    Ok(())
}

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

    /// Ensure required models exist, prompting the user to download them if missing.
    ///
    /// `models_dir` will contain subdirectories for TTS and STT models.
    pub async fn init(models_dir: PathBuf) -> Result<Self> {
        // Select speech-to-text model.
        let stt_choice = select_model(STT_MODELS, "speech-to-text")?;
        let stt_path = models_dir.join("stt").join(stt_choice.filename);
        ensure_file(&stt_path, stt_choice).await?;

        // Select text-to-speech model and voice database.
        let tts_choice = select_model(TTS_MODELS, "text-to-speech")?;
        let tts_model_path = models_dir.join("tts").join(tts_choice.filename);
        ensure_file(&tts_model_path, tts_choice).await?;

        let voice_choice = select_model(TTS_VOICES, "voice")?;
        let voice_path = models_dir
            .join("tts")
            .join("voices")
            .join(voice_choice.filename);
        ensure_file(&voice_path, voice_choice).await?;

        Ok(Self::new(tts_model_path, voice_path, stt_path))
    }

    /// Transcribe audio bytes to text using local whisper models.
    pub async fn transcribe_audio(&self, audio: &[u8], _mime_type: &str) -> Result<String> {
        // Decode using rodio; expect 16kHz mono PCM.
        let cursor = Cursor::new(audio.to_vec());
        let decoder =
            Decoder::new(cursor).map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        if sample_rate != 16_000 || channels != 1 {
            return Err(CodexErr::Io(IoError::other("audio must be 16kHz mono")));
        }
        let samples: Vec<f32> = decoder.convert_samples().collect();

        let model_path = self
            .stt_model
            .to_str()
            .ok_or_else(|| CodexErr::Io(IoError::other("invalid model path")))?;
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
        let mut state = ctx
            .create_state()
            .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        state
            .full(params, &samples)
            .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
        let num_segments = state.full_n_segments();
        let mut out = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let text = segment
                    .to_str()
                    .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
                out.push_str(text);
            }
        }
        Ok(out)
    }

    /// Synthesize speech audio bytes from text using Kokoro TTS.
    pub async fn synthesize_speech(&self, text: &str, _voice: &str) -> Result<Vec<u8>> {
        let model_path = self
            .tts_model
            .to_str()
            .ok_or_else(|| CodexErr::Io(IoError::other("invalid TTS model path")))?;
        let voice_path = self
            .tts_voice
            .to_str()
            .ok_or_else(|| CodexErr::Io(IoError::other("invalid voice path")))?;
        let tts = KokoroTts::new(model_path, voice_path)
            .await
            .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
        let (audio, _took) = tts
            .synth(text, Voice::AfAlloy(1.0))
            .await
            .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;

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
                .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
            for s in pcm {
                writer
                    .write_sample(s)
                    .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
            }
            writer
                .finalize()
                .map_err(|e| CodexErr::Io(IoError::other(e.to_string())))?;
        }
        Ok(out)
    }
}
