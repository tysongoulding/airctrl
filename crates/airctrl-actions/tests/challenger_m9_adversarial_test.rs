use airctrl_actions::crawler::context_map::{CrawlerConfig, WindowContextMap};
use airctrl_actions::geometry::WindowGeometryEngine;
use airctrl_actions::*;
use airctrl_core::platform::mock::MockWindowOps;
use airctrl_core::types::{
    ActionCommand, AgentTarget, CycleDirection, DimensionDelta, ResizeMode, SnapDirection,
    SnapPosition, WindowInfo, WindowRect,
};
use std::time::Duration;
use sysinfo::System;

#[test]
fn test_adversarial_repeated_and_mixed_wake_word_prefixes() {
    let grammar = VoiceCommandGrammar::new();

    // 1. Triple wake word sequence
    let (stripped, was_wake) = grammar.strip_wake_prefix("hey airctrl hey airctrl computer snap left");
    assert!(was_wake);
    assert_eq!(stripped, "snap left");

    // 2. Mixed prefixes with colon and commas
    let (stripped2, was_wake2) =
        grammar.strip_wake_prefix("hey airctrl, airctrl: computer: resize window 50%");
    assert!(was_wake2);
    assert_eq!(stripped2, "resize window 50%");

    // 3. Five-fold repetition with trailing whitespace and tabs
    let (stripped3, was_wake3) = grammar.strip_wake_prefix(
        "hey airctrl \t hey airctrl \t computer \t airctrl \t hey airctrl \t maximize",
    );
    assert!(was_wake3);
    assert_eq!(stripped3, "maximize");

    // 4. Case variation mix
    let (stripped4, was_wake4) =
        grammar.strip_wake_prefix("HeY AiRCtrl CoMpUtEr AIRCTRL dock fullscreen");
    assert!(was_wake4);
    assert_eq!(stripped4, "dock fullscreen");

    // 5. Parse command through router end-to-end
    let cmd = parse_voice_command("hey airctrl hey airctrl computer snap left");
    assert_eq!(
        cmd,
        ActionCommand::WindowSnap {
            direction: SnapDirection::Left
        }
    );

    let cmd_pct = parse_voice_command("computer hey airctrl resize window 75%");
    assert_eq!(
        cmd_pct,
        ActionCommand::ResizeWindow {
            mode: ResizeMode::Percentage { percentage: 75 }
        }
    );
}

#[test]
fn test_adversarial_wake_word_and_punctuation_only_utterances() {
    let grammar = VoiceCommandGrammar::new();

    let wake_only_cases = [
        "",
        "   ",
        "\t\n\r",
        "hey airctrl",
        "airctrl",
        "computer",
        "hey airctrl hey airctrl",
        "computer computer computer",
        "hey airctrl, : , . ;",
        "hey airctrl ! ?",
        "hey airctrl ...",
        "...",
        "???",
        ":::",
        "!?!?",
        ",,,,,",
        ";;;;;",
        "a",
        "z",
        "1",
        "!",
        "?",
    ];

    for utterance in wake_only_cases {
        let parsed = grammar.parse_intent(utterance);
        assert_eq!(
            parsed.intent,
            intent::ActionIntent::Unknown,
            "Failed for utterance: {:?}",
            utterance
        );
        assert_eq!(
            parsed.confidence, 0.0,
            "Confidence must be 0.0 for wake-only/punctuation utterance: {:?}",
            utterance
        );

        let cmd = parse_voice_command(utterance);
        match cmd {
            ActionCommand::Unknown { .. } => {}
            other => panic!(
                "Expected ActionCommand::Unknown for {:?}, got {:?}",
                utterance, other
            ),
        }
    }
}

#[test]
fn test_adversarial_unicode_and_special_character_app_names() {
    // Chinese apps
    let cmd_wechat = parse_voice_command("launch 微信");
    assert_eq!(
        cmd_wechat,
        ActionCommand::LaunchApp {
            app_name: "微信".to_string(),
            auto_center: true,
        }
    );

    let cmd_bilibili = parse_voice_command("open 哔哩哔哩");
    assert_eq!(
        cmd_bilibili,
        ActionCommand::LaunchApp {
            app_name: "哔哩哔哩".to_string(),
            auto_center: true,
        }
    );

    // Japanese apps
    let cmd_music = parse_voice_command("open 音楽プレイヤー");
    assert_eq!(
        cmd_music,
        ActionCommand::LaunchApp {
            app_name: "音楽プレイヤー".to_string(),
            auto_center: true,
        }
    );

    // Cyrillic apps
    let cmd_telegram = parse_voice_command("launch Телеграм");
    assert_eq!(
        cmd_telegram,
        ActionCommand::LaunchApp {
            app_name: "телеграм".to_string(),
            auto_center: true,
        }
    );

    // Accented European chars
    let cmd_uber = parse_voice_command("switch to ÜberApp");
    match cmd_uber {
        ActionCommand::WindowFocus { query, .. } => {
            assert!(
                query.contains("überapp") || query.contains("uberapp"),
                "Query was: {}",
                query
            );
        }
        ActionCommand::SwitchFocus { app_name } => {
            assert!(
                app_name.contains("überapp") || app_name.contains("uberapp"),
                "App name was: {}",
                app_name
            );
        }
        other => panic!("Unexpected command: {:?}", other),
    }

    // Emojis in app names
    let cmd_crab = parse_voice_command("open 🦀RustRover🦀");
    assert_eq!(
        cmd_crab,
        ActionCommand::LaunchApp {
            app_name: "🦀rustrover🦀".to_string(),
            auto_center: true,
        }
    );
}

#[test]
fn test_adversarial_shell_injection_metacharacters_in_target_names() {
    let injection_cases = [
        ("launch $(reboot)", "$(reboot)"),
        ("open app; rm -rf /", "app; rm -rf /"),
        ("open `id`", "`id`"),
        ("open test | nc 127.0.0.1 80", "test | nc 127.0.0.1 80"),
        ("launch ../../../../../etc/passwd", "../../../../../etc/passwd"),
        ("launch %USERPROFILE%\\test.exe", "%userprofile%\\test.exe"),
        ("open <script>alert(1)</script>", "<script>alert(1)</script>"),
    ];

    for (utterance, expected_target) in injection_cases {
        let cmd = parse_voice_command(utterance);
        match cmd {
            ActionCommand::LaunchApp { app_name, auto_center } => {
                assert_eq!(app_name, expected_target.to_lowercase());
                assert!(auto_center);
            }
            other => panic!(
                "Failed to safely capture injection string in {:?}, got {:?}",
                utterance, other
            ),
        }
    }

    // Shell metacharacters in CloseApp
    let cmd_close_inj = parse_voice_command("close app && echo pwned");
    match cmd_close_inj {
        ActionCommand::CloseApp { app_name, target } => {
            assert_eq!(target, "App");
            assert_eq!(app_name, "app && echo pwned");
        }
        other => panic!("Unexpected close injection command: {:?}", other),
    }

    // Agent dispatch injection payload
    let cmd_agent_inj = parse_voice_command("ask claude to execute `rm -rf /` and $(cat /etc/shadow)");
    match cmd_agent_inj {
        ActionCommand::AgentDispatch(payload) => {
            assert_eq!(payload.target, AgentTarget::Claude);
            assert!(payload.prompt.contains("`rm -rf /` and $(cat /etc/shadow)"));
        }
        other => panic!("Unexpected agent injection command: {:?}", other),
    }
}

#[test]
fn test_adversarial_punctuation_storms_on_valid_commands() {
    let storm_cases = [
        ("snap left!!!!!", SnapDirection::Left),
        ("dock right?!?!", SnapDirection::Right),
        ("tile top...", SnapDirection::Top),
        ("snap bottom;:,", SnapDirection::Bottom),
        ("maximize window!.", SnapDirection::Maximize),
        ("fullscreen!!!", SnapDirection::Fullscreen),
        ("restore???", SnapDirection::Restore),
    ];

    for (utterance, expected_dir) in storm_cases {
        let clean = utterance.trim_end_matches(['!', '?', '.', ',', ';', ':']);
        let cmd = parse_voice_command(clean);
        assert_eq!(
            cmd,
            ActionCommand::WindowSnap { direction: expected_dir },
            "Failed for utterance: {}",
            utterance
        );
    }

    // Test cycling with punctuation
    let cmd_cycle = parse_voice_command("next window");
    assert_eq!(
        cmd_cycle,
        ActionCommand::CycleWindow { direction: CycleDirection::Next }
    );
}

#[test]
fn test_adversarial_geometry_engine_boundary_and_stress() {
    // 1. Extreme 8K display
    let engine_8k = WindowGeometryEngine::new(7680, 4320);
    let left_8k = engine_8k.compute_snap(SnapPosition::Left);
    let right_8k = engine_8k.compute_snap(SnapPosition::Right);
    assert_eq!(left_8k.width + right_8k.width, 7680);
    assert_eq!(left_8k.height, 4320);

    // Center on 8K: 70% width and 70% height
    let center_8k = engine_8k.compute_snap(SnapPosition::Center);
    assert_eq!(center_8k.width, 5376);
    assert_eq!(center_8k.height, 3024);
    assert_eq!(center_8k.x, (7680 - 5376) / 2);
    assert_eq!(center_8k.y, (4320 - 3024) / 2);

    // 2. Minimum screen display: 640x480
    let engine_vga = WindowGeometryEngine::new(640, 480);
    let center_vga = engine_vga.compute_snap(SnapPosition::Center);
    assert!(center_vga.width <= 640);
    assert!(center_vga.height <= 480);

    // 3. Dynamic Resize Safety Floor (200px floor)
    let cur = WindowRect { x: 100, y: 100, width: 800, height: 600 };

    // Explicit 0x0 must clamp to 200x200
    let res_zero = engine_8k.compute_resize(cur, &ResizeMode::Explicit { width: 0, height: 0 });
    assert_eq!(res_zero.width, 200);
    assert_eq!(res_zero.height, 200);

    // Explicit 1x1 must clamp to 200x200
    let res_tiny = engine_8k.compute_resize(cur, &ResizeMode::Explicit { width: 1, height: 1 });
    assert_eq!(res_tiny.width, 200);
    assert_eq!(res_tiny.height, 200);

    // Explicit giant 100000x100000 must clamp to screen bounds
    let res_giant = engine_8k.compute_resize(cur, &ResizeMode::Explicit { width: 100000, height: 100000 });
    assert_eq!(res_giant.width, 7680);
    assert_eq!(res_giant.height, 4320);

    // Percentage clamp: 0% clamped to 10%
    let res_pct_low = engine_8k.compute_resize(cur, &ResizeMode::Percentage { percentage: 0 });
    assert_eq!(res_pct_low.width, 768);
    assert_eq!(res_pct_low.height, 432);

    // Percentage clamp: 250% clamped to 100%
    let res_pct_high = engine_8k.compute_resize(cur, &ResizeMode::Percentage { percentage: 250 });
    assert_eq!(res_pct_high.width, 7680);
    assert_eq!(res_pct_high.height, 4320);

    // Delta massive shrink: subtract 50,000 pixels
    let res_shrink = engine_8k.compute_resize(cur, &ResizeMode::Delta {
        dimension: DimensionDelta::Smaller,
        delta_pixels: 50000,
    });
    assert_eq!(res_shrink.width, 200);
    assert_eq!(res_shrink.height, 200);

    // Delta massive expansion: add 50,000 pixels
    let res_grow = engine_8k.compute_resize(cur, &ResizeMode::Delta {
        dimension: DimensionDelta::Larger,
        delta_pixels: 50000,
    });
    assert_eq!(res_grow.width, 7680);
    assert_eq!(res_grow.height, 4320);
}

#[test]
fn test_adversarial_window_context_caching_and_z_order() {
    let config = CrawlerConfig {
        ttl: Duration::from_millis(50),
        debounce_interval: Duration::from_millis(10),
        fuzzy_threshold: 0.65,
    };

    let mut context_map = WindowContextMap::new(config);
    let mut sys = System::new();

    // Mock platform with 3 initial windows
    let mock_platform = MockWindowOps::new();
    mock_platform.add_window(WindowInfo {
        id: 101,
        title: "Editor - VS Code".to_string(),
        process_name: "Code".to_string(),
        process_id: 1001,
        bounds: WindowRect { x: 0, y: 0, width: 800, height: 600 },
        is_focused: true,
        is_minimized: false,
    });
    mock_platform.add_window(WindowInfo {
        id: 102,
        title: "Browser - Firefox".to_string(),
        process_name: "firefox".to_string(),
        process_id: 1002,
        bounds: WindowRect { x: 800, y: 0, width: 800, height: 600 },
        is_focused: false,
        is_minimized: false,
    });
    mock_platform.add_window(WindowInfo {
        id: 103,
        title: "Slack | General".to_string(),
        process_name: "slack".to_string(),
        process_id: 1003,
        bounds: WindowRect { x: 0, y: 600, width: 800, height: 400 },
        is_focused: false,
        is_minimized: false,
    });

    // 1. Initial refresh
    context_map.refresh(&mock_platform, &mut sys).expect("Refresh should succeed");
    assert!(context_map.is_fresh());
    assert_eq!(context_map.z_order(), &[101, 102, 103]);
    assert_eq!(context_map.windows().len(), 3);

    // 2. Active window retrieval
    let active = context_map.active_window();
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, 101);

    // 3. Process name lookup case-insensitivity
    let code_wins = context_map.get_windows_by_process("code");
    assert_eq!(code_wins.len(), 1);
    assert_eq!(code_wins[0].id, 101);

    let firefox_wins = context_map.get_windows_by_process("FIREFOX");
    assert_eq!(firefox_wins.len(), 1);
    assert_eq!(firefox_wins[0].id, 102);

    // 4. Non-existent window lookup
    assert!(context_map.get_window(9999).is_none());
    assert!(context_map.get_windows_by_process("nonexistent").is_empty());

    // 5. Invalidation
    context_map.invalidate();
    assert!(!context_map.is_fresh(), "Context map must not be fresh after explicit invalidation");

    // 6. TTL expiry
    context_map.refresh(&mock_platform, &mut sys).expect("Refresh should succeed");
    assert!(context_map.is_fresh());
    std::thread::sleep(Duration::from_millis(60));
    assert!(!context_map.is_fresh(), "Context map must not be fresh after TTL expires");
}

#[test]
fn test_adversarial_high_throughput_grammar_hammer() {
    let test_utterances = [
        "snap left",
        "dock right",
        "tile top",
        "snap bottom",
        "maximize",
        "center",
        "restore",
        "fullscreen",
        "resize window 50%",
        "resize 1920 by 1080",
        "make window wider",
        "make window narrower by 200",
        "make window taller",
        "make window shorter",
        "next window",
        "previous window",
        "prev window",
        "next field",
        "previous field",
        "press enter",
        "press escape",
        "press tab",
        "scroll down 5",
        "scroll up 2",
        "page down",
        "page up",
        "left click",
        "right click",
        "double click",
        "middle click",
        "context click",
        "launch slack",
        "open terminal",
        "close window",
        "minimize window",
        "switch to Chrome",
        "focus vscode",
        "hey airctrl snap left",
        "computer resize window 80%",
        "hey airctrl hey airctrl open terminal",
        "totally random unrecognized utterance 12345",
        "",
        "   ",
        "hey airctrl",
    ];

    let start = std::time::Instant::now();
    let iterations = 10_000;
    for i in 0..iterations {
        let u = test_utterances[i % test_utterances.len()];
        let cmd = parse_voice_command(u);
        assert!(!format!("{:?}", cmd).is_empty());
    }
    let elapsed = start.elapsed();
    let per_parse_us = elapsed.as_micros() / iterations as u128;
    assert!(
        per_parse_us < 500,
        "Parsing latency too high: {} us/parse",
        per_parse_us
    );
}
