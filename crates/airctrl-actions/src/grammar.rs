use crate::intent::{ActionIntent, ClickPosition, ExtractedSlots, ParsedIntent, ScrollDirection, SlotValue};
use airctrl_core::types::{AgentTarget, ClickAction, SnapDirection};
use regex::Regex;
use std::sync::LazyLock;

static SNAP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:snap|dock|tile)(?:\s+window)?\s+(left|right|top|bottom|maximize)$").unwrap()
});

static MAXIMIZE_STANDALONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:dock\s+)?maximize(?:\s+(?:window|current|this))?$").unwrap()
});

static FOCUS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:switch\s+to|focus|bring\s+up|go\s+to)\s+(.+)$").unwrap()
});

static CLOSE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:close|quit|exit|kill)\s+(?:window|current(?:\s+window)?|this(?:\s+window)?)$").unwrap()
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

    /// Normalizes and parses raw voice utterance into a ParsedIntent.
    pub fn parse_intent(&self, raw: &str) -> ParsedIntent {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.len() <= 1 {
            return ParsedIntent {
                intent: ActionIntent::Unknown,
                confidence: 0.0,
                slots: ExtractedSlots::default(),
                raw_utterance: raw.to_string(),
                normalized_utterance: trimmed.to_lowercase(),
            };
        }

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

        // 3. Window Close
        if CLOSE_REGEX.is_match(trimmed) || normalized == "close window" || normalized == "close current" {
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
            return ParsedIntent {
                intent: ActionIntent::FocusWindow,
                confidence: 0.90,
                slots: ExtractedSlots {
                    verb: Some("switch to".to_string()),
                    target: Some(target.to_lowercase()),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
        }

        // 10. Launch App
        if let Some(caps) = LAUNCH_REGEX.captures(trimmed) {
            let app_name = caps.get(1).map_or("", |m| m.as_str()).trim();
            return ParsedIntent {
                intent: ActionIntent::LaunchApp,
                confidence: 0.90,
                slots: ExtractedSlots {
                    verb: Some("launch".to_string()),
                    target: Some(app_name.to_lowercase()),
                    ..Default::default()
                },
                raw_utterance: raw.to_string(),
                normalized_utterance: normalized,
            };
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
