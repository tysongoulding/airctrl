//! `airctrl-stt`: Offline Whisper speech-to-text pipeline and active window text injection.
//!
//! Provides:
//! - Offline Whisper STT trait and deterministic mock engine.
//! - Audio capture, downmixing, FIR resampling, and thread-safe circular ring buffer.
//! - Energy and dBFS-based Voice Activity Detection (VAD) with silence trimming and hangover.
//! - Push-to-Talk / Toggle hotkey state machine with micro-press cancellation (<100ms).
//! - Active window text injection with focus restoration and clipboard fallback.
//! - Offline model discovery, resolution, and streaming SHA-256 integrity verification.
//! - Synthetic WAV latency benchmarking ensuring <1.0s budget compliance.

pub mod audio;
pub mod benchmark;
pub mod config;
pub mod engine;
pub mod error;
pub mod hotkey;
pub mod injection;
pub mod model;
pub mod types;
pub mod vad;

// Re-exports
pub use audio::{
    create_synthetic_wav, encode_wav, parse_wav_samples, AudioChunker, AudioPreprocessor,
    AudioRingBuffer, UtteranceChunk,
};
pub use benchmark::{run_synthetic_wav_benchmark, LatencyBenchmarkResult};
pub use config::{AudioStreamConfig, HotkeyConfig, TextInjectionConfig, VadConfig};
pub use engine::{MockWhisperEngine, SpeechToTextEngine};
pub use error::{Result, SttError};
pub use hotkey::{HotkeyStateMachine, StateMachineAction, StateTransitionResult};
pub use injection::{
    ActiveWindowTextInjector, InjectionMethod, InjectionReport, InjectionStatus,
};
pub use model::{
    compute_file_sha256, get_default_model_dir, verify_model_file, ModelManager,
};
pub use types::{
    ModelInfo, SttConfig, TranscriptionResult, TranscriptionSegment, WhisperModelVariant,
};
pub use vad::{
    DiscardReason, StreamVadState, TrimMetadata, VadDecision, VoiceActivityDetector,
};

pub use airctrl_core as core;

/// Convenience factory for creating a configured speech-to-text inference engine.
pub fn create_whisper_engine(config: &SttConfig) -> Box<dyn SpeechToTextEngine> {
    if let Some(path) = &config.model_path {
        Box::new(MockWhisperEngine::with_path(path.clone()))
    } else {
        Box::new(MockWhisperEngine::new())
    }
}

/// Convenience factory for creating a Voice Activity Detector.
pub fn create_vad_detector(config: Option<VadConfig>) -> VoiceActivityDetector {
    VoiceActivityDetector::new(config.unwrap_or_default())
}
