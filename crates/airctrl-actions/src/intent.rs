pub use airctrl_core::types::{ClickPosition, ScrollDirection, SystemInputAction};
use airctrl_core::types::{
    AgentTarget, ClickAction, CycleDirection, ResizeMode, SnapDirection, SnapPosition, UiTarget,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// High-level semantic classification of user spoken utterance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionIntent {
    LaunchApp,
    CloseApp,
    CenterApp,
    FocusWindow,
    SwitchFocus,
    WindowSnap,
    ResizeWindow,
    CycleWindow,
    NavigateUi,
    WindowMinimize,
    WindowClose,
    SystemInput,
    AgentDispatch,
    ReaInspect,
    RadialDialToggle,
    Unknown,
}

/// Strongly typed semantic slot value extracted from voice command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SlotValue {
    Text(String),
    Integer(i64),
    Direction(SnapDirection),
    Agent(AgentTarget),
    ScrollDirection(ScrollDirection),
    ClickPosition(ClickPosition),
    ClickButton(ClickAction),
    Boolean(bool),
    ResizeMode(ResizeMode),
    CycleDirection(CycleDirection),
    UiTarget(UiTarget),
}

impl SlotValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_direction(&self) -> Option<SnapDirection> {
        match self {
            Self::Direction(d) => Some(*d),
            _ => None,
        }
    }

    pub fn as_agent(&self) -> Option<AgentTarget> {
        match self {
            Self::Agent(a) => Some(*a),
            _ => None,
        }
    }

    pub fn as_scroll_direction(&self) -> Option<ScrollDirection> {
        match self {
            Self::ScrollDirection(s) => Some(*s),
            _ => None,
        }
    }

    pub fn as_click_position(&self) -> Option<ClickPosition> {
        match self {
            Self::ClickPosition(p) => Some(*p),
            _ => None,
        }
    }

    pub fn as_click_button(&self) -> Option<ClickAction> {
        match self {
            Self::ClickButton(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Structured collection of semantic slots extracted from an utterance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedSlots {
    pub verb: Option<String>,
    pub target: Option<String>,
    pub direction: Option<SnapPosition>,
    pub agent: Option<AgentTarget>,
    pub payload: Option<String>,
    pub numeric: Option<i64>,
    pub scroll_direction: Option<ScrollDirection>,
    pub click_position: Option<ClickPosition>,
    pub resize_mode: Option<ResizeMode>,
    pub cycle_direction: Option<CycleDirection>,
    pub ui_target: Option<UiTarget>,
    pub auto_center: Option<bool>,
    pub extra: HashMap<String, SlotValue>,
}

/// Fully parsed intent with confidence score, slots, and source utterance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub intent: ActionIntent,
    pub confidence: f32,
    pub slots: ExtractedSlots,
    pub raw_utterance: String,
    pub normalized_utterance: String,
}
