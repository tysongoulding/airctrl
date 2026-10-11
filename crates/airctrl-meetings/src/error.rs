use airctrl_core::error::CoreError;
use thiserror::Error;

/// Error type for the `airctrl-meetings` subsystem.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum MeetingError {
    #[error("Process scan failure: {0}")]
    ProcessScanFailed(String),

    #[error("Window enumeration failure: {0}")]
    WindowEnumerationFailed(String),

    #[error("Audio device failure: {0}")]
    AudioDeviceError(String),

    #[error("Audio playback failure: {0}")]
    AudioPlaybackFailed(String),

    #[error("Audio capture failure: {0}")]
    AudioCaptureFailed(String),

    #[error("Compliance generation failure: {0}")]
    ComplianceGenerationFailed(String),

    #[error("Audio volume violation: requested {requested}, but legal minimum volume floor is {minimum}")]
    VolumeBelowComplianceFloor { requested: f32, minimum: f32 },

    #[error("Camera kill switch failure: {0}")]
    CameraKillSwitchError(String),

    #[error("Recording session blocked: compliance announcement has not completed")]
    RecordingBlockedByCompliance,

    #[error("Audio monitor error: {0}")]
    AudioMonitorError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Watcher is already running")]
    AlreadyRunning,

    #[error("Watcher is not running")]
    NotRunning,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Core platform error: {0}")]
    CoreError(String),
}

pub type Result<T> = std::result::Result<T, MeetingError>;

impl From<CoreError> for MeetingError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::Audio(s) => MeetingError::AudioPlaybackFailed(s),
            CoreError::Config(s) => MeetingError::InvalidConfig(s),
            CoreError::ProcessNotFound(pid) => {
                MeetingError::ProcessScanFailed(format!("PID {pid} not found"))
            }
            CoreError::WindowNotFound(wid) => {
                MeetingError::WindowEnumerationFailed(format!("Window {wid} not found"))
            }
            CoreError::EventBus(s) => MeetingError::ChannelError(s),
            other => MeetingError::CoreError(other.to_string()),
        }
    }
}

impl From<MeetingError> for CoreError {
    fn from(err: MeetingError) -> Self {
        match err {
            MeetingError::ProcessScanFailed(s) => CoreError::Internal(s),
            MeetingError::WindowEnumerationFailed(s) => CoreError::Window(s),
            MeetingError::AudioDeviceError(s) => CoreError::DeviceNotFound(s),
            MeetingError::AudioPlaybackFailed(s) | MeetingError::AudioCaptureFailed(s) => {
                CoreError::Audio(s)
            }
            MeetingError::ComplianceGenerationFailed(s) => CoreError::Audio(s),
            MeetingError::VolumeBelowComplianceFloor { requested, minimum } => {
                CoreError::Config(format!(
                    "Volume {requested} below legal compliance floor {minimum}"
                ))
            }
            MeetingError::CameraKillSwitchError(s) => CoreError::Internal(s),
            MeetingError::RecordingBlockedByCompliance => {
                CoreError::PermissionDenied("Compliance audio required before recording".into())
            }
            MeetingError::AudioMonitorError(s) => CoreError::Audio(s),
            MeetingError::ChannelError(s) => CoreError::EventBus(s),
            MeetingError::AlreadyRunning => CoreError::Internal("Already running".into()),
            MeetingError::NotRunning => CoreError::Internal("Not running".into()),
            MeetingError::InvalidConfig(s) => CoreError::Config(s),
            MeetingError::CoreError(s) => CoreError::Internal(s),
        }
    }
}
