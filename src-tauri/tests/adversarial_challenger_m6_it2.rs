use airctrl_shell::{AppState, IpcBridge, IpcError};
use airctrl_stt::{ModelInfo, Result as SttResult, SpeechToTextEngine, TranscriptionResult};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// ============================================================================
// 1. Adversarial Fuzzing of updateSettings Root Object
// ============================================================================

#[test]
fn test_challenger_update_settings_rejects_non_object_roots() {
    let bridge = IpcBridge::default();

    let non_object_roots: Vec<Value> = vec![
        Value::String("not an object".into()),
        Value::Array(vec![]),
        Value::Number(123.into()),
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        json!([{"hotkey_stt": "Ctrl+Spacebar"}]),
    ];

    for root in non_object_roots {
        let err = bridge
            .invoke("updateSettings", Some(root.clone()))
            .expect_err(&format!("Failed to reject non-object root: {:?}", root));
        match err {
            IpcError::InvalidArguments(msg) => {
                assert!(
                    msg.contains("must be a JSON object"),
                    "Unexpected error message for {:?}: {}",
                    root,
                    msg
                );
            }
            other => panic!("Expected IpcError::InvalidArguments for {:?}, got {:?}", root, other),
        }
    }
}

// ============================================================================
// 2. Adversarial Fuzzing of hotkey_stt Type & Format Mismatches
// ============================================================================

#[test]
fn test_challenger_update_settings_rejects_hotkey_stt_mismatches() {
    let bridge = IpcBridge::default();

    let invalid_stt_values = vec![
        json!(123),
        json!(true),
        json!(false),
        json!(vec!["Ctrl+A"]),
        json!("Alt+Space"),
        json!(""),
        json!(null),
        json!({"hotkey": "Ctrl+A"}),
        json!(3.5),
    ];

    for val in invalid_stt_values {
        let payload = json!({ "hotkey_stt": val });
        let err = bridge
            .invoke("updateSettings", Some(payload.clone()))
            .expect_err(&format!("Failed to reject invalid hotkey_stt: {:?}", val));
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for hotkey_stt: {:?}, got {:?}",
            val,
            err
        );
    }

    // Valid format verification
    let valid_payload = json!({ "hotkey_stt": "Ctrl+Spacebar" });
    let res = bridge.invoke("updateSettings", Some(valid_payload)).unwrap();
    assert_eq!(res["success"], true);
    assert_eq!(res["settings"]["hotkey_stt"], "Ctrl+Spacebar");
}

// ============================================================================
// 3. Adversarial Fuzzing of hotkey_dial Type & Format Mismatches
// ============================================================================

#[test]
fn test_challenger_update_settings_rejects_hotkey_dial_mismatches() {
    let bridge = IpcBridge::default();

    let invalid_dial_values = vec![
        json!(123),
        json!(false),
        json!(true),
        json!("Shift+Alt"),
        json!("Command+Alt"),
        json!(""),
        json!(null),
        json!(["Ctrl+Alt"]),
        json!({"key": "Ctrl+Alt"}),
    ];

    for val in invalid_dial_values {
        let payload = json!({ "hotkey_dial": val });
        let err = bridge
            .invoke("updateSettings", Some(payload.clone()))
            .expect_err(&format!("Failed to reject invalid hotkey_dial: {:?}", val));
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for hotkey_dial: {:?}, got {:?}",
            val,
            err
        );
    }

    // Valid format verification
    let valid_payload = json!({ "hotkey_dial": "Ctrl+Alt" });
    let res = bridge.invoke("updateSettings", Some(valid_payload)).unwrap();
    assert_eq!(res["success"], true);
    assert_eq!(res["settings"]["hotkey_dial"], "Ctrl+Alt");
}

// ============================================================================
// 4. Adversarial Fuzzing of camera_auto_kill Type Mismatches
// ============================================================================

#[test]
fn test_challenger_update_settings_rejects_camera_auto_kill_mismatches() {
    let bridge = IpcBridge::default();

    let invalid_kill_values = vec![
        json!("true"),
        json!("false"),
        json!(1),
        json!(0),
        json!([]),
        json!({}),
        json!(null),
        json!(1.0),
    ];

    for val in invalid_kill_values {
        let payload = json!({ "camera_auto_kill": val });
        let err = bridge
            .invoke("updateSettings", Some(payload.clone()))
            .expect_err(&format!("Failed to reject invalid camera_auto_kill: {:?}", val));
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for camera_auto_kill: {:?}, got {:?}",
            val,
            err
        );
    }

    // Valid boolean verification
    let valid_false = json!({ "camera_auto_kill": false });
    let res = bridge.invoke("updateSettings", Some(valid_false)).unwrap();
    assert_eq!(res["settings"]["camera_auto_kill"], false);

    let valid_true = json!({ "camera_auto_kill": true });
    let res2 = bridge.invoke("updateSettings", Some(valid_true)).unwrap();
    assert_eq!(res2["settings"]["camera_auto_kill"], true);
}

// ============================================================================
// 5. Adversarial Fuzzing of compliance_audio_enabled Type Mismatches
// ============================================================================

#[test]
fn test_challenger_update_settings_rejects_compliance_audio_mismatches() {
    let bridge = IpcBridge::default();

    let invalid_compliance_values = vec![
        json!("false"),
        json!("true"),
        json!(0),
        json!(1),
        json!([]),
        json!({}),
        json!(null),
        json!(0.5),
    ];

    for val in invalid_compliance_values {
        let payload = json!({ "compliance_audio_enabled": val });
        let err = bridge
            .invoke("updateSettings", Some(payload.clone()))
            .expect_err(&format!("Failed to reject invalid compliance_audio_enabled: {:?}", val));
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for compliance_audio_enabled: {:?}, got {:?}",
            val,
            err
        );
    }

    // Valid boolean verification
    let valid_payload = json!({ "compliance_audio_enabled": false });
    let res = bridge.invoke("updateSettings", Some(valid_payload)).unwrap();
    assert_eq!(res["settings"]["compliance_audio_enabled"], false);
}

// ============================================================================
// 6. Adversarial Fuzzing of radial_dial_sectors Type & Range Mismatches
// ============================================================================

#[test]
fn test_challenger_update_settings_rejects_radial_dial_sectors_mismatches() {
    let bridge = IpcBridge::default();

    let invalid_sectors_values = vec![
        json!("6"),
        json!(5.5),
        json!(-1),
        json!(-10),
        json!(0),
        json!(1),
        json!(2),
        json!(3),
        json!(9),
        json!(10),
        json!(100),
        json!(true),
        json!(false),
        json!(null),
        json!([]),
        json!({}),
    ];

    for val in invalid_sectors_values {
        let payload = json!({ "radial_dial_sectors": val });
        let err = bridge
            .invoke("updateSettings", Some(payload.clone()))
            .expect_err(&format!("Failed to reject invalid radial_dial_sectors: {:?}", val));
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for radial_dial_sectors: {:?}, got {:?}",
            val,
            err
        );
    }

    // Valid range verification [4, 8]
    for valid_s in 4..=8 {
        let payload = json!({ "radial_dial_sectors": valid_s });
        let res = bridge.invoke("updateSettings", Some(payload)).unwrap();
        assert_eq!(res["settings"]["radial_dial_sectors"], valid_s);
        assert_eq!(bridge.state.config.read().radial_dial_sectors, valid_s);
        assert_eq!(bridge.state.dial_overlay.read().sector_count(), valid_s);
    }
}

// ============================================================================
// 7. Transactional Rollback Test
// ============================================================================

#[test]
fn test_challenger_update_settings_transactional_rollback() {
    let bridge = IpcBridge::default();

    // Initial state check
    let initial_stt = bridge.state.config.read().stt_hotkey.clone();
    let initial_dial = bridge.state.config.read().dial_hotkey.clone();
    let initial_kill = bridge.state.config.read().camera_auto_kill;
    let initial_sectors = bridge.state.config.read().radial_dial_sectors;
    let initial_overlay_sectors = bridge.state.dial_overlay.read().sector_count();

    // Scenario A: Valid hotkey_stt ("Ctrl+F"), but invalid radial_dial_sectors (99)
    let payload_a = json!({
        "hotkey_stt": "Ctrl+F",
        "radial_dial_sectors": 99
    });
    let err_a = bridge
        .invoke("updateSettings", Some(payload_a))
        .expect_err("updateSettings must fail on radial_dial_sectors: 99");
    assert!(matches!(err_a, IpcError::InvalidArguments(_)));

    // Verify atomic rollback: config.hotkey_stt must NOT be modified!
    assert_eq!(bridge.state.config.read().stt_hotkey, initial_stt);
    assert_eq!(bridge.state.config.read().radial_dial_sectors, initial_sectors);
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), initial_overlay_sectors);

    // Scenario B: Valid camera_auto_kill (false) and hotkey_stt ("Ctrl+Shift+T"), but invalid hotkey_dial ("Alt+Shift")
    let payload_b = json!({
        "camera_auto_kill": false,
        "hotkey_stt": "Ctrl+Shift+T",
        "hotkey_dial": "Alt+Shift"
    });
    let err_b = bridge
        .invoke("updateSettings", Some(payload_b))
        .expect_err("updateSettings must fail on invalid hotkey_dial");
    assert!(matches!(err_b, IpcError::InvalidArguments(_)));

    // Verify rollback: none of the fields should be modified
    assert_eq!(bridge.state.config.read().camera_auto_kill, initial_kill);
    assert_eq!(bridge.state.config.read().stt_hotkey, initial_stt);
    assert_eq!(bridge.state.config.read().dial_hotkey, initial_dial);

    // Scenario C: Valid sectors (5), but invalid compliance_audio_enabled ("false" as string)
    let payload_c = json!({
        "radial_dial_sectors": 5,
        "compliance_audio_enabled": "false"
    });
    let err_c = bridge
        .invoke("updateSettings", Some(payload_c))
        .expect_err("updateSettings must fail on string compliance_audio_enabled");
    assert!(matches!(err_c, IpcError::InvalidArguments(_)));

    // Verify rollback: overlay sectors and config sectors must remain initial
    assert_eq!(bridge.state.config.read().radial_dial_sectors, initial_sectors);
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), initial_overlay_sectors);
}

// ============================================================================
// 8. STT Edge Cases & Dependency Injection
// ============================================================================

#[test]
fn test_challenger_stt_stop_recording_empty_buffer_returns_error_not_hello() {
    let bridge = IpcBridge::default();

    // Call stopRecording on fresh AppState without startRecording
    let err = bridge
        .invoke("stopRecording", None)
        .expect_err("stopRecording on empty buffer must return error");

    match err {
        IpcError::ExecutionFailed(msg) => {
            assert!(
                !msg.to_lowercase().contains("hello"),
                "Returned message must NOT contain 'hello': {}",
                msg
            );
            assert!(
                msg.contains("No audio samples recorded"),
                "Expected 'No audio samples recorded', got: {}",
                msg
            );
        }
        other => panic!("Expected IpcError::ExecutionFailed, got {:?}", other),
    }
}

#[test]
fn test_challenger_stt_sample_draining_prevents_replay() {
    let bridge = IpcBridge::default();

    // 1. Record and stop
    bridge.invoke("startRecording", None).unwrap();
    let res1 = bridge.invoke("stopRecording", None).unwrap();
    assert_eq!(res1["status"], "TranscriptionReady");
    assert_eq!(res1["text"], "airctrl speech transcription result");

    // 2. Second stopRecording without new startRecording must fail because samples were drained
    let err = bridge
        .invoke("stopRecording", None)
        .expect_err("Second stopRecording without startRecording must fail");
    assert!(matches!(err, IpcError::ExecutionFailed(_)));
}

struct MockCustomEngine {
    transcription_text: String,
    call_count: AtomicUsize,
}

impl SpeechToTextEngine for MockCustomEngine {
    fn transcribe(&self, samples: &[f32]) -> SttResult<TranscriptionResult> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(TranscriptionResult {
            text: self.transcription_text.clone(),
            duration_ms: 10,
            confidence: 0.98,
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

#[test]
fn test_challenger_stt_dependency_injection_custom_transcription_xyz() {
    let custom_token = "CUSTOM_TRANSCRIPTION_XYZ";
    let mock_engine = Box::new(MockCustomEngine {
        transcription_text: custom_token.to_string(),
        call_count: AtomicUsize::new(0),
    });

    let state = Arc::new(AppState::with_engine(mock_engine));
    let bridge = IpcBridge::new(state);

    // Call startRecording -> stopRecording
    bridge.invoke("startRecording", None).expect("startRecording failed");
    let res = bridge
        .invoke("stopRecording", None)
        .expect("stopRecording failed");

    assert_eq!(res["status"], "TranscriptionReady");
    assert_eq!(res["text"], custom_token);

    // Draining check with custom engine: next stopRecording fails
    let err = bridge
        .invoke("stopRecording", None)
        .expect_err("Expected error on drained buffer with custom engine");
    assert!(matches!(err, IpcError::ExecutionFailed(_)));
}

// ============================================================================
// 9. Dial Overlay State Machine & Dynamic Sector Adjustment
// ============================================================================

#[test]
fn test_challenger_dial_overlay_hover_boundary_on_sector_shrink() {
    let bridge = IpcBridge::default();

    // Show dial and hover sector 5 (valid for default 6 sectors)
    bridge.state.dial_overlay.write().show();
    bridge.state.dial_overlay.write().hover_sector(5);
    assert_eq!(bridge.state.dial_overlay.read().active_sector(), Some(5));

    // Update settings to shrink sectors to 4
    let req = json!({ "radial_dial_sectors": 4 });
    bridge.invoke("updateSettings", Some(req)).unwrap();

    // Overlay sector count must now be 4, and active_sector must be reset to None because 5 >= 4
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), 4);
    assert_eq!(bridge.state.dial_overlay.read().active_sector(), None);
    assert!(bridge.state.dial_overlay.read().is_visible());

    // Hover valid sector 3 in new [0, 4) range
    bridge.state.dial_overlay.write().hover_sector(3);
    assert_eq!(bridge.state.dial_overlay.read().active_sector(), Some(3));

    // Expand sectors to 8: active sector 3 remains valid and preserved
    let req_expand = json!({ "radial_dial_sectors": 8 });
    bridge.invoke("updateSettings", Some(req_expand)).unwrap();
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), 8);
    assert_eq!(bridge.state.dial_overlay.read().active_sector(), Some(3));
}
