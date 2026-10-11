use crate::error::CoreError;
use crate::types::*;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Domain events emitted across the `airctrl` application bus.
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

/// Centralized thread-safe asynchronous broadcast event bus.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    /// Creates a new `EventBus` with the given channel buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publishes a domain event to all active subscribers.
    ///
    /// Returns the number of subscribers that received the event (`Ok(count)`).
    /// If there are zero active subscribers, returns `Ok(0)` rather than treating
    /// the absence of listeners as a channel error.
    pub fn publish(&self, event: DomainEvent) -> Result<usize, CoreError> {
        match self.sender.send(event) {
            Ok(count) => Ok(count),
            Err(broadcast::error::SendError(_)) => Ok(0),
        }
    }

    /// Subscribes to the event stream, receiving all future published events.
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }

    /// Returns the number of active receivers listening to the bus.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
