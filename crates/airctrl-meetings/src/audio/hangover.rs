/// Operational state of the speech activity hangover state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechState {
    Silence,
    SpeechPending,
    SpeechActive,
    HangoverHold,
}

/// State machine providing onset smoothing and hangover decay
/// to eliminate rapid fluttering between speech and silence.
#[derive(Debug, Clone)]
pub struct HangoverStateMachine {
    min_speech_frames: usize,
    hangover_frames: usize,
    consecutive_active_frames: usize,
    remaining_hangover_frames: usize,
    current_state: SpeechState,
}

impl HangoverStateMachine {
    pub fn new(min_speech_frames: usize, hangover_frames: usize) -> Self {
        Self {
            min_speech_frames: min_speech_frames.max(1),
            hangover_frames,
            consecutive_active_frames: 0,
            remaining_hangover_frames: 0,
            current_state: SpeechState::Silence,
        }
    }

    /// Evaluates a single chunk energy decision and updates the state machine.
    /// Returns `true` if speech is considered active (SpeechActive or HangoverHold).
    pub fn update(&mut self, is_frame_active: bool) -> bool {
        if is_frame_active {
            self.consecutive_active_frames += 1;
            self.remaining_hangover_frames = self.hangover_frames;

            if self.consecutive_active_frames >= self.min_speech_frames {
                self.current_state = SpeechState::SpeechActive;
            } else {
                self.current_state = SpeechState::SpeechPending;
            }
        } else {
            self.consecutive_active_frames = 0;

            match self.current_state {
                SpeechState::SpeechActive | SpeechState::HangoverHold => {
                    if self.remaining_hangover_frames > 0 {
                        self.remaining_hangover_frames -= 1;
                        self.current_state = SpeechState::HangoverHold;
                    } else {
                        self.current_state = SpeechState::Silence;
                    }
                }
                SpeechState::SpeechPending | SpeechState::Silence => {
                    self.current_state = SpeechState::Silence;
                }
            }
        }

        self.is_speech_active()
    }

    /// Returns whether the speech state is currently active.
    pub fn is_speech_active(&self) -> bool {
        matches!(
            self.current_state,
            SpeechState::SpeechActive | SpeechState::HangoverHold
        )
    }

    /// Returns the current state enum variant.
    pub fn current_state(&self) -> SpeechState {
        self.current_state
    }

    /// Resets the state machine back to `SpeechState::Silence`.
    pub fn reset(&mut self) {
        self.consecutive_active_frames = 0;
        self.remaining_hangover_frames = 0;
        self.current_state = SpeechState::Silence;
    }
}
