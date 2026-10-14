use crate::types::input::{ClickAction, InputKey, ModifierKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnapDirection {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Maximize,
    Minimize,
    Restore,
    Fullscreen,
}

pub type SnapPosition = SnapDirection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DimensionDelta {
    Larger,
    Smaller,
    Wider,
    Narrower,
    Taller,
    Shorter,
}

pub fn default_delta_pixels() -> u32 {
    100
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum ResizeMode {
    Delta {
        dimension: DimensionDelta,
        #[serde(default = "default_delta_pixels")]
        delta_pixels: u32,
    },
    Percentage {
        percentage: u8,
    },
    Explicit {
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CycleDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UiTarget {
    NextField,
    PrevField,
    Enter,
    Escape,
    Tab,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target")]
pub enum CloseTarget {
    CurrentWindow,
    App { app_name: String },
}

pub fn default_auto_center() -> bool {
    true
}

pub fn default_app_target() -> String {
    "App".to_string()
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
    PageUp,
    PageDown,
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
    WindowFocus {
        window_id: u64,
        query: String,
    },
    WindowClose {
        window_id: u64,
    },
    WindowMinimize {
        window_id: u64,
    },
    WindowSnap {
        direction: SnapDirection,
    },
    LaunchApp {
        app_name: String,
        #[serde(default = "default_auto_center")]
        auto_center: bool,
    },
    CloseApp {
        #[serde(default)]
        app_name: String,
        #[serde(default = "default_app_target")]
        target: String,
    },
    CenterApp {
        app_name: String,
    },
    AgentDispatch(AgentPayload),
    ReaInspect {
        target_pid: u32,
        binary_path: Option<String>,
    },
    RadialDialToggle {},
    SystemInput(SystemInputAction),
    Unknown {
        query: String,
    },

    // Milestone M9 Hands-Free Additions
    SnapWindow {
        position: SnapPosition,
    },
    ResizeWindow {
        mode: ResizeMode,
    },
    SwitchFocus {
        app_name: String,
    },
    CycleWindow {
        direction: CycleDirection,
    },
    NavigateUi {
        element: UiTarget,
    },
    Scroll {
        direction: ScrollDirection,
        amount: u32,
    },
    MouseClick {
        button: ClickAction,
        count: u32,
    },
}
