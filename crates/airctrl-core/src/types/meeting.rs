use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeetingPlatform {
    Zoom,
    MicrosoftTeams,
    GoogleMeet,
    Webex,
    Discord,
    SlackHuddle,
}

/// Meeting application identity enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeetingApp {
    Zoom,
    Teams,
    GoogleMeet,
    SlackHuddle,
    Webex,
    Discord,
}

impl MeetingApp {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zoom => "Zoom",
            Self::Teams => "Teams",
            Self::GoogleMeet => "Meet",
            Self::SlackHuddle => "SlackHuddle",
            Self::Webex => "Webex",
            Self::Discord => "Discord",
        }
    }
}

impl std::fmt::Display for MeetingApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<MeetingApp> for MeetingPlatform {
    fn from(app: MeetingApp) -> Self {
        match app {
            MeetingApp::Zoom => MeetingPlatform::Zoom,
            MeetingApp::Teams => MeetingPlatform::MicrosoftTeams,
            MeetingApp::GoogleMeet => MeetingPlatform::GoogleMeet,
            MeetingApp::SlackHuddle => MeetingPlatform::SlackHuddle,
            MeetingApp::Webex => MeetingPlatform::Webex,
            MeetingApp::Discord => MeetingPlatform::Discord,
        }
    }
}

impl From<MeetingPlatform> for MeetingApp {
    fn from(platform: MeetingPlatform) -> Self {
        match platform {
            MeetingPlatform::Zoom => MeetingApp::Zoom,
            MeetingPlatform::MicrosoftTeams => MeetingApp::Teams,
            MeetingPlatform::GoogleMeet => MeetingApp::GoogleMeet,
            MeetingPlatform::SlackHuddle => MeetingApp::SlackHuddle,
            MeetingPlatform::Webex => MeetingApp::Webex,
            MeetingPlatform::Discord => MeetingApp::Discord,
        }
    }
}

/// Lifecycle status for meeting detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeetingStatus {
    Inactive,
    Detected,
    Active,
    Ended,
}

impl std::fmt::Display for MeetingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inactive => write!(f, "Inactive"),
            Self::Detected => write!(f, "Detected"),
            Self::Active => write!(f, "Active"),
            Self::Ended => write!(f, "Ended"),
        }
    }
}

/// Detailed active meeting information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveMeetingInfo {
    pub app: MeetingApp,
    pub window_id: Option<u64>,
    pub pid: u32,
    pub title: String,
    pub start_time: DateTime<Utc>,
}

impl From<ActiveMeetingInfo> for MeetingInfo {
    fn from(info: ActiveMeetingInfo) -> Self {
        Self {
            platform: info.app.into(),
            process_id: info.pid,
            process_name: info.app.as_str().to_lowercase(),
            window_title: info.title,
            detected_at: info.start_time.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetingInfo {
    pub platform: MeetingPlatform,
    pub process_id: u32,
    pub process_name: String,
    pub window_title: String,
    pub detected_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraState {
    Active,
    KilledByMeeting,
    PausedByUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillSwitchStatus {
    pub status: String,
    pub camera_handle_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompliancePlaybackResult {
    pub announcement: String,
    pub duration_ms: u32,
    pub injected_into_stream: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplianceInjectionEvent {
    pub meeting_info: MeetingInfo,
    pub message: String,
    pub audio_duration_ms: u32,
    pub status: String,
}
