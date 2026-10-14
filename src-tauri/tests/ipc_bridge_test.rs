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
        fn model_info(&self) -> Option<ModelInfo> {
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

#[test]
fn test_ipc_toggle_stt_overlay() {
    let bridge = IpcBridge::default();

    let res1 = bridge.invoke("toggleSttOverlay", None).expect("toggleSttOverlay failed");
    assert_eq!(res1["stt_overlay_visible"], true);

    let res2 = bridge.invoke("toggleSttOverlay", None).expect("toggleSttOverlay failed");
    assert_eq!(res2["stt_overlay_visible"], false);
}

#[test]
fn test_ipc_toggle_command_palette() {
    let bridge = IpcBridge::default();

    let res1 = bridge.invoke("toggleCommandPalette", None).expect("toggleCommandPalette failed");
    assert_eq!(res1["command_palette_visible"], true);

    let res2 = bridge.invoke("toggleCommandPalette", None).expect("toggleCommandPalette failed");
    assert_eq!(res2["command_palette_visible"], false);
}

#[test]
fn test_ipc_trigger_action_parameter_first() {
    let bridge = IpcBridge::default();

    // 1. open slack
    let res_open = bridge
        .invoke("triggerAction", Some(json!({ "action": "open slack" })))
        .expect("triggerAction open slack failed");
    assert_eq!(res_open["success"], true);

    // 2. close slack
    let res_close = bridge
        .invoke("triggerAction", Some(json!({ "action": "close slack" })))
        .expect("triggerAction close slack failed");
    assert_eq!(res_close["success"], true);

    // 3. center slack
    let res_center = bridge
        .invoke("triggerAction", Some(json!({ "action": "center slack" })))
        .expect("triggerAction center slack failed");
    assert_eq!(res_center["success"], true);

    // 4. center window
    let res_win_center = bridge
        .invoke("triggerAction", Some(json!({ "action": "center window" })))
        .expect("triggerAction center window failed");
    assert_eq!(res_win_center["success"], true);
}

#[test]
fn test_ipc_inject_text_and_auto_inject() {
    let bridge = IpcBridge::default();

    // 1. Direct injectText command
    let res_inject = bridge
        .invoke("injectText", Some(json!({ "text": "Hello world from Handy dictation" })))
        .expect("injectText failed");
    assert_eq!(res_inject["success"], true);
    assert_eq!(res_inject["text_length"], 32);

    // 2. stopRecording with auto_inject
    bridge.invoke("startRecording", None).expect("startRecording failed");
    let res_stop = bridge
        .invoke("stopRecording", Some(json!({ "auto_inject": true })))
        .expect("stopRecording with auto_inject failed");
    assert_eq!(res_stop["status"], "TranscriptionReady");
    assert!(!res_stop["injected"].is_null());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ipc_loop_status_and_lifecycle() {
    let bridge = IpcBridge::default();

    // 1. Initial loop status: not running
    let status = bridge
        .invoke("getLoopStatus", None)
        .expect("getLoopStatus failed");
    assert_eq!(status["all_healthy"], false);
    assert_eq!(status["audio_vad"]["is_running"], false);

    // 2. Start worker loops
    let start_res = bridge
        .invoke("startWorkerLoops", None)
        .expect("startWorkerLoops failed");
    assert_eq!(start_res["success"], true);
    assert_eq!(start_res["status"], "WorkerLoopsStarted");

    // Let loops initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 3. Inspect running status
    let running_status = bridge
        .invoke("getLoopStatus", None)
        .expect("getLoopStatus after start failed");
    assert_eq!(running_status["all_healthy"], true);
    assert_eq!(running_status["audio_vad"]["is_running"], true);
    assert_eq!(running_status["vision_gesture"]["is_running"], true);
    assert_eq!(running_status["meeting_watcher"]["is_running"], true);
    assert_eq!(running_status["global_hotkey"]["is_running"], true);

    // 4. Stop worker loops
    let stop_res = bridge
        .invoke("stopWorkerLoops", None)
        .expect("stopWorkerLoops failed");
    assert_eq!(stop_res["success"], true);
    assert_eq!(stop_res["status"], "WorkerLoopsStopped");

    // 5. Inspect stopped status
    let stopped_status = bridge
        .invoke("getLoopStatus", None)
        .expect("getLoopStatus after stop failed");
    assert_eq!(stopped_status["all_healthy"], false);
    assert_eq!(stopped_status["audio_vad"]["is_running"], false);
}

#[test]
fn test_ipc_get_dag_status() {
    let bridge = IpcBridge::default();

    let dag_status = bridge
        .invoke("getDagStatus", None)
        .expect("getDagStatus failed");
    assert_eq!(dag_status["active"], true);
    assert_eq!(dag_status["capacity"], 256);
    assert!(dag_status["stages"].is_array());
}

#[test]
fn test_ipc_zero_mock_audio_empty_buffer_error() {
    let bridge = IpcBridge::default();

    // Directly calling stopRecording on empty buffer must fail with ZeroSamples error
    let err = bridge
        .invoke("stopRecording", None)
        .expect_err("stopRecording on empty buffer must fail");
    match err {
        IpcError::ExecutionFailed(msg) => {
            assert!(
                msg.contains("ZeroSamples") || msg.contains("No audio samples recorded"),
                "Expected ZeroSamples in error message, got: {}",
                msg
            );
        }
        other => panic!("Expected ExecutionFailed, got {:?}", other),
    }
}

#[test]
fn test_ipc_overlay_position_transitions() {
    let bridge = IpcBridge::default();

    // 1. Valid transitions: Top, Center, None, Bottom
    for pos in ["Top", "Center", "None", "Bottom"] {
        let res = bridge
            .invoke("setOverlayPosition", Some(json!({ "position": pos })))
            .expect("setOverlayPosition failed");
        assert_eq!(res["position"], pos);
        assert_eq!(res["status"], "success");
        if pos == "None" {
            assert_eq!(res["bounds"]["visible"], false);
            assert_eq!(res["bounds"]["width"], 0);
        } else {
            assert_eq!(res["bounds"]["visible"], true);
        }

        let get_res = bridge
            .invoke("getOverlayPosition", None)
            .expect("getOverlayPosition failed");
        assert_eq!(get_res["position"], pos);
    }

    // 2. Invalid position string must return InvalidArguments
    let err = bridge
        .invoke("setOverlayPosition", Some(json!({ "position": "InvalidCorner" })))
        .expect_err("Invalid position must fail");
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_ipc_models_and_recording_modes() {
    let bridge = IpcBridge::default();

    // 1. List models
    let list_res = bridge.invoke("listModels", None).unwrap();
    assert_eq!(list_res["status"], "success");
    let models = list_res["models"].as_array().unwrap();
    assert!(models.iter().any(|m| m.as_str() == Some("whisper-turbo.en")));
    assert!(models.iter().any(|m| m.as_str() == Some("parakeet-unified-en-0.6b-q8_0")));

    // 2. Select model
    let sel_res = bridge
        .invoke("selectModel", Some(json!({ "model": "parakeet-unified-en-0.6b-q8_0" })))
        .unwrap();
    assert_eq!(sel_res["selected_model"], "parakeet-unified-en-0.6b-q8_0");

    // 3. Set & get recording modes across all 4 variants
    for mode in ["SmartDualMode", "HoldOnly", "ToggleOnly", "ContinuousHandsFree"] {
        let set_res = bridge
            .invoke("setRecordingMode", Some(json!({ "mode": mode })))
            .unwrap();
        assert_eq!(set_res["mode"], mode);

        let get_res = bridge.invoke("getRecordingMode", None).unwrap();
        assert_eq!(get_res["recording_mode"], mode);
    }
}

#[test]
fn test_ipc_dictionary_engine_and_post_processing() {
    let bridge = IpcBridge::default();

    // 1. Update dictionary with literal and regex rules
    let update_res = bridge
        .invoke(
            "updateDictionary",
            Some(json!({
                "rules": [
                    { "trigger": "air control", "replacement": "airctrl", "is_regex": false, "preserve_case": true },
                    { "trigger": "ty sun", "replacement": "Tyson", "is_regex": false, "preserve_case": true }
                ]
            })),
        )
        .unwrap();
    assert_eq!(update_res["total_rules"], 2);

    // 2. Get dictionary
    let get_res = bridge.invoke("getDictionary", None).unwrap();
    assert_eq!(get_res["rules"].as_array().unwrap().len(), 2);

    // 3. Toggle post-processing
    let toggle_res = bridge.invoke("togglePostProcessing", None).unwrap();
    assert!(toggle_res["post_processing_enabled"].is_boolean());
}

#[test]
fn test_ipc_transcript_history_and_search() {
    use airctrl_core::types::TranscriptItem;
    let bridge = IpcBridge::default();

    // 1. Clear transcripts
    bridge.invoke("clearTranscripts", None).unwrap();

    // 2. Seed transcripts into state
    let item1 = TranscriptItem {
        id: "txn_1".to_string(),
        timestamp: 1000.0,
        text: "Standup meeting update on rust".to_string(),
        raw_text: "standup meeting update on rust".to_string(),
        duration_ms: 3500,
        model_used: "whisper-base.en".to_string(),
        confidence: 0.95,
        session_id: "default_session".to_string(),
        target_app: Some("slack".to_string()),
        target_title: Some("Standup".to_string()),
        injected: true,
        starred: false,
    };
    let item2 = TranscriptItem {
        id: "txn_2".to_string(),
        timestamp: 2000.0,
        text: "Fixing compiler errors in tauri".to_string(),
        raw_text: "fixing compiler errors in tauri".to_string(),
        duration_ms: 2200,
        model_used: "whisper-turbo.en".to_string(),
        confidence: 0.98,
        session_id: "default_session".to_string(),
        target_app: Some("code".to_string()),
        target_title: Some("VS Code".to_string()),
        injected: true,
        starred: false,
    };
    bridge.state.transcript_store.store(item1).unwrap();
    bridge.state.transcript_store.store(item2).unwrap();

    // 3. Search all transcripts (empty query returns reverse-chronological)
    let search_all = bridge
        .invoke("searchTranscripts", Some(json!({ "query": "" })))
        .unwrap();
    assert_eq!(search_all["count"], 2);
    let results = search_all["transcripts"].as_array().unwrap();
    assert_eq!(results[0]["id"], "txn_2"); // Most recent first
    assert_eq!(results[1]["id"], "txn_1");

    // 4. Search by keyword
    let search_rust = bridge
        .invoke("searchTranscripts", Some(json!({ "query": "rust" })))
        .unwrap();
    assert_eq!(search_rust["count"], 1);
    assert_eq!(search_rust["transcripts"][0]["id"], "txn_1");

    // 5. Search by model filter
    let search_model = bridge
        .invoke("searchTranscripts", Some(json!({ "query": "", "model": "turbo" })))
        .unwrap();
    assert_eq!(search_model["count"], 1);
    assert_eq!(search_model["transcripts"][0]["id"], "txn_2");

    // 6. Clear transcripts
    bridge.invoke("clearTranscripts", None).unwrap();
    let search_empty = bridge.invoke("searchTranscripts", None).unwrap();
    assert_eq!(search_empty["count"], 0);
}

#[test]
fn test_ipc_debug_drawer_toggle_and_status() {
    let bridge = IpcBridge::default();

    let initial = bridge.invoke("getDebugDrawerStatus", None).unwrap();
    assert_eq!(initial["debug_drawer_visible"], false);

    let toggled = bridge.invoke("toggleDebugDrawer", None).unwrap();
    assert_eq!(toggled["debug_drawer_visible"], true);

    let status = bridge.invoke("getDebugDrawerStatus", None).unwrap();
    assert_eq!(status["debug_drawer_visible"], true);
}




