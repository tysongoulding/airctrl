use airctrl_shell::{IpcBridge, IpcError};
use serde_json::{json, Value};
use std::sync::Arc;
use std::thread;

/// Adversarial Test 1: Fuzzing `setGestureTrackingMode` payloads
/// All invalid inputs must return IpcError::InvalidArguments, and MUST NOT corrupt state.
#[test]
fn test_fuzz_set_gesture_tracking_mode_arguments() {
    let bridge = IpcBridge::default();

    // 1. Invalid mode strings
    let invalid_modes = vec![
        json!({ "mode": "TripleHand" }),
        json!({ "mode": "QuadHand" }),
        json!({ "mode": "" }),
        json!({ "mode": "   " }),
        json!({ "mode": "none" }),
        json!({ "mode": "onehand" }), // Invalid string
    ];

    for payload in invalid_modes {
        let err = bridge
            .invoke("setGestureTrackingMode", Some(payload))
            .expect_err("Expected invalid mode to fail");
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments error, got {:?}",
            err
        );
    }

    // 2. Invalid mode types (non-string)
    let invalid_type_modes = vec![
        json!({ "mode": 123 }),
        json!({ "mode": true }),
        json!({ "mode": [ "DualHand" ] }),
        json!({ "mode": { "name": "DualHand" } }),
    ];

    for payload in invalid_type_modes {
        let err = bridge
            .invoke("setGestureTrackingMode", Some(payload))
            .expect_err("Expected non-string mode to fail");
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments error, got {:?}",
            err
        );
    }

    // 3. Fuzzing sensitivity boundaries and types
    let invalid_sensitivities = vec![
        json!({ "sensitivity": -1.0 }),
        json!({ "sensitivity": -0.001 }),
        json!({ "sensitivity": 0.0 }), // Must be > 0.0
        json!({ "sensitivity": 1.0001 }),
        json!({ "sensitivity": 100.0 }),
        json!({ "sensitivity": "0.5" }), // string instead of number
        json!({ "sensitivity": true }),
        json!({ "sensitivity": null }),
    ];

    for payload in invalid_sensitivities {
        let err = bridge
            .invoke("setGestureTrackingMode", Some(payload))
            .expect_err("Expected invalid sensitivity to fail");
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for invalid sensitivity, got {:?}",
            err
        );
    }

    // 4. Fuzzing cooldown values
    let invalid_cooldowns = vec![
        json!({ "cooldown_secs": -0.1 }),
        json!({ "cooldown_secs": -10.0 }),
        json!({ "cooldown_secs": "0.5" }),
        json!({ "cooldown_ms": -1.0 }),
        json!({ "cooldown_ms": -500.0 }),
        json!({ "cooldown_ms": "100" }),
    ];

    for payload in invalid_cooldowns {
        let err = bridge
            .invoke("setGestureTrackingMode", Some(payload))
            .expect_err("Expected invalid cooldown to fail");
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for invalid cooldown, got {:?}",
            err
        );
    }
}

/// Adversarial Test 2: State corruption detection on partial validation failure
/// If a payload contains a valid sensitivity (e.g. 0.42) but an INVALID cooldown (e.g. -1.0),
/// the IPC call must fail AND the state must NOT be partially mutated.
#[test]
fn test_gesture_tracking_mode_atomic_state_integrity() {
    let bridge = IpcBridge::default();

    // Read initial status
    let initial_status = bridge
        .invoke("getGestureEngineStatus", None)
        .expect("get status failed");
    let initial_sens = initial_status["sensitivity"].as_f64().unwrap();
    let initial_mode = initial_status["tracking_mode"].as_str().unwrap().to_string();

    // Attempt payload with valid mode and sensitivity, but invalid cooldown
    let malicious_payload = json!({
        "mode": "SingleHand",
        "sensitivity": 0.42,
        "cooldown_secs": -5.0
    });

    let res = bridge.invoke("setGestureTrackingMode", Some(malicious_payload));
    assert!(
        res.is_err(),
        "Call with negative cooldown should return Err"
    );

    // Verify state was NOT corrupted!
    let post_status = bridge
        .invoke("getGestureEngineStatus", None)
        .expect("get status failed");
    let post_sens = post_status["sensitivity"].as_f64().unwrap();
    let post_mode = post_status["tracking_mode"].as_str().unwrap();

    eprintln!(
        "State integrity check: initial_sens={}, post_sens={}, initial_mode={}, post_mode={}",
        initial_sens, post_sens, initial_mode, post_mode
    );

    assert_eq!(
        post_mode, initial_mode,
        "Tracking mode must not be modified when cooldown validation fails!"
    );
    assert!(
        (post_sens - initial_sens).abs() < 1e-4,
        "State corruption detected! Sensitivity was mutated to {} despite call failure!",
        post_sens
    );
}

/// Adversarial Test 3: Fuzzing `setVirtualCameraPrivacy` arguments
#[test]
fn test_fuzz_set_virtual_camera_privacy_arguments() {
    let bridge = IpcBridge::default();

    // 1. Unknown or malformed mode strings
    let invalid_modes = vec![
        json!({ "mode": "InvalidMode" }),
        json!({ "mode": "SuperPrivate" }),
        json!({ "mode": 12345 }),
        json!({ "mode": true }),
        json!({ "mode": [ "Active" ] }),
        json!({ "mode": null }),
    ];

    for payload in invalid_modes {
        let err = bridge
            .invoke("setVirtualCameraPrivacy", Some(payload))
            .expect_err("Expected invalid mode to fail");
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments, got {:?}",
            err
        );
    }

    // 2. Invalid non-boolean types for privacy_muted / dual_mute
    let invalid_types = vec![
        json!({ "privacy_muted": "true" }),
        json!({ "privacy_muted": 1 }),
        json!({ "privacy_muted": [ true ] }),
        json!({ "dual_mute": "false" }),
        json!({ "dual_mute": 0 }),
        json!({ "dual_mute": [ false ] }),
    ];

    for payload in invalid_types {
        let err = bridge
            .invoke("setVirtualCameraPrivacy", Some(payload))
            .expect_err("Expected invalid boolean type to fail");
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for invalid type, got {:?}",
            err
        );
    }
}

/// Adversarial Test 4: IPC State Machine Privacy Leak Test
/// Sequence:
///
/// 1. Set mode to "DualMute"
/// 2. Set mode to "PrivacySlate"
///
/// Assert: Mode MUST be "PrivacySlate", privacy_muted MUST be true,
/// and outbound feed MUST be "privacy_slate_frame" (NOT raw physical frame!).
#[test]
fn test_ipc_transition_dual_mute_to_privacy_slate_privacy_leak() {
    let bridge = IpcBridge::default();

    // Step 1: Engage DualMute
    let res1 = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "DualMute" })))
        .expect("set DualMute failed");
    assert_eq!(res1["dual_muted"], true);
    assert_eq!(res1["privacy_muted"], true);
    assert_eq!(res1["privacy_mode"], "DualMute");

    // Feed frame: Consumer 1 is None, Consumer 2 is slate
    let feed_dual = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 501 })))
        .expect("feed failed");
    assert_eq!(feed_dual["consumer_1_mediapipe"], serde_json::Value::Null);
    assert_eq!(feed_dual["consumer_2_meeting"], "privacy_slate_frame");

    // Step 2: Request PrivacySlate mode
    let res2 = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "PrivacySlate" })))
        .expect("set PrivacySlate failed");

    eprintln!(
        "Response for PrivacySlate request: privacy_mode={}, privacy_muted={}, dual_muted={}",
        res2["privacy_mode"], res2["privacy_muted"], res2["dual_muted"]
    );

    // The user explicitly requested PrivacySlate!
    // Assert that mode is NOT Active, and privacy_muted is true!
    assert_eq!(
        res2["privacy_mode"], "PrivacySlate",
        "CRITICAL PRIVACY BUG: setVirtualCameraPrivacy(mode='PrivacySlate') resulted in mode '{:?}' instead of PrivacySlate!",
        res2["privacy_mode"]
    );
    assert_eq!(
        res2["privacy_muted"], true,
        "CRITICAL PRIVACY BUG: privacy_muted is false after requesting PrivacySlate!"
    );
    assert_eq!(
        res2["dual_muted"], false,
        "dual_muted should be false in PrivacySlate mode"
    );

    // Step 3: Feed frame and verify outbound stream is privacy slate and NOT raw frame
    let feed_slate = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 502 })))
        .expect("feed failed");

    assert_eq!(
        feed_slate["consumer_1_mediapipe"], "raw_frame_502",
        "Consumer 1 (MediaPipe) must receive raw frames in PrivacySlate mode"
    );
    assert_eq!(
        feed_slate["consumer_2_meeting"], "privacy_slate_frame",
        "CRITICAL PRIVACY LEAK: Consumer 2 (meeting feed) received raw frame instead of privacy slate!"
    );
}

/// Adversarial Test 5: Comprehensive Fuzzing of all 5 M11 IPC commands
/// Asserts strict transactional rejection with IpcError::InvalidArguments and 0 state side-effects.
#[test]
fn test_fuzz_all_5_m11_ipc_commands_transactional_zero_side_effects() {
    let bridge = IpcBridge::default();

    // ------------------------------------------------------------------------
    // Command 1: setGestureTrackingMode (transactional atomicity)
    // ------------------------------------------------------------------------
    let initial_status = bridge
        .invoke("getGestureEngineStatus", None)
        .expect("get initial status failed");
    let init_mode = initial_status["tracking_mode"].as_str().unwrap().to_string();
    let init_sens = initial_status["sensitivity"].as_f64().unwrap();

    // Case 1A: Valid mode ("SingleHand") + invalid sensitivity (-0.5)
    // Must fail, and mode MUST NOT become "SingleHand"
    let err_1a = bridge
        .invoke(
            "setGestureTrackingMode",
            Some(json!({ "mode": "SingleHand", "sensitivity": -0.5 })),
        )
        .expect_err("Expected failure on negative sensitivity");
    assert!(matches!(err_1a, IpcError::InvalidArguments(_)));

    let status_after_1a = bridge.invoke("getGestureEngineStatus", None).unwrap();
    assert_eq!(
        status_after_1a["tracking_mode"], init_mode,
        "0 side-effect violation: mode mutated despite sensitivity failure"
    );
    assert_eq!(
        status_after_1a["sensitivity"], init_sens,
        "0 side-effect violation: sensitivity mutated on failure"
    );

    // Case 1B: Valid mode ("SingleHand") + valid sensitivity (0.8) + invalid cooldown_ms (-50.0)
    let err_1b = bridge
        .invoke(
            "setGestureTrackingMode",
            Some(json!({ "mode": "SingleHand", "sensitivity": 0.8, "cooldown_ms": -50.0 })),
        )
        .expect_err("Expected failure on negative cooldown_ms");
    assert!(matches!(err_1b, IpcError::InvalidArguments(_)));

    let status_after_1b = bridge.invoke("getGestureEngineStatus", None).unwrap();
    assert_eq!(
        status_after_1b["tracking_mode"], init_mode,
        "0 side-effect violation: mode mutated despite cooldown_ms failure"
    );
    assert_eq!(
        status_after_1b["sensitivity"], init_sens,
        "0 side-effect violation: sensitivity mutated despite cooldown_ms failure"
    );

    // ------------------------------------------------------------------------
    // Command 2: setVirtualCameraPrivacy (transactional atomicity)
    // ------------------------------------------------------------------------
    // First, establish DualMute state
    bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "DualMute" })))
        .expect("set DualMute failed");

    // Case 2A: Invalid mode string when in DualMute -> state must stay DualMute
    let err_2a = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "HackedMode" })))
        .expect_err("Expected failure on unknown mode");
    assert!(matches!(err_2a, IpcError::InvalidArguments(_)));

    let cam_status_2a = bridge.invoke("getVirtualCameraStatus", None).unwrap();
    assert_eq!(cam_status_2a["privacy_mode"], "DualMute");
    assert_eq!(cam_status_2a["dual_muted"], true);
    assert_eq!(cam_status_2a["privacy_muted"], true);

    // Case 2B: Partial valid privacy_muted (false) with invalid dual_mute ("string")
    // If validation is not transactional, privacy_muted might be set before dual_mute errors!
    let err_2b = bridge
        .invoke(
            "setVirtualCameraPrivacy",
            Some(json!({ "privacy_muted": false, "dual_mute": "invalid_type" })),
        )
        .expect_err("Expected failure on non-boolean dual_mute");
    assert!(matches!(err_2b, IpcError::InvalidArguments(_)));

    let cam_status_2b = bridge.invoke("getVirtualCameraStatus", None).unwrap();
    assert_eq!(
        cam_status_2b["privacy_mode"], "DualMute",
        "0 side-effect violation: privacy_mode altered despite argument failure"
    );
    assert_eq!(
        cam_status_2b["dual_muted"], true,
        "0 side-effect violation: dual_muted altered despite argument failure"
    );

    // ------------------------------------------------------------------------
    // Command 3: feedCameraFrame (argument fuzzing & zero side-effects)
    // ------------------------------------------------------------------------
    let frames_before = bridge.invoke("getVirtualCameraStatus", None).unwrap()["frames_fanned_out"]
        .as_u64()
        .unwrap();

    let invalid_frame_ids = vec![
        json!({ "frame_id": -1 }),
        json!({ "frame_id": -99999 }),
        json!({ "frame_id": "frame_99" }),
        json!({ "frame_id": true }),
        json!({ "frame_id": [ 1, 2, 3 ] }),
        json!({ "frame_id": { "id": 10 } }),
        json!({ "frame_id": null }),
    ];

    for invalid_payload in invalid_frame_ids {
        let err = bridge
            .invoke("feedCameraFrame", Some(invalid_payload))
            .expect_err("feedCameraFrame must reject invalid frame_id");
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments, got {:?}",
            err
        );
    }

    let frames_after = bridge.invoke("getVirtualCameraStatus", None).unwrap()["frames_fanned_out"]
        .as_u64()
        .unwrap();
    assert_eq!(
        frames_before, frames_after,
        "0 side-effect violation: frames_fanned_out incremented during rejected calls"
    );

    // ------------------------------------------------------------------------
    // Commands 4, 5, 6: Query commands with unexpected payloads
    // ------------------------------------------------------------------------
    // getGestureEngineStatus
    let q1 = bridge
        .invoke("getGestureEngineStatus", Some(json!({ "unexpected": 42 })))
        .expect("getGestureEngineStatus query must succeed safely");
    assert_eq!(q1["status"], "success");

    // getVirtualCameraStatus
    let q2 = bridge
        .invoke("getVirtualCameraStatus", Some(json!({ "garbage": [ "a", "b" ] })))
        .expect("getVirtualCameraStatus query must succeed safely");
    assert_eq!(q2["status"], "success");

    // getMultiHandGestureStatus
    let q3 = bridge
        .invoke("getMultiHandGestureStatus", Some(json!({ "invalid_key": true })))
        .expect("getMultiHandGestureStatus query must succeed safely");
    assert_eq!(q3["status"], "success");
}

/// Adversarial Test 6: Exhaustive Privacy Mode Lifecycle & Zero Leakage
/// Validates that across all combinations of transitions between Active, PrivacySlate,
/// and DualMute, Consumer 2 NEVER receives a raw physical frame when privacy is requested.
#[test]
fn test_exhaustive_privacy_mode_lifecycle_and_zero_leakage() {
    let bridge = IpcBridge::default();

    // 1. Initial state: Active
    bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "Active" })))
        .expect("set Active failed");
    let feed0 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 1001 })))
        .expect("feed 1001 failed");
    assert_eq!(feed0["consumer_1_mediapipe"], "raw_frame_1001");
    assert_eq!(feed0["consumer_2_meeting"], "outbound_frame_1001");

    // 2. Transition Active -> PrivacySlate
    let res_ps = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "PrivacySlate" })))
        .expect("set PrivacySlate failed");
    assert_eq!(res_ps["privacy_mode"], "PrivacySlate");
    assert_eq!(res_ps["privacy_muted"], true);
    assert_eq!(res_ps["dual_muted"], false);

    let feed1 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 1002 })))
        .expect("feed 1002 failed");
    assert_eq!(feed1["consumer_1_mediapipe"], "raw_frame_1002", "C1 must receive raw frames");
    assert_eq!(feed1["consumer_2_meeting"], "privacy_slate_frame", "C2 must receive privacy slate");

    // 3. Transition PrivacySlate -> DualMute
    let res_dm = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "DualMute" })))
        .expect("set DualMute failed");
    assert_eq!(res_dm["privacy_mode"], "DualMute");
    assert_eq!(res_dm["privacy_muted"], true);
    assert_eq!(res_dm["dual_muted"], true);

    let feed2 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 1003 })))
        .expect("feed 1003 failed");
    assert_eq!(feed2["consumer_1_mediapipe"], Value::Null, "C1 must be muted in DualMute");
    assert_eq!(feed2["consumer_2_meeting"], "privacy_slate_frame", "C2 must be slate in DualMute");

    // 4. Critical Transition: DualMute -> PrivacySlate
    let res_back_ps = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "PrivacySlate" })))
        .expect("set PrivacySlate from DualMute failed");
    assert_eq!(res_back_ps["privacy_mode"], "PrivacySlate");
    assert_eq!(res_back_ps["privacy_muted"], true);
    assert_eq!(res_back_ps["dual_muted"], false);

    let feed3 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 1004 })))
        .expect("feed 1004 failed");
    assert_eq!(feed3["consumer_1_mediapipe"], "raw_frame_1004", "C1 must resume raw frames");
    assert_eq!(
        feed3["consumer_2_meeting"], "privacy_slate_frame",
        "ZERO-LEAK INVARIANT: C2 MUST NOT leak raw frame when returning to PrivacySlate!"
    );

    // 5. DualMute via dual_mute flag toggle
    bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "dual_mute": true })))
        .expect("set dual_mute=true failed");
    let feed4 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 1005 })))
        .expect("feed 1005 failed");
    assert_eq!(feed4["consumer_1_mediapipe"], Value::Null);
    assert_eq!(feed4["consumer_2_meeting"], "privacy_slate_frame");

    // 6. Release dual_mute flag: should restore PrivacySlate because privacy_muted was previously active!
    let res_rel_dm = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "dual_mute": false })))
        .expect("set dual_mute=false failed");
    assert_eq!(
        res_rel_dm["privacy_mode"], "PrivacySlate",
        "Releasing dual_mute must restore PrivacySlate if privacy mute was active!"
    );
    assert_eq!(res_rel_dm["privacy_muted"], true);

    let feed5 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 1006 })))
        .expect("feed 1006 failed");
    assert_eq!(feed5["consumer_1_mediapipe"], "raw_frame_1006");
    assert_eq!(
        feed5["consumer_2_meeting"], "privacy_slate_frame",
        "ZERO-LEAK INVARIANT: C2 MUST NOT leak raw frame on dual_mute release!"
    );

    // 7. Explicit un-mute back to Active
    let res_active = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "privacy_muted": false })))
        .expect("unmute failed");
    assert_eq!(res_active["privacy_mode"], "Active");
    assert_eq!(res_active["privacy_muted"], false);

    let feed6 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 1007 })))
        .expect("feed 1007 failed");
    assert_eq!(feed6["consumer_1_mediapipe"], "raw_frame_1007");
    assert_eq!(feed6["consumer_2_meeting"], "outbound_frame_1007");
}

/// Adversarial Test 7: Multi-threaded Concurrent IPC Fuzzing Stress Test
/// Runs concurrent threads hammering invalid arguments while validating that bridge
/// state remains uncorrupted and never panics.
#[test]
fn test_adversarial_multithreaded_ipc_fuzzing() {
    let bridge = Arc::new(IpcBridge::default());
    let mut handles = Vec::new();

    // 4 threads fuzzing setGestureTrackingMode
    for _ in 0..4 {
        let b = Arc::clone(&bridge);
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                let payload = if i % 2 == 0 {
                    json!({ "mode": "InvalidMode", "sensitivity": -0.5 })
                } else {
                    json!({ "sensitivity": 2.5, "cooldown_secs": -1.0 })
                };
                let err = b.invoke("setGestureTrackingMode", Some(payload));
                assert!(err.is_err(), "Must reject invalid payload");
            }
        }));
    }

    // 4 threads fuzzing setVirtualCameraPrivacy
    for _ in 0..4 {
        let b = Arc::clone(&bridge);
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                let payload = if i % 2 == 0 {
                    json!({ "mode": "InvalidState" })
                } else {
                    json!({ "privacy_muted": "not_bool", "dual_mute": 99 })
                };
                let err = b.invoke("setVirtualCameraPrivacy", Some(payload));
                assert!(err.is_err(), "Must reject invalid payload");
            }
        }));
    }

    // 2 threads fuzzing feedCameraFrame
    for _ in 0..2 {
        let b = Arc::clone(&bridge);
        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                let err = b.invoke("feedCameraFrame", Some(json!({ "frame_id": -99 })));
                assert!(err.is_err(), "Must reject negative frame_id");
            }
        }));
    }

    for h in handles {
        h.join().expect("Fuzzing thread panicked!");
    }

    // Validate bridge remains fully operational and uncorrupted
    let status_cam = bridge.invoke("getVirtualCameraStatus", None).unwrap();
    assert_eq!(status_cam["status"], "success");

    let status_gest = bridge.invoke("getGestureEngineStatus", None).unwrap();
    assert_eq!(status_gest["status"], "success");
    let sens = status_gest["sensitivity"].as_f64().unwrap();
    assert!(sens > 0.0 && sens <= 1.0, "Sensitivity corrupted after multithreaded fuzzing: {}", sens);
}
