use serde::{Deserialize, Serialize};

/// Master application configuration settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirCtrlConfig {
    /// Hotkey triggering Push-to-Talk or Toggle STT transcription (default: "Ctrl+Space").
    pub stt_hotkey: String,

    /// Hotkey toggling the center-focus radial dial overlay (default: "Ctrl+Alt").
    pub dial_hotkey: String,

    /// Port on which the embedded LAN distribution HTTP daemon listens (default: 8080).
    pub lan_port: u16,

    /// Path to the offline Whisper model weights file.
    pub whisper_model_path: String,

    /// Whether compliance disclosure audio is automatically played when recording starts.
    pub compliance_audio_enabled: bool,

    /// Whether the camera stream is automatically killed when meetings are detected.
    pub camera_auto_kill: bool,

    /// Target FPS for CPU MediaPipe hand tracking (default: 30).
    pub gesture_fps_target: u32,

    /// Number of action sectors rendered in the radial dial (default: 6).
    pub radial_dial_sectors: usize,

    /// Whether to force the in-memory mock platform for all operations.
    pub is_mock_platform: bool,
}

impl Default for AirCtrlConfig {
    fn default() -> Self {
        Self {
            stt_hotkey: "Ctrl+Space".to_string(),
            dial_hotkey: "Ctrl+Alt".to_string(),
            lan_port: 8080,
            whisper_model_path: "models/ggml-tiny.en.bin".to_string(),
            compliance_audio_enabled: true,
            camera_auto_kill: true,
            gesture_fps_target: 30,
            radial_dial_sectors: 6,
            is_mock_platform: false,
        }
    }
}
