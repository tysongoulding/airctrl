use airctrl_core::types::ActionCommand;
use serde::{Deserialize, Serialize};

/// Discrete lifecycle states for the Speech-to-Action (`Ctrl+Alt`) state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionState {
    Idle,
    ActionListening,
    ActionParsing,
    ActionExecuting,
    ActionError(String),
}

/// Commands emitted by the state machine to drive underlying runtime components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionMachineInstruction {
    None,
    StartListening { target_window_id: Option<u64> },
    CancelListeningDiscardBuffer,
    StopListeningAndParse,
    ExecuteCommand(ActionCommand),
    ReturnToIdle,
    ReportError(String),
}

/// Result of evaluating a key or lifecycle event in the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionStateTransitionResult {
    NoChange,
    Transitioned {
        from: ActionState,
        to: ActionState,
        instruction: ActionMachineInstruction,
    },
    IgnoredAccidentalTap {
        duration_ms: u64,
    },
}

/// Outcome of executing an ActionCommand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub description: String,
    pub execution_time_ms: u64,
}
