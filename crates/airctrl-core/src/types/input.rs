use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierKey {
    Control,
    Alt,
    Shift,
    Super, // Windows key or Command key
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputKey {
    Char(char),
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    FKey(u8),
    Raw(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickAction {
    LeftClick,
    RightClick,
    MiddleClick,
    DoubleClick,
    LeftDown,
    LeftUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PointerPosition {
    pub x: i32,
    pub y: i32,
}

/// Recorded input operation used by deterministic test mock harnesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedInputAction {
    TypedText(String),
    PastedText(String),
    KeyCombination {
        modifiers: Vec<ModifierKey>,
        key: InputKey,
    },
    PointerMoved { x: i32, y: i32 },
    PointerClicked(ClickAction),
}
