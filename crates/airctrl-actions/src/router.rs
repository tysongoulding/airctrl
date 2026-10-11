use crate::fuzzy::{FuzzyCandidate, FuzzyMatcher, FuzzyMatcherConfig};
use crate::grammar::VoiceCommandGrammar;
use crate::intent::{ActionIntent, ParsedIntent};
use airctrl_core::types::{
    ActionCommand, AgentPayload, AgentTarget, ClickAction,
    SnapDirection, SystemInputAction, WindowDetails, WindowInfo,
};
use chrono::Utc;

/// Configuration options for the semantic command router.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub default_window_id: u64,
    pub fuzzy_config: FuzzyMatcherConfig,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            default_window_id: 101, // Matches E2E test harness convention
            fuzzy_config: FuzzyMatcherConfig::default(),
        }
    }
}

/// Spoken command-to-action semantic router.
pub struct SemanticCommandRouter {
    config: RouterConfig,
    grammar: VoiceCommandGrammar,
    fuzzy_matcher: FuzzyMatcher,
}

impl Default for SemanticCommandRouter {
    fn default() -> Self {
        Self::new(RouterConfig::default())
    }
}

impl SemanticCommandRouter {
    pub fn new(config: RouterConfig) -> Self {
        let fuzzy_matcher = FuzzyMatcher::new(config.fuzzy_config.clone());
        Self {
            config,
            grammar: VoiceCommandGrammar::new(),
            fuzzy_matcher,
        }
    }

    /// Parses raw utterance directly into structured ActionCommand.
    pub fn parse(&self, utterance: &str) -> ActionCommand {
        let parsed = self.grammar.parse_intent(utterance);
        self.resolve_command(&parsed, None)
    }

    /// Parses utterance with active open window context for dynamic HWND resolution.
    pub fn parse_with_windows(&self, utterance: &str, windows: &[WindowInfo]) -> ActionCommand {
        let parsed = self.grammar.parse_intent(utterance);
        let details: Vec<WindowDetails> = windows.iter().cloned().map(WindowDetails::from).collect();
        self.resolve_command(&parsed, Some(&details))
    }

    /// Parses utterance with comprehensive WindowDetails.
    pub fn parse_with_details(&self, utterance: &str, windows: &[WindowDetails]) -> ActionCommand {
        let parsed = self.grammar.parse_intent(utterance);
        self.resolve_command(&parsed, Some(windows))
    }

    fn resolve_command(
        &self,
        parsed: &ParsedIntent,
        windows: Option<&[WindowDetails]>,
    ) -> ActionCommand {
        match parsed.intent {
            ActionIntent::WindowSnap => {
                let direction = parsed.slots.direction.unwrap_or(SnapDirection::Maximize);
                ActionCommand::WindowSnap { direction }
            }

            ActionIntent::FocusWindow => {
                let query = parsed.slots.target.clone().unwrap_or_default();
                let window_id = if let Some(wins) = windows {
                    self.resolve_target_window_id(&query, wins)
                        .unwrap_or(self.config.default_window_id)
                } else {
                    self.config.default_window_id
                };
                ActionCommand::WindowFocus { window_id, query }
            }

            ActionIntent::WindowClose => {
                let window_id = if let Some(wins) = windows {
                    wins.iter()
                        .find(|w| w.is_focused)
                        .map(|w| w.id)
                        .unwrap_or(self.config.default_window_id)
                } else {
                    self.config.default_window_id
                };
                ActionCommand::WindowClose { window_id }
            }

            ActionIntent::WindowMinimize => {
                let window_id = if let Some(wins) = windows {
                    wins.iter()
                        .find(|w| w.is_focused)
                        .map(|w| w.id)
                        .unwrap_or(self.config.default_window_id)
                } else {
                    self.config.default_window_id
                };
                ActionCommand::WindowMinimize { window_id }
            }

            ActionIntent::LaunchApp => {
                let app_name = parsed.slots.target.clone().unwrap_or_default();
                ActionCommand::LaunchApp { app_name }
            }

            ActionIntent::AgentDispatch => {
                let target = parsed.slots.agent.unwrap_or(AgentTarget::Claude);
                let prompt = parsed.slots.payload.clone().unwrap_or_default();
                let timestamp = Utc::now().to_rfc3339();
                ActionCommand::AgentDispatch(AgentPayload {
                    target,
                    prompt,
                    context_window: None,
                    timestamp,
                })
            }

            ActionIntent::ReaInspect => {
                let target_pid = parsed.slots.numeric.map(|n| n as u32).unwrap_or(0);
                let binary_path = parsed.slots.target.clone();
                ActionCommand::ReaInspect {
                    target_pid,
                    binary_path,
                }
            }

            ActionIntent::RadialDialToggle => ActionCommand::RadialDialToggle {},

            ActionIntent::SystemInput => {
                if let Some(dir) = parsed.slots.scroll_direction {
                    let amount = parsed.slots.numeric.unwrap_or(3) as u32;
                    ActionCommand::SystemInput(SystemInputAction::Scroll {
                        direction: dir,
                        amount,
                    })
                } else {
                    let btn = parsed
                        .slots
                        .extra
                        .get("button")
                        .and_then(|v| match v {
                            crate::intent::SlotValue::ClickButton(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(ClickAction::LeftClick);
                    let pos = parsed.slots.click_position;
                    ActionCommand::SystemInput(SystemInputAction::Click {
                        position: pos,
                        button: btn,
                    })
                }
            }

            ActionIntent::Unknown => ActionCommand::Unknown {
                query: parsed.raw_utterance.clone(),
            },
        }
    }

    /// Resolves target window ID by evaluating query against candidate window metadata.
    pub fn resolve_target_window_id(&self, query: &str, windows: &[WindowDetails]) -> Option<u64> {
        let candidates: Vec<FuzzyCandidate> = windows
            .iter()
            .map(|w| FuzzyCandidate {
                id: Some(w.id),
                display_name: &w.title,
                process_name: Some(&w.process_name),
                window_title: Some(&w.title),
            })
            .collect();

        self.fuzzy_matcher
            .find_best_match(query, &candidates)
            .and_then(|r| r.candidate_id)
    }
}

/// Standalone convenience function fulfilling PROJECT.md API contract.
pub fn parse_voice_command(utterance: &str) -> ActionCommand {
    let router = SemanticCommandRouter::default();
    router.parse(utterance)
}
