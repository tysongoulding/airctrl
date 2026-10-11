use thiserror::Error;

/// Unified domain error type across the `airctrl` suite.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("Window management error: {0}")]
    Window(String),

    #[error("Input simulation error: {0}")]
    Input(String),

    #[error("Audio stream or device error: {0}")]
    Audio(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Platform operation not supported on this operating system: {0}")]
    PlatformUnsupported(String),

    #[error("Hardware device not found: {0}")]
    DeviceNotFound(String),

    #[error("Target window not found (ID: {0})")]
    WindowNotFound(u64),

    #[error("Process not found (PID: {0})")]
    ProcessNotFound(u32),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Event bus lag or communication channel closed: {0}")]
    EventBus(String),

    #[error("Internal engine error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        CoreError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        CoreError::Serialization(err.to_string())
    }
}
