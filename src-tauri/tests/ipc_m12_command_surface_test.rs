use airctrl_shell::ipc::{IpcBridge, IpcError};
use serde_json::json;

// ============================================================================
// Milestone M12: R1 Handy Command Aliases & Verification
// ============================================================================

#[test]
fn test_m12_discover_whisper_models_aliases() {
    let bridge = IpcBridge::default();

    // Test camelCase alias
    let res_camel = bridge
        .invoke("discoverWhisperModels", None)
        .expect("discoverWhisperModels should succeed");
    assert_eq!(res_camel["status"], "success");
    let models_camel = res_camel["models"].as_array().expect("models must be array");
    assert!(models_camel.iter().any(|m| m.as_str() == Some("whisper-tiny.en")));

    // Test snake_case alias
    let res_snake = bridge
        .invoke("discover_whisper_models", None)
        .expect("discover_whisper_models should succeed");
    assert_eq!(res_snake["status"], "success");
    let models_snake = res_snake["models"].as_array().expect("models must be array");
    assert_eq!(models_camel.len(), models_snake.len());
}

#[test]
fn test_m12_load_whisper_model_and_get_active_model_aliases() {
    let bridge = IpcBridge::default();

    // Load model using camelCase alias
    let res_load = bridge
        .invoke("loadWhisperModel", Some(json!({ "model": "whisper-medium.en" })))
        .expect("loadWhisperModel should succeed");
    assert_eq!(res_load["status"], "success");
    assert_eq!(res_load["model"], "whisper-medium.en");

    // Verify active model via getActiveModel
    let res_active = bridge
        .invoke("getActiveModel", None)
        .expect("getActiveModel should succeed");
    assert_eq!(res_active["status"], "success");
    assert_eq!(res_active["model_path"], "whisper-medium.en");

    // Load model using snake_case alias with model_path parameter
    let res_load_snake = bridge
        .invoke("load_whisper_model", Some(json!({ "model_path": "parakeet-unified-en-0.6b-q8_0" })))
        .expect("load_whisper_model should succeed");
    assert_eq!(res_load_snake["status"], "success");

    // Verify active model via get_active_model
    let res_active_snake = bridge
        .invoke("get_active_model", None)
        .expect("get_active_model should succeed");
    assert_eq!(res_active_snake["status"], "success");
    assert_eq!(res_active_snake["model_path"], "parakeet-unified-en-0.6b-q8_0");

    // Missing model argument must fail with InvalidArguments
    let err = bridge
        .invoke("loadWhisperModel", Some(json!({})))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_update_custom_dictionary_aliases() {
    let bridge = IpcBridge::default();

    // camelCase with rules array
    let res1 = bridge
        .invoke(
            "updateCustomDictionary",
            Some(json!({
                "rules": [
                    { "trigger": "AFK", "replacement": "away from keyboard", "preserve_case": true }
                ]
            })),
        )
        .expect("updateCustomDictionary should succeed");
    assert_eq!(res1["status"], "success");
    assert_eq!(res1["total_rules"], 1);

    // snake_case with custom_dictionary map
    let res2 = bridge
        .invoke(
            "update_custom_dictionary",
            Some(json!({
                "custom_dictionary": {
                    "BRB": "be right back",
                    "BTW": "by the way"
                }
            })),
        )
        .expect("update_custom_dictionary should succeed");
    assert_eq!(res2["status"], "success");
    assert_eq!(res2["total_rules"], 2);

    // Missing rules must fail transactionally
    let err = bridge
        .invoke("updateCustomDictionary", Some(json!({ "invalid_key": 123 })))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_search_transcript_history_aliases() {
    let bridge = IpcBridge::default();

    let res_camel = bridge
        .invoke("searchTranscriptHistory", Some(json!({ "query": "test" })))
        .expect("searchTranscriptHistory should succeed");
    assert_eq!(res_camel["status"], "success");

    let res_snake = bridge
        .invoke("search_transcript_history", Some(json!({ "query": "test" })))
        .expect("search_transcript_history should succeed");
    assert_eq!(res_snake["status"], "success");
}

#[test]
fn test_m12_inject_text_reliable_aliases() {
    let bridge = IpcBridge::default();

    let res_camel = bridge
        .invoke("injectTextReliable", Some(json!({ "text": "Hello M12" })))
        .expect("injectTextReliable should succeed");
    assert_eq!(res_camel["success"], true);

    let res_snake = bridge
        .invoke("inject_text_reliable", Some(json!({ "text": "Hello Snake" })))
        .expect("inject_text_reliable should succeed");
    assert_eq!(res_snake["success"], true);
}

// ============================================================================
// Milestone M12: R2 Speech-to-Action Command Aliases & Verification
// ============================================================================

#[test]
fn test_m12_execute_voice_action_aliases() {
    let bridge = IpcBridge::default();

    // camelCase
    let res1 = bridge
        .invoke("executeVoiceAction", Some(json!({ "utterance": "snap window left" })))
        .expect("executeVoiceAction should succeed");
    assert_eq!(res1["status"], "success");
    assert_eq!(res1["action"], "SnapWindow");

    // snake_case with command fallback parameter
    let res2 = bridge
        .invoke("execute_voice_action", Some(json!({ "command": "resize window 80%" })))
        .expect("execute_voice_action should succeed");
    assert_eq!(res2["status"], "success");
    assert_eq!(res2["action"], "ResizeWindow");

    // Missing utterance must fail with InvalidArguments
    let err = bridge
        .invoke("executeVoiceAction", Some(json!({})))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_snap_window_all_9_positions_and_aliases() {
    let bridge = IpcBridge::default();

    let positions = [
        "left", "right", "top", "bottom", "center",
        "maximize", "minimize", "restore", "fullscreen",
    ];

    for pos in positions {
        let res_camel = bridge
            .invoke("snapWindow", Some(json!({ "position": pos })))
            .unwrap_or_else(|e| panic!("snapWindow with position '{}' failed: {:?}", pos, e));
        assert_eq!(res_camel["status"], "success");
        if pos == "minimize" {
            assert_eq!(res_camel["bounds"]["width"], 0);
            assert_eq!(res_camel["bounds"]["height"], 0);
        } else {
            assert!(res_camel["bounds"]["width"].as_u64().unwrap_or(0) > 0);
            assert!(res_camel["bounds"]["height"].as_u64().unwrap_or(0) > 0);
        }

        let res_snake = bridge
            .invoke("snap_window", Some(json!({ "position": pos })))
            .unwrap_or_else(|e| panic!("snap_window with position '{}' failed: {:?}", pos, e));
        assert_eq!(res_snake["status"], "success");
        if pos == "minimize" {
            assert_eq!(res_snake["bounds"]["width"], 0);
        } else {
            assert!(res_snake["bounds"]["width"].as_u64().unwrap_or(0) > 0);
        }
    }

    // Invalid position must fail
    let err = bridge
        .invoke("snapWindow", Some(json!({ "position": "diagonal" })))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_resize_window_work_area_and_delta_aliases() {
    let bridge = IpcBridge::default();

    // resizeWindowWorkArea default
    let res_wa = bridge
        .invoke("resizeWindowWorkArea", None)
        .expect("resizeWindowWorkArea should succeed");
    assert_eq!(res_wa["status"], "success");
    assert_eq!(res_wa["bounds"]["width"], 1280);
    assert_eq!(res_wa["bounds"]["height"], 720);

    // resize_window_work_area explicit
    let res_wa_exp = bridge
        .invoke("resize_window_work_area", Some(json!({ "width": 1920, "height": 1080 })))
        .expect("resize_window_work_area should succeed");
    assert_eq!(res_wa_exp["status"], "success");
    assert_eq!(res_wa_exp["bounds"]["width"], 1920);
    assert_eq!(res_wa_exp["bounds"]["height"], 1080);

    // resizeWindowDelta larger
    let res_delta = bridge
        .invoke("resizeWindowDelta", Some(json!({ "dimension": "larger", "delta_pixels": 100 })))
        .expect("resizeWindowDelta should succeed");
    assert_eq!(res_delta["status"], "success");

    // resize_window_delta smaller
    let res_delta_sm = bridge
        .invoke("resize_window_delta", Some(json!({ "dimension": "smaller", "delta": 50 })))
        .expect("resize_window_delta should succeed");
    assert_eq!(res_delta_sm["status"], "success");

    // Invalid dimension must fail
    let err = bridge
        .invoke("resizeWindowDelta", Some(json!({ "dimension": "infinitely_big" })))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_focus_window_aliases_and_validation() {
    let bridge = IpcBridge::default();

    // camelCase with app parameter
    let res1 = bridge
        .invoke("focusWindow", Some(json!({ "app": "Visual Studio Code" })))
        .expect("focusWindow should succeed");
    assert_eq!(res1["status"], "success");
    assert_eq!(res1["action"], "SwitchFocus");

    // snake_case with name parameter
    let res2 = bridge
        .invoke("focus_window", Some(json!({ "name": "Google Chrome" })))
        .expect("focus_window should succeed");
    assert_eq!(res2["status"], "success");
    assert_eq!(res2["action"], "SwitchFocus");

    // Missing app argument must return InvalidArguments
    let err1 = bridge.invoke("focusWindow", None).unwrap_err();
    assert!(matches!(err1, IpcError::InvalidArguments(_)));

    // Empty app argument must return InvalidArguments
    let err2 = bridge.invoke("focus_window", Some(json!({ "app": "   " }))).unwrap_err();
    assert!(matches!(err2, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_cycle_windows_plural_alias() {
    let bridge = IpcBridge::default();

    let res_plural = bridge
        .invoke("cycleWindows", Some(json!({ "direction": "Next" })))
        .expect("cycleWindows should succeed");
    assert_eq!(res_plural["status"], "success");
    assert_eq!(res_plural["direction"], "Next");

    let res_plural_prev = bridge
        .invoke("cycle_windows", Some(json!({ "direction": "Previous" })))
        .expect("cycle_windows should succeed");
    assert_eq!(res_plural_prev["status"], "success");
    assert_eq!(res_plural_prev["direction"], "Previous");

    // Invalid direction must fail
    let err = bridge
        .invoke("cycleWindows", Some(json!({ "direction": "sideways" })))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_navigate_ui_field_and_simulate_ui_key_aliases() {
    let bridge = IpcBridge::default();

    // navigateUiField default to nextfield
    let res1 = bridge
        .invoke("navigateUiField", None)
        .expect("navigateUiField should succeed");
    assert_eq!(res1["status"], "success");
    assert_eq!(res1["element"], "NextField");

    // navigate_ui_field explicit PrevField
    let res2 = bridge
        .invoke("navigate_ui_field", Some(json!({ "element": "PrevField" })))
        .expect("navigate_ui_field should succeed");
    assert_eq!(res2["status"], "success");
    assert_eq!(res2["element"], "PrevField");

    // simulateUiKey with Enter
    let res3 = bridge
        .invoke("simulateUiKey", Some(json!({ "key": "Enter" })))
        .expect("simulateUiKey with Enter should succeed");
    assert_eq!(res3["status"], "success");
    assert_eq!(res3["element"], "Enter");

    // simulate_ui_key with Escape
    let res4 = bridge
        .invoke("simulate_ui_key", Some(json!({ "key": "Escape" })))
        .expect("simulate_ui_key with Escape should succeed");
    assert_eq!(res4["status"], "success");
    assert_eq!(res4["element"], "Escape");

    // simulateUiKey with Tab
    let res5 = bridge
        .invoke("simulateUiKey", Some(json!({ "key": "Tab" })))
        .expect("simulateUiKey with Tab should succeed");
    assert_eq!(res5["status"], "success");
    assert_eq!(res5["element"], "Tab");

    // Invalid key must return InvalidArguments
    let err = bridge
        .invoke("simulateUiKey", Some(json!({ "key": "F24_SuperKey" })))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

// ============================================================================
// Milestone M12: R3 Meetily Command Aliases & Verification
// ============================================================================

#[test]
fn test_m12_audio_mixer_volumes_set_and_get() {
    let bridge = IpcBridge::default();

    // Initial state check
    let res_init = bridge
        .invoke("getAudioMixerVolumes", None)
        .expect("getAudioMixerVolumes should succeed");
    assert_eq!(res_init["status"], "success");
    assert_eq!(res_init["mic_volume"], 1.0);
    assert_eq!(res_init["system_volume"], 1.0);

    // Set volumes with camelCase
    let res_set = bridge
        .invoke(
            "setAudioMixerVolumes",
            Some(json!({ "mic_volume": 1.5, "system_volume": 0.5 })),
        )
        .expect("setAudioMixerVolumes should succeed");
    assert_eq!(res_set["status"], "success");
    assert_eq!(res_set["mic_volume"], 1.5);
    assert_eq!(res_set["system_volume"], 0.5);

    // Verify updated volumes with snake_case
    let res_get_snake = bridge
        .invoke("get_audio_mixer_volumes", None)
        .expect("get_audio_mixer_volumes should succeed");
    assert_eq!(res_get_snake["status"], "success");
    assert_eq!(res_get_snake["mic_volume"], 1.5);
    assert_eq!(res_get_snake["system_volume"], 0.5);

    // Boundary value testing: 0.0 and 2.0
    let res_bounds = bridge
        .invoke(
            "set_audio_mixer_volumes",
            Some(json!({ "mic_volume": 0.0, "system_volume": 2.0 })),
        )
        .expect("boundary volumes (0.0 and 2.0) should succeed");
    assert_eq!(res_bounds["mic_volume"], 0.0);
    assert_eq!(res_bounds["system_volume"], 2.0);

    // Transactional error handling: out of range (> 2.0) must fail and rollback/preserve
    let err_high = bridge
        .invoke(
            "setAudioMixerVolumes",
            Some(json!({ "mic_volume": 1.0, "system_volume": 2.5 })),
        )
        .unwrap_err();
    assert!(matches!(err_high, IpcError::InvalidArguments(_)));

    // Verify state was not mutated by failed call
    let res_unchanged = bridge.invoke("getAudioMixerVolumes", None).unwrap();
    assert_eq!(res_unchanged["mic_volume"], 0.0);
    assert_eq!(res_unchanged["system_volume"], 2.0);

    // Negative volume must fail
    let err_neg = bridge
        .invoke(
            "set_audio_mixer_volumes",
            Some(json!({ "mic_volume": -0.1, "system_volume": 1.0 })),
        )
        .unwrap_err();
    assert!(matches!(err_neg, IpcError::InvalidArguments(_)));

    // Missing parameter must fail
    let err_missing = bridge
        .invoke("setAudioMixerVolumes", Some(json!({ "mic_volume": 1.0 })))
        .unwrap_err();
    assert!(matches!(err_missing, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_generate_ai_summary_aliases_and_validation() {
    let bridge = IpcBridge::default();

    let sample_transcript = "Alice: Good morning. Let's discuss the M12 integration release. Bob: Tests are 100% passing.";

    // camelCase
    let res_camel = bridge
        .invoke(
            "generateAiSummary",
            Some(json!({
                "transcript": sample_transcript,
                "template": "ExecutiveSummary"
            })),
        )
        .expect("generateAiSummary should succeed");
    assert_eq!(res_camel["status"], "success");
    let summary_md1 = res_camel["summary"]["summary_markdown"].as_str().unwrap();
    assert!(summary_md1.contains("Executive Summary"));

    // snake_case
    let res_snake = bridge
        .invoke(
            "generate_ai_summary",
            Some(json!({
                "transcript": sample_transcript,
                "template": "ActionItems"
            })),
        )
        .expect("generate_ai_summary should succeed");
    assert_eq!(res_snake["status"], "success");
    let summary_md2 = res_snake["summary"]["summary_markdown"].as_str().unwrap();
    assert!(summary_md2.contains("Action Items") || summary_md2.contains("Tasks"));

    // Empty transcript must fail
    let err_empty = bridge
        .invoke("generateAiSummary", Some(json!({ "transcript": "   \n\t  " })))
        .unwrap_err();
    assert!(matches!(err_empty, IpcError::InvalidArguments(_)));
}

#[test]
fn test_m12_meeting_recording_lifecycle_and_archive_search() {
    let bridge = IpcBridge::default();

    // Start meeting
    let start_res = bridge
        .invoke("startMeetingRecording", Some(json!({ "meeting_id": "m12_meet_test" })))
        .expect("startMeetingRecording should succeed");
    assert_eq!(start_res["status"], "success");
    assert_eq!(start_res["recording_started"], true);

    // Stop meeting
    let stop_res = bridge
        .invoke("stopMeetingRecording", None)
        .expect("stopMeetingRecording should succeed");
    assert_eq!(stop_res["status"], "success");
    assert_eq!(stop_res["recording_stopped"], true);

    // Search archive via alias
    let search_res = bridge
        .invoke("searchMeetingArchive", Some(json!({ "query": "m12_meet_test" })))
        .expect("searchMeetingArchive should succeed");
    assert_eq!(search_res["status"], "success");

    let get_archive_res = bridge
        .invoke("get_meeting_archive", None)
        .expect("get_meeting_archive should succeed");
    assert_eq!(get_archive_res["status"], "success");
}

#[test]
fn test_m12_export_summary_all_formats() {
    let bridge = IpcBridge::default();

    let formats = ["Markdown", "PDF", "DOCX"];
    for fmt in formats {
        let res = bridge
            .invoke(
                "exportSummary",
                Some(json!({
                    "content": "# Test Summary\n\n- Item 1\n- Item 2",
                    "format": fmt
                })),
            )
            .unwrap_or_else(|e| panic!("exportSummary failed for format '{}': {:?}", fmt, e));
        assert_eq!(res["status"], "success");
        assert!(res["bytes_count"].as_u64().unwrap_or(0) > 0);
        assert_eq!(res["format"], fmt);
    }

    // Invalid format must fail
    let err = bridge
        .invoke("export_summary", Some(json!({ "format": "INVALID_XLS" })))
        .unwrap_err();
    assert!(matches!(err, IpcError::InvalidArguments(_)));
}

// ============================================================================
// Milestone M12: R4 Gestures & Virtual Camera Splitter Surface
// ============================================================================

#[test]
fn test_m12_gestures_and_virtual_camera_surface() {
    let bridge = IpcBridge::default();

    // Gesture engine status
    let res_gest_status = bridge
        .invoke("getGestureEngineStatus", None)
        .expect("getGestureEngineStatus should succeed");
    assert_eq!(res_gest_status["status"], "success");

    // Set gesture tracking mode (atomic validation)
    let res_set_mode = bridge
        .invoke(
            "setGestureTrackingMode",
            Some(json!({
                "mode": "DualHand",
                "sensitivity": 0.9,
                "cooldown_secs": 0.25
            })),
        )
        .expect("setGestureTrackingMode should succeed");
    assert_eq!(res_set_mode["status"], "success");
    assert_eq!(res_set_mode["mode"], "DualHand");

    // Virtual camera status
    let res_cam_status = bridge
        .invoke("getVirtualCameraStatus", None)
        .expect("getVirtualCameraStatus should succeed");
    assert_eq!(res_cam_status["status"], "success");
    assert_eq!(res_cam_status["device_name"], "AirCtrl Virtual Camera");
    assert_eq!(res_cam_status["active_consumers"], 2);

    // Set virtual camera privacy
    let res_priv = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "PrivacySlate" })))
        .expect("setVirtualCameraPrivacy should succeed");
    assert_eq!(res_priv["status"], "success");
    assert_eq!(res_priv["privacy_muted"], true);

    // Reset camera privacy to Active
    let res_active = bridge
        .invoke("set_virtual_camera_privacy", Some(json!({ "mode": "Active" })))
        .expect("set_virtual_camera_privacy should succeed");
    assert_eq!(res_active["status"], "success");
    assert_eq!(res_active["privacy_muted"], false);

    // Multi-hand gesture status
    let res_hand = bridge
        .invoke("getMultiHandGestureStatus", None)
        .expect("getMultiHandGestureStatus should succeed");
    assert_eq!(res_hand["status"], "success");

    // Feed camera frame
    let res_frame = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 42 })))
        .expect("feedCameraFrame should succeed");
    assert_eq!(res_frame["frame_id"], 42);
    assert_eq!(res_frame["consumer_1_mediapipe"], "raw_frame_42");
    assert_eq!(res_frame["consumer_2_meeting"], "outbound_frame_42");
}
