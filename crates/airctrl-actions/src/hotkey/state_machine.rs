use super::events::*;
use super::radial_dial::RadialDialOverlay;
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::{ActionCommand, HotkeyMode};

/// Dedicated Speech-to-Action (`Ctrl+Alt`) Hotkey State Machine.
/// Coordinates center focus, radial dial overlay, speech capture, intent parsing, and execution.
pub struct ActionHotkeyStateMachine {
    mode: HotkeyMode,
    state: ActionState,
    dial: RadialDialOverlay,
    press_start_time_ms: u64,
    min_press_duration_ms: u64,
    target_window_id: Option<u64>,
    event_bus: Option<EventBus>,
}

impl ActionHotkeyStateMachine {
    pub const DEFAULT_MIN_DURATION_MS: u64 = 100;

    pub fn new(mode: HotkeyMode, event_bus: Option<EventBus>) -> Self {
        Self {
            mode,
            state: ActionState::Idle,
            dial: RadialDialOverlay::new(),
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

    pub fn state(&self) -> &ActionState {
        &self.state
    }

    pub fn dial(&self) -> &RadialDialOverlay {
        &self.dial
    }

    pub fn dial_mut(&mut self) -> &mut RadialDialOverlay {
        &mut self.dial
    }

    pub fn target_window_id(&self) -> Option<u64> {
        self.target_window_id
    }

    pub fn set_min_press_duration_ms(&mut self, duration_ms: u64) {
        self.min_press_duration_ms = duration_ms;
    }

    /// Handles key-down event (`Ctrl+Alt` pressed).
    pub fn on_key_down(
        &mut self,
        timestamp_ms: u64,
        active_window_id: Option<u64>,
    ) -> ActionStateTransitionResult {
        match self.mode {
            HotkeyMode::PushToTalk => {
                if self.state == ActionState::Idle {
                    let from = self.state.clone();
                    let to = ActionState::ActionListening;
                    self.state = to.clone();
                    self.press_start_time_ms = timestamp_ms;
                    self.target_window_id = active_window_id;
                    self.dial.show();

                    ActionStateTransitionResult::Transitioned {
                        from,
                        to,
                        instruction: ActionMachineInstruction::StartListening {
                            target_window_id: active_window_id,
                        },
                    }
                } else {
                    ActionStateTransitionResult::NoChange
                }
            }
            HotkeyMode::Toggle => match self.state {
                ActionState::Idle => {
                    let from = self.state.clone();
                    let to = ActionState::ActionListening;
                    self.state = to.clone();
                    self.press_start_time_ms = timestamp_ms;
                    self.target_window_id = active_window_id;
                    self.dial.show();

                    ActionStateTransitionResult::Transitioned {
                        from,
                        to,
                        instruction: ActionMachineInstruction::StartListening {
                            target_window_id: active_window_id,
                        },
                    }
                }
                ActionState::ActionListening => {
                    let from = self.state.clone();
                    let to = ActionState::ActionParsing;
                    self.state = to.clone();

                    ActionStateTransitionResult::Transitioned {
                        from,
                        to,
                        instruction: ActionMachineInstruction::StopListeningAndParse,
                    }
                }
                _ => ActionStateTransitionResult::NoChange,
            },
            HotkeyMode::SmartDual => match self.state {
                ActionState::Idle => {
                    let from = self.state.clone();
                    let to = ActionState::ActionListening;
                    self.state = to.clone();
                    self.press_start_time_ms = timestamp_ms;
                    self.target_window_id = active_window_id;
                    self.dial.show();

                    ActionStateTransitionResult::Transitioned {
                        from,
                        to,
                        instruction: ActionMachineInstruction::StartListening {
                            target_window_id: active_window_id,
                        },
                    }
                }
                ActionState::ActionListening => {
                    let from = self.state.clone();
                    let to = ActionState::ActionParsing;
                    self.state = to.clone();

                    ActionStateTransitionResult::Transitioned {
                        from,
                        to,
                        instruction: ActionMachineInstruction::StopListeningAndParse,
                    }
                }
                _ => ActionStateTransitionResult::NoChange,
            },
        }
    }

    /// Handles key-up event (`Ctrl+Alt` released).
    pub fn on_key_up(&mut self, timestamp_ms: u64) -> ActionStateTransitionResult {
        match self.mode {
            HotkeyMode::PushToTalk => {
                if self.state == ActionState::ActionListening {
                    let duration = timestamp_ms.saturating_sub(self.press_start_time_ms);
                    if duration < self.min_press_duration_ms {
                        // Accidental micro-press: discard and return to Idle
                        let from = self.state.clone();
                        let to = ActionState::Idle;
                        self.state = to.clone();
                        self.target_window_id = None;
                        self.dial.hide();

                        ActionStateTransitionResult::Transitioned {
                            from,
                            to,
                            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
                        }
                    } else {
                        // Utterance complete: transition to ActionParsing
                        let from = self.state.clone();
                        let to = ActionState::ActionParsing;
                        self.state = to.clone();

                        ActionStateTransitionResult::Transitioned {
                            from,
                            to,
                            instruction: ActionMachineInstruction::StopListeningAndParse,
                        }
                    }
                } else {
                    ActionStateTransitionResult::NoChange
                }
            }
            HotkeyMode::Toggle => ActionStateTransitionResult::NoChange,
            HotkeyMode::SmartDual => {
                if self.state == ActionState::ActionListening {
                    let duration = timestamp_ms.saturating_sub(self.press_start_time_ms);
                    if duration >= 250 {
                        let from = self.state.clone();
                        let to = ActionState::ActionParsing;
                        self.state = to.clone();

                        ActionStateTransitionResult::Transitioned {
                            from,
                            to,
                            instruction: ActionMachineInstruction::StopListeningAndParse,
                        }
                    } else if duration < 30 {
                        let from = self.state.clone();
                        let to = ActionState::Idle;
                        self.state = to.clone();
                        self.target_window_id = None;
                        self.dial.hide();

                        ActionStateTransitionResult::Transitioned {
                            from,
                            to,
                            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
                        }
                    } else {
                        ActionStateTransitionResult::NoChange
                    }
                } else {
                    ActionStateTransitionResult::NoChange
                }
            }
        }
    }

    /// Handles completion of speech parsing into an ActionCommand.
    pub fn on_command_parsed(&mut self, command: ActionCommand) -> ActionStateTransitionResult {
        if self.state == ActionState::ActionParsing {
            if let Some(bus) = &self.event_bus {
                let _ = bus.publish(DomainEvent::ActionTriggered(command.clone()));
            }

            let from = self.state.clone();
            let to = ActionState::ActionExecuting;
            self.state = to.clone();

            ActionStateTransitionResult::Transitioned {
                from,
                to,
                instruction: ActionMachineInstruction::ExecuteCommand(command),
            }
        } else {
            ActionStateTransitionResult::NoChange
        }
    }

    /// Handles completion of action execution, returning to Idle.
    pub fn on_execution_completed(&mut self) -> ActionStateTransitionResult {
        if self.state == ActionState::ActionExecuting {
            let from = self.state.clone();
            let to = ActionState::Idle;
            self.state = to.clone();
            self.target_window_id = None;
            self.dial.hide();

            ActionStateTransitionResult::Transitioned {
                from,
                to,
                instruction: ActionMachineInstruction::ReturnToIdle,
            }
        } else {
            ActionStateTransitionResult::NoChange
        }
    }

    /// Handles error condition.
    pub fn on_error(&mut self, err_msg: &str) -> ActionStateTransitionResult {
        let from = self.state.clone();
        let to = ActionState::ActionError(err_msg.to_string());
        self.state = to.clone();
        self.dial.hide();

        ActionStateTransitionResult::Transitioned {
            from,
            to,
            instruction: ActionMachineInstruction::ReportError(err_msg.to_string()),
        }
    }

    /// Inactivity timeout dismisses overlay and returns to Idle if listening.
    pub fn on_inactivity_timeout(&mut self) -> ActionStateTransitionResult {
        if self.state == ActionState::ActionListening {
            self.dial.on_inactivity_timeout();
            let from = self.state.clone();
            let to = ActionState::Idle;
            self.state = to.clone();
            self.target_window_id = None;

            ActionStateTransitionResult::Transitioned {
                from,
                to,
                instruction: ActionMachineInstruction::ReturnToIdle,
            }
        } else {
            ActionStateTransitionResult::NoChange
        }
    }

    /// Resets state unconditionally to Idle.
    pub fn reset(&mut self) -> ActionStateTransitionResult {
        let from = self.state.clone();
        let to = ActionState::Idle;
        self.state = to.clone();
        self.target_window_id = None;
        self.dial.hide();

        ActionStateTransitionResult::Transitioned {
            from,
            to,
            instruction: ActionMachineInstruction::ReturnToIdle,
        }
    }
}
