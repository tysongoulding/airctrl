use serde::{Deserialize, Serialize};

/// Type of compliance announcement message to inject into meeting streams.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceMessageKind {
    /// "This meeting is being recorded" (exactly 30 characters)
    #[default]
    LegalRecordingDisclosure,
    /// "This meeting is being transcribed and automated by airctrl assistant."
    AssistantTranscriptionNotice,
    /// Custom user-defined disclosure
    Custom(String),
}

impl ComplianceMessageKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::LegalRecordingDisclosure => "This meeting is being recorded",
            Self::AssistantTranscriptionNotice => {
                "This meeting is being transcribed and automated by airctrl assistant."
            }
            Self::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for ComplianceMessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration parameters for compliance audio injection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceConfig {
    /// Message variant to announce
    pub message: ComplianceMessageKind,
    /// Output sample rate in Hz (default: 16,000)
    pub sample_rate: u32,
    /// Audio channels (default: 1 for mono)
    pub channels: u16,
    /// Requested volume [0.0, 1.0]. Clamped to minimum floor of 0.50 (cannot be muted).
    pub volume: f32,
    /// Duration of announcement in milliseconds (default: 1450ms)
    pub duration_ms: u32,
    /// Enforces recording lockout until announcement completes
    pub block_recording_until_complete: bool,
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        Self {
            message: ComplianceMessageKind::LegalRecordingDisclosure,
            sample_rate: 16_000,
            channels: 1,
            volume: 1.0,
            duration_ms: 1450,
            block_recording_until_complete: true,
        }
    }
}

impl ComplianceConfig {
    /// Enforces legal volume floor of 0.50 (cannot be muted).
    pub fn effective_volume(&self) -> f32 {
        self.volume.clamp(0.50, 1.0)
    }
}

/// Configuration parameters for audio stream energy monitoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMonitorConfig {
    /// Normalized RMS threshold for active speech (default: 0.05)
    pub energy_threshold: f32,
    /// Silence floor in dBFS (default: -35.0 dBFS)
    pub silence_threshold_dbfs: f32,
    /// Minimum speech duration in ms to trigger active voice state (default: 300 ms)
    pub min_speech_duration_ms: u64,
    /// Silence timeout in ms before dropping active voice state (default: 2000 ms)
    pub silence_timeout_ms: u64,
    /// Audio chunk size in samples (default: 1600 = 100ms at 16kHz)
    pub chunk_size_samples: usize,
    /// Audio sample rate in Hz (default: 16,000)
    pub sample_rate: u32,
}

impl Default for AudioMonitorConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.05,
            silence_threshold_dbfs: -35.0,
            min_speech_duration_ms: 300,
            silence_timeout_ms: 2000,
            chunk_size_samples: 1600,
            sample_rate: 16_000,
        }
    }
}

/// Configuration parameters for the meeting watcher background loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeetingWatcherConfig {
    /// Polling interval in milliseconds (default: 1000ms)
    pub poll_interval_ms: u64,
    /// Debounce ticks before confirming state transitions (default: 2)
    pub debounce_ticks: u32,
    /// Automatically engage camera privacy kill switch upon meeting discovery
    pub auto_kill_camera: bool,
    /// Automatically trigger compliance disclosure upon meeting discovery
    pub auto_compliance_audio: bool,
    /// Compliance audio parameters
    pub compliance_config: ComplianceConfig,
    /// Audio stream monitor parameters
    pub audio_config: AudioMonitorConfig,
    /// Whether to monitor real-time microphone stream activity
    pub monitor_audio_activity: bool,
}

impl Default for MeetingWatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
            debounce_ticks: 2,
            auto_kill_camera: true,
            auto_compliance_audio: false,
            compliance_config: ComplianceConfig::default(),
            audio_config: AudioMonitorConfig::default(),
            monitor_audio_activity: false,
        }
    }
}
