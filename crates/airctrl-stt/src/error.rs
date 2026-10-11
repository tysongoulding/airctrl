use airctrl_core::error::CoreError;
use std::path::PathBuf;
use thiserror::Error;

/// Domain error types for speech-to-text, audio processing, VAD, hotkey, and text injection.
#[derive(Debug, Error)]
pub enum SttError {
    #[error("Audio buffer is empty: cannot transcribe empty audio")]
    EmptyAudioBuffer,

    #[error("Whisper model is not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Whisper model file not found at path: {0}")]
    ModelNotFound(PathBuf),

    #[error("Model integrity check failed for '{path}': expected SHA-256 {expected}, got {actual}")]
    ModelCorrupted {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("Invalid WAV header or format: {0}")]
    InvalidWavHeader(String),

    #[error("WAV decoding error: {0}")]
    WavDecodeError(String),

    #[error("Unsupported audio format: {0}")]
    UnsupportedAudioFormat(String),

    #[error("Inference failure: {0}")]
    InferenceError(String),

    #[error("Voice activity detection error: {0}")]
    VadError(String),

    #[error("VAD speech rejection: audio buffer did not contain human speech")]
    NoSpeechDetected,

    #[error("Window focus restoration failed for window ID {0}: {1}")]
    WindowFocus(u64, String),

    #[error("Input keystroke injection failed: {0}")]
    InputInjection(String),

    #[error("Clipboard operation failed: {0}")]
    Clipboard(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core platform error: {0}")]
    Core(#[from] CoreError),
}

impl From<SttError> for CoreError {
    fn from(err: SttError) -> Self {
        match err {
            SttError::Core(ce) => ce,
            SttError::WindowFocus(id, msg) => CoreError::Window(format!("HWND {id}: {msg}")),
            SttError::InputInjection(msg) => CoreError::Input(msg),
            SttError::Clipboard(msg) => CoreError::Input(format!("Clipboard fallback error: {msg}")),
            SttError::InvalidWavHeader(msg) | SttError::WavDecodeError(msg) | SttError::UnsupportedAudioFormat(msg) => {
                CoreError::Audio(msg)
            }
            SttError::Io(e) => CoreError::Io(e.to_string()),
            other => CoreError::Internal(other.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, SttError>;
