use crate::error::CoreError;
use crate::types::action::{ActionCommand, AgentPayload};
use crate::types::audio::ComplianceAudioSpec;
use crate::types::gesture::{DialCoordinates, GestureType, HandLandmarks};
use crate::types::meeting::MeetingInfo;
use crate::types::stt::TranscriptionPayload;
use crate::types::window::ProcessInfo;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

// ============================================================================
// Stage 0: Sensors
// ============================================================================

/// Normalized audio chunk payload from microphone capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawAudioChunkPayload {
    /// Normalized audio samples [-1.0, 1.0]
    pub samples: Vec<f32>,
    /// Sample rate in Hz (typically 16,000 for Whisper/VAD)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Monotonic timestamp in nanoseconds
    pub timestamp_ns: u64,
}

/// Pixel format enum for raw camera frame buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoPixelFormat {
    Rgb8,
    Rgba8,
    Bgr8,
    Bgra8,
}

/// Raw camera frame payload from video capture loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoFramePayload {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub format: VideoPixelFormat,
    pub data: Vec<u8>,
    pub timestamp_ns: u64,
}

/// Periodic operating system process scan payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessScanPayload {
    pub processes: Vec<ProcessInfo>,
    pub timestamp_ns: u64,
}

/// Hotkey trigger category for low-level keyboard listeners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotkeyTrigger {
    SttToggle,
    SttPushToTalkPress,
    SttPushToTalkRelease,
    ActionRadialDial,
    ActionCommandPalette,
}

/// Low-level global hotkey trigger event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyEventPayload {
    pub trigger: HotkeyTrigger,
    pub raw_combination: String,
    pub duration_ms: u64,
    pub timestamp_ns: u64,
}

/// Stage 0 (Sensors): Raw physical and OS sensor inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SensorEvent {
    /// 16kHz mono PCM audio chunk from microphone ring buffer
    RawAudioChunk(RawAudioChunkPayload),
    /// Raw camera frame from video capture loop
    VideoFrame(VideoFramePayload),
    /// Periodic system process snapshot from meeting/app watcher
    ProcessScan(ProcessScanPayload),
    /// Low-level global hotkey trigger event with duration timing
    HotkeyEvent(HotkeyEventPayload),
}

// ============================================================================
// Stage 1: Inferences
// ============================================================================

/// Real-time voice activity detection inference payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechDetectedPayload {
    pub is_speech: bool,
    pub energy_dbfs: f32,
    pub hangover_active: bool,
    pub samples: Vec<f32>,
    pub timestamp_ns: u64,
}

/// Hand gesture and radial dial coordinate inference payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureRecognizedPayload {
    pub gesture: GestureType,
    pub dial: DialCoordinates,
    pub landmarks: Option<HandLandmarks>,
    pub confidence: f32,
    pub timestamp_ns: u64,
}

/// Semantic command parsing inference payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandParsedPayload {
    pub utterance: String,
    pub command: ActionCommand,
    pub confidence_score: u8,
    pub timestamp_ns: u64,
}

/// Stage 1 (Inferences): Semantic inferences produced by local AI and heuristic models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum InferenceEvent {
    /// Voice activity detection energy assessment and hangover state
    SpeechDetected(SpeechDetectedPayload),
    /// Speech segment transcribed into text by Whisper STT
    TranscriptionCompleted(TranscriptionPayload),
    /// MediaPipe 21-landmark hand gesture and radial polar coordinates
    GestureRecognized(GestureRecognizedPayload),
    /// Conferencing application discovered (Zoom, Teams, Meet, Webex, Discord)
    MeetingDetected(MeetingInfo),
    /// Voice utterance parsed into structured desktop action command
    CommandParsed(CommandParsedPayload),
}

// ============================================================================
// Stage 2: Sinks
// ============================================================================

/// Target application text injection payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectTextPayload {
    pub text: String,
    pub target_window_id: Option<u64>,
    pub force_clipboard: bool,
}

/// UI Overlay category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayType {
    RadialDial,
    SttFloatingPill,
    CommandPalette,
}

/// UI overlay visibility and state toggle payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleOverlayPayload {
    pub overlay: OverlayType,
    pub visible: Option<bool>,
    pub active_sector: Option<usize>,
}

/// Hardware camera privacy kill switch trigger payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillCameraPayload {
    pub meeting: MeetingInfo,
    pub reason: String,
    pub force_hardware_release: bool,
}

/// Statutory recording compliance disclosure audio playback payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayDisclosurePayload {
    pub spec: ComplianceAudioSpec,
    pub meeting: Option<MeetingInfo>,
}

/// Stage 2 (Sinks): Concrete operating system side-effects and action triggers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SinkAction {
    /// Inject transcribed text into active window with clipboard fallback
    InjectText(InjectTextPayload),
    /// Toggle visibility or active sector of floating pill or radial dial
    ToggleOverlay(ToggleOverlayPayload),
    /// Sub-millisecond hardware camera handle revocation
    KillCamera(KillCameraPayload),
    /// Statutory legal compliance disclosure audio playback
    PlayDisclosure(PlayDisclosurePayload),
    /// Route structured instruction payload to developer agent CLI/API
    DispatchAgent(AgentPayload),
}

// ============================================================================
// Unified DAG Envelope & Stage
// ============================================================================

/// Stage classification in the 3-stage execution DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DagStage {
    Stage0Sensor,
    Stage1Inference,
    Stage2Sink,
}

/// Unified envelope representing any event across the 3 DAG stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "stage", content = "payload")]
pub enum DagEvent {
    Sensor(SensorEvent),
    Inference(InferenceEvent),
    Sink(SinkAction),
}

impl DagEvent {
    pub fn stage(&self) -> DagStage {
        match self {
            Self::Sensor(_) => DagStage::Stage0Sensor,
            Self::Inference(_) => DagStage::Stage1Inference,
            Self::Sink(_) => DagStage::Stage2Sink,
        }
    }
}

// ============================================================================
// DAG Propagation Policy & Engine
// ============================================================================

/// Configurable policy governing causal downstream propagation in the DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DagPropagationPolicy {
    pub auto_inject_transcription: bool,
    pub auto_kill_camera_on_meeting: bool,
    pub auto_play_disclosure_on_meeting: bool,
    pub auto_dispatch_agent_commands: bool,
    pub text_length_clipboard_threshold: usize,
}

impl Default for DagPropagationPolicy {
    fn default() -> Self {
        Self {
            auto_inject_transcription: true,
            auto_kill_camera_on_meeting: true,
            auto_play_disclosure_on_meeting: true,
            auto_dispatch_agent_commands: true,
            text_length_clipboard_threshold: 200,
        }
    }
}

/// Pure evaluation engine calculating downstream DAG events from upstream events.
pub struct DagPropagationEngine;

impl DagPropagationEngine {
    /// Evaluates causal downstream propagation for an incoming DAG event according to policy.
    pub fn evaluate(event: &DagEvent, policy: &DagPropagationPolicy) -> Vec<DagEvent> {
        match event {
            DagEvent::Inference(InferenceEvent::TranscriptionCompleted(payload)) => {
                let mut results = Vec::new();
                if policy.auto_inject_transcription && !payload.text.trim().is_empty() {
                    results.push(DagEvent::Sink(SinkAction::InjectText(InjectTextPayload {
                        text: payload.text.clone(),
                        target_window_id: None,
                        force_clipboard: payload.text.len() > policy.text_length_clipboard_threshold,
                    })));
                }
                results
            }
            DagEvent::Inference(InferenceEvent::MeetingDetected(info)) => {
                let mut results = Vec::new();
                if policy.auto_kill_camera_on_meeting {
                    results.push(DagEvent::Sink(SinkAction::KillCamera(KillCameraPayload {
                        meeting: info.clone(),
                        reason: "Automatic meeting discovery privacy kill-switch".to_string(),
                        force_hardware_release: true,
                    })));
                }
                if policy.auto_play_disclosure_on_meeting {
                    results.push(DagEvent::Sink(SinkAction::PlayDisclosure(PlayDisclosurePayload {
                        spec: ComplianceAudioSpec::default(),
                        meeting: Some(info.clone()),
                    })));
                }
                results
            }
            DagEvent::Inference(InferenceEvent::CommandParsed(payload)) => {
                let mut results = Vec::new();
                match &payload.command {
                    ActionCommand::AgentDispatch(agent_payload) => {
                        if policy.auto_dispatch_agent_commands {
                            results.push(DagEvent::Sink(SinkAction::DispatchAgent(agent_payload.clone())));
                        }
                    }
                    ActionCommand::RadialDialToggle {} => {
                        results.push(DagEvent::Sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                            overlay: OverlayType::RadialDial,
                            visible: None,
                            active_sector: None,
                        })));
                    }
                    _ => {}
                }
                results
            }
            DagEvent::Inference(InferenceEvent::GestureRecognized(payload)) => {
                let mut results = Vec::new();
                if payload.gesture == GestureType::Pinch && payload.dial.is_selected {
                    results.push(DagEvent::Sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                        overlay: OverlayType::RadialDial,
                        visible: Some(true),
                        active_sector: Some(payload.dial.active_sector),
                    })));
                }
                results
            }
            DagEvent::Sensor(SensorEvent::HotkeyEvent(payload)) => {
                let mut results = Vec::new();
                match payload.trigger {
                    HotkeyTrigger::ActionRadialDial => {
                        results.push(DagEvent::Sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                            overlay: OverlayType::RadialDial,
                            visible: None,
                            active_sector: None,
                        })));
                    }
                    HotkeyTrigger::SttToggle => {
                        results.push(DagEvent::Sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                            overlay: OverlayType::SttFloatingPill,
                            visible: None,
                            active_sector: None,
                        })));
                    }
                    _ => {}
                }
                results
            }
            _ => Vec::new(),
        }
    }
}

// ============================================================================
// DagEventBus & Metrics
// ============================================================================

/// Real-time telemetry snapshot of DAG routing activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagMetrics {
    pub active: bool,
    pub stages: Vec<String>,
    pub events_routed_sensors: usize,
    pub events_routed_inferences: usize,
    pub events_routed_sinks: usize,
    pub dropped_events: usize,
    pub capacity: usize,
}

/// Centralized typed Directed Acyclic Execution Graph event bus.
#[derive(Clone)]
pub struct DagEventBus {
    sensors: broadcast::Sender<SensorEvent>,
    inferences: broadcast::Sender<InferenceEvent>,
    sinks: broadcast::Sender<SinkAction>,
    unified: broadcast::Sender<DagEvent>,
    policy: Arc<RwLock<DagPropagationPolicy>>,
    sensor_counter: Arc<AtomicUsize>,
    inference_counter: Arc<AtomicUsize>,
    sink_counter: Arc<AtomicUsize>,
    dropped_counter: Arc<AtomicUsize>,
    capacity: usize,
}

impl DagEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sensors, _) = broadcast::channel(capacity);
        let (inferences, _) = broadcast::channel(capacity);
        let (sinks, _) = broadcast::channel(capacity);
        let (unified, _) = broadcast::channel(capacity);

        Self {
            sensors,
            inferences,
            sinks,
            unified,
            policy: Arc::new(RwLock::new(DagPropagationPolicy::default())),
            sensor_counter: Arc::new(AtomicUsize::new(0)),
            inference_counter: Arc::new(AtomicUsize::new(0)),
            sink_counter: Arc::new(AtomicUsize::new(0)),
            dropped_counter: Arc::new(AtomicUsize::new(0)),
            capacity,
        }
    }

    /// Publishes a Stage 0 Sensor event to sensor subscribers and the unified DAG stream.
    pub fn publish_sensor(&self, event: SensorEvent) -> Result<usize, CoreError> {
        self.sensor_counter.fetch_add(1, Ordering::Relaxed);
        let _ = self.unified.send(DagEvent::Sensor(event.clone()));
        match self.sensors.send(event) {
            Ok(count) => Ok(count),
            Err(broadcast::error::SendError(_)) => Ok(0),
        }
    }

    /// Publishes a Stage 1 Inference event to inference subscribers and the unified DAG stream.
    pub fn publish_inference(&self, event: InferenceEvent) -> Result<usize, CoreError> {
        self.inference_counter.fetch_add(1, Ordering::Relaxed);
        let _ = self.unified.send(DagEvent::Inference(event.clone()));
        match self.inferences.send(event) {
            Ok(count) => Ok(count),
            Err(broadcast::error::SendError(_)) => Ok(0),
        }
    }

    /// Publishes a Stage 2 Sink action to sink subscribers and the unified DAG stream.
    pub fn publish_sink(&self, action: SinkAction) -> Result<usize, CoreError> {
        self.sink_counter.fetch_add(1, Ordering::Relaxed);
        let _ = self.unified.send(DagEvent::Sink(action.clone()));
        match self.sinks.send(action) {
            Ok(count) => Ok(count),
            Err(broadcast::error::SendError(_)) => Ok(0),
        }
    }

    /// Publishes an arbitrary DagEvent to the unified stream and the appropriate stage channel.
    pub fn publish(&self, event: DagEvent) -> Result<usize, CoreError> {
        match &event {
            DagEvent::Sensor(s) => {
                self.sensor_counter.fetch_add(1, Ordering::Relaxed);
                let _ = self.sensors.send(s.clone());
            }
            DagEvent::Inference(i) => {
                self.inference_counter.fetch_add(1, Ordering::Relaxed);
                let _ = self.inferences.send(i.clone());
            }
            DagEvent::Sink(s) => {
                self.sink_counter.fetch_add(1, Ordering::Relaxed);
                let _ = self.sinks.send(s.clone());
            }
        }
        match self.unified.send(event) {
            Ok(count) => Ok(count),
            Err(broadcast::error::SendError(_)) => Ok(0),
        }
    }

    /// Publishes an event, evaluates propagation rules according to policy,
    /// and publishes all resulting downstream events.
    pub fn propagate_and_publish(&self, event: DagEvent) -> Result<Vec<DagEvent>, CoreError> {
        let _ = self.publish(event.clone());
        let policy = self.policy();
        let downstream = DagPropagationEngine::evaluate(&event, &policy);
        for sub_event in &downstream {
            let _ = self.publish(sub_event.clone());
        }
        Ok(downstream)
    }

    pub fn subscribe_sensors(&self) -> broadcast::Receiver<SensorEvent> {
        self.sensors.subscribe()
    }

    pub fn subscribe_inferences(&self) -> broadcast::Receiver<InferenceEvent> {
        self.inferences.subscribe()
    }

    pub fn subscribe_sinks(&self) -> broadcast::Receiver<SinkAction> {
        self.sinks.subscribe()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DagEvent> {
        self.unified.subscribe()
    }

    pub fn sensor_subscriber_count(&self) -> usize {
        self.sensors.receiver_count()
    }

    pub fn inference_subscriber_count(&self) -> usize {
        self.inferences.receiver_count()
    }

    pub fn sink_subscriber_count(&self) -> usize {
        self.sinks.receiver_count()
    }

    pub fn unified_subscriber_count(&self) -> usize {
        self.unified.receiver_count()
    }

    pub fn policy(&self) -> DagPropagationPolicy {
        self.policy
            .read()
            .map(|p| p.clone())
            .unwrap_or_default()
    }

    pub fn set_policy(&self, policy: DagPropagationPolicy) {
        if let Ok(mut lock) = self.policy.write() {
            *lock = policy;
        }
    }

    /// Returns runtime telemetry metrics for IPC and monitoring.
    pub fn metrics(&self) -> DagMetrics {
        DagMetrics {
            active: true,
            stages: vec![
                "Sensors".to_string(),
                "Inferences".to_string(),
                "Sinks".to_string(),
            ],
            events_routed_sensors: self.sensor_counter.load(Ordering::Relaxed),
            events_routed_inferences: self.inference_counter.load(Ordering::Relaxed),
            events_routed_sinks: self.sink_counter.load(Ordering::Relaxed),
            dropped_events: self.dropped_counter.load(Ordering::Relaxed),
            capacity: self.capacity,
        }
    }
}

impl Default for DagEventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
