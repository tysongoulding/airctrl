use crate::intent::{ActionIntent, ClickPosition, ExtractedSlots, ParsedIntent, ScrollDirection, SlotValue};
use airctrl_core::types::{
    AgentTarget, ClickAction, CycleDirection, DimensionDelta, ResizeMode, SnapDirection, UiTarget,
};
use regex::Regex;
use std::sync::LazyLock;

static SNAP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:snap|dock|tile)(?:\s+window)?\s+(left|right|top|bottom|maximize|center|minimize|restore|fullscreen)$").unwrap()
});

static MAXIMIZE_STANDALONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:dock\s+)?maximize(?:\s+(?:window|current|this))?$").unwrap()
});

static CENTER_STANDALONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:dock\s+)?center(?:\s+(?:window|current(?:\s+window)?|this(?:\s+window)?))?$").unwrap()
});

static RESTORE_STANDALONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:dock\s+)?restore(?:\s+(?:window|current(?:\s+window)?|this(?:\s+window)?))?$").unwrap()
});

static FULLSCREEN_STANDALONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:dock\s+)?fullscreen(?:\s+(?:window|current(?:\s+window)?|this(?:\s+window)?))?$").unwrap()
});

static RESIZE_EXPLICIT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:resize|size)(?:\s+window)?\s+(\d+)\s*(?:by|x)\s*(\d+)$").unwrap()
});

static RESIZE_PCT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:resize|scale)(?:\s+window)?\s+(\d+)\s*%$").unwrap()
});

static RESIZE_DELTA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:make\s+window\s+|resize(?:\s+window)?\s+)(larger|smaller|wider|narrower|taller|shorter)(?:\s+by\s+(\d+))?$").unwrap()
});

static CYCLE_NEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^next\s+window$").unwrap()
});

static CYCLE_PREV_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:previous|prev)\s+window$").unwrap()
});

static FOCUS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:switch\s+to|focus|bring\s+up|go\s+to)\s+(.+)$").unwrap()
});

static CLOSE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:close|quit|exit|kill)\s+(?:window|current(?:\s+window)?|this(?:\s+window)?)$").unwrap()
});

static CLOSE_APP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:close|quit|exit|kill)\s+(.+)$").unwrap()
});

static CENTER_APP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:center|centre)\s+(.+)$").unwrap()
});

static MINIMIZE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:minimize|hide)(?:\s+(?:window|current(?:\s+window)?|this(?:\s+window)?))?$").unwrap()
});

static LAUNCH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:launch|open|start|run)\s+(.+)$").unwrap()
});

static AGENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:ask\s+)?(cursor|claude|antigravity|grokbot|dots|muse|viktor|gemini)(?:[:,\s]+|\s+to\s+)(.+)$").unwrap()
});

static REA_INSPECT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:rea\s+inspect|inspect)\s+(\d+)(?:\s+(.+))?$").unwrap()
});

static REA_ANALYZE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:rea\s+analyze)\s+(.+)$").unwrap()
});

static RADIAL_DIAL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:toggle\s+dial|radial\s+dial|open\s+dial|close\s+dial|show\s+dial)$").unwrap()
});

static SCROLL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^scroll\s+(down|up|left|right)(?:\s+(\d+))?$").unwrap()
});

static PAGE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^page\s+(down|up)$").unwrap()
});

static CLICK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:click|tap)\s+(center|middle|left|right|double)$").unwrap()
});

static CLICK_INVERTED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(left|right|double|middle)\s+click$").unwrap()
});

static CONTEXT_CLICK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^context\s+click$").unwrap()
});

pub const WAKE_PREFIXES: &[&str] = &["hey airctrl", "airctrl", "computer"];

pub struct VoiceCommandGrammar;

impl Default for VoiceCommandGrammar {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceCommandGrammar {
    pub fn new() -> Self {
        Self
    }

    /// Strips leading wake word prefix (e.g. "hey airctrl", "airctrl", "computer")
    /// handling repeated prefixes and trailing whitespace or punctuation.
    pub fn strip_wake_prefix<'a>(&self, raw: &'a str) -> (&'a str, bool) {
        let mut current = raw.trim();
        let mut stripped_any = false;
        loop {
            let lower = current.to_lowercase();
            let mut matched = false;
            for prefix in WAKE_PREFIXES {
                if lower.starts_with(prefix) {
                    let remainder = current[prefix.len()..].trim_start();
                    current = remainder
                        .trim_start_matches(|c: char| c == ':' || c == ',' || c.is_whitespace())
                        .trim();
                    matched = true;
                    stripped_any = true;
                    break;
                }
            }
            if !matched {
                break;
            }
        }
        (current, stripped_any)
    }

    /// Normalizes and parses raw voice utterance into a ParsedIntent.
    pub fn parse_intent(&self, raw: &str) -> ParsedIntent {
        let trimmed_raw = raw.trim();
        let (stripped, was_wake) = self.strip_wake_prefix(trimmed_raw);

        if stripped.is_empty() {
            return ParsedIntent {
                intent: ActionIntent::Unknown,
                confidence: 0.0,
                slots: ExtractedSlots::default(),
                raw_utterance: if was_wake { String::new() } else { raw.to_string() },
                normalized_utterance: String::new(),
            };
        }

        if stripped.len() <= 1 {
            return ParsedIntent {
                intent: ActionIntent::Unknown,
                confidence: 0.0,
                slots: ExtractedSlots::default(),
                raw_utterance: if was_wake { stripped.to_string() } else { raw.to_string() },
                normalized_utterance: stripped.to_lowercase(),
            };
        }

        let trimmed = stripped;
        let normalized = trimmed.to_lowercase();

        // 1. Radial Dial Toggle
        if RADIAL_DIAL_REGEX.is_match(trimmed) || normalized == "dial" {
            return ParsedIntent {
                intent: ActionIntent::RadialDialToggle,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("toggle".to_string()),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2. Window Snap / Dock
        if let Some(caps) = SNAP_REGEX.captures(trimmed) {
            let dir_str = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let direction = match dir_str.as_str() {
                "left" => SnapDirection::Left,
                "right" => SnapDirection::Right,
                "top" => SnapDirection::Top,
                "bottom" => SnapDirection::Bottom,
                "maximize" => SnapDirection::Maximize,
                "center" => SnapDirection::Center,
                "minimize" => SnapDirection::Minimize,
                "restore" => SnapDirection::Restore,
                "fullscreen" => SnapDirection::Fullscreen,
                _ => SnapDirection::Maximize,
            };
            return ParsedIntent {
                intent: ActionIntent::WindowSnap,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("snap".to_string()),
                    direction: Some(direction),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2b. Standalone Maximize
        if MAXIMIZE_STANDALONE_REGEX.is_match(trimmed) {
            return ParsedIntent {
                intent: ActionIntent::WindowSnap,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("maximize".to_string()),
                    direction: Some(SnapDirection::Maximize),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2c. Standalone Center
        if CENTER_STANDALONE_REGEX.is_match(trimmed) || normalized == "center" || normalized == "centre" {
            return ParsedIntent {
                intent: ActionIntent::WindowSnap,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("center".to_string()),
                    direction: Some(SnapDirection::Center),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2d. Standalone Restore
        if RESTORE_STANDALONE_REGEX.is_match(trimmed) || normalized == "restore" || normalized == "restore window" {
            return ParsedIntent {
                intent: ActionIntent::WindowSnap,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("restore".to_string()),
                    direction: Some(SnapDirection::Restore),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2e. Standalone Fullscreen
        if FULLSCREEN_STANDALONE_REGEX.is_match(trimmed) || normalized == "fullscreen" || normalized == "fullscreen window" {
            return ParsedIntent {
                intent: ActionIntent::WindowSnap,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("fullscreen".to_string()),
                    direction: Some(SnapDirection::Fullscreen),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2f. Dynamic Window Resizing: Explicit
        if let Some(caps) = RESIZE_EXPLICIT_REGEX.captures(trimmed) {
            let w = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(800);
            let h = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(600);
            return ParsedIntent {
                intent: ActionIntent::ResizeWindow,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("resize".to_string()),
                    resize_mode: Some(ResizeMode::Explicit { width: w, height: h }),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2g. Dynamic Window Resizing: Percentage
        if let Some(caps) = RESIZE_PCT_REGEX.captures(trimmed) {
            let raw_pct = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(100);
            let pct = raw_pct.clamp(10, 100) as u8;
            return ParsedIntent {
                intent: ActionIntent::ResizeWindow,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("resize".to_string()),
                    resize_mode: Some(ResizeMode::Percentage { percentage: pct }),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2h. Dynamic Window Resizing: Delta
        if let Some(caps) = RESIZE_DELTA_REGEX.captures(trimmed) {
            let dim_str = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let delta_px = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(100);
            let dimension = match dim_str.as_str() {
                "larger" => DimensionDelta::Larger,
                "smaller" => DimensionDelta::Smaller,
                "wider" => DimensionDelta::Wider,
                "narrower" => DimensionDelta::Narrower,
                "taller" => DimensionDelta::Taller,
                "shorter" => DimensionDelta::Shorter,
                _ => DimensionDelta::Larger,
            };
            return ParsedIntent {
                intent: ActionIntent::ResizeWindow,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("resize".to_string()),
                    resize_mode: Some(ResizeMode::Delta { dimension, delta_pixels: delta_px }),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2i. Window Cycling
        if CYCLE_NEXT_REGEX.is_match(trimmed) || normalized == "next window" {
            return ParsedIntent {
                intent: ActionIntent::CycleWindow,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("cycle".to_string()),
                    cycle_direction: Some(CycleDirection::Next),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        if CYCLE_PREV_REGEX.is_match(trimmed) || normalized == "previous window" || normalized == "prev window" {
            return ParsedIntent {
                intent: ActionIntent::CycleWindow,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("cycle".to_string()),
                    cycle_direction: Some(CycleDirection::Previous),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2j. UI Navigation
        if normalized == "next field" || normalized == "next input" {
            return ParsedIntent {
                intent: ActionIntent::NavigateUi,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("navigate".to_string()),
                    ui_target: Some(UiTarget::NextField),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }
        if normalized == "previous field" || normalized == "prev field" {
            return ParsedIntent {
                intent: ActionIntent::NavigateUi,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("navigate".to_string()),
                    ui_target: Some(UiTarget::PrevField),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }
        if normalized == "press enter" || normalized == "enter" {
            return ParsedIntent {
                intent: ActionIntent::NavigateUi,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("navigate".to_string()),
                    ui_target: Some(UiTarget::Enter),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }
        if normalized == "press escape" || normalized == "escape" {
            return ParsedIntent {
                intent: ActionIntent::NavigateUi,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("navigate".to_string()),
                    ui_target: Some(UiTarget::Escape),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }
        if normalized == "press tab" || normalized == "tab" {
            return ParsedIntent {
                intent: ActionIntent::NavigateUi,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("navigate".to_string()),
                    ui_target: Some(UiTarget::Tab),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 2k. Context Click
        if CONTEXT_CLICK_REGEX.is_match(trimmed) || normalized == "context click" {
            let mut extra = std::collections::HashMap::new();
            extra.insert("button".to_string(), SlotValue::ClickButton(ClickAction::RightClick));
            return ParsedIntent {
                intent: ActionIntent::SystemInput,
                confidence: 0.95,
                slots: ExtractedSlots {
                    verb: Some("click".to_string()),
                    extra,
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 3. Window Close (generic current window)
        if CLOSE_REGEX.is_match(trimmed) || normalized == "close window" || normalized == "close current window" || normalized == "quit window" || normalized == "close current" || normalized == "close this" {
            return ParsedIntent {
                intent: ActionIntent::WindowClose,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("close".to_string()),
                    target: Some("current window".to_string()),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 3b. Center Application ([center] {application}, e.g. "center slack")
        if let Some(caps) = CENTER_APP_REGEX.captures(trimmed) {
            let app_name = caps.get(1).map_or("", |m| m.as_str()).trim();
            if !app_name.is_empty() && app_name != "window" && app_name != "current" && app_name != "this" {
                return ParsedIntent {
                    intent: ActionIntent::CenterApp,
                    confidence: 0.95,
                    slots: ExtractedSlots {
                        verb: Some("center".to_string()),
                        target: Some(app_name.to_lowercase()),
                        ..Default::default()
                    },
                    raw_utterance: raw.to_string(),
                    normalized_utterance: normalized,
                };
            }
        }

        // 3c. Close Application ([close] {application}, e.g. "close slack")
        if let Some(caps) = CLOSE_APP_REGEX.captures(trimmed) {
            let app_name = caps.get(1).map_or("", |m| m.as_str()).trim();
            if !app_name.is_empty() && app_name != "window" && app_name != "current" && app_name != "this" {
                return ParsedIntent {
                    intent: ActionIntent::CloseApp,
                    confidence: 0.95,
                    slots: ExtractedSlots {
                        verb: Some("close".to_string()),
                        target: Some(app_name.to_lowercase()),
                        ..Default::default()
                    },
                    raw_utterance: raw.to_string(),
                    normalized_utterance: normalized,
                };
            }
        }

        // 4. Window Minimize
        if MINIMIZE_REGEX.is_match(trimmed) || normalized == "minimize window" || normalized == "minimize current" {
            return ParsedIntent {
                intent: ActionIntent::WindowMinimize,
                confidence: 1.0,
                slots: ExtractedSlots {
                    verb: Some("minimize".to_string()),
                    target: Some("current window".to_string()),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 5. Agent Dispatch (Cursor, Claude, Antigravity, Grokbot, Dots, Muse, Viktor, Gemini)
        if let Some(caps) = AGENT_REGEX.captures(trimmed) {
            let target_str = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let raw_prompt = caps.get(2).map_or("", |m| m.as_str());
            // Clean up leading colons/spaces (e.g. ": : do task" -> "do task")
            let prompt = raw_prompt
                .trim_start_matches(|c: char| c == ':' || c.is_whitespace())
                .trim();

            let target = match target_str.as_str() {
                "cursor" => Some(AgentTarget::Cursor),
                "claude" => Some(AgentTarget::Claude),
                "antigravity" => Some(AgentTarget::Antigravity),
                "grokbot" => Some(AgentTarget::Grokbot),
                "dots" => Some(AgentTarget::Dots),
                "muse" => Some(AgentTarget::Muse),
                "viktor" => Some(AgentTarget::Viktor),
                "gemini" => Some(AgentTarget::Gemini),
                _ => None,
            };

            if let Some(agent) = target {
                return ParsedIntent {
                    intent: ActionIntent::AgentDispatch,
                    confidence: 0.98,
                    slots: ExtractedSlots {
                        verb: Some("ask".to_string()),
                        agent: Some(agent),
                        payload: Some(prompt.to_string()),
                        ..Default::default()
                    },
                    raw_utterance: raw.to_string(),
                    normalized_utterance: normalized,
                };
            }
        }

        // 6. REA Inspection (PID or analyze)
        if let Some(caps) = REA_INSPECT_REGEX.captures(trimmed) {
            let pid = caps.get(1).and_then(|m| m.as_str().parse::<i64>().ok()).unwrap_or(0);
            let bin_path = caps.get(2).map(|m| m.as_str().trim().to_string());
            return ParsedIntent {
                intent: ActionIntent::ReaInspect,
                confidence: 0.95,
                slots: ExtractedSlots {
                    verb: Some("inspect".to_string()),
                    numeric: Some(pid),
                    target: bin_path,
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        if let Some(caps) = REA_ANALYZE_REGEX.captures(trimmed) {
            let target_app = caps.get(1).map(|m| m.as_str().trim().to_string());
            return ParsedIntent {
                intent: ActionIntent::ReaInspect,
                confidence: 0.90,
                slots: ExtractedSlots {
                    verb: Some("analyze".to_string()),
                    target: target_app,
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 7. System Input: Scroll
        if let Some(caps) = SCROLL_REGEX.captures(trimmed) {
            let dir_str = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let amount = caps.get(2).and_then(|m| m.as_str().parse::<i64>().ok()).unwrap_or(3);
            let dir = match dir_str.as_str() {
                "up" => ScrollDirection::Up,
                "down" => ScrollDirection::Down,
                "left" => ScrollDirection::Left,
                "right" => ScrollDirection::Right,
                _ => ScrollDirection::Down,
            };
            return ParsedIntent {
                intent: ActionIntent::SystemInput,
                confidence: 0.95,
                slots: ExtractedSlots {
                    verb: Some("scroll".to_string()),
                    scroll_direction: Some(dir),
                    numeric: Some(amount),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        if let Some(caps) = PAGE_REGEX.captures(trimmed) {
            let dir_str = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let dir = if dir_str == "up" { ScrollDirection::Up } else { ScrollDirection::Down };
            return ParsedIntent {
                intent: ActionIntent::SystemInput,
                confidence: 0.95,
                slots: ExtractedSlots {
                    verb: Some("page".to_string()),
                    scroll_direction: Some(dir),
                    numeric: Some(10),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 8. System Input: Click
        if let Some(caps) = CLICK_REGEX.captures(trimmed) {
            let arg = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let (pos, btn) = match arg.as_str() {
                "center" => (Some(ClickPosition::Center), ClickAction::LeftClick),
                "right" => (None, ClickAction::RightClick),
                "middle" => (None, ClickAction::MiddleClick),
                "double" => (None, ClickAction::DoubleClick),
                _ => (None, ClickAction::LeftClick),
            };
            let mut extra = std::collections::HashMap::new();
            extra.insert("button".to_string(), SlotValue::ClickButton(btn));
            return ParsedIntent {
                intent: ActionIntent::SystemInput,
                confidence: 0.95,
                slots: ExtractedSlots {
                    verb: Some("click".to_string()),
                    click_position: pos,
                    extra,
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        if let Some(caps) = CLICK_INVERTED_REGEX.captures(trimmed) {
            let btn_str = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            let btn = match btn_str.as_str() {
                "right" => ClickAction::RightClick,
                "double" => ClickAction::DoubleClick,
                "middle" => ClickAction::MiddleClick,
                _ => ClickAction::LeftClick,
            };
            let mut extra = std::collections::HashMap::new();
            extra.insert("button".to_string(), SlotValue::ClickButton(btn));
            return ParsedIntent {
                intent: ActionIntent::SystemInput,
                confidence: 0.95,
                slots: ExtractedSlots {
                    verb: Some("click".to_string()),
                    extra,
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 9. Window Focus / Switch To
        if let Some(caps) = FOCUS_REGEX.captures(trimmed) {
            let target = caps.get(1).map_or("", |m| m.as_str()).trim();
            let clean_target = target.trim_end_matches(['.', ',', '!', '?']).trim();
            if !clean_target.is_empty() {
                return ParsedIntent {
                    intent: ActionIntent::FocusWindow,
                    confidence: 0.90,
                    slots: ExtractedSlots {
                        verb: Some("switch to".to_string()),
                        target: Some(clean_target.to_lowercase()),
                        ..Default::default()
                    },
                    raw_utterance: raw.to_string(),
                    normalized_utterance: normalized,
                };
            }
        }

        // 10. Launch App
        if let Some(caps) = LAUNCH_REGEX.captures(trimmed) {
            let app_name = caps.get(1).map_or("", |m| m.as_str()).trim();
            let clean_app = app_name.trim_end_matches(['.', ',', '!', '?']).trim();
            if !clean_app.is_empty() {
                return ParsedIntent {
                    intent: ActionIntent::LaunchApp,
                    confidence: 0.90,
                    slots: ExtractedSlots {
                        verb: Some("launch".to_string()),
                        target: Some(clean_app.to_lowercase()),
                        auto_center: Some(true),
                        ..Default::default()
                    },
                    raw_utterance: raw.to_string(),
                    normalized_utterance: normalized,
                };
            }
        }

        // Fallback: Unknown
        ParsedIntent {
            intent: ActionIntent::Unknown,
            confidence: 0.0,
            slots: ExtractedSlots::default(),
            raw_utterance: raw.to_string(),
            normalized_utterance: normalized,
        }
    }
}
