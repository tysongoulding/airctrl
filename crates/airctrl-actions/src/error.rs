use airctrl_core::CoreError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ActionError>;

/// Domain errors encountered during voice command parsing, fuzzy matching, crawler, and action routing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ActionError {
    #[error("Speech-to-action parse error: {0}")]
    ParseError(String),

    #[error("Target window '{0}' not found")]
    WindowNotFound(String),

    #[error("Target window or application '{0}' not found")]
    TargetNotFound(String),

    #[error("Process not found (PID: {0})")]
    ProcessNotFound(u32),

    #[error("Invalid PID: {0}. PID must be greater than 0")]
    InvalidPid(i64),

    #[error("Ambiguous target query '{query}': matching candidates [{candidates}]")]
    AmbiguousTarget {
        query: String,
        candidates: String,
    },

    #[error("Unsupported agent target: {0}")]
    UnsupportedAgent(String),

    #[error("Agent environment detection error: {0}")]
    AgentEnvironment(String),

    #[error("Application crawler error: {0}")]
    CrawlerError(String),

    #[error("Action execution failed: {0}")]
    ExecutionFailed(String),

    #[error("REA reverse-engineering inspection failed: {0}")]
    ReaInspectionError(String),

    #[error("Invalid state machine transition: from {from:?} to {to:?}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Core platform error: {0}")]
    Core(String),
}

impl From<CoreError> for ActionError {
    fn from(err: CoreError) -> Self {
        ActionError::Core(err.to_string())
    }
}
