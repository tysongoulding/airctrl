use serde::{Deserialize, Serialize};

// Re-export domain types from airctrl-core
pub use airctrl_core::types::meeting::{
    ActiveMeetingInfo, CameraState, ComplianceInjectionEvent, CompliancePlaybackResult,
    KillSwitchStatus, MeetingApp, MeetingInfo, MeetingPlatform, MeetingStatus,
};

/// Represents an inspected process or window item evaluated by the meeting detector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessScanItem {
    #[serde(alias = "pid")]
    pub process_id: u32,
    pub process_name: String,
    #[serde(alias = "window_title")]
    pub title: String,
    #[serde(default)]
    pub window_id: Option<u64>,
}

impl ProcessScanItem {
    pub fn new(process_id: u32, process_name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            process_id,
            process_name: process_name.into(),
            title: title.into(),
            window_id: None,
        }
    }

    pub fn with_window_id(mut self, window_id: u64) -> Self {
        self.window_id = Some(window_id);
        self
    }
}

/// Static signature criteria defining a detectable conferencing platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingSignature {
    pub app: MeetingApp,
    pub process_names: &'static [&'static str],
    pub title_keywords: &'static [&'static str],
    pub requires_title_match: bool,
}

/// A timestamped segment of live or imported meeting transcription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveTranscriptChunk {
    pub id: String,
    pub speaker: String, // "Host", "Participant", "Mixed"
    pub text: String,
    #[serde(alias = "start_ms")]
    pub start_time_ms: u64,
    #[serde(alias = "end_ms")]
    pub end_time_ms: u64,
    pub confidence: f32,
}

