pub use airctrl_core::types::{
    AudioCueType, DictionaryRule, HotkeyMode, ModelFormat, OverlayPosition, RecordingMode,
    TranscriptItem, WhisperModelVariant,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata describing a discovered or loaded Whisper model file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub variant: WhisperModelVariant,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub format: ModelFormat,
    pub sha256: String,
    pub is_verified: bool,
}

impl ModelInfo {
    pub fn new(
        variant: WhisperModelVariant,
        file_path: PathBuf,
        size_bytes: u64,
        format: ModelFormat,
        sha256: String,
        is_verified: bool,
    ) -> Self {
        Self {
            variant,
            file_path,
            size_bytes,
            format,
            sha256,
            is_verified,
        }
    }
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
