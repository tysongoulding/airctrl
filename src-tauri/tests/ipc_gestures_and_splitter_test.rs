use airctrl_shell::{IpcBridge, IpcError};
use serde_json::json;

#[test]
fn test_ipc_get_gesture_engine_status() {
    let bridge = IpcBridge::default();

    // camelCase
    let res = bridge.invoke("getGestureEngineStatus", None).expect("failed");
    assert_eq!(res["status"], "success");
    assert_eq!(res["tracking_mode"], "DualHand");
    assert_eq!(res["active_hands_count"], 0);
    assert!(res["fps"].as_f64().is_some());

    // snake_case
    let res_snake = bridge.invoke("get_gesture_engine_status", None).expect("failed");
    assert_eq!(res_snake["status"], "success");
    assert_eq!(res_snake["tracking_mode"], "DualHand");
}

#[test]
fn test_ipc_set_gesture_tracking_mode_and_validation() {
    let bridge = IpcBridge::default();

    // Set to SingleHand
    let res = bridge
        .invoke("setGestureTrackingMode", Some(json!({ "mode": "SingleHand", "sensitivity": 0.9, "cooldown_secs": 0.5 })))
        .expect("failed");
    assert_eq!(res["status"], "success");
    assert_eq!(res["mode"], "SingleHand");
    assert!((res["sensitivity"].as_f64().unwrap() - 0.9).abs() < 1e-4);

    // Verify in engine status
    let status = bridge.invoke("getGestureEngineStatus", None).expect("failed");
    assert_eq!(status["tracking_mode"], "SingleHand");

    // Invalid mode name -> error
    let err = bridge
        .invoke("setGestureTrackingMode", Some(json!({ "mode": "TripleHand" })))
        .unwrap_err();
    match err {
        IpcError::InvalidArguments(msg) => assert!(msg.contains("TripleHand")),
        other => panic!("Expected InvalidArguments, got {:?}", other),
    }

    // Invalid mode type -> error
    let err_type = bridge
        .invoke("setGestureTrackingMode", Some(json!({ "mode": 123 })))
        .unwrap_err();
    assert!(matches!(err_type, IpcError::InvalidArguments(_)));

    // Sensitivity boundary tests: <= 0.0 or > 1.0 -> error
    let err_sens_0 = bridge
        .invoke("setGestureTrackingMode", Some(json!({ "mode": "DualHand", "sensitivity": 0.0 })))
        .unwrap_err();
    assert!(matches!(err_sens_0, IpcError::InvalidArguments(_)));

    let err_sens_high = bridge
        .invoke("setGestureTrackingMode", Some(json!({ "mode": "DualHand", "sensitivity": 1.5 })))
        .unwrap_err();
    assert!(matches!(err_sens_high, IpcError::InvalidArguments(_)));

    // Negative cooldown -> error
    let err_cd = bridge
        .invoke("setGestureTrackingMode", Some(json!({ "cooldown_secs": -0.1 })))
        .unwrap_err();
    assert!(matches!(err_cd, IpcError::InvalidArguments(_)));
}

#[test]
fn test_ipc_get_and_set_virtual_camera_status() {
    let bridge = IpcBridge::default();

    // Default status
    let status = bridge.invoke("getVirtualCameraStatus", None).expect("failed");
    assert_eq!(status["status"], "success");
    assert_eq!(status["device_name"], "AirCtrl Virtual Camera");
    assert_eq!(status["active_consumers"], 2);
    assert_eq!(status["consumer_count"], 2);
    assert_eq!(status["privacy_muted"], false);
    assert_eq!(status["dual_muted"], false);
    assert_eq!(status["resolution"]["width"], 1280);
    assert_eq!(status["resolution"]["height"], 720);

    // Set privacy mute = true
    let priv_res = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "privacy_muted": true })))
        .expect("failed");
    assert_eq!(priv_res["status"], "success");
    assert_eq!(priv_res["privacy_muted"], true);
    assert_eq!(priv_res["dual_muted"], false);

    // Query status again
    let status_muted = bridge.invoke("getVirtualCameraStatus", None).expect("failed");
    assert_eq!(status_muted["privacy_muted"], true);

    // Set mode by string: "DualMute"
    let dual_res = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "DualMute" })))
        .expect("failed");
    assert_eq!(dual_res["dual_muted"], true);
    assert_eq!(dual_res["privacy_muted"], true);

    // Set mode back to "Active"
    let active_res = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "Active" })))
        .expect("failed");
    assert_eq!(active_res["dual_muted"], false);
    assert_eq!(active_res["privacy_muted"], false);

    // Invalid mode string -> error
    let err_mode = bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "mode": "InvalidMode" })))
        .unwrap_err();
    assert!(matches!(err_mode, IpcError::InvalidArguments(_)));
}

#[test]
fn test_ipc_feed_camera_frame_fanout() {
    let bridge = IpcBridge::default();

    // Feed frame 100 in Active mode
    let feed1 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 100 })))
        .expect("failed");
    assert_eq!(feed1["device"], "AirCtrl Virtual Camera");
    assert_eq!(feed1["frame_id"], 100);
    assert_eq!(feed1["consumer_1_mediapipe"], "raw_frame_100");
    assert_eq!(feed1["consumer_2_meeting"], "outbound_frame_100");
    assert_eq!(feed1["privacy_muted"], false);
    assert_eq!(feed1["dual_muted"], false);

    // Engage privacy mute
    bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "privacy_muted": true })))
        .expect("failed");

    // Feed frame 101 in PrivacySlate mode
    // Consumer 1 (MediaPipe) CONTINUES receiving raw frame!
    let feed2 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 101 })))
        .expect("failed");
    assert_eq!(feed2["consumer_1_mediapipe"], "raw_frame_101");
    assert_eq!(feed2["consumer_2_meeting"], "privacy_slate_frame");
    assert_eq!(feed2["privacy_muted"], true);

    // Engage DualMute
    bridge
        .invoke("setVirtualCameraPrivacy", Some(json!({ "dual_mute": true })))
        .expect("failed");

    // Feed frame 102 in DualMute mode: Consumer 1 is None, Consumer 2 is slate
    let feed3 = bridge
        .invoke("feedCameraFrame", Some(json!({ "frame_id": 102 })))
        .expect("failed");
    assert_eq!(feed3["consumer_1_mediapipe"], serde_json::Value::Null);
    assert_eq!(feed3["consumer_2_meeting"], "privacy_slate_frame");
    assert_eq!(feed3["dual_muted"], true);

    // Verify frames_fanned_out counter
    let status = bridge.invoke("getVirtualCameraStatus", None).expect("failed");
    assert_eq!(status["frames_fanned_out"], 3);
}

#[test]
fn test_ipc_get_multi_hand_gesture_status() {
    let bridge = IpcBridge::default();

    let res = bridge.invoke("getMultiHandGestureStatus", None).expect("failed");
    assert_eq!(res["status"], "success");
    assert_eq!(res["detected_hands"], 0);
    assert!(res["hands"].is_object());
    assert!(res["active_gestures"].is_object());
    assert!(res["active_gestures"]["pinch"].is_object());
    assert!(res["active_gestures"]["two_finger"].is_object());
}
