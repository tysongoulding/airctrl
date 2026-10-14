use airctrl_actions::*;
use airctrl_core::types::{
    ActionCommand, ClickAction, CycleDirection, DimensionDelta, ResizeMode, ScrollDirection,
    SnapDirection, UiTarget,
};

#[test]
fn test_wake_word_stripping() {
    let grammar = VoiceCommandGrammar::new();

    // Single prefix
    let (stripped, was_wake) = grammar.strip_wake_prefix("hey airctrl snap left");
    assert!(was_wake);
    assert_eq!(stripped, "snap left");

    // Alternate prefix
    let (stripped2, was_wake2) = grammar.strip_wake_prefix("airctrl close window");
    assert!(was_wake2);
    assert_eq!(stripped2, "close window");

    // Computer prefix
    let (stripped3, was_wake3) = grammar.strip_wake_prefix("computer resize window 50%");
    assert!(was_wake3);
    assert_eq!(stripped3, "resize window 50%");

    // Wake prefix only
    let (stripped_empty, was_wake_empty) = grammar.strip_wake_prefix("hey airctrl   ");
    assert!(was_wake_empty);
    assert_eq!(stripped_empty, "");

    // Repeated wake prefixes
    let (stripped_rep, was_wake_rep) = grammar.strip_wake_prefix("hey airctrl hey airctrl open terminal");
    assert!(was_wake_rep);
    assert_eq!(stripped_rep, "open terminal");

    // No wake prefix
    let (stripped_none, was_wake_none) = grammar.strip_wake_prefix("switch to Slack");
    assert!(!was_wake_none);
    assert_eq!(stripped_none, "switch to Slack");
}

#[test]
fn test_all_9_snap_positions() {
    let positions = [
        ("snap left", SnapDirection::Left),
        ("dock right", SnapDirection::Right),
        ("tile top", SnapDirection::Top),
        ("snap bottom", SnapDirection::Bottom),
        ("snap center", SnapDirection::Center),
        ("snap maximize", SnapDirection::Maximize),
        ("maximize window", SnapDirection::Maximize),
        ("snap minimize", SnapDirection::Minimize),
        ("restore", SnapDirection::Restore),
        ("fullscreen", SnapDirection::Fullscreen),
        ("dock fullscreen", SnapDirection::Fullscreen),
    ];

    for (utterance, expected_dir) in positions {
        let cmd = parse_voice_command(utterance);
        assert_eq!(
            cmd,
            ActionCommand::WindowSnap { direction: expected_dir },
            "Failed for utterance: {}",
            utterance
        );
    }
}

#[test]
fn test_dynamic_window_resizing() {
    // Explicit
    let cmd_exp = parse_voice_command("resize 1280 by 720");
    assert_eq!(
        cmd_exp,
        ActionCommand::ResizeWindow {
            mode: ResizeMode::Explicit { width: 1280, height: 720 }
        }
    );

    // Percentage
    let cmd_pct = parse_voice_command("resize window 80%");
    assert_eq!(
        cmd_pct,
        ActionCommand::ResizeWindow {
            mode: ResizeMode::Percentage { percentage: 80 }
        }
    );

    // Delta larger
    let cmd_larger = parse_voice_command("make window larger by 50");
    assert_eq!(
        cmd_larger,
        ActionCommand::ResizeWindow {
            mode: ResizeMode::Delta {
                dimension: DimensionDelta::Larger,
                delta_pixels: 50,
            }
        }
    );

    // Delta narrower default 100
    let cmd_narrower = parse_voice_command("make window narrower");
    assert_eq!(
        cmd_narrower,
        ActionCommand::ResizeWindow {
            mode: ResizeMode::Delta {
                dimension: DimensionDelta::Narrower,
                delta_pixels: 100,
            }
        }
    );
}

#[test]
fn test_window_cycling_and_focus() {
    let cmd_next = parse_voice_command("next window");
    assert_eq!(
        cmd_next,
        ActionCommand::CycleWindow { direction: CycleDirection::Next }
    );

    let cmd_prev = parse_voice_command("previous window");
    assert_eq!(
        cmd_prev,
        ActionCommand::CycleWindow { direction: CycleDirection::Previous }
    );

    let cmd_prev_short = parse_voice_command("prev window");
    assert_eq!(
        cmd_prev_short,
        ActionCommand::CycleWindow { direction: CycleDirection::Previous }
    );

    // Focus with trailing punctuation
    let cmd_focus_punct = parse_voice_command("switch to VS Code.");
    assert_eq!(
        cmd_focus_punct,
        ActionCommand::WindowFocus {
            window_id: 101,
            query: "vs code".to_string()
        }
    );

    // Bare "focus" returns unknown
    let cmd_bare = parse_voice_command("focus");
    assert_eq!(cmd_bare, ActionCommand::Unknown { query: "focus".to_string() });
}

#[test]
fn test_ui_navigation_and_clicking() {
    let nav_cases = [
        ("next field", UiTarget::NextField),
        ("previous field", UiTarget::PrevField),
        ("press enter", UiTarget::Enter),
        ("press escape", UiTarget::Escape),
        ("press tab", UiTarget::Tab),
    ];

    for (utterance, expected) in nav_cases {
        let cmd = parse_voice_command(utterance);
        assert_eq!(
            cmd,
            ActionCommand::NavigateUi { element: expected },
            "Failed for: {}",
            utterance
        );
    }

    // Context click maps to RightClick
    let cmd_context = parse_voice_command("context click");
    match cmd_context {
        ActionCommand::SystemInput(airctrl_core::types::SystemInputAction::Click { button, .. }) => {
            assert_eq!(button, ClickAction::RightClick);
        }
        other => panic!("Expected SystemInput Click, got {:?}", other),
    }

    // Scrolling
    let cmd_scroll = parse_voice_command("scroll up 5");
    assert_eq!(
        cmd_scroll,
        ActionCommand::SystemInput(airctrl_core::types::SystemInputAction::Scroll {
            direction: ScrollDirection::Up,
            amount: 5,
        })
    );
}

#[test]
fn test_geometry_engine_9way_and_resizing() {
    let engine = WindowGeometryEngine::new(1920, 1080);

    // Left snap
    let b_left = engine.compute_snap(SnapDirection::Left);
    assert_eq!(b_left.x, 0);
    assert_eq!(b_left.y, 0);
    assert_eq!(b_left.width, 960);
    assert_eq!(b_left.height, 1080);

    // Right snap
    let b_right = engine.compute_snap(SnapDirection::Right);
    assert_eq!(b_right.x, 960);
    assert_eq!(b_right.y, 0);
    assert_eq!(b_right.width, 960);
    assert_eq!(b_right.height, 1080);

    // Center snap 70%
    let b_center = engine.compute_snap(SnapDirection::Center);
    assert_eq!(b_center.width, (1920 * 70) / 100);
    assert_eq!(b_center.height, (1080 * 70) / 100);

    // Minimize snap zero area
    let b_min = engine.compute_snap(SnapDirection::Minimize);
    assert_eq!(b_min.width, 0);
    assert_eq!(b_min.height, 0);

    // Dynamic resize floor safety (200px)
    let small_rect = airctrl_core::types::WindowRect { x: 50, y: 50, width: 250, height: 250 };
    let resized = engine.compute_resize(
        small_rect,
        &ResizeMode::Delta {
            dimension: DimensionDelta::Smaller,
            delta_pixels: 100,
        },
    );
    assert_eq!(resized.width, 200);
    assert_eq!(resized.height, 200);
}
