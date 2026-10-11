use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Known variants of OpenAI Whisper models supported for offline speech-to-text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WhisperModelVariant {
    #[serde(rename = "tiny.en")]
    TinyEn,
    #[serde(rename = "base.en")]
    BaseEn,
    #[serde(rename = "small.en")]
    SmallEn,
    #[serde(rename = "custom")]
    Custom,
}

impl WhisperModelVariant {
    /// Standard filename for model weights on disk.
    pub fn default_filename(&self) -> &'static str {
        match self {
            Self::TinyEn => "whisper-tiny.en.bin",
            Self::BaseEn => "whisper-base.en.bin",
            Self::SmallEn => "whisper-small.en.bin",
            Self::Custom => "custom-model.bin",
        }
    }

    /// Approximate size in bytes for model weights.
    pub fn approximate_size_bytes(&self) -> u64 {
        match self {
            Self::TinyEn => 77_700_000,
            Self::BaseEn => 142_000_000,
            Self::SmallEn => 488_000_000,
            Self::Custom => 0,
        }
    }

    /// Known official SHA-256 checksum for model weights.
    pub fn official_sha256(&self) -> Option<&'static str> {
        match self {
            Self::TinyEn => Some("bd577a113a8644458634039ac77fb8d66a33c02c582b240d84a5ffc6c6bfaeee"),
            Self::BaseEn => Some("137c40409d784d83bd7a8f62c08ea2773d700f83c63dec438e149a3775f970aa"),
            _ => None,
        }
    }
}

/// Metadata describing a discovered or loaded Whisper model file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub variant: WhisperModelVariant,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
    pub is_verified: bool,
}

/// Timestamped segment of recognized speech within an utterance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptionSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub confidence: f32,
}

/// Complete result returned by a speech-to-text inference run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: u64,
    pub confidence: f32,
    pub sample_count: usize,
    pub language: Option<String>,
    pub segments: Vec<TranscriptionSegment>,
}

/// Overall configuration for the Speech-To-Text subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SttConfig {
    pub model_variant: WhisperModelVariant,
    pub model_path: Option<PathBuf>,
    pub model_dir: Option<PathBuf>,
    pub use_mock: bool,
    pub language: Option<String>,
    pub temperature: f32,
    pub sample_rate: u32,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            model_variant: WhisperModelVariant::TinyEn,
            model_path: None,
            model_dir: None,
            use_mock: true, // Safe zero-network offline default
            language: Some("en".to_string()),
            temperature: 0.0,
            sample_rate: 16_000,
        }
    }
}
