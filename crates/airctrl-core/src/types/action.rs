use crate::types::input::{ClickAction, InputKey, ModifierKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapDirection {
    Left,
    Right,
    Top,
    Bottom,
    Maximize,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentTarget {
    Cursor,
    Claude,
    Antigravity,
    Grokbot,
    Dots,
    Muse,
    Viktor,
    Gemini,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPayload {
    pub target: AgentTarget,
    pub prompt: String,
    pub context_window: Option<String>,
    pub timestamp: String,
}

/// Vertical or horizontal scroll direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Target screen position for pointer click simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickPosition {
    Center,
    Coordinates { x: i32, y: i32 },
}

/// Low-level system desktop input action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "params")]
pub enum SystemInputAction {
    Scroll {
        direction: ScrollDirection,
        amount: u32,
    },
    Click {
        position: Option<ClickPosition>,
        button: ClickAction,
    },
    KeyCombination {
        modifiers: Vec<ModifierKey>,
        key: InputKey,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ActionCommand {
    WindowFocus { window_id: u64, query: String },
    WindowClose { window_id: u64 },
    WindowMinimize { window_id: u64 },
    WindowSnap { direction: SnapDirection },
    LaunchApp { app_name: String },
    CloseApp { app_name: String },
    CenterApp { app_name: String },
    AgentDispatch(AgentPayload),
    ReaInspect { target_pid: u32, binary_path: Option<String> },
    RadialDialToggle {},
    SystemInput(SystemInputAction),
    Unknown { query: String },
}
