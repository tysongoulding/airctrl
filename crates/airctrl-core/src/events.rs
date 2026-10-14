use crate::dag::{
    CommandParsedPayload, DagEvent, DagEventBus, GestureRecognizedPayload, InferenceEvent,
    KillCameraPayload, PlayDisclosurePayload, SensorEvent, SinkAction,
};
use crate::error::CoreError;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Domain events emitted across the `airctrl` application bus (Legacy Facade).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "category", content = "data")]
pub enum DomainEvent {
    /// STT lifecycle state changed (Idle, Recording, Processing, etc.)
    SttStateChanged(SttState),

    /// A speech segment was successfully transcribed to text
    TranscriptionCompleted(TranscriptionPayload),

    /// Speech recognition lifecycle state changed (Idle, Listening, Transcribing, Injecting, Error)
    SpeechStateChanged(SpeechState),

    /// Speech recognized event containing transcription payload
    SpeechRecognized(TranscriptionPayload),

    /// An action was triggered via voice command or radial dial
    ActionTriggered(ActionCommand),

    /// A hand gesture was recognized by the MediaPipe engine
    GestureRecognized {
        gesture: GestureType,
        dial: DialCoordinates,
    },

    /// Meeting lifecycle status changed (Inactive, Detected, Active, Ended)
    MeetingStatusChanged(MeetingStatus),

    /// An active meeting process or audio session was detected
    MeetingDetected(MeetingInfo),

    /// An active meeting process terminated
    MeetingEnded(MeetingInfo),

    /// Camera access was automatically revoked due to meeting privacy rules
    CameraKillSwitchTriggered {
        meeting: MeetingInfo,
        camera_state: CameraState,
    },

    /// Legal compliance audio was injected into the output audio stream
    ComplianceAudioInjected(ComplianceInjectionEvent),

    /// The LAN distribution daemon started successfully
    LanServerStarted(ServerMetadata),
}

impl DomainEvent {
    /// Translates a DomainEvent into its typed DAG counterpart.
    pub fn to_dag_event(&self) -> Option<DagEvent> {
        match self {
            Self::TranscriptionCompleted(p) | Self::SpeechRecognized(p) => {
                Some(DagEvent::Inference(InferenceEvent::TranscriptionCompleted(p.clone())))
            }
            Self::ActionTriggered(cmd) => match cmd {
                ActionCommand::AgentDispatch(payload) => {
                    Some(DagEvent::Sink(SinkAction::DispatchAgent(payload.clone())))
                }
                _ => Some(DagEvent::Inference(InferenceEvent::CommandParsed(
                    CommandParsedPayload {
                        utterance: String::new(),
                        command: cmd.clone(),
                        confidence_score: 100,
                        timestamp_ns: 0,
                    },
                ))),
            },
            Self::GestureRecognized { gesture, dial } => {
                Some(DagEvent::Inference(InferenceEvent::GestureRecognized(
                    GestureRecognizedPayload {
                        gesture: *gesture,
                        dial: dial.clone(),
                        landmarks: None,
                        confidence: 1.0,
                        timestamp_ns: 0,
                    },
                )))
            }
            Self::MeetingDetected(info) => {
                Some(DagEvent::Inference(InferenceEvent::MeetingDetected(info.clone())))
            }
            Self::CameraKillSwitchTriggered { meeting, .. } => {
                Some(DagEvent::Sink(SinkAction::KillCamera(KillCameraPayload {
                    meeting: meeting.clone(),
                    reason: "Camera privacy kill switch triggered".into(),
                    force_hardware_release: true,
                })))
            }
            Self::ComplianceAudioInjected(ev) => {
                Some(DagEvent::Sink(SinkAction::PlayDisclosure(PlayDisclosurePayload {
                    spec: ComplianceAudioSpec {
                        announcement_text: ev.message.clone(),
                        sample_rate: COMPLIANCE_AUDIO_SAMPLE_RATE,
                        volume: 1.0,
                    },
                    meeting: Some(ev.meeting_info.clone()),
                })))
            }
            _ => None,
        }
    }
}

/// Centralized thread-safe asynchronous broadcast event bus with DAG integration.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<DomainEvent>,
    dag_bus: Arc<DagEventBus>,
}

impl EventBus {
    /// Creates a new `EventBus` with the given channel buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        let dag_bus = Arc::new(DagEventBus::new(capacity));
        Self { sender, dag_bus }
    }

    /// Publishes a domain event to all active subscribers.
    ///
    /// Preserves exact legacy semantics: returns the recipient count of legacy
    /// subscribers (`Ok(count)`) or `Ok(0)` when zero subscribers exist.
    /// Also forwards mapped events to the internal DAG execution graph.
    pub fn publish(&self, event: DomainEvent) -> Result<usize, CoreError> {
        let count = self.sender.send(event.clone()).unwrap_or_default();

        if let Some(dag_ev) = event.to_dag_event() {
            let _ = self.dag_bus.publish(dag_ev);
        }

        Ok(count)
    }

    /// Subscribes to the legacy event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }

    /// Returns the number of active receivers listening to legacy DomainEvents.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Accesses the underlying typed Directed Acyclic Graph bus.
    pub fn dag(&self) -> &DagEventBus {
        &self.dag_bus
    }

    /// Publishes a Stage 0 sensor event directly to the DAG.
    pub fn publish_sensor(&self, event: SensorEvent) -> Result<usize, CoreError> {
        self.dag_bus.publish_sensor(event)
    }

    /// Publishes a Stage 1 inference event directly to the DAG, bridging to legacy domain events where applicable.
    pub fn publish_inference(&self, event: InferenceEvent) -> Result<usize, CoreError> {
        match &event {
            InferenceEvent::TranscriptionCompleted(p) => {
                let _ = self.sender.send(DomainEvent::TranscriptionCompleted(p.clone()));
            }
            InferenceEvent::GestureRecognized(g) => {
                let _ = self.sender.send(DomainEvent::GestureRecognized {
                    gesture: g.gesture,
                    dial: g.dial.clone(),
                });
            }
            InferenceEvent::MeetingDetected(m) => {
                let _ = self.sender.send(DomainEvent::MeetingDetected(m.clone()));
            }
            InferenceEvent::CommandParsed(c) => {
                let _ = self.sender.send(DomainEvent::ActionTriggered(c.command.clone()));
            }
            _ => {}
        }
        self.dag_bus.publish_inference(event)
    }

    /// Publishes a Stage 2 sink action directly to the DAG, bridging to legacy domain events where applicable.
    pub fn publish_sink(&self, action: SinkAction) -> Result<usize, CoreError> {
        match &action {
            SinkAction::DispatchAgent(a) => {
                let _ = self.sender.send(DomainEvent::ActionTriggered(ActionCommand::AgentDispatch(a.clone())));
            }
            SinkAction::KillCamera(k) => {
                let _ = self.sender.send(DomainEvent::CameraKillSwitchTriggered {
                    meeting: k.meeting.clone(),
                    camera_state: CameraState::KilledByMeeting,
                });
            }
            SinkAction::PlayDisclosure(p) => {
                if let Some(ref m) = p.meeting {
                    let _ = self.sender.send(DomainEvent::ComplianceAudioInjected(ComplianceInjectionEvent {
                        meeting_info: m.clone(),
                        message: p.spec.announcement_text.clone(),
                        audio_duration_ms: 1500,
                        status: "Injected".into(),
                    }));
                }
            }
            _ => {}
        }
        self.dag_bus.publish_sink(action)
    }

    pub fn subscribe_sensors(&self) -> broadcast::Receiver<SensorEvent> {
        self.dag_bus.subscribe_sensors()
    }

    pub fn subscribe_inferences(&self) -> broadcast::Receiver<InferenceEvent> {
        self.dag_bus.subscribe_inferences()
    }

    pub fn subscribe_sinks(&self) -> broadcast::Receiver<SinkAction> {
        self.dag_bus.subscribe_sinks()
    }

    pub fn subscribe_dag(&self) -> broadcast::Receiver<DagEvent> {
        self.dag_bus.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
