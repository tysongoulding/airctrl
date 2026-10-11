use airctrl_actions::agent::dispatcher::AgentDispatcher;
use airctrl_actions::crawler::context_map::{CrawlerConfig, WindowContextMap};
use airctrl_actions::crawler::matching::WindowMatcher;
use airctrl_actions::error::ActionError;
use airctrl_actions::fuzzy::{FuzzyCandidate, FuzzyMatcher};
use airctrl_actions::rea::inspector::ReaInspector;
use airctrl_core::platform::create_mock_platform;
use airctrl_core::types::*;
use std::time::Duration;
use sysinfo::System;

// =========================================================================
// 1. Fuzzy Matcher Scoring, Thresholds, & Multi-Metric Ranking
// =========================================================================

#[test]
fn test_adversarial_fuzzy_matcher_scoring_and_acronym_boosts() {
    let matcher = FuzzyMatcher::default();

    let candidates = vec![
        FuzzyCandidate {
            id: Some(101),
            display_name: "Visual Studio Code",
            process_name: Some("Code.exe"),
            window_title: Some("airctrl - Visual Studio Code"),
        },
        FuzzyCandidate {
            id: Some(102),
            display_name: "Google Chrome",
            process_name: Some("chrome.exe"),
            window_title: Some("New Tab - Google Chrome"),
        },
        FuzzyCandidate {
            id: Some(103),
            display_name: "Windows Terminal",
            process_name: Some("wt.exe"),
            window_title: Some("PowerShell"),
        },
        FuzzyCandidate {
            id: Some(104),
            display_name: "Spotify",
            process_name: Some("spotify.exe"),
            window_title: Some("Spotify Free"),
        },
    ];

    // 1. Acronym match: "vsc" -> Visual Studio Code (acronym boost >= 0.95)
    let vsc_match = matcher.find_best_match("vsc", &candidates).expect("Expected match for vsc");
    assert_eq!(vsc_match.matched_name, "Visual Studio Code");
    assert!(vsc_match.score >= 0.95, "Score {} should be >= 0.95", vsc_match.score);

    // 2. Acronym match: "gc" -> Google Chrome (acronym boost >= 0.95)
    let gc_match = matcher.find_best_match("gc", &candidates).expect("Expected match for gc");
    assert_eq!(gc_match.matched_name, "Google Chrome");
    assert!(gc_match.score >= 0.95, "Score {} should be >= 0.95", gc_match.score);

    // 3. Known colloquial alias: "term" -> Windows Terminal
    let term_match = matcher.find_best_match("term", &candidates).expect("Expected match for term");
    assert_eq!(term_match.matched_name, "Windows Terminal");
    assert!(term_match.score >= 0.95);

    // 4. Typo tolerance: "spotfy" -> Spotify
    let spot_match = matcher.find_best_match("spotfy", &candidates).expect("Expected match for spotfy");
    assert_eq!(spot_match.matched_name, "Spotify");
    assert!(spot_match.score >= 0.90);

    // 5. Unrelated gibberish query falls below similarity threshold (0.65) -> returns None
    let none_match = matcher.find_best_match("zxqkj999", &candidates);
    assert!(none_match.is_none(), "Unrelated string should not match any candidate");

    // 6. Empty query returns None
    assert!(matcher.find_best_match("", &candidates).is_none());
    assert!(matcher.find_best_match("   ", &candidates).is_none());

    // 7. Empty candidates returns None
    assert!(matcher.find_best_match("chrome", &[]).is_none());
}

// =========================================================================
// 2. Context Cache TTL & Debounce Invalidation Invariants
// =========================================================================

#[test]
fn test_adversarial_context_cache_ttl_and_debounce() {
    let config = CrawlerConfig {
        ttl: Duration::from_millis(50),
        debounce_interval: Duration::from_millis(20),
        fuzzy_threshold: 0.65,
    };

    let mut context_map = WindowContextMap::new(config);

    // Invariant 1: Fresh instance is dirty
    assert!(!context_map.is_fresh(), "Initial context map must not be fresh");

    let (provider, controllers) = create_mock_platform();
    controllers.window.add_window(WindowInfo {
        id: 501,
        process_id: 1234,
        process_name: "test.exe".to_string(),
        title: "Test Application".to_string(),
        bounds: WindowRect::default(),
        is_focused: true,
        is_minimized: false,
    });

    let mut sys = System::new();
    context_map.refresh(&*provider.window, &mut sys).unwrap();

    // Invariant 2: Immediately after refresh, map is fresh and within debounce interval
    assert!(context_map.is_fresh());
    assert!(context_map.should_debounce());
    assert_eq!(context_map.windows().len(), 1);
    assert_eq!(context_map.get_window(501).unwrap().id, 501);

    // Invariant 3: Explicit invalidation immediately clears freshness
    context_map.invalidate();
    assert!(!context_map.is_fresh(), "Invalidated context map must report !is_fresh()");

    // Refresh again and let TTL expire
    context_map.refresh(&*provider.window, &mut sys).unwrap();
    assert!(context_map.is_fresh());

    std::thread::sleep(Duration::from_millis(60)); // Sleep past 50ms TTL
    assert!(!context_map.is_fresh(), "Map must expire after TTL duration");
    assert!(!context_map.should_debounce(), "Map must not debounce after interval");
}

// =========================================================================
// 3. Window Matcher Multi-Attribute Heuristics & Contextual Boosts
// =========================================================================

#[test]
fn test_adversarial_window_matcher_heuristics_and_priority_boosts() {
    let matcher = WindowMatcher::new(0.65);

    let candidates = vec![
        WindowDetails {
            id: 1,
            process_id: 10,
            process_name: "code.exe".to_string(),
            title: "airctrl - main.rs - Visual Studio Code".to_string(),
            class_name: "Code".to_string(),
            bounds: WindowRect::default(),
            is_focused: true, // +0.05 focus boost
            is_minimized: false, // +0.04 visible boost
            is_maximized: false,
        },
        WindowDetails {
            id: 2,
            process_id: 20,
            process_name: "code.exe".to_string(),
            title: "notes.txt - Visual Studio Code".to_string(),
            class_name: "Code".to_string(),
            bounds: WindowRect::default(),
            is_focused: false,
            is_minimized: true, // minimized -> loses 0.04 boost
            is_maximized: false,
        },
        WindowDetails {
            id: 3,
            process_id: 30,
            process_name: "chrome.exe".to_string(),
            title: "GitHub - airctrl repository".to_string(),
            class_name: "Chrome_WidgetWin_1".to_string(),
            bounds: WindowRect::default(),
            is_focused: false,
            is_minimized: false,
            is_maximized: true,
        },
    ];

    // 1. Exact process match for "code"
    let matches = matcher.match_windows("code", &candidates);
    assert!(!matches.is_empty());
    // Focused window (id: 1) must rank higher than minimized window (id: 2)
    assert_eq!(matches[0].window.id, 1);
    assert_eq!(matches[0].match_type, MatchType::ExactProcess);
    assert_eq!(matches[0].score, 1.0); // Exact process 1.0 capped

    // 2. Token overlap: "github repository" matching title "GitHub - airctrl repository"
    let token_matches = matcher.match_windows("github repository", &candidates);
    assert!(!token_matches.is_empty());
    assert_eq!(token_matches[0].window.id, 3);
    assert_eq!(token_matches[0].match_type, MatchType::TokenOverlap);

    // 3. Empty query returns empty Vec
    assert!(matcher.match_windows("", &candidates).is_empty());
    assert!(matcher.match_windows("   ", &candidates).is_empty());

    // 4. Strict descending score order invariant
    for i in 0..matches.len().saturating_sub(1) {
        assert!(matches[i].score >= matches[i + 1].score);
    }
}

// =========================================================================
// 4. Agent Dispatcher Payload Formatting & Case Sensitivity Enforcement
// =========================================================================

#[test]
fn test_adversarial_agent_dispatcher_case_sensitivity_and_payloads() {
    // 1. Strict case-sensitivity validation
    assert!(AgentDispatcher::parse_strict_target("Cursor").is_ok());
    assert!(AgentDispatcher::parse_strict_target("Claude").is_ok());
    assert!(AgentDispatcher::parse_strict_target("Antigravity").is_ok());
    assert!(AgentDispatcher::parse_strict_target("Grokbot").is_ok());
    assert!(AgentDispatcher::parse_strict_target("Dots").is_ok());
    assert!(AgentDispatcher::parse_strict_target("Muse").is_ok());
    assert!(AgentDispatcher::parse_strict_target("Viktor").is_ok());
    assert!(AgentDispatcher::parse_strict_target("Gemini").is_ok());

    // Case mismatches MUST be rejected with UnsupportedAgent
    assert!(matches!(
        AgentDispatcher::parse_strict_target("cursor"),
        Err(ActionError::UnsupportedAgent(_))
    ));
    assert!(matches!(
        AgentDispatcher::parse_strict_target("CLAUDE"),
        Err(ActionError::UnsupportedAgent(_))
    ));
    assert!(matches!(
        AgentDispatcher::parse_strict_target("antigravity"),
        Err(ActionError::UnsupportedAgent(_))
    ));
    assert!(matches!(
        AgentDispatcher::parse_strict_target("NonExistentAgent"),
        Err(ActionError::UnsupportedAgent(_))
    ));

    // 2. Validate CLI formatting across all 8 agents with special characters
    let prompt = "Refactor error handler with 'single' and \"double\" quotes \u{1F680}\nNew line";
    let context = "crates/airctrl-core/src/error.rs:42";

    // A. Cursor
    let cursor_cli = AgentDispatcher::format_cli(AgentTarget::Cursor, prompt, context);
    assert_eq!(cursor_cli.executable, "cursor");
    assert_eq!(cursor_cli.args, vec!["."]);
    assert!(cursor_cli.stdin.unwrap().contains(prompt));

    // B. Claude
    let claude_cli = AgentDispatcher::format_cli(AgentTarget::Claude, prompt, context);
    assert_eq!(claude_cli.executable, "claude");
    assert_eq!(claude_cli.args[0], "--prompt");
    assert!(claude_cli.args[1].contains(prompt));

    // C. Antigravity
    let ag_cli = AgentDispatcher::format_cli(AgentTarget::Antigravity, prompt, context);
    assert_eq!(ag_cli.executable, "antigravity");
    assert_eq!(ag_cli.args[0], "run");
    assert_eq!(ag_cli.args[1], "--task");
    let ag_json: serde_json::Value = serde_json::from_str(&ag_cli.args[2]).expect("Valid JSON");
    assert_eq!(ag_json["protocol"], "antigravity-task-v1");
    assert_eq!(ag_json["mission"], prompt);

    // D. Grokbot
    let grok_cli = AgentDispatcher::format_cli(AgentTarget::Grokbot, prompt, context);
    assert_eq!(grok_cli.executable, "grokbot");
    assert_eq!(grok_cli.args, vec!["exec", "--query", prompt, "--context", context]);

    // E. Dots
    let dots_cli = AgentDispatcher::format_cli(AgentTarget::Dots, prompt, context);
    assert_eq!(dots_cli.executable, "dots");
    assert_eq!(dots_cli.args, vec!["run", "--instruction", prompt, "--json"]);

    // F. Muse
    let muse_cli = AgentDispatcher::format_cli(AgentTarget::Muse, prompt, context);
    assert_eq!(muse_cli.executable, "muse");
    assert_eq!(muse_cli.args, vec!["generate", "--prompt", prompt]);

    // G. Viktor
    let viktor_cli = AgentDispatcher::format_cli(AgentTarget::Viktor, prompt, context);
    assert_eq!(viktor_cli.executable, "viktor");
    assert_eq!(viktor_cli.args, vec!["dispatch", "--task", prompt]);

    // H. Gemini
    let gemini_cli = AgentDispatcher::format_cli(AgentTarget::Gemini, prompt, context);
    assert_eq!(gemini_cli.executable, "gemini");
    assert_eq!(gemini_cli.args, vec!["query", "--prompt", prompt]);
}

// =========================================================================
// 5. REA Inspector Bounds & Schema Extraction
// =========================================================================

#[test]
fn test_adversarial_rea_inspector_pid_bounds_and_schemas() {
    // 1. PID <= 0 rejected with InvalidPid
    assert!(matches!(
        ReaInspector::inspect(0, None),
        Err(ActionError::InvalidPid(0))
    ));
    assert!(matches!(
        ReaInspector::inspect(-1, None),
        Err(ActionError::InvalidPid(-1))
    ));
    assert!(matches!(
        ReaInspector::inspect(-999999, Some("/bin/app")),
        Err(ActionError::InvalidPid(-999999))
    ));

    // 2. Default binary path fallback when binary is None or blank
    let schema_none = ReaInspector::inspect(42, None).expect("Valid PID");
    assert_eq!(schema_none.binary_path, ReaInspector::DEFAULT_MOCK_BINARY);
    assert_eq!(schema_none.target_pid, 42);

    let schema_blank = ReaInspector::inspect(42, Some("   \t  ")).expect("Valid PID");
    assert_eq!(schema_blank.binary_path, ReaInspector::DEFAULT_MOCK_BINARY);

    // 3. Custom binary path preserved
    let schema_custom = ReaInspector::inspect(12345, Some("/custom/bin/app")).expect("Valid PID");
    assert_eq!(schema_custom.binary_path, "/custom/bin/app");
    assert_eq!(schema_custom.target_pid, 12345);

    // 4. Recursive task specification conversion
    let spec = ReaInspector::to_recursive_spec(&schema_custom, 3);
    assert_eq!(spec.recursive_depth, 3);
    assert_eq!(spec.target_pid, 12345);
    assert!(spec.task_id.starts_with("rea-spec-"));
    assert_eq!(spec.rea_version, "1.0");
    assert!(!spec.action_schema.buttons.is_empty());
}
