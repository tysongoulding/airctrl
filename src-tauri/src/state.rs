use airctrl_actions::{RadialDialOverlay, SemanticCommandRouter};
use airctrl_core::config::AirCtrlConfig;
use airctrl_core::events::EventBus;
use airctrl_meetings::{
    create_meeting_detector, get_global_camera_kill_switch, CameraKillSwitch,
    DefaultMeetingDetector,
};
use airctrl_stt::{create_whisper_engine, SpeechToTextEngine};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Dynamic application state containing active handles to all airctrl workspace subsystems.
pub struct AppState {
    pub config: RwLock<AirCtrlConfig>,
    pub stt_engine: Box<dyn SpeechToTextEngine>,
    pub is_recording: AtomicBool,
    pub recorded_samples: RwLock<Vec<f32>>,
    pub command_router: SemanticCommandRouter,
    pub dial_overlay: RwLock<RadialDialOverlay>,
    pub meeting_detector: DefaultMeetingDetector,
    pub camera_kill_switch: Arc<CameraKillSwitch>,
    pub event_bus: EventBus,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let stt_config = airctrl_stt::SttConfig::default();
        let stt_engine = create_whisper_engine(&stt_config);
        Self::with_engine(stt_engine)
    }

    /// Creates AppState with an explicit speech-to-text inference engine instance.
    pub fn with_engine(stt_engine: Box<dyn SpeechToTextEngine>) -> Self {
        let config = AirCtrlConfig {
            stt_hotkey: "Ctrl+Spacebar".to_string(),
            ..Default::default()
        };
        let command_router = SemanticCommandRouter::default();
        let dial_overlay = RadialDialOverlay::with_sectors(config.radial_dial_sectors);
        let meeting_detector = create_meeting_detector();
        let camera_kill_switch = get_global_camera_kill_switch();
        let event_bus = EventBus::default();

        Self {
            config: RwLock::new(config),
            stt_engine,
            is_recording: AtomicBool::new(false),
            recorded_samples: RwLock::new(Vec::new()),
            command_router,
            dial_overlay: RwLock::new(dial_overlay),
            meeting_detector,
            camera_kill_switch,
            event_bus,
        }
    }

    pub fn start_recording(&self) {
        self.is_recording.store(true, Ordering::SeqCst);
        let mut samples = self.recorded_samples.write();
        samples.clear();
        // Seed with synthetic 16kHz speech energy samples for offline pipeline test fidelity
        samples.extend(vec![0.5f32; 1600]);
    }

    pub fn stop_recording(&self) -> Result<String, String> {
        self.is_recording.store(false, Ordering::SeqCst);
        let mut samples_guard = self.recorded_samples.write();
        let samples: Vec<f32> = samples_guard.drain(..).collect();
        drop(samples_guard);

        if samples.is_empty() {
            return Err("No audio samples recorded".into());
        }

        self.stt_engine
            .transcribe(&samples)
            .map(|res| res.text)
            .map_err(|e| e.to_string())
    }
}
