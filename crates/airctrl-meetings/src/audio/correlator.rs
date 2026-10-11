use serde::{Deserialize, Serialize};

/// High-level correlated state combining meeting process inspection and audio stream activity.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrelatedState {
    /// Neither meeting process nor voice audio is detected.
    #[default]
    Inactive,
    /// Meeting process is detected, but microphone/output audio is silent.
    MeetingDetectedSilent,
    /// Meeting process is detected AND voice audio activity is present (active meeting).
    MeetingActiveVoice,
    /// Audio activity is present, but no meeting process is detected (e.g. YouTube, Spotify).
    NonMeetingAudio,
}

/// Correlates meeting process presence with real-time audio activity to eliminate false positives.
#[derive(Debug, Default, Clone)]
pub struct AudioProcessCorrelator {
    current_state: CorrelatedState,
}

impl AudioProcessCorrelator {
    pub fn new() -> Self {
        Self {
            current_state: CorrelatedState::Inactive,
        }
    }

    /// Evaluates combined meeting presence and voice activity flags.
    pub fn evaluate(&mut self, meeting_detected: bool, is_speech: bool) -> CorrelatedState {
        let state = match (meeting_detected, is_speech) {
            (false, false) => CorrelatedState::Inactive,
            (true, false) => CorrelatedState::MeetingDetectedSilent,
            (true, true) => CorrelatedState::MeetingActiveVoice,
            (false, true) => CorrelatedState::NonMeetingAudio,
        };
        self.current_state = state;
        state
    }

    /// Returns the current correlated state.
    pub fn current_state(&self) -> CorrelatedState {
        self.current_state
    }

    /// Resets the correlator back to `Inactive`.
    pub fn reset(&mut self) {
        self.current_state = CorrelatedState::Inactive;
    }
}
