use crate::error::{Result, SttError};
use crate::types::{ModelInfo, TranscriptionResult, WhisperModelVariant};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Trait defining the speech-to-text inference engine contract.
pub trait SpeechToTextEngine: Send + Sync {
    /// Transcribes normalized 16kHz mono f32 samples in range [-1.0, 1.0] to text.
    fn transcribe(&self, samples: &[f32]) -> Result<TranscriptionResult>;

    /// Returns whether the engine is ready and weights are loaded.
    fn is_loaded(&self) -> bool;

    /// Returns information about the loaded model, if available.
    fn model_info(&self) -> Option<&ModelInfo>;
}

/// Deterministic, zero-dependency in-memory mock Whisper engine.
///
/// Features:
/// - Sub-millisecond latency (<1s budget).
/// - Clamps confidence based on audio energy into [0.40, 0.99].
/// - Raises `EmptyAudioBuffer` when samples slice is empty.
/// - Raises `ModelNotLoaded` when unloaded.
pub struct MockWhisperEngine {
    is_loaded: AtomicBool,
    model_path: PathBuf,
    model_info: ModelInfo,
}

impl Default for MockWhisperEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MockWhisperEngine {
    pub fn new() -> Self {
        Self::with_path(PathBuf::from("/models/whisper-tiny.en.bin"))
    }

    pub fn with_path(model_path: PathBuf) -> Self {
        let model_info = ModelInfo {
            variant: WhisperModelVariant::TinyEn,
            file_path: model_path.clone(),
            size_bytes: WhisperModelVariant::TinyEn.approximate_size_bytes(),
            sha256: WhisperModelVariant::TinyEn
                .official_sha256()
                .unwrap_or("")
                .to_string(),
            is_verified: true,
        };
        Self {
            is_loaded: AtomicBool::new(true),
            model_path,
            model_info,
        }
    }

    pub fn set_loaded(&self, loaded: bool) {
        self.is_loaded.store(loaded, Ordering::SeqCst);
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }
}

impl SpeechToTextEngine for MockWhisperEngine {
    fn transcribe(&self, samples: &[f32]) -> Result<TranscriptionResult> {
        let start = Instant::now();

        if samples.is_empty() {
            return Err(SttError::EmptyAudioBuffer);
        }

        if !self.is_loaded() {
            return Err(SttError::ModelNotLoaded(
                "Whisper model weights not loaded".into(),
            ));
        }

        // Energy heuristic matching E2E test harness
        let window = if samples.len() > 1000 {
            &samples[..1000]
        } else {
            samples
        };
        let energy: f32 =
            window.iter().map(|s| s.abs()).sum::<f32>() / (window.len().max(1) as f32);
        let confidence = (energy * 3.0).clamp(0.40, 0.99);

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text: "airctrl speech transcription result".to_string(),
            duration_ms,
            confidence: (confidence * 100.0).round() / 100.0,
            sample_count: samples.len(),
            language: Some("en".to_string()),
            segments: vec![],
        })
    }

    fn is_loaded(&self) -> bool {
        self.is_loaded.load(Ordering::SeqCst)
    }

    fn model_info(&self) -> Option<&ModelInfo> {
        Some(&self.model_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_engine_transcribe_happy_path() {
        let engine = MockWhisperEngine::new();
        assert!(engine.is_loaded());

        let samples = vec![0.5; 1600];
        let result = engine.transcribe(&samples).unwrap();
        assert!(result.text.contains("airctrl"));
        assert!(result.confidence >= 0.40 && result.confidence <= 0.99);
        assert_eq!(result.sample_count, 1600);
    }

    #[test]
    fn test_mock_engine_empty_buffer_error() {
        let engine = MockWhisperEngine::new();
        let result = engine.transcribe(&[]);
        assert!(matches!(result, Err(SttError::EmptyAudioBuffer)));
    }

    #[test]
    fn test_mock_engine_unloaded_error() {
        let engine = MockWhisperEngine::new();
        engine.set_loaded(false);
        let samples = vec![0.5; 1600];
        let result = engine.transcribe(&samples);
        assert!(matches!(result, Err(SttError::ModelNotLoaded(_))));
    }

    #[test]
    fn test_mock_engine_confidence_clamping() {
        let engine = MockWhisperEngine::new();

        // Zero energy -> clamped to 0.40
        let silent = vec![0.0; 1600];
        let res_silent = engine.transcribe(&silent).unwrap();
        assert_eq!(res_silent.confidence, 0.40);

        // High energy -> clamped to 0.99
        let loud = vec![1.0; 1600];
        let res_loud = engine.transcribe(&loud).unwrap();
        assert_eq!(res_loud.confidence, 0.99);
    }
}
