use airctrl_shell::ipc::{IpcBridge, IpcError};
use serde_json::json;
use std::sync::Arc;
use std::thread;

#[test]
fn test_adversarial_ipc_execute_voice_command_fuzzing() {
    let bridge = IpcBridge::default();

    // 1. Missing or invalid payload structures
    let invalid_payloads = vec![
        None,
        Some(json!({})),
        Some(json!({ "utterance": null })),
        Some(json!({ "utterance": 12345 })),
        Some(json!({ "utterance": true })),
        Some(json!({ "utterance": ["snap", "left"] })),
        Some(json!({ "utterance": { "sub": "command" } })),
    ];

    for payload in invalid_payloads {
        let err = bridge
            .invoke("executeVoiceCommand", payload)
            .expect_err("Invalid payload must fail");
        match err {
            IpcError::InvalidArguments(msg) => {
                assert!(msg.contains("utterance"));
            }
            other => panic!("Expected InvalidArguments, got {:?}", other),
        }
    }

    // 2. All 9 snap positions via speech utterance
    let snap_commands = [
        ("snap left", "SnapWindow", "Left"),
        ("dock right", "SnapWindow", "Right"),
        ("tile top", "SnapWindow", "Top"),
        ("snap bottom", "SnapWindow", "Bottom"),
        ("snap center", "SnapWindow", "Center"),
        ("dock maximize", "SnapWindow", "Maximize"),
        ("snap minimize", "SnapWindow", "Minimize"),
        ("restore", "SnapWindow", "Restore"),
        ("fullscreen", "SnapWindow", "Fullscreen"),
    ];

    for (utterance, expected_action, expected_pos) in snap_commands {
        let res = bridge
            .invoke("executeVoiceCommand", Some(json!({ "utterance": utterance })))
            .unwrap_or_else(|e| panic!("Failed for utterance {}: {:?}", utterance, e));
        assert_eq!(res["status"], "success");
        assert_eq!(res["action"], expected_action);
        assert_eq!(res["payload"]["position"], expected_pos);
    }

    // 3. Dynamic resize speech utterances
    let res_exp = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "resize 1920 by 1080" })))
        .expect("resize explicit failed");
    assert_eq!(res_exp["action"], "ResizeWindow");
    assert_eq!(res_exp["payload"]["mode"], "Explicit");
    assert_eq!(res_exp["payload"]["width"], 1920);
    assert_eq!(res_exp["payload"]["height"], 1080);

    let res_pct = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "resize window 75%" })))
        .expect("resize percentage failed");
    assert_eq!(res_pct["action"], "ResizeWindow");
    assert_eq!(res_pct["payload"]["mode"], "Percentage");
    assert_eq!(res_pct["payload"]["percentage"], 75);

    let res_delta = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "make window wider by 150" })))
        .expect("resize delta failed");
    assert_eq!(res_delta["action"], "ResizeWindow");
    assert_eq!(res_delta["payload"]["mode"], "Delta");
    assert_eq!(res_delta["payload"]["dimension"], "Wider");
    assert_eq!(res_delta["payload"]["delta_pixels"], 150);

    // 4. Window cycling and UI navigation
    let res_cycle = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "next window" })))
        .expect("cycle failed");
    assert_eq!(res_cycle["action"], "CycleWindow");
    assert_eq!(res_cycle["payload"]["direction"], "Next");

    let res_nav = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "next field" })))
        .expect("nav failed");
    assert_eq!(res_nav["action"], "NavigateUi");
    assert_eq!(res_nav["payload"]["element"], "NextField");

    // 5. Wake word only produces Unknown with empty query
    let res_wake = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "hey airctrl hey airctrl" })))
        .expect("wake only failed");
    assert_eq!(res_wake["action"], "Unknown");
    assert_eq!(res_wake["payload"]["query"], "");

    // 6. Huge 50KB utterance does not crash
    let huge_str = "a".repeat(50_000);
    let res_huge = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": huge_str })))
        .expect("huge utterance must succeed safely");
    assert_eq!(res_huge["status"], "success");
    assert_eq!(res_huge["action"], "Unknown");
}

#[test]
fn test_adversarial_ipc_snap_window_validation() {
    let bridge = IpcBridge::default();

    // 1. Missing or invalid argument types
    let invalid_args = vec![
        None,
        Some(json!({})),
        Some(json!({ "position": null })),
        Some(json!({ "position": 123 })),
        Some(json!({ "position": false })),
        Some(json!({ "position": ["Left"] })),
        Some(json!({ "position": "" })),
        Some(json!({ "position": "diagonal" })),
        Some(json!({ "position": "up_and_away" })),
    ];

    for args in invalid_args {
        let err = bridge.invoke("snapWindow", args).expect_err("Invalid snap position must error");
        match err {
            IpcError::InvalidArguments(_) => {}
            other => panic!("Expected InvalidArguments, got {:?}", other),
        }
    }

    // 2. All 9 valid positions with case insensitivity
    let valid_positions = [
        ("Left", 0, 0, 960, 1080),
        ("rIgHt", 960, 0, 960, 1080),
        ("TOP", 0, 0, 1920, 540),
        ("bottom", 0, 540, 1920, 540),
        ("Center", 288, 162, 1344, 756),
        ("MAXIMIZE", 0, 0, 1920, 1080),
        ("minimize", 0, 0, 0, 0),
        ("Restore", 100, 100, 1024, 768),
        ("fullscreen", 0, 0, 1920, 1080),
    ];

    for (pos_str, exp_x, exp_y, exp_w, exp_h) in valid_positions {
        let res = bridge
            .invoke("snapWindow", Some(json!({ "position": pos_str })))
            .unwrap_or_else(|e| panic!("snapWindow failed for {}: {:?}", pos_str, e));

        assert_eq!(res["status"], "success");
        assert_eq!(res["bounds"]["x"], exp_x);
        assert_eq!(res["bounds"]["y"], exp_y);
        assert_eq!(res["bounds"]["width"], exp_w);
        assert_eq!(res["bounds"]["height"], exp_h);
    }
}

#[test]
fn test_adversarial_ipc_resize_window_modes_and_boundaries() {
    let bridge = IpcBridge::default();

    // 1. Invalid modes and dimensions
    let invalid_cases = vec![
        json!({ "mode": "hyperdimensional" }),
        json!({ "dimension": "fourth_dimension" }),
        json!({ "mode": "quantum" }),
    ];

    for payload in invalid_cases {
        let err = bridge
            .invoke("resizeWindow", Some(payload))
            .expect_err("Invalid resize args must fail");
        match err {
            IpcError::InvalidArguments(_) => {}
            other => panic!("Expected InvalidArguments, got {:?}", other),
        }
    }

    // 2. Explicit resize floor clamping (200px floor)
    let res_floor = bridge
        .invoke("resizeWindow", Some(json!({ "width": 10, "height": 20 })))
        .expect("resize floor failed");
    assert_eq!(res_floor["status"], "success");
    assert_eq!(res_floor["bounds"]["width"], 200);
    assert_eq!(res_floor["bounds"]["height"], 200);

    // 3. Explicit resize ceiling clamping (1920x1080 ceiling)
    let res_ceiling = bridge
        .invoke("resizeWindow", Some(json!({ "width": 100000, "height": 100000 })))
        .expect("resize ceiling failed");
    assert_eq!(res_ceiling["bounds"]["width"], 1920);
    assert_eq!(res_ceiling["bounds"]["height"], 1080);

    // 4. Percentage mode clamping (10% floor, 100% ceiling)
    let res_pct_low = bridge
        .invoke("resizeWindow", Some(json!({ "mode": "percentage", "percentage": 0 })))
        .expect("resize pct 0 failed");
    assert_eq!(res_pct_low["bounds"]["width"], 192);
    assert_eq!(res_pct_low["bounds"]["height"], 108);

    let res_pct_high = bridge
        .invoke("resizeWindow", Some(json!({ "mode": "percentage", "percentage": 500 })))
        .expect("resize pct 500 failed");
    assert_eq!(res_pct_high["bounds"]["width"], 1920);
    assert_eq!(res_pct_high["bounds"]["height"], 1080);

    // 5. Delta mode in all 6 directions
    for dim in &["larger", "smaller", "wider", "narrower", "taller", "shorter"] {
        let res = bridge
            .invoke("resizeWindow", Some(json!({ "dimension": dim, "delta_pixels": 50 })))
            .unwrap_or_else(|e| panic!("Delta failed for {}: {:?}", dim, e));
        assert_eq!(res["status"], "success");
    }

    // 6. Massive delta shrink down to safety floor
    let res_shrink = bridge
        .invoke("resizeWindow", Some(json!({ "dimension": "smaller", "delta_pixels": 50000 })))
        .expect("massive shrink failed");
    assert_eq!(res_shrink["bounds"]["width"], 200);
    assert_eq!(res_shrink["bounds"]["height"], 200);
}

#[test]
fn test_adversarial_ipc_cycle_window_and_navigate_ui_validation() {
    let bridge = IpcBridge::default();

    // 1. CycleWindow default
    let res_def = bridge.invoke("cycleWindow", None).expect("cycle default failed");
    assert_eq!(res_def["status"], "success");
    assert_eq!(res_def["direction"], "Next");

    // 2. CycleWindow valid case insensitivity
    for dir in &["next", "NEXT", "Previous", "prev", "PREV"] {
        let res = bridge
            .invoke("cycleWindow", Some(json!({ "direction": dir })))
            .unwrap_or_else(|e| panic!("cycleWindow failed for {}: {:?}", dir, e));
        assert_eq!(res["status"], "success");
    }

    // 3. CycleWindow invalid direction
    let err_cycle = bridge
        .invoke("cycleWindow", Some(json!({ "direction": "diagonal" })))
        .expect_err("invalid cycle direction must fail");
    match err_cycle {
        IpcError::InvalidArguments(msg) => assert!(msg.contains("diagonal")),
        other => panic!("Expected InvalidArguments, got {:?}", other),
    }

    // 4. NavigateUi valid elements
    let valid_elements = [
        "NextField", "next_field", "PrevField", "prev_field", "Enter", "enter", "Escape", "escape", "Tab", "tab",
    ];
    for elem in valid_elements {
        let res = bridge
            .invoke("navigateUi", Some(json!({ "element": elem })))
            .unwrap_or_else(|e| panic!("navigateUi failed for {}: {:?}", elem, e));
        assert_eq!(res["status"], "success");
    }

    // 5. NavigateUi invalid elements
    let invalid_elements = ["Submit", "OkButton", "123", "", "cancel"];
    for elem in invalid_elements {
        let err = bridge
            .invoke("navigateUi", Some(json!({ "element": elem })))
            .expect_err("invalid navigateUi element must fail");
        match err {
            IpcError::InvalidArguments(_) => {}
            other => panic!("Expected InvalidArguments, got {:?}", other),
        }
    }
}

#[test]
fn test_adversarial_transactional_validation_zero_side_effects() {
    let bridge = IpcBridge::default();

    // 1. Establish known baseline state
    bridge
        .invoke("snapWindow", Some(json!({ "position": "Center" })))
        .expect("Baseline snap failed");
    let initial_bounds = *bridge.state.current_window_bounds.read();

    // 2. Subscribe to event bus
    let mut rx = bridge.state.event_bus.subscribe();

    // 3. Battery of 50 invalid requests across M9 IPC commands
    let invalid_calls = vec![
        ("snapWindow", Some(json!({ "position": "invalid_pos_1" }))),
        ("snapWindow", Some(json!({ "position": "nowhere" }))),
        ("snapWindow", Some(json!({ "position": 12345 }))),
        ("snapWindow", None),
        ("resizeWindow", Some(json!({ "mode": "invalid_mode_1" }))),
        ("resizeWindow", Some(json!({ "dimension": "invalid_dim_1" }))),
        ("cycleWindow", Some(json!({ "direction": "invalid_dir_1" }))),
        ("cycleWindow", Some(json!({ "direction": "sideways" }))),
        ("navigateUi", Some(json!({ "element": "invalid_elem_1" }))),
        ("navigateUi", Some(json!({ "element": 9999 }))),
        ("navigateUi", None),
        ("executeVoiceCommand", None),
        ("executeVoiceCommand", Some(json!({}))),
        ("executeVoiceCommand", Some(json!({ "utterance": null }))),
        ("executeVoiceCommand", Some(json!({ "utterance": 42 }))),
    ];

    for (cmd, args) in invalid_calls {
        let err = bridge.invoke(cmd, args).expect_err("Invalid call must fail");
        match err {
            IpcError::InvalidArguments(_) => {}
            other => panic!("Expected InvalidArguments for {}, got {:?}", cmd, other),
        }
    }

    // 4. Assert zero side-effects on window bounds
    let final_bounds = *bridge.state.current_window_bounds.read();
    assert_eq!(
        initial_bounds, final_bounds,
        "System state (current_window_bounds) was mutated by invalid IPC requests!"
    );

    // 5. Assert zero side-effects on event bus
    assert!(
        rx.try_recv().is_err(),
        "Spurious DomainEvent was published to event bus by invalid IPC request!"
    );
}

#[test]
fn test_adversarial_concurrent_ipc_hammer_m9() {
    let bridge = Arc::new(IpcBridge::default());
    let mut handles = Vec::new();

    for thread_id in 0..10 {
        let b = Arc::clone(&bridge);
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                match (thread_id + i) % 5 {
                    0 => {
                        let _ = b.invoke(
                            "executeVoiceCommand",
                            Some(json!({ "utterance": "hey airctrl snap left" })),
                        );
                    }
                    1 => {
                        let _ = b.invoke("snapWindow", Some(json!({ "position": "Center" })));
                    }
                    2 => {
                        let _ = b.invoke(
                            "resizeWindow",
                            Some(json!({ "mode": "percentage", "percentage": 50 })),
                        );
                    }
                    3 => {
                        let _ = b.invoke("cycleWindow", Some(json!({ "direction": "Next" })));
                    }
                    4 => {
                        let _ = b.invoke("navigateUi", Some(json!({ "element": "NextField" })));
                    }
                    _ => unreachable!(),
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("Concurrent worker thread panicked");
    }

    // Ensure state is intact and valid after hammer
    let bounds = *bridge.state.current_window_bounds.read();
    assert!(bounds.width >= 200);
    assert!(bounds.height >= 200);
}
