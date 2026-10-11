use airctrl_shell::{IpcBridge, IpcError};
use serde_json::json;

#[test]
fn test_ipc_start_recording_happy_path() {
    let bridge = IpcBridge::default();
    let res = bridge
        .invoke("startRecording", None)
        .expect("startRecording failed");
    assert_eq!(res["status"], "RecordingStarted");
}

#[test]
fn test_ipc_start_recording_empty_dict() {
    let bridge = IpcBridge::default();
    let res = bridge
        .invoke("startRecording", Some(json!({})))
        .expect("startRecording failed");
    assert_eq!(res["status"], "RecordingStarted");
}

#[test]
fn test_ipc_stop_recording_returns_transcription() {
    let bridge = IpcBridge::default();
    bridge.invoke("startRecording", None).unwrap();
    let res = bridge
        .invoke("stopRecording", None)
        .expect("stopRecording failed");
    assert_eq!(res["status"], "TranscriptionReady");
    assert_eq!(res["text"], "airctrl speech transcription result");
}

#[test]
fn test_ipc_stop_recording_empty_buffer_returns_error() {
    let bridge = IpcBridge::default();
    let err = bridge
        .invoke("stopRecording", None)
        .expect_err("stopRecording without recorded audio must fail");
    assert!(matches!(err, IpcError::ExecutionFailed(_)));
}

#[test]
fn test_ipc_custom_stt_engine_dependency_injection() {
    use airctrl_shell::AppState;
    use airctrl_stt::{ModelInfo, Result as SttResult, SpeechToTextEngine, TranscriptionResult};
    use std::sync::Arc;

    struct InjectedEngine;
    impl SpeechToTextEngine for InjectedEngine {
        fn transcribe(&self, samples: &[f32]) -> SttResult<TranscriptionResult> {
            Ok(TranscriptionResult {
                text: "custom injected transcription".to_string(),
                duration_ms: 5,
                confidence: 0.99,
                sample_count: samples.len(),
                language: Some("en".to_string()),
                segments: vec![],
            })
        }
        fn is_loaded(&self) -> bool {
            true
        }
        fn model_info(&self) -> Option<&ModelInfo> {
            None
        }
    }

    let state = Arc::new(AppState::with_engine(Box::new(InjectedEngine)));
    let bridge = IpcBridge::new(state);
    bridge.invoke("startRecording", None).unwrap();
    let res = bridge
        .invoke("stopRecording", None)
        .expect("stopRecording failed");
    assert_eq!(res["status"], "TranscriptionReady");
    assert_eq!(res["text"], "custom injected transcription");
}

#[test]
fn test_ipc_trigger_action_with_payload() {
    let bridge = IpcBridge::default();
    let payload = json!({ "action": "WindowSnap" });
    let res = bridge
        .invoke("triggerAction", Some(payload))
        .expect("triggerAction failed");
    assert_eq!(res["success"], true);
    assert_eq!(res["action"], "WindowSnap");
}

#[test]
fn test_ipc_toggle_radial_dial() {
    let bridge = IpcBridge::default();
    // First toggle: hidden -> visible
    let res1 = bridge
        .invoke("toggleRadialDial", None)
        .expect("toggleRadialDial failed");
    assert_eq!(res1["dial_visible"], true);

    // Second toggle: visible -> hidden
    let res2 = bridge
        .invoke("toggleRadialDial", None)
        .expect("toggleRadialDial failed");
    assert_eq!(res2["dial_visible"], false);
}

#[test]
fn test_ipc_unknown_command_returns_error() {
    let bridge = IpcBridge::default();
    let err = bridge
        .invoke("unknownCommand", None)
        .expect_err("should have failed");
    assert!(matches!(err, IpcError::CommandNotFound(_)));
}

#[test]
fn test_ipc_empty_command_returns_error() {
    let bridge = IpcBridge::default();
    let err = bridge.invoke("", None).expect_err("should have failed");
    assert!(matches!(err, IpcError::EmptyCommand));
}

#[test]
fn test_ipc_deeply_nested_payload() {
    let bridge = IpcBridge::default();
    let deep_payload = json!({
        "action": "Nested",
        "nested": {
            "level1": {
                "level2": [1, 2, 3]
            }
        }
    });
    let res = bridge
        .invoke("triggerAction", Some(deep_payload))
        .expect("triggerAction deep failed");
    assert_eq!(res["success"], true);
    assert_eq!(res["action"], "Nested");
}

#[test]
fn test_ipc_rapid_burst_50_calls() {
    let bridge = IpcBridge::default();
    for _ in 0..50 {
        let res = bridge
            .invoke("startRecording", None)
            .expect("burst call failed");
        assert_eq!(res["status"], "RecordingStarted");
    }
}

#[test]
fn test_ipc_settings_lifecycle() {
    let bridge = IpcBridge::default();

    // 1. Get initial settings
    let initial = bridge.invoke("getSettings", None).unwrap();
    assert_eq!(initial["hotkey_stt"], "Ctrl+Spacebar");
    assert_eq!(initial["hotkey_dial"], "Ctrl+Alt");
    assert_eq!(initial["camera_auto_kill"], true);

    // 2. Update settings
    let update_req = json!({
        "hotkey_stt": "Ctrl+Shift+R",
        "radial_dial_sectors": 6,
        "camera_auto_kill": false
    });
    let update_res = bridge
        .invoke("updateSettings", Some(update_req))
        .unwrap();
    assert_eq!(update_res["success"], true);

    // 3. Verify updated state
    let updated = bridge.invoke("getSettings", None).unwrap();
    assert_eq!(updated["hotkey_stt"], "Ctrl+Shift+R");
    assert_eq!(updated["camera_auto_kill"], false);
}

#[test]
fn test_ipc_settings_validation_invalid_hotkey() {
    let bridge = IpcBridge::default();
    let invalid_req = json!({
        "hotkey_stt": "Alt+Shift+X"
    });
    let err = bridge
        .invoke("updateSettings", Some(invalid_req))
        .expect_err("should reject hotkey not starting with Ctrl+");
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_ipc_settings_validation_invalid_sectors() {
    let bridge = IpcBridge::default();
    let invalid_req = json!({
        "radial_dial_sectors": 3
    });
    let err = bridge
        .invoke("updateSettings", Some(invalid_req))
        .expect_err("should reject sectors < 4");
    assert!(matches!(err, IpcError::InvalidArguments(_)));

    let invalid_req_high = json!({
        "radial_dial_sectors": 10
    });
    let err_high = bridge
        .invoke("updateSettings", Some(invalid_req_high))
        .expect_err("should reject sectors > 8");
    assert!(matches!(err_high, IpcError::InvalidArguments(_)));
}

#[test]
fn test_ipc_meeting_and_kill_switch() {
    let bridge = IpcBridge::default();

    // Query meeting status
    let status = bridge.invoke("getMeetingStatus", None).unwrap();
    assert!(status["status"] == "Active" || status["status"] == "Inactive");

    // Trigger camera kill switch
    let kill_res = bridge.invoke("triggerCameraKillSwitch", None).unwrap();
    assert_eq!(kill_res["success"], true);
    assert_eq!(kill_res["camera_handle_closed"], true);
}

#[test]
fn test_ipc_update_settings_synchronizes_dial_overlay_sector_count() {
    let bridge = IpcBridge::default();
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), 6);

    for valid_sectors in [4, 5, 6, 7, 8] {
        let req = json!({ "radial_dial_sectors": valid_sectors });
        let res = bridge.invoke("updateSettings", Some(req)).unwrap();
        assert_eq!(res["success"], true);
        assert_eq!(res["settings"]["radial_dial_sectors"], valid_sectors);
        assert_eq!(bridge.state.config.read().radial_dial_sectors, valid_sectors);
        assert_eq!(
            bridge.state.dial_overlay.read().sector_count(),
            valid_sectors
        );
    }
}

#[test]
fn test_ipc_settings_type_validation_hotkey_stt_invalid_types() {
    let bridge = IpcBridge::default();
    let invalid_values = vec![
        json!(123),
        json!(true),
        json!(["Ctrl+Spacebar"]),
        json!({ "key": "Space" }),
        json!(null),
        json!("Alt+Spacebar"),
        json!(""),
    ];
    for val in invalid_values {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "hotkey_stt": val })))
            .expect_err("should reject invalid hotkey_stt type or prefix");
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }
}

#[test]
fn test_ipc_settings_type_validation_hotkey_dial_invalid_types() {
    let bridge = IpcBridge::default();
    let invalid_values = vec![
        json!(999),
        json!(false),
        json!(["Ctrl+Alt"]),
        json!({ "dial": true }),
        json!(null),
        json!("Command+Alt"),
        json!(""),
    ];
    for val in invalid_values {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "hotkey_dial": val })))
            .expect_err("should reject invalid hotkey_dial type or prefix");
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }
}

#[test]
fn test_ipc_settings_type_validation_camera_auto_kill_invalid_types() {
    let bridge = IpcBridge::default();
    let invalid_values = vec![
        json!("true"),
        json!("1"),
        json!(1),
        json!(0),
        json!([true]),
        json!({}),
        json!(null),
    ];
    for val in invalid_values {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "camera_auto_kill": val })))
            .expect_err("should reject non-boolean camera_auto_kill");
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }
}

#[test]
fn test_ipc_settings_type_validation_compliance_audio_invalid_types() {
    let bridge = IpcBridge::default();
    let invalid_values = vec![
        json!("false"),
        json!("0"),
        json!(1),
        json!(0),
        json!([false]),
        json!({}),
        json!(null),
    ];
    for val in invalid_values {
        let err = bridge
            .invoke(
                "updateSettings",
                Some(json!({ "compliance_audio_enabled": val })),
            )
            .expect_err("should reject non-boolean compliance_audio_enabled");
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }
}

#[test]
fn test_ipc_settings_type_validation_radial_dial_sectors_invalid_types() {
    let bridge = IpcBridge::default();
    let invalid_values = vec![
        json!("invalid"),
        json!("6"),
        json!(true),
        json!(5.5),
        json!(-1),
        json!(-10),
        json!(0),
        json!(1),
        json!(3),
        json!(9),
        json!(10),
        json!(100),
        json!([6]),
        json!({ "count": 6 }),
        json!(null),
    ];
    for val in invalid_values {
        let err = bridge
            .invoke(
                "updateSettings",
                Some(json!({ "radial_dial_sectors": val })),
            )
            .expect_err("should reject invalid radial_dial_sectors");
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }
}

#[test]
fn test_ipc_settings_type_validation_non_object_root() {
    let bridge = IpcBridge::default();
    let invalid_roots = vec![json!("invalid_payload"), json!(12345), json!([1, 2, 3]), json!(true)];
    for root in invalid_roots {
        let err = bridge
            .invoke("updateSettings", Some(root))
            .expect_err("should reject non-object root arguments");
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }
}

#[test]
fn test_ipc_settings_atomic_rollback_on_partial_failure() {
    let bridge = IpcBridge::default();
    let initial_settings = bridge.invoke("getSettings", None).unwrap();
    let initial_hotkey = initial_settings["hotkey_stt"].as_str().unwrap().to_string();

    let mixed_req = json!({
        "hotkey_stt": "Ctrl+Shift+F12",
        "radial_dial_sectors": "not_a_number"
    });
    let err = bridge
        .invoke("updateSettings", Some(mixed_req))
        .expect_err("should reject request with invalid radial_dial_sectors");
    assert!(matches!(err, IpcError::InvalidArguments(_)));

    // Verify that hotkey_stt was NOT modified (transactional rollback / atomic guard)
    let current_settings = bridge.invoke("getSettings", None).unwrap();
    assert_eq!(current_settings["hotkey_stt"], initial_hotkey);
}

#[test]
fn test_ipc_get_and_refresh_cataloged_apps() {
    let bridge = IpcBridge::default();

    let res = bridge.invoke("getCatalogedApps", None).expect("getCatalogedApps failed");
    assert!(res["apps"].is_array(), "apps must be an array");
    let apps = res["apps"].as_array().unwrap();
    assert!(!apps.is_empty(), "apps array must not be empty");

    let res_refresh = bridge.invoke("refreshCatalogedApps", None).expect("refreshCatalogedApps failed");
    assert!(res_refresh["apps"].is_array());
}

#[test]
fn test_ipc_launch_app_validation() {
    let bridge = IpcBridge::default();

    // Rejects missing path/name
    let err = bridge.invoke("launchApp", Some(json!({}))).expect_err("Must reject empty payload");
    assert!(matches!(err, IpcError::InvalidArguments(_)));

    // Rejects non-string path
    let err2 = bridge.invoke("launchApp", Some(json!({ "path": 123 }))).expect_err("Must reject non-string");
    assert!(matches!(err2, IpcError::InvalidArguments(_)));
}

