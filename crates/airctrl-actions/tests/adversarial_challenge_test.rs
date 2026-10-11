use airctrl_actions::*;
use airctrl_core::types::{ActionCommand, AgentTarget, SnapDirection};
use std::time::Instant;

// ============================================================================
// 1. Utterance Fuzzing: Single-char, Empty, Extreme Large, Punctuation, Casing
// ============================================================================

#[test]
fn test_adversarial_utterance_fuzzing_single_char_and_empty() {
    let edge_cases = [
        "",
        " ",
        "   ",
        "\t",
        "\n",
        "\r\n",
        "a",
        "Z",
        "?",
        "!",
        ".",
        ":",
        ",",
        "-",
        "_",
        "\0",
        "🚀",
        "🎙️",
        "あ",
    ];

    for input in &edge_cases {
        let cmd = parse_voice_command(input);
        match cmd {
            ActionCommand::Unknown { query } => {
                assert_eq!(
                    query, *input,
                    "Expected Unknown with verbatim query for {:?}",
                    input
                );
            }
            other => panic!(
                "Expected ActionCommand::Unknown for input {:?}, got {:?}",
                input, other
            ),
        }
    }
}

#[test]
fn test_adversarial_utterance_fuzzing_extreme_large_inputs() {
    // 1. Extreme large gibberish utterance (20,000 characters)
    let massive_gibberish = "a".repeat(20_000);
    let start = Instant::now();
    let cmd_gibberish = parse_voice_command(&massive_gibberish);
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 500,
        "Parsing 20k gibberish took too long: {:?}",
        duration
    );
    match cmd_gibberish {
        ActionCommand::Unknown { query } => {
            assert_eq!(query.len(), 20_000);
        }
        other => panic!("Expected Unknown for massive gibberish, got {:?}", other),
    }

    // 2. Extreme large agent prompt (15,000 characters)
    let large_body = "explain the quantum computing algorithms in depth ".repeat(300);
    let agent_utterance = format!("ask claude: {}", large_body);
    let start = Instant::now();
    let cmd_agent = parse_voice_command(&agent_utterance);
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 500,
        "Parsing 15k agent prompt took too long: {:?}",
        duration
    );
    match cmd_agent {
        ActionCommand::AgentDispatch(payload) => {
            assert_eq!(payload.target, AgentTarget::Claude);
            assert_eq!(payload.prompt, large_body.trim());
            assert!(payload.prompt.len() > 14_000);
        }
        other => panic!("Expected AgentDispatch for large utterance, got {:?}", other),
    }

    // 3. Extreme large switch to query (10,000 characters)
    let long_target = "myapp_window_title_".repeat(500);
    let focus_utterance = format!("switch to {}", long_target);
    let cmd_focus = parse_voice_command(&focus_utterance);
    match cmd_focus {
        ActionCommand::WindowFocus { query, .. } => {
            assert_eq!(query, long_target.to_lowercase());
        }
        other => panic!("Expected WindowFocus for large query, got {:?}", other),
    }
}

#[test]
fn test_adversarial_punctuation_storms() {
    // Variations of colons, commas, and excessive whitespace
    let punctuation_cases = [
        ("ask cursor: : :   refactor this", AgentTarget::Cursor, "refactor this"),
        ("ask claude::: explain rust", AgentTarget::Claude, "explain rust"),
        ("ask antigravity, , , : : build system", AgentTarget::Antigravity, "build system"),
        ("ask grokbot   :::   status report", AgentTarget::Grokbot, "status report"),
        ("ask dots: , : run verification", AgentTarget::Dots, "run verification"),
        ("ask muse: : compose audio", AgentTarget::Muse, "compose audio"),
        ("ask viktor::: execute plan", AgentTarget::Viktor, "execute plan"),
        ("ask gemini   : : analyze multimodal", AgentTarget::Gemini, "analyze multimodal"),
        ("cursor: : : fix bug", AgentTarget::Cursor, "fix bug"),
        ("claude: explain code", AgentTarget::Claude, "explain code"),
        ("antigravity:   launch workflow", AgentTarget::Antigravity, "launch workflow"),
    ];

    for (utterance, expected_target, expected_prompt) in &punctuation_cases {
        let cmd = parse_voice_command(utterance);
        match cmd {
            ActionCommand::AgentDispatch(payload) => {
                assert_eq!(
                    payload.target, *expected_target,
                    "Target mismatch for {:?}",
                    utterance
                );
                assert_eq!(
                    payload.prompt, *expected_prompt,
                    "Prompt mismatch for {:?}",
                    utterance
                );
            }
            other => panic!(
                "Expected AgentDispatch for punctuation storm {:?}, got {:?}",
                utterance, other
            ),
        }
    }

    // Whitespace storms in window snap
    let snap_cases = [
        "   snap     left   ",
        "dock\t\tmaximize",
        "  tile   right  ",
        "snap   window   bottom",
    ];
    for s in &snap_cases {
        let cmd = parse_voice_command(s);
        match cmd {
            ActionCommand::WindowSnap { .. } => {}
            other => panic!("Expected WindowSnap for {:?}, got {:?}", s, other),
        }
    }
}

#[test]
fn test_adversarial_case_sensitivity_and_insensitivity() {
    // A. Voice commands MUST be case-insensitive
    let voice_casing_tests = [
        ("SNAP LEFT", ActionCommand::WindowSnap { direction: SnapDirection::Left }),
        ("sNaP lEfT", ActionCommand::WindowSnap { direction: SnapDirection::Left }),
        ("dock MAXIMIZE", ActionCommand::WindowSnap { direction: SnapDirection::Maximize }),
        ("CLOSE WINDOW", ActionCommand::WindowClose { window_id: 101 }),
        ("mInImIzE wInDoW", ActionCommand::WindowMinimize { window_id: 101 }),
        ("SWITCH TO CHROME", ActionCommand::WindowFocus { window_id: 101, query: "chrome".to_string() }),
        ("Switch To VSCode", ActionCommand::WindowFocus { window_id: 101, query: "vscode".to_string() }),
        ("TOGGLE DIAL", ActionCommand::RadialDialToggle {}),
        ("rAdIaL dIaL", ActionCommand::RadialDialToggle {}),
    ];

    for (utterance, expected) in &voice_casing_tests {
        let actual = parse_voice_command(utterance);
        assert_eq!(&actual, expected, "Voice casing failed for {:?}", utterance);
    }

    // Voice agent dispatch in diverse casings
    let agent_cases = [
        ("ASK CURSOR: test", AgentTarget::Cursor),
        ("Ask Claude: test", AgentTarget::Claude),
        ("aSk AnTiGrAvItY: test", AgentTarget::Antigravity),
        ("ASK GROKBOT: test", AgentTarget::Grokbot),
        ("Ask Dots: test", AgentTarget::Dots),
        ("aSk MuSe: test", AgentTarget::Muse),
        ("ASK VIKTOR: test", AgentTarget::Viktor),
        ("Ask Gemini: test", AgentTarget::Gemini),
    ];

    for (utterance, expected_target) in &agent_cases {
        let cmd = parse_voice_command(utterance);
        match cmd {
            ActionCommand::AgentDispatch(payload) => {
                assert_eq!(
                    payload.target, *expected_target,
                    "Voice agent casing failed for {:?}",
                    utterance
                );
            }
            other => panic!("Expected AgentDispatch for {:?}, got {:?}", utterance, other),
        }
    }

    // B. Agent target dispatch MUST strictly validate PascalCase target names
    let valid_targets = [
        "Cursor", "Claude", "Antigravity", "Grokbot", "Dots", "Muse", "Viktor", "Gemini",
    ];
    for target in &valid_targets {
        let res = AgentDispatcher::dispatch(target, "hello", None);
        assert!(
            res.is_ok(),
            "Expected valid PascalCase target {:?} to succeed",
            target
        );
        let dispatched = res.unwrap();
        assert_eq!(dispatched.target, *target);
    }

    let invalid_casings = [
        "cursor", "CURSOR", "cUrSoR",
        "claude", "CLAUDE", "cLaUdE",
        "antigravity", "ANTIGRAVITY", "AntiGravity",
        "grokbot", "GROKBOT", "GrokBot",
        "dots", "DOTS",
        "muse", "MUSE",
        "viktor", "VIKTOR",
        "gemini", "GEMINI",
        "unknown", "invalid", "", "   ",
    ];
    for invalid in &invalid_casings {
        let res = AgentDispatcher::dispatch(invalid, "hello", None);
        assert!(
            res.is_err(),
            "Expected invalid casing/target {:?} to be strictly rejected",
            invalid
        );
        match res.unwrap_err() {
            ActionError::UnsupportedAgent(msg) => {
                assert!(msg.contains(invalid.trim()));
            }
            other => panic!("Expected UnsupportedAgent error, got {:?}", other),
        }
    }
}

// ============================================================================
// 2. REA Inspector Boundary Attacks: PID <= 0, Extreme PIDs, Paths, Fallbacks
// ============================================================================

#[test]
fn test_adversarial_rea_inspector_pid_boundary_attacks() {
    // 1. PID <= 0 MUST be rejected with ActionError::InvalidPid
    let invalid_pids = [0, -1, -2, -100, -999999, i64::MIN, -2_147_483_648];
    for &pid in &invalid_pids {
        let res = ReaInspector::inspect(pid, None);
        assert!(
            res.is_err(),
            "PID {} should have been rejected with ActionError",
            pid
        );
        match res.unwrap_err() {
            ActionError::InvalidPid(err_pid) => {
                assert_eq!(err_pid, pid);
            }
            other => panic!("Expected InvalidPid error for {}, got {:?}", pid, other),
        }
    }

    // 2. Direct execute_rea_inspection wrapper with PID 0
    let res_wrapper_zero = execute_rea_inspection(0, None);
    assert!(res_wrapper_zero.is_err());
    match res_wrapper_zero.unwrap_err() {
        ActionError::InvalidPid(0) => {}
        other => panic!("Expected InvalidPid(0), got {:?}", other),
    }

    // 3. Extreme valid PIDs: PID 1 and PID 2,147,483,647 (i32 max)
    let res_pid1 = ReaInspector::inspect(1, None).expect("PID 1 should be valid");
    assert_eq!(res_pid1.target_pid, 1);
    assert_eq!(res_pid1.binary_path, ReaInspector::DEFAULT_MOCK_BINARY);
    assert_eq!(res_pid1.status, "SchemaExtracted");

    let res_i32_max = ReaInspector::inspect(2_147_483_647, None)
        .expect("PID 2,147,483,647 (i32::MAX) should be valid");
    assert_eq!(res_i32_max.target_pid, 2_147_483_647);

    let res_wrapper_max = execute_rea_inspection(2_147_483_647, None)
        .expect("execute_rea_inspection with i32::MAX should succeed");
    assert_eq!(res_wrapper_max.target_pid, 2_147_483_647);
}

#[test]
fn test_adversarial_rea_inspector_path_variations_and_fallback() {
    // 1. None and empty / whitespace binary path fall back to /usr/bin/mock-app
    let fallback_paths: [Option<&str>; 4] = [None, Some(""), Some("   "), Some("\t\n")];
    for path_opt in fallback_paths {
        let res = ReaInspector::inspect(42, path_opt).expect("Inspection should succeed");
        assert_eq!(
            res.binary_path, "/usr/bin/mock-app",
            "Expected fallback to default mock binary for {:?}",
            path_opt
        );
    }

    // 2. Spaces and complex paths (Windows, Linux paths with special chars)
    let complex_paths = [
        r#"C:\Program Files\App\run.exe"#,
        r#"C:\Program Files (x86)\Common Files\Service.exe"#,
        r#""C:\Program Files\App with quotes\run.exe""#,
        "/usr/local/bin/my app (v2) #1/run.bin",
        "/opt/deeply/nested/directory/structure/with-dashes_and_underscores/binary",
    ];

    for path in &complex_paths {
        let res = ReaInspector::inspect(1001, Some(path)).expect("Complex path failed");
        assert_eq!(res.binary_path, *path);
        assert_eq!(res.target_pid, 1001);

        // Verify recursive spec preserves exact path
        let spec = ReaInspector::to_recursive_spec(&res, 5);
        assert_eq!(spec.binary_path, *path);
        assert_eq!(spec.recursive_depth, 5);
    }

    // 3. execute_rea_inspection_by_name helper
    let res_by_name = execute_rea_inspection_by_name("chrome.exe").expect("Inspect by name failed");
    assert_eq!(res_by_name.target_pid, 1234);
    assert_eq!(res_by_name.binary_path, "chrome.exe");
}

// ============================================================================
// 3. Agent Dispatch Edge Cases: Multiline Prompts, Context Sizes, Payloads
// ============================================================================

#[test]
fn test_adversarial_agent_dispatch_multiline_prompts() {
    // 1. Multiline prompt with 120 newlines
    let newlines_prompt = "\n".repeat(120);
    let res = AgentDispatcher::dispatch("Claude", &newlines_prompt, None)
        .expect("120 newlines dispatch failed");
    assert_eq!(res.prompt_len, 120);
    assert_eq!(res.payload.instruction, newlines_prompt);

    // Verify Claude CLI argument contains the full prompt
    let cli = res.cli.expect("Claude CLI missing");
    assert_eq!(cli.executable, "claude");
    assert_eq!(cli.args[1], newlines_prompt);

    // 2. Complex multiline code snippet (150 lines with comments, tabs, unicode)
    let mut code_lines = Vec::new();
    for i in 0..150 {
        code_lines.push(format!("    fn function_{}(val: i32) -> Result<String, ()> {{ // Line {}\n        println!(\"Test 日本語 🚀\");\n        Ok(val.to_string())\n    }}", i, i));
    }
    let code_prompt = code_lines.join("\n");

    for target in AgentDispatcher::SUPPORTED_AGENTS {
        let res_target = AgentDispatcher::dispatch(target, &code_prompt, None)
            .expect("Multiline code dispatch failed");
        assert_eq!(res_target.target, target);
        assert_eq!(res_target.payload.instruction, code_prompt);
        assert_eq!(res_target.prompt_len, code_prompt.chars().count());

        let cli = res_target.cli.expect("CLI payload missing");
        match target {
            "Cursor" => {
                assert_eq!(cli.executable, "cursor");
                assert_eq!(cli.stdin, Some(code_prompt.clone()));
            }
            "Claude" => {
                assert_eq!(cli.executable, "claude");
                assert_eq!(cli.args, vec!["--prompt", &code_prompt]);
            }
            "Antigravity" => {
                assert_eq!(cli.executable, "antigravity");
                assert_eq!(cli.args[0], "run");
                assert_eq!(cli.args[1], "--task");
                // Validate that the JSON is valid and preserves the multiline prompt exactly
                let parsed_json: serde_json::Value =
                    serde_json::from_str(&cli.args[2]).expect("Invalid Antigravity JSON payload");
                assert_eq!(parsed_json["protocol"], "antigravity-task-v1");
                assert_eq!(parsed_json["mission"], code_prompt);
            }
            "Grokbot" => {
                assert_eq!(cli.executable, "grokbot");
                assert_eq!(cli.args[0], "exec");
                assert_eq!(cli.args[1], "--query");
                assert_eq!(cli.args[2], code_prompt);
            }
            "Dots" => {
                assert_eq!(cli.executable, "dots");
                assert_eq!(cli.args[0], "run");
                assert_eq!(cli.args[1], "--instruction");
                assert_eq!(cli.args[2], code_prompt);
            }
            "Muse" => {
                assert_eq!(cli.executable, "muse");
                assert_eq!(cli.args, vec!["generate", "--prompt", &code_prompt]);
            }
            "Viktor" => {
                assert_eq!(cli.executable, "viktor");
                assert_eq!(cli.args, vec!["dispatch", "--task", &code_prompt]);
            }
            "Gemini" => {
                assert_eq!(cli.executable, "gemini");
                assert_eq!(cli.args, vec!["query", "--prompt", &code_prompt]);
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_adversarial_agent_dispatch_context_window_sizes() {
    // 1. None vs Empty context string
    let res_none = AgentDispatcher::dispatch("Cursor", "prompt", None)
        .expect("None context failed");
    assert_eq!(res_none.payload.context_window, "");
    assert_eq!(res_none.cli.unwrap().stdin, Some("prompt".to_string()));

    let res_empty = AgentDispatcher::dispatch("Cursor", "prompt", Some(""))
        .expect("Empty context failed");
    assert_eq!(res_empty.payload.context_window, "");
    assert_eq!(res_empty.cli.unwrap().stdin, Some("prompt".to_string()));

    // 2. Large context window: 60,000+ characters
    let large_context = "CONTEXT_LINE_ABC_123\n".repeat(3000);
    assert!(large_context.len() >= 60_000);

    // Test Cursor with large context
    let res_cursor = AgentDispatcher::dispatch("Cursor", "my prompt", Some(&large_context))
        .expect("Cursor large context failed");
    assert_eq!(res_cursor.payload.context_window.len(), large_context.len());
    let stdin = res_cursor.cli.unwrap().stdin.unwrap();
    assert!(stdin.starts_with("[Context: CONTEXT_LINE_ABC_123"));
    assert!(stdin.ends_with("\n\nmy prompt"));

    // Test Claude with large context
    let res_claude = AgentDispatcher::dispatch("Claude", "my prompt", Some(&large_context))
        .expect("Claude large context failed");
    let claude_prompt = &res_claude.cli.unwrap().args[1];
    assert!(claude_prompt.starts_with("[Context: "));
    assert!(claude_prompt.ends_with("\n\nmy prompt"));

    // Test Antigravity with large context in JSON
    let res_antigravity =
        AgentDispatcher::dispatch("Antigravity", "mission alpha", Some(&large_context))
            .expect("Antigravity large context failed");
    let json_arg = &res_antigravity.cli.unwrap().args[2];
    let parsed_json: serde_json::Value =
        serde_json::from_str(json_arg).expect("Antigravity JSON deserialization failed");
    assert_eq!(parsed_json["mission"], "mission alpha");
    assert_eq!(
        parsed_json["context"].as_str().unwrap().len(),
        large_context.len()
    );

    // Test Grokbot with large context
    let res_grokbot = AgentDispatcher::dispatch("Grokbot", "query", Some(&large_context))
        .expect("Grokbot large context failed");
    let grok_args = res_grokbot.cli.unwrap().args;
    assert_eq!(grok_args[3], "--context");
    assert_eq!(grok_args[4].len(), large_context.len());
}

// ============================================================================
// 4. Fuzzy Matcher Boundary Stress: Empty Candidates, 10k Strings, Timing
// ============================================================================

#[test]
fn test_adversarial_fuzzy_matcher_stress_and_edge_cases() {
    let matcher = FuzzyMatcher::default();

    // 1. Empty candidates list
    let no_match = matcher.find_best_match("chrome", &[]);
    assert!(no_match.is_none());

    // 2. Candidates with empty / whitespace strings
    let empty_candidates = vec![
        FuzzyCandidate {
            id: Some(1),
            display_name: "",
            process_name: None,
            window_title: None,
        },
        FuzzyCandidate {
            id: Some(2),
            display_name: "   ",
            process_name: Some(""),
            window_title: Some("   "),
        },
    ];
    assert!(matcher.find_best_match("chrome", &empty_candidates).is_none());

    // 3. Extreme 10,000-character query evaluated against 50 candidates
    let long_query = "super_long_query_string_".repeat(400);
    let mut candidates = Vec::new();
    for i in 0..50 {
        candidates.push(FuzzyCandidate {
            id: Some(i),
            display_name: "Google Chrome",
            process_name: Some("chrome.exe"),
            window_title: Some("Google Chrome - airctrl development"),
        });
    }

    let start = Instant::now();
    let res = matcher.find_best_match(&long_query, &candidates);
    let elapsed = start.elapsed();

    // Must complete safely without panic or memory corruption, and correctly reject
    assert!(
        elapsed.as_secs() < 15,
        "Fuzzy matching with 10k query took too long: {:?}",
        elapsed
    );
    assert!(
        res.is_none(),
        "Completely unrelated 10k query should have been rejected below threshold"
    );

    // 4. Exact match against candidate with 10,000-character display name
    let big_name = "visual_studio_code_long_title_".repeat(350);
    let big_candidate = [FuzzyCandidate {
        id: Some(999),
        display_name: &big_name,
        process_name: None,
        window_title: None,
    }];
    let exact_match = matcher.find_best_match(&big_name, &big_candidate);
    assert!(exact_match.is_some());
    assert_eq!(exact_match.unwrap().candidate_id, Some(999));
}
