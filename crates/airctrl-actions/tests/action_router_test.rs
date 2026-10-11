use airctrl_actions::*;

#[test]
fn test_window_snap_left_and_maximize() {
    let cmd1 = parse_voice_command("snap left");
    assert_eq!(cmd1, ActionCommand::WindowSnap { direction: SnapDirection::Left });

    let cmd2 = parse_voice_command("dock maximize");
    assert_eq!(cmd2, ActionCommand::WindowSnap { direction: SnapDirection::Maximize });

    let cmd3 = parse_voice_command("tile right");
    assert_eq!(cmd3, ActionCommand::WindowSnap { direction: SnapDirection::Right });

    let cmd4 = parse_voice_command("snap window bottom");
    assert_eq!(cmd4, ActionCommand::WindowSnap { direction: SnapDirection::Bottom });

    let cmd5 = parse_voice_command("maximize");
    assert_eq!(cmd5, ActionCommand::WindowSnap { direction: SnapDirection::Maximize });
}

#[test]
fn test_window_close_and_minimize() {
    let cmd_close = parse_voice_command("close window");
    assert_eq!(cmd_close, ActionCommand::WindowClose { window_id: 101 });

    let cmd_min = parse_voice_command("minimize window");
    assert_eq!(cmd_min, ActionCommand::WindowMinimize { window_id: 101 });
}

#[test]
fn test_window_focus_and_switching() {
    let cmd = parse_voice_command("switch to Chrome");
    assert_eq!(cmd, ActionCommand::WindowFocus { window_id: 101, query: "chrome".to_string() });

    let cmd_focus = parse_voice_command("focus vscode");
    assert_eq!(cmd_focus, ActionCommand::WindowFocus { window_id: 101, query: "vscode".to_string() });
}

#[test]
fn test_fuzzy_window_focus_with_context() {
    let router = SemanticCommandRouter::default();
    let windows = vec![
        WindowInfo {
            id: 404,
            process_id: 1234,
            process_name: "chrome.exe".to_string(),
            title: "GitHub - airctrl - Google Chrome".to_string(),
            bounds: WindowRect::default(),
            is_focused: false,
            is_minimized: false,
        },
        WindowInfo {
            id: 505,
            process_id: 5678,
            process_name: "Code.exe".to_string(),
            title: "Visual Studio Code - router.rs".to_string(),
            bounds: WindowRect::default(),
            is_focused: true,
            is_minimized: false,
        },
    ];

    let cmd = router.parse_with_windows("switch to chrom", &windows);
    assert_eq!(cmd, ActionCommand::WindowFocus { window_id: 404, query: "chrom".to_string() });

    let cmd_vsc = router.parse_with_windows("focus vsc", &windows);
    assert_eq!(cmd_vsc, ActionCommand::WindowFocus { window_id: 505, query: "vsc".to_string() });
}

#[test]
fn test_agent_dispatch_parsing() {
    let cmd1 = parse_voice_command("ask claude: explain rust traits");
    match cmd1 {
        ActionCommand::AgentDispatch(payload) => {
            assert_eq!(payload.target, AgentTarget::Claude);
            assert_eq!(payload.prompt, "explain rust traits");
        }
        other => panic!("Expected AgentDispatch, got {:?}", other),
    }

    let cmd2 = parse_voice_command("ask cursor: : do task");
    match cmd2 {
        ActionCommand::AgentDispatch(payload) => {
            assert_eq!(payload.target, AgentTarget::Cursor);
            assert_eq!(payload.prompt, "do task");
        }
        other => panic!("Expected AgentDispatch, got {:?}", other),
    }

    let cmd3 = parse_voice_command("ask antigravity implement feature");
    match cmd3 {
        ActionCommand::AgentDispatch(payload) => {
            assert_eq!(payload.target, AgentTarget::Antigravity);
            assert_eq!(payload.prompt, "implement feature");
        }
        other => panic!("Expected AgentDispatch, got {:?}", other),
    }
}

#[test]
fn test_router_boundary_conditions() {
    // Single character utterance
    let cmd_single = parse_voice_command("a");
    assert_eq!(cmd_single, ActionCommand::Unknown { query: "a".to_string() });

    // Uppercase command
    let cmd_upper = parse_voice_command("SNAP LEFT");
    assert_eq!(cmd_upper, ActionCommand::WindowSnap { direction: SnapDirection::Left });

    // 1000 character prompt
    let long_prompt = "analyze this code ".repeat(55);
    let long_utterance = format!("ask claude: {}", long_prompt);
    let cmd_long = parse_voice_command(&long_utterance);
    match cmd_long {
        ActionCommand::AgentDispatch(payload) => {
            assert_eq!(payload.target, AgentTarget::Claude);
            assert_eq!(payload.prompt, long_prompt.trim());
        }
        other => panic!("Expected AgentDispatch for long prompt, got {:?}", other),
    }
}

#[test]
fn test_system_input_actions() {
    let cmd_scroll = parse_voice_command("scroll down 5");
    assert_eq!(
        cmd_scroll,
        ActionCommand::SystemInput(SystemInputAction::Scroll {
            direction: ScrollDirection::Down,
            amount: 5,
        })
    );

    let cmd_page = parse_voice_command("page up");
    assert_eq!(
        cmd_page,
        ActionCommand::SystemInput(SystemInputAction::Scroll {
            direction: ScrollDirection::Up,
            amount: 10,
        })
    );

    let cmd_click = parse_voice_command("click center");
    assert_eq!(
        cmd_click,
        ActionCommand::SystemInput(SystemInputAction::Click {
            position: Some(ClickPosition::Center),
            button: ClickAction::LeftClick,
        })
    );

    let cmd_double = parse_voice_command("double click");
    assert_eq!(
        cmd_double,
        ActionCommand::SystemInput(SystemInputAction::Click {
            position: None,
            button: ClickAction::DoubleClick,
        })
    );
}

#[test]
fn test_radial_dial_and_rea_inspect() {
    let cmd_dial = parse_voice_command("toggle dial");
    assert_eq!(cmd_dial, ActionCommand::RadialDialToggle {});

    let cmd_rea = parse_voice_command("rea inspect 1234 /usr/bin/test-app");
    assert_eq!(
        cmd_rea,
        ActionCommand::ReaInspect {
            target_pid: 1234,
            binary_path: Some("/usr/bin/test-app".to_string()),
        }
    );

    let cmd_unknown = parse_voice_command("unrecognized gibberish command");
    assert_eq!(
        cmd_unknown,
        ActionCommand::Unknown {
            query: "unrecognized gibberish command".to_string(),
        }
    );
}
