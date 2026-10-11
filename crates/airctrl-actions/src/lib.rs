//! airctrl-actions: Speech-to-action semantic router, radial dial overlay,
//! application crawler, and agentic dispatch harness for airctrl.

pub mod agent;
pub mod crawler;
pub mod error;
pub mod fuzzy;
pub mod grammar;
pub mod hotkey;
pub mod intent;
pub mod rea;
pub mod router;

pub use airctrl_core as core;

// Core re-exports
pub use error::{ActionError, Result};
pub use fuzzy::{AliasRegistry, FuzzyCandidate, FuzzyMatchResult, FuzzyMatcher, FuzzyMatcherConfig};
pub use grammar::VoiceCommandGrammar;
pub use intent::{
    ActionIntent, ClickPosition, ExtractedSlots, ParsedIntent, ScrollDirection, SlotValue,
    SystemInputAction,
};
pub use router::{parse_voice_command, RouterConfig, SemanticCommandRouter};

// Crawler re-exports
pub use crawler::{
    crawl_running_applications, CrawlerConfig, WindowContextMap, WindowMatcher, WindowProcessCrawler,
};

// Hotkey and radial dial re-exports
pub use hotkey::{
    ActionHotkeyStateMachine, ActionMachineInstruction, ActionState, ActionStateTransitionResult,
    ActionResult, RadialDialOverlay, RadialDialState,
};

// Agent harness re-exports
pub use agent::{
    dispatch_agentic_instruction, AgentDispatcher, AgentEnvironmentDetector, AgentEnvironmentStatus,
    AgenticHarness, DispatchPayloadDetails, DispatchResult, FormattedCliPayload,
};

// REA inspection re-exports
pub use rea::{
    execute_rea_inspection, execute_rea_inspection_by_name, ReaActionSchema, ReaInspectionSchema,
    ReaInspector, RecursiveReaTaskSpec,
};

// High-level types from core for convenience
pub use airctrl_core::types::{
    ActionCommand, AgentPayload, AgentTarget, ClickAction, MatchType, ProcessInfo, SnapDirection,
    WindowDetails, WindowInfo, WindowMatch, WindowRect,
};
