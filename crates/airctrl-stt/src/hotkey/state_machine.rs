use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::{HotkeyMode, SpeechState, SttState, TranscriptionPayload};

/// Discrete action instructions emitted by the hotkey state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMachineAction {
    None,
    StartRecording { target_window_id: Option<u64> },
    CancelRecordingDiscardBuffer,
    StopRecordingAndTranscribe,
    StartInjection,
    ReturnToIdle,
    ReportError(String),
}

/// Result of processing a hotkey or lifecycle event in the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionResult {
    NoChange,
    Transitioned {
        from: SpeechState,
        to: SpeechState,
        action: StateMachineAction,
    },
    IgnoredAccidentalTap {
        duration_ms: u64,
    },
}

/// Global hotkey state machine coordinating Push-to-Talk (`Ctrl+Spacebar`) and Toggle transcription.
pub struct HotkeyStateMachine {
    mode: HotkeyMode,
    state: SpeechState,
    press_start_time_ms: u64,
    min_press_duration_ms: u64,
    target_window_id: Option<u64>,
    event_bus: Option<EventBus>,
}

impl HotkeyStateMachine {
    pub const DEFAULT_MIN_DURATION_MS: u64 = 100;

    pub fn new(mode: HotkeyMode, event_bus: Option<EventBus>) -> Self {
        Self {
            mode,
            state: SpeechState::Idle,
            press_start_time_ms: 0,
            min_press_duration_ms: Self::DEFAULT_MIN_DURATION_MS,
            target_window_id: None,
            event_bus,
        }
    }

    pub fn mode(&self) -> HotkeyMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: HotkeyMode) {
        self.mode = mode;
    }

    pub fn state(&self) -> &SpeechState {
        &self.state
    }

    pub fn target_window_id(&self) -> Option<u64> {
        self.target_window_id
    }

    pub fn min_press_duration_ms(&self) -> u64 {
        self.min_press_duration_ms
    }

    pub fn set_min_press_duration_ms(&mut self, duration_ms: u64) {
        self.min_press_duration_ms = duration_ms;
    }

    /// Handles key-down event (`Ctrl+Spacebar` pressed).
    pub fn on_key_down(
        &mut self,
        timestamp_ms: u64,
        active_window_id: Option<u64>,
    ) -> StateTransitionResult {
        match self.mode {
            HotkeyMode::PushToTalk => {
                if self.state == SpeechState::Idle {
                    let from = self.state.clone();
                    let to = SpeechState::Listening;
                    self.state = to.clone();
                    self.press_start_time_ms = timestamp_ms;
                    self.target_window_id = active_window_id;
                    self.publish_state_changed(&to);

                    StateTransitionResult::Transitioned {
                        from,
                        to,
                        action: StateMachineAction::StartRecording {
                            target_window_id: active_window_id,
                        },
                    }
                } else {
                    StateTransitionResult::NoChange
                }
            }
            HotkeyMode::Toggle => match self.state {
                SpeechState::Idle => {
                    let from = self.state.clone();
                    let to = SpeechState::Listening;
                    self.state = to.clone();
                    self.press_start_time_ms = timestamp_ms;
                    self.target_window_id = active_window_id;
                    self.publish_state_changed(&to);

                    StateTransitionResult::Transitioned {
                        from,
                        to,
                        action: StateMachineAction::StartRecording {
                            target_window_id: active_window_id,
                        },
                    }
                }
                SpeechState::Listening => {
                    let from = self.state.clone();
                    let to = SpeechState::Transcribing;
                    self.state = to.clone();
                    self.publish_state_changed(&to);

                    StateTransitionResult::Transitioned {
                        from,
                        to,
                        action: StateMachineAction::StopRecordingAndTranscribe,
                    }
                }
                _ => StateTransitionResult::NoChange,
            },
            HotkeyMode::SmartDual => match self.state {
                SpeechState::Idle => {
                    let from = self.state.clone();
                    let to = SpeechState::Listening;
                    self.state = to.clone();
                    self.press_start_time_ms = timestamp_ms;
                    self.target_window_id = active_window_id;
                    self.publish_state_changed(&to);

                    StateTransitionResult::Transitioned {
                        from,
                        to,
                        action: StateMachineAction::StartRecording {
                            target_window_id: active_window_id,
                        },
                    }
                }
                SpeechState::Listening => {
                    let from = self.state.clone();
                    let to = SpeechState::Transcribing;
                    self.state = to.clone();
                    self.publish_state_changed(&to);

                    StateTransitionResult::Transitioned {
                        from,
                        to,
                        action: StateMachineAction::StopRecordingAndTranscribe,
                    }
                }
                _ => StateTransitionResult::NoChange,
            },
        }
    }

    /// Handles key-up event (`Ctrl+Spacebar` released).
    pub fn on_key_up(&mut self, timestamp_ms: u64) -> StateTransitionResult {
        match self.mode {
            HotkeyMode::PushToTalk => {
                if self.state == SpeechState::Listening {
                    let duration = timestamp_ms.saturating_sub(self.press_start_time_ms);
                    if duration < self.min_press_duration_ms {
                        // Accidental micro-press (<100ms): cancel and return to Idle
                        let from = self.state.clone();
                        let to = SpeechState::Idle;
                        self.state = to.clone();
                        self.target_window_id = None;
                        self.publish_state_changed(&to);

                        StateTransitionResult::Transitioned {
                            from,
                            to,
                            action: StateMachineAction::CancelRecordingDiscardBuffer,
                        }
                    } else {
                        // Valid speech utterance (>=100ms): transition to Transcribing
                        let from = self.state.clone();
                        let to = SpeechState::Transcribing;
                        self.state = to.clone();
                        self.publish_state_changed(&to);

                        StateTransitionResult::Transitioned {
                            from,
                            to,
                            action: StateMachineAction::StopRecordingAndTranscribe,
                        }
                    }
                } else {
                    StateTransitionResult::NoChange
                }
            }
            HotkeyMode::Toggle => StateTransitionResult::NoChange,
            HotkeyMode::SmartDual => {
                if self.state == SpeechState::Listening {
                    let duration = timestamp_ms.saturating_sub(self.press_start_time_ms);
                    if duration >= 250 {
                        // Push-to-Talk hold (>= 250ms): release to finish and transcribe
                        let from = self.state.clone();
                        let to = SpeechState::Transcribing;
                        self.state = to.clone();
                        self.publish_state_changed(&to);

                        StateTransitionResult::Transitioned {
                            from,
                            to,
                            action: StateMachineAction::StopRecordingAndTranscribe,
                        }
                    } else if duration < 30 {
                        // Accidental flutter (<30ms): discard and return to Idle
                        let from = self.state.clone();
                        let to = SpeechState::Idle;
                        self.state = to.clone();
                        self.target_window_id = None;
                        self.publish_state_changed(&to);

                        StateTransitionResult::Transitioned {
                            from,
                            to,
                            action: StateMachineAction::CancelRecordingDiscardBuffer,
                        }
                    } else {
                        // Quick tap (< 250ms): latched into toggle mode, stay listening!
                        StateTransitionResult::NoChange
                    }
                } else {
                    StateTransitionResult::NoChange
                }
            }
        }
    }

    /// Cancels active recording/transcription immediately and returns to Idle.
    pub fn cancel(&mut self) -> StateTransitionResult {
        if self.state != SpeechState::Idle {
            let from = self.state.clone();
            let to = SpeechState::Idle;
            self.state = to.clone();
            self.target_window_id = None;
            self.publish_state_changed(&to);

            StateTransitionResult::Transitioned {
                from,
                to,
                action: StateMachineAction::CancelRecordingDiscardBuffer,
            }
        } else {
            StateTransitionResult::NoChange
        }
    }

    /// Handles transcription completion from the STT engine.
    pub fn on_transcription_completed(
        &mut self,
        payload: TranscriptionPayload,
    ) -> StateTransitionResult {
        if self.state == SpeechState::Transcribing {
            if let Some(bus) = &self.event_bus {
                let _ = bus.publish(DomainEvent::SpeechRecognized(payload.clone()));
                let _ = bus.publish(DomainEvent::TranscriptionCompleted(payload));
            }

            let from = self.state.clone();
            let to = SpeechState::Injecting;
            self.state = to.clone();
            self.publish_state_changed(&to);

            StateTransitionResult::Transitioned {
                from,
                to,
                action: StateMachineAction::StartInjection,
            }
        } else {
            StateTransitionResult::NoChange
        }
    }

    /// Handles injection completion.
    pub fn on_injection_completed(&mut self) -> StateTransitionResult {
        if self.state == SpeechState::Injecting {
            let from = self.state.clone();
            let to = SpeechState::Idle;
            self.state = to.clone();
            self.target_window_id = None;
            self.publish_state_changed(&to);

            StateTransitionResult::Transitioned {
                from,
                to,
                action: StateMachineAction::ReturnToIdle,
            }
        } else {
            StateTransitionResult::NoChange
        }
    }

    /// Handles processing or device errors.
    pub fn on_error(&mut self, err_msg: &str) -> StateTransitionResult {
        let from = self.state.clone();
        let to = SpeechState::Error(err_msg.to_string());
        self.state = to.clone();
        self.publish_state_changed(&to);

        StateTransitionResult::Transitioned {
            from,
            to,
            action: StateMachineAction::ReportError(err_msg.to_string()),
        }
    }

    /// Resets the state machine unconditionally to Idle.
    pub fn reset(&mut self) -> StateTransitionResult {
        let from = self.state.clone();
        let to = SpeechState::Idle;
        self.state = to.clone();
        self.target_window_id = None;
        self.publish_state_changed(&to);

        StateTransitionResult::Transitioned {
            from,
            to,
            action: StateMachineAction::ReturnToIdle,
        }
    }

    fn publish_state_changed(&self, state: &SpeechState) {
        if let Some(bus) = &self.event_bus {
            let _ = bus.publish(DomainEvent::SpeechStateChanged(state.clone()));
            let legacy_state: SttState = state.clone().into();
            let _ = bus.publish(DomainEvent::SttStateChanged(legacy_state));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ptt_happy_path() {
        let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);
        assert_eq!(*sm.state(), SpeechState::Idle);

        // Key down at 1000ms
        let res = sm.on_key_down(1000, Some(101));
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Listening,
                ..
            }
        ));
        assert_eq!(*sm.state(), SpeechState::Listening);

        // Key up at 1500ms (500ms >= 100ms)
        let res = sm.on_key_up(1500);
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Transcribing,
                action: StateMachineAction::StopRecordingAndTranscribe,
                ..
            }
        ));
        assert_eq!(*sm.state(), SpeechState::Transcribing);

        // Transcription completed
        let payload = TranscriptionPayload {
            text: "Hello".to_string(),
            duration_ms: 500,
            confidence: 0.95,
            target_window_title: None,
            target_process_name: None,
        };
        let res = sm.on_transcription_completed(payload);
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Injecting,
                ..
            }
        ));

        // Injection completed
        let res = sm.on_injection_completed();
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Idle,
                ..
            }
        ));
    }

    #[test]
    fn test_ptt_accidental_micropress_cancelled() {
        let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

        // Key down at 1000ms
        sm.on_key_down(1000, Some(101));

        // Key up at 1050ms (50ms < 100ms)
        let res = sm.on_key_up(1050);
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Idle,
                action: StateMachineAction::CancelRecordingDiscardBuffer,
                ..
            }
        ));
        assert_eq!(*sm.state(), SpeechState::Idle);
    }

    #[test]
    fn test_ptt_boundary_press_duration() {
        let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

        // 99ms press -> cancelled
        sm.on_key_down(1000, None);
        let res = sm.on_key_up(1099);
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Idle,
                action: StateMachineAction::CancelRecordingDiscardBuffer,
                ..
            }
        ));

        // Exactly 100ms press -> accepted
        sm.on_key_down(2000, None);
        let res = sm.on_key_up(2100);
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Transcribing,
                action: StateMachineAction::StopRecordingAndTranscribe,
                ..
            }
        ));
    }

    #[test]
    fn test_toggle_mode_transitions() {
        let mut sm = HotkeyStateMachine::new(HotkeyMode::Toggle, None);

        // First press -> Listening
        let res = sm.on_key_down(1000, Some(42));
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Listening,
                ..
            }
        ));

        // Key up is a no-op in Toggle
        assert_eq!(sm.on_key_up(1500), StateTransitionResult::NoChange);

        // Second press -> Transcribing
        let res = sm.on_key_down(2000, Some(42));
        assert!(matches!(
            res,
            StateTransitionResult::Transitioned {
                to: SpeechState::Transcribing,
                ..
            }
        ));
    }
}
