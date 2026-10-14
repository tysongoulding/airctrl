//! Adversarial stress-test suite for Milestone M8: Handy 1:1 Parity
//! Targeting CliOptions fuzzing, malformed CLI flags, and headless IPC
//! transactional rollback (0 side-effects) and command edge cases.

use airctrl_shell::cli::CliOptions;
use airctrl_shell::ipc::{IpcBridge, IpcError};
use serde_json::json;

// ============================================================================
// 1. CliOptions Fuzzing & Malformed Argument Resilience
// ============================================================================

#[test]
fn test_adversarial_cli_options_fuzzing_and_unknown_flags() {
    // Array with mixed valid flags, unknown flags, empty strings, and numeric arguments
    let args = vec![
        "",
        "--unknown-flag",
        "   ",
        "--foo=bar",
        "--start-hidden",
        "-x",
        "--debug",
        "random_positional",
        "--no-tray",
        "12345",
        "---triple-dash",
        "--cancel",
    ];

    let opts = CliOptions::parse(args);

    assert!(opts.start_hidden);
    assert!(opts.debug);
    assert!(opts.no_tray);
    assert!(opts.cancel);
    assert!(opts.is_remote_command());

    // Other flags not passed must remain default (false / None)
    assert!(!opts.toggle_transcription);
    assert!(!opts.toggle_post_process);
    assert!(!opts.help);
    assert_eq!(opts.model, None);
}

#[test]
fn test_adversarial_cli_options_malformed_model_flag() {
    // Case 1: --model is the very last argument in argv with no value
    let args_missing_val = vec!["--start-hidden", "--model"];
    let opts = CliOptions::parse(args_missing_val);
    assert!(opts.start_hidden);
    assert_eq!(opts.model, None);

    // Case 2: Multiple --model flags specified (last one should win)
    let args_multiple = vec![
        "--model",
        "whisper-tiny.en",
        "--model",
        "whisper-turbo.en",
        "--model",
        "parakeet-v3",
    ];
    let opts_multi = CliOptions::parse(args_multiple);
    assert_eq!(opts_multi.model.as_deref(), Some("parakeet-v3"));

    // Case 3: Empty string passed as model argument
    let args_empty_model = vec!["--model", ""];
    let opts_empty = CliOptions::parse(args_empty_model);
    assert_eq!(opts_empty.model.as_deref(), Some(""));
}

#[test]
fn test_adversarial_cli_options_remote_command_classification() {
    // No remote commands
    let opts_daemon = CliOptions::parse(vec!["--start-hidden", "--no-tray", "--debug"]);
    assert!(!opts_daemon.is_remote_command());

    // Single remote command
    let opts_trans = CliOptions::parse(vec!["--toggle-transcription"]);
    assert!(opts_trans.is_remote_command());

    let opts_post = CliOptions::parse(vec!["--toggle-post-process"]);
    assert!(opts_post.is_remote_command());

    let opts_cancel = CliOptions::parse(vec!["--cancel"]);
    assert!(opts_cancel.is_remote_command());

    // Combined
    let opts_combo = CliOptions::parse(vec!["--start-hidden", "--cancel"]);
    assert!(opts_combo.is_remote_command());
}

// ============================================================================
// 2. Headless IPC: updateSettings Transactional Atomicity (0 Side-Effects)
// ============================================================================

#[test]
fn test_adversarial_ipc_update_settings_transactional_rollback() {
    let bridge = IpcBridge::default();

    // Baseline configuration snapshot
    let initial_settings = bridge.invoke("getSettings", None).unwrap();
    let orig_stt_hk = initial_settings["hotkey_stt"].as_str().unwrap().to_string();
    let orig_overlay = initial_settings["overlay_position"].as_str().unwrap().to_string();
    let orig_sectors = initial_settings["radial_dial_sectors"].as_u64().unwrap();
    let orig_paste = initial_settings["reliable_paste"].as_bool().unwrap();

    // Attack 1: Valid hotkey_stt BUT invalid radial_dial_sectors (12, out of 4..=8)
    let res = bridge.invoke(
        "updateSettings",
        Some(json!({
            "hotkey_stt": "Ctrl+Alt+Shift+Z",
            "radial_dial_sectors": 12
        })),
    );
    assert!(res.is_err());
    match res.unwrap_err() {
        IpcError::InvalidArguments(msg) => {
            assert!(msg.contains("radial_dial_sectors"));
        }
        err => panic!("Expected InvalidArguments, got: {:?}", err),
    }

    // Verify ZERO side-effects on config: hotkey_stt must NOT have been updated!
    let cur_settings = bridge.invoke("getSettings", None).unwrap();
    assert_eq!(cur_settings["hotkey_stt"].as_str().unwrap(), orig_stt_hk);
    assert_eq!(cur_settings["radial_dial_sectors"].as_u64().unwrap(), orig_sectors);

    // Attack 2: Valid overlay_position BUT invalid hotkey_dial (missing "Ctrl+" prefix)
    let res2 = bridge.invoke(
        "updateSettings",
        Some(json!({
            "overlay_position": "Top",
            "hotkey_dial": "Alt+Win+D"
        })),
    );
    assert!(res2.is_err());
    match res2.unwrap_err() {
        IpcError::InvalidArguments(msg) => {
            assert!(msg.contains("hotkey_dial"));
        }
        err => panic!("Expected InvalidArguments, got: {:?}", err),
    }

    // Verify ZERO side-effects: overlay_position must NOT have changed!
    let cur_settings2 = bridge.invoke("getSettings", None).unwrap();
    assert_eq!(cur_settings2["overlay_position"].as_str().unwrap(), orig_overlay);

    // Attack 3: Valid reliable_paste BUT invalid recording_mode string
    let res3 = bridge.invoke(
        "updateSettings",
        Some(json!({
            "reliable_paste": !orig_paste,
            "recording_mode": "InvalidFictionalMode"
        })),
    );
    assert!(res3.is_err());

    // Verify ZERO side-effects: reliable_paste must NOT have changed!
    let cur_settings3 = bridge.invoke("getSettings", None).unwrap();
    assert_eq!(cur_settings3["reliable_paste"].as_bool().unwrap(), orig_paste);

    // Attack 4: Non-object JSON payload (array instead of object)
    let res4 = bridge.invoke("updateSettings", Some(json!(["invalid", "array"])));
    assert!(res4.is_err());

    // Success Case: All fields valid must commit atomically
    let res_valid = bridge.invoke(
        "updateSettings",
        Some(json!({
            "hotkey_stt": "Ctrl+Shift+U",
            "radial_dial_sectors": 6,
            "overlay_position": "Center",
            "reliable_paste": true,
            "paste_delay_before_ms": 75,
            "paste_delay_after_ms": 150
        })),
    );
    assert!(res_valid.is_ok());

    let final_settings = bridge.invoke("getSettings", None).unwrap();
    assert_eq!(final_settings["hotkey_stt"].as_str().unwrap(), "Ctrl+Shift+U");
    assert_eq!(final_settings["radial_dial_sectors"].as_u64().unwrap(), 6);
    assert_eq!(final_settings["overlay_position"].as_str().unwrap(), "Center");
    assert!(final_settings["reliable_paste"].as_bool().unwrap());
    assert_eq!(final_settings["paste_delay_before_ms"].as_u64().unwrap(), 75);
    assert_eq!(final_settings["paste_delay_after_ms"].as_u64().unwrap(), 150);
}

// ============================================================================
// 3. Headless IPC: Milestone M8 Command Surface & Edge Cases
// ============================================================================

#[test]
fn test_adversarial_ipc_model_and_recording_mode_commands() {
    let bridge = IpcBridge::default();

    // 1. listModels
    let list_res = bridge.invoke("listModels", None).unwrap();
    let models = list_res["models"].as_array().expect("Models should be array");
    assert!(models.iter().any(|m| m.as_str() == Some("whisper-turbo.en")));
    assert!(models.iter().any(|m| m.as_str() == Some("parakeet-unified-en-0.6b-q8_0")));

    // 2. selectModel: error on missing argument
    let err_select = bridge.invoke("selectModel", None);
    assert!(err_select.is_err());

    // selectModel: success
    let ok_select = bridge.invoke("selectModel", Some(json!({ "model": "whisper-turbo.en" }))).unwrap();
    assert_eq!(ok_select["model"].as_str().unwrap(), "whisper-turbo.en");

    // 3. setRecordingMode & getRecordingMode: error on missing argument
    let err_mode = bridge.invoke("setRecordingMode", None);
    assert!(err_mode.is_err());

    // setRecordingMode: error on unknown mode
    let err_unknown = bridge.invoke("setRecordingMode", Some(json!({ "mode": "InvalidMode123" })));
    assert!(err_unknown.is_err());

    // setRecordingMode: valid modes
    for mode in &["SmartDualMode", "HoldOnly", "ToggleOnly", "ContinuousHandsFree"] {
        let set_res = bridge.invoke("setRecordingMode", Some(json!({ "mode": mode }))).unwrap();
        assert_eq!(set_res["mode"].as_str().unwrap(), *mode);

        let get_res = bridge.invoke("getRecordingMode", None).unwrap();
        assert!(get_res["recording_mode"].as_str().unwrap().contains(mode));
    }
}

#[test]
fn test_adversarial_ipc_overlay_position_zero_focus_stealing() {
    let bridge = IpcBridge::default();

    // Invalid position string
    let err_pos = bridge.invoke("setOverlayPosition", Some(json!({ "position": "FloatingLeft" })));
    assert!(err_pos.is_err());

    // None position: must strictly enforce zero focus stealing and hidden bounds
    let none_res = bridge.invoke("setOverlayPosition", Some(json!({ "position": "None" }))).unwrap();
    let bounds = &none_res["bounds"];
    assert!(!bounds["visible"].as_bool().unwrap());
    assert!(!bounds["focus_stealing"].as_bool().unwrap());
    assert_eq!(bounds["width"].as_u64().unwrap(), 0);
    assert_eq!(bounds["height"].as_u64().unwrap(), 0);

    let cur_pos = bridge.invoke("getOverlayPosition", None).unwrap();
    assert_eq!(cur_pos["position"].as_str().unwrap(), "None");

    // Top position
    let top_res = bridge.invoke("setOverlayPosition", Some(json!({ "position": "Top" }))).unwrap();
    let top_bounds = &top_res["bounds"];
    assert!(top_bounds["visible"].as_bool().unwrap());
    assert!(!top_bounds["focus_stealing"].as_bool().unwrap());
    assert!(top_bounds["width"].as_u64().unwrap() > 0);
}

#[test]
fn test_adversarial_ipc_dictionary_and_transcripts_commands() {
    let bridge = IpcBridge::default();

    // 1. updateDictionary: missing rules field
    let err_dict = bridge.invoke("updateDictionary", None);
    assert!(err_dict.is_err());

    // 2. updateDictionary: map format
    let dict_map_res = bridge.invoke(
        "updateDictionary",
        Some(json!({
            "rules": {
                "tyson": "Tyson Goulding",
                "airctrl": "AirCtrl App"
            }
        })),
    ).unwrap();
    assert_eq!(dict_map_res["total_rules"].as_u64().unwrap(), 2);

    let get_dict = bridge.invoke("getDictionary", None).unwrap();
    let rules = get_dict["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 2);

    // 3. togglePostProcessing
    let toggle_pp1 = bridge.invoke("togglePostProcessing", None).unwrap();
    let pp_state1 = toggle_pp1["post_processing_enabled"].as_bool().unwrap();

    let toggle_pp2 = bridge.invoke("togglePostProcessing", None).unwrap();
    let pp_state2 = toggle_pp2["post_processing_enabled"].as_bool().unwrap();
    assert_ne!(pp_state1, pp_state2);

    // 4. searchTranscripts & clearTranscripts
    let search_empty = bridge.invoke("searchTranscripts", None).unwrap();
    assert_eq!(search_empty["count"].as_u64().unwrap(), 0);

    // Directly store an item via state
    bridge.state.transcript_store.store(airctrl_core::types::TranscriptItem {
        id: "tx_ipc_1".into(),
        timestamp: 1000.0,
        text: "IPC search test entry".into(),
        raw_text: "ipc search test entry".into(),
        duration_ms: 1200,
        model_used: "whisper-turbo.en".into(),
        confidence: 0.99,
        session_id: "s1".into(),
        target_app: Some("code".into()),
        target_title: None,
        injected: true,
        starred: false,
    }).unwrap();

    let search_res = bridge.invoke("searchTranscripts", Some(json!({ "query": "search" }))).unwrap();
    assert_eq!(search_res["count"].as_u64().unwrap(), 1);

    // clearTranscripts
    let clear_res = bridge.invoke("clearTranscripts", None).unwrap();
    assert!(clear_res["cleared"].as_bool().unwrap());

    let search_after_clear = bridge.invoke("searchTranscripts", None).unwrap();
    assert_eq!(search_after_clear["count"].as_u64().unwrap(), 0);

    // 5. toggleDebugDrawer & getDebugDrawerStatus
    let drawer_initial = bridge.invoke("getDebugDrawerStatus", None).unwrap();
    let vis1 = drawer_initial["debug_drawer_visible"].as_bool().unwrap();

    let toggle_drawer = bridge.invoke("toggleDebugDrawer", None).unwrap();
    let vis2 = toggle_drawer["debug_drawer_visible"].as_bool().unwrap();
    assert_ne!(vis1, vis2);
}
