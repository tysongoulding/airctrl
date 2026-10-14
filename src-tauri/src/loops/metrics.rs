use airctrl_core::types::meeting::{CameraState, MeetingInfo, MeetingStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioVadHealthMetrics {
    pub is_running: bool,
    pub total_samples_ingested: u64,
    pub total_frames_processed: u64,
    pub speech_segments_detected: u64,
    pub last_rms_energy: f32,
    pub last_dbfs: f32,
    pub current_vad_state: String,
    pub ring_buffer_len: usize,
    pub last_tick_timestamp_ms: u64,
}

impl Default for AudioVadHealthMetrics {
    fn default() -> Self {
        Self {
            is_running: false,
            total_samples_ingested: 0,
            total_frames_processed: 0,
            speech_segments_detected: 0,
            last_rms_energy: 0.0,
            last_dbfs: -96.0,
            current_vad_state: "Silence".to_string(),
            ring_buffer_len: 0,
            last_tick_timestamp_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionGestureHealthMetrics {
    pub is_running: bool,
    pub camera_active: bool,
    pub target_fps: u32,
    pub actual_fps: f32,
    pub total_frames_processed: u64,
    pub hands_detected_count: u64,
    pub last_gesture: String,
    pub is_pinching: bool,
    pub pinch_distance: f32,
    pub active_sector: usize,
    pub polar_angle_rad: f32,
    pub normalized_radius: f32,
    pub last_tick_timestamp_ms: u64,
}

impl Default for VisionGestureHealthMetrics {
    fn default() -> Self {
        Self {
            is_running: false,
            camera_active: true,
            target_fps: 30,
            actual_fps: 0.0,
            total_frames_processed: 0,
            hands_detected_count: 0,
            last_gesture: "Neutral".to_string(),
            is_pinching: false,
            pinch_distance: 1.0,
            active_sector: 0,
            polar_angle_rad: 0.0,
            normalized_radius: 0.0,
            last_tick_timestamp_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeetingWatcherHealthMetrics {
    pub is_running: bool,
    pub poll_count: u64,
    pub status: MeetingStatus,
    pub active_meeting: Option<MeetingInfo>,
    pub camera_state: CameraState,
    pub compliance_audio_played: bool,
    pub last_poll_duration_ms: u64,
    pub last_tick_timestamp_ms: u64,
}

impl Default for MeetingWatcherHealthMetrics {
    fn default() -> Self {
        Self {
            is_running: false,
            poll_count: 0,
            status: MeetingStatus::Inactive,
            active_meeting: None,
            camera_state: CameraState::Active,
            compliance_audio_played: false,
            last_poll_duration_ms: 0,
            last_tick_timestamp_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalHotkeyHealthMetrics {
    pub is_running: bool,
    pub events_received: u64,
    pub stt_presses_count: u64,
    pub dial_presses_count: u64,
    pub last_press_duration_ms: u64,
    pub last_mode_detected: String,
    pub stt_speech_state: String,
    pub action_state: String,
    pub last_tick_timestamp_ms: u64,
}

impl Default for GlobalHotkeyHealthMetrics {
    fn default() -> Self {
        Self {
            is_running: false,
            events_received: 0,
            stt_presses_count: 0,
            dial_presses_count: 0,
            last_press_duration_ms: 0,
            last_mode_detected: "None".to_string(),
            stt_speech_state: "Idle".to_string(),
            action_state: "Idle".to_string(),
            last_tick_timestamp_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopHealthMetrics {
    pub uptime_secs: u64,
    pub all_healthy: bool,
    pub audio_vad: AudioVadHealthMetrics,
    pub vision_gesture: VisionGestureHealthMetrics,
    pub meeting_watcher: MeetingWatcherHealthMetrics,
    pub global_hotkey: GlobalHotkeyHealthMetrics,
}
