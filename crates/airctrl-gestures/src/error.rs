//! Domain Error Types for MediaPipe Gestures and Video Frame Processing.

use airctrl_core::error::CoreError;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum GestureError {
    #[error("Invalid frame dimensions: width={width}, height={height}")]
    InvalidFrameDimensions { width: u32, height: u32 },

    #[error("Invalid buffer length: expected at least {expected} bytes for {width}x{height} with stride {stride}, got {actual}")]
    InvalidBufferLength {
        expected: usize,
        actual: usize,
        width: u32,
        height: u32,
        stride: usize,
    },

    #[error("Insufficient landmarks: expected 21 landmarks, got {0}")]
    InsufficientLandmarks(usize),

    #[error("MediaPipe engine is not ready: {0}")]
    EngineNotReady(String),

    #[error("Landmark inference error: {0}")]
    InferenceError(String),

    #[error("Landmark coordinates out of bounds: {0}")]
    LandmarkOutOfBounds(String),

    #[error("Invalid coordinate at index {index}: ({x}, {y}, {z}) contains NaN or Inf")]
    InvalidCoordinate {
        index: usize,
        x: f32,
        y: f32,
        z: f32,
    },

    #[error("Low confidence: {confidence} < threshold {threshold}")]
    LowConfidence { confidence: f32, threshold: f32 },

    #[error("Temporal filtering error: {0}")]
    TemporalFilter(String),

    #[error("Core error: {0}")]
    CoreError(String),
}

impl From<CoreError> for GestureError {
    fn from(err: CoreError) -> Self {
        GestureError::CoreError(err.to_string())
    }
}

impl From<GestureError> for CoreError {
    fn from(err: GestureError) -> Self {
        match err {
            GestureError::CoreError(msg) => CoreError::Internal(msg),
            GestureError::InvalidFrameDimensions { width, height } => {
                CoreError::Internal(format!("Invalid frame dimensions: {width}x{height}"))
            }
            GestureError::InvalidBufferLength {
                expected, actual, ..
            } => CoreError::Internal(format!(
                "Frame buffer length mismatch: expected {expected}, got {actual}"
            )),
            other => CoreError::Internal(other.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, GestureError>;
