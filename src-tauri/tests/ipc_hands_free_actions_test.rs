use airctrl_shell::ipc::IpcBridge;
use serde_json::json;

#[test]
fn test_ipc_execute_voice_command_snap_window() {
    let bridge = IpcBridge::default();

    let res = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "snap left" })))
        .expect("executeVoiceCommand should succeed");

    assert_eq!(res["status"], "success");
    assert_eq!(res["action"], "SnapWindow");
    assert_eq!(res["payload"]["position"], "Left");
}

#[test]
fn test_ipc_execute_voice_command_with_wake_word() {
    let bridge = IpcBridge::default();

    let res = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "hey airctrl resize window 80%" })))
        .expect("executeVoiceCommand with wake prefix should succeed");

    assert_eq!(res["status"], "success");
    assert_eq!(res["action"], "ResizeWindow");
    assert_eq!(res["payload"]["percentage"], 80);
}

#[test]
fn test_ipc_execute_voice_command_wake_word_only_returns_unknown() {
    let bridge = IpcBridge::default();

    let res = bridge
        .invoke("executeVoiceCommand", Some(json!({ "utterance": "hey airctrl" })))
        .expect("wake word only should return success with Unknown action");

    assert_eq!(res["status"], "success");
    assert_eq!(res["action"], "Unknown");
    assert_eq!(res["payload"]["query"], "");
}

#[test]
fn test_ipc_execute_voice_command_missing_utterance_errors() {
    let bridge = IpcBridge::default();

    let err = bridge
        .invoke("executeVoiceCommand", Some(json!({})))
        .unwrap_err();

    match err {
        airctrl_shell::ipc::IpcError::InvalidArguments(_) => {}
        other => panic!("Expected InvalidArguments, got {:?}", other),
    }
}

#[test]
fn test_ipc_snap_window_direct() {
    let bridge = IpcBridge::default();

    let res = bridge
        .invoke("snapWindow", Some(json!({ "position": "Right" })))
        .expect("snapWindow should succeed");

    assert_eq!(res["status"], "success");
    assert_eq!(res["bounds"]["x"], 960);
    assert_eq!(res["bounds"]["y"], 0);
    assert_eq!(res["bounds"]["width"], 960);
    assert_eq!(res["bounds"]["height"], 1080);
}

#[test]
fn test_ipc_snap_window_invalid_position_errors() {
    let bridge = IpcBridge::default();

    let err = bridge
        .invoke("snapWindow", Some(json!({ "position": "NonExistentCorner" })))
        .unwrap_err();

    match err {
        airctrl_shell::ipc::IpcError::InvalidArguments(_) => {}
        other => panic!("Expected InvalidArguments, got {:?}", other),
    }
}

#[test]
fn test_ipc_resize_window_delta_and_percentage() {
    let bridge = IpcBridge::default();

    // Resize delta larger
    let res_delta = bridge
        .invoke("resizeWindow", Some(json!({ "dimension": "larger" })))
        .expect("resizeWindow delta should succeed");

    assert_eq!(res_delta["status"], "success");
    assert!(res_delta["bounds"]["width"].as_u64().unwrap() >= 1280);

    // Resize percentage 50%
    let res_pct = bridge
        .invoke("resizeWindow", Some(json!({ "mode": "percentage", "percentage": 50 })))
        .expect("resizeWindow percentage should succeed");

    assert_eq!(res_pct["status"], "success");
    assert_eq!(res_pct["bounds"]["width"], 960);
    assert_eq!(res_pct["bounds"]["height"], 540);
}

#[test]
fn test_ipc_cycle_window() {
    let bridge = IpcBridge::default();

    let res_next = bridge
        .invoke("cycleWindow", Some(json!({ "direction": "Next" })))
        .expect("cycleWindow Next should succeed");

    assert_eq!(res_next["status"], "success");
    assert_eq!(res_next["direction"], "Next");

    let res_prev = bridge
        .invoke("cycleWindow", Some(json!({ "direction": "Previous" })))
        .expect("cycleWindow Previous should succeed");

    assert_eq!(res_prev["status"], "success");
    assert_eq!(res_prev["direction"], "Previous");
}

#[test]
fn test_ipc_navigate_ui() {
    let bridge = IpcBridge::default();

    for elem in &["NextField", "PrevField", "Enter", "Escape", "Tab"] {
        let res = bridge
            .invoke("navigateUi", Some(json!({ "element": elem })))
            .unwrap_or_else(|e| panic!("navigateUi failed for {}: {:?}", elem, e));

        assert_eq!(res["status"], "success");
        assert_eq!(res["element"], *elem);
    }

    // Invalid target element
    let err = bridge
        .invoke("navigateUi", Some(json!({ "element": "UnknownElement" })))
        .unwrap_err();

    match err {
        airctrl_shell::ipc::IpcError::InvalidArguments(_) => {}
        other => panic!("Expected InvalidArguments, got {:?}", other),
    }
}
