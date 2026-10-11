//! challenger_m7_tauri_adversarial_test.rs
//!
//! Milestone M7 Phase 2 Tier 5 Adversarial Coverage Hardening:
//! Deep white-box adversarial verification of `src-tauri`:
//! 1. Headless IPC bridge dispatcher 7-command verification & error contracts:
//!    - All 7 core commands (startRecording, stopRecording, triggerAction, toggleRadialDial,
//!      getSettings, updateSettings, getMeetingStatus) + triggerCameraKillSwitch.
//!    - Whitespace trimming, empty command rejection, unknown command rejection.
//! 2. Transactional type validation in `updateSettings`:
//!    - Non-object roots (strings, arrays, booleans, numbers, nulls).
//!    - Strict field validation (`hotkey_stt`, `hotkey_dial`, `camera_auto_kill`,
//!      `compliance_audio_enabled`, `radial_dial_sectors`).
//!    - Transactional rollback guarantees on partial payload failure.
//!    - Dynamic synchronization with `dial_overlay` sector count & active sector invalidation.
//! 3. Audio buffer draining in `stop_recording` (Anti-Replay Protection):
//!    - Draining guarantees on audio samples buffer.
//!    - Prevention of accidental transcription replay attacks.
//! 4. Multi-threaded concurrent burst stress:
//!    - 20 concurrent threads hammering all IPC endpoints simultaneously.
//!    - Concurrency safety with RwLock write and read lock interleaving.
//! 5. Static frontend build fallback in `ui/dist/index.html`:
//!    - Verifies file integrity, HTML5 structure, embedded mock IPC bridge handlers,
//!      and accessibility/HUD DOM components.

use airctrl_shell::{IpcBridge, IpcError};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

// ============================================================================
// 1. Headless IPC Bridge 7 Core Commands & Dispatcher Routing
// ============================================================================

#[test]
fn test_ipc_bridge_all_seven_commands_and_whitespace_trimming() {
    let bridge = IpcBridge::default();

    // 1. startRecording with leading/trailing whitespace
    let res_start = bridge
        .invoke("  startRecording  ", None)
        .expect("startRecording should succeed");
    assert_eq!(res_start["status"], "RecordingStarted");

    // 2. stopRecording
    let res_stop = bridge
        .invoke("stopRecording\n", None)
        .expect("stopRecording should succeed");
    assert_eq!(res_stop["status"], "TranscriptionReady");
    assert!(res_stop["text"].is_string());

    // 3. triggerAction with WindowSnap
    let res_action = bridge
        .invoke("triggerAction", Some(json!({ "action": "WindowSnap", "direction": "left" })))
        .expect("triggerAction should succeed");
    assert_eq!(res_action["success"], true);

    // 4. toggleRadialDial
    let res_dial = bridge
        .invoke("toggleRadialDial", None)
        .expect("toggleRadialDial should succeed");
    assert!(res_dial["dial_visible"].is_boolean());

    // 5. getSettings
    let res_settings = bridge
        .invoke("getSettings", None)
        .expect("getSettings should succeed");
    assert_eq!(res_settings["hotkey_stt"], "Ctrl+Spacebar");
    assert_eq!(res_settings["hotkey_dial"], "Ctrl+Alt");
    assert_eq!(res_settings["radial_dial_sectors"], 6);

    // 6. updateSettings
    let res_update = bridge
        .invoke("updateSettings", Some(json!({ "radial_dial_sectors": 7 })))
        .expect("updateSettings should succeed");
    assert_eq!(res_update["success"], true);
    assert_eq!(res_update["settings"]["radial_dial_sectors"], 7);

    // 7. getMeetingStatus
    let res_meeting = bridge
        .invoke("getMeetingStatus", None)
        .expect("getMeetingStatus should succeed");
    assert!(res_meeting["status"] == "Active" || res_meeting["status"] == "Inactive");

    // 8. triggerCameraKillSwitch
    let res_kill = bridge
        .invoke("triggerCameraKillSwitch", None)
        .expect("triggerCameraKillSwitch should succeed");
    assert_eq!(res_kill["success"], true);
    assert_eq!(res_kill["status"], "CameraKilled");

    // 9. Empty and whitespace-only command names rejected
    assert!(matches!(
        bridge.invoke("", None).unwrap_err(),
        IpcError::EmptyCommand
    ));
    assert!(matches!(
        bridge.invoke("   \t  \n  ", None).unwrap_err(),
        IpcError::EmptyCommand
    ));

    // 10. Unknown command names rejected
    assert!(matches!(
        bridge.invoke("nonExistentRpcCommand", None).unwrap_err(),
        IpcError::CommandNotFound(_)
    ));
}

// ============================================================================
// 2. Transactional Type Validation & Atomic Rollback in `updateSettings`
// ============================================================================

#[test]
fn test_update_settings_transactional_type_validation_and_rollback() {
    let bridge = IpcBridge::default();

    let initial_stt = bridge.state.config.read().stt_hotkey.clone();
    let initial_dial = bridge.state.config.read().dial_hotkey.clone();
    let initial_sectors = bridge.state.config.read().radial_dial_sectors;

    // 1. Non-object roots rejected
    let non_objects = vec![
        json!("string_payload"),
        json!(12345),
        json!(true),
        json!(false),
        json!([1, 2, 3]),
        json!(null),
    ];
    for root in non_objects {
        let err = bridge
            .invoke("updateSettings", Some(root))
            .expect_err("Non-object root must fail");
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    // 2. Partial payload failure with atomic rollback guarantee:
    // Supply valid hotkey_stt, but invalid radial_dial_sectors (99).
    // The entire transaction must fail and hotkey_stt must NOT be committed!
    let mixed_hostile_req = json!({
        "hotkey_stt": "Ctrl+Shift+Z",
        "radial_dial_sectors": 99
    });
    let err_mixed = bridge
        .invoke("updateSettings", Some(mixed_hostile_req))
        .expect_err("Invalid sector count must abort update");
    assert!(matches!(err_mixed, IpcError::InvalidArguments(_)));

    // Verify atomic rollback
    assert_eq!(
        bridge.state.config.read().stt_hotkey,
        initial_stt,
        "Transaction rollback failed: hotkey_stt was mutated!"
    );
    assert_eq!(
        bridge.state.config.read().radial_dial_sectors,
        initial_sectors
    );

    // 3. Field-specific boundary attacks
    // A. hotkey_stt without "Ctrl+" prefix
    let invalid_stt = ["Alt+Space", "Shift+Space", "Space", "ctrl+space", ""];
    for hk in invalid_stt {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "hotkey_stt": hk })))
            .unwrap_err();
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    // B. hotkey_dial without "Ctrl+" prefix
    let invalid_dial = ["Command+Alt", "Super+Alt", "Alt+Ctrl", ""];
    for hk in invalid_dial {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "hotkey_dial": hk })))
            .unwrap_err();
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    // C. camera_auto_kill type mismatch
    let invalid_bools = vec![json!("true"), json!(1), json!(0), json!(null), json!([])];
    for b in invalid_bools {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "camera_auto_kill": b })))
            .unwrap_err();
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    // D. compliance_audio_enabled type mismatch
    let invalid_comp = vec![json!("false"), json!(1), json!(null), json!({})];
    for c in invalid_comp {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "compliance_audio_enabled": c })))
            .unwrap_err();
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    // E. radial_dial_sectors boundaries: valid strictly 4..=8
    let invalid_sec = vec![json!(0), json!(1), json!(2), json!(3), json!(9), json!(10), json!(-1), json!(5.5)];
    for s in invalid_sec {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "radial_dial_sectors": s })))
            .unwrap_err();
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    // Verify config state remained completely untouched throughout all failed attempts
    assert_eq!(bridge.state.config.read().stt_hotkey, initial_stt);
    assert_eq!(bridge.state.config.read().dial_hotkey, initial_dial);
    assert_eq!(bridge.state.config.read().radial_dial_sectors, initial_sectors);
}

// ============================================================================
// 3. Audio Buffer Draining & Anti-Replay Protection in `stop_recording`
// ============================================================================

#[test]
fn test_audio_buffer_draining_anti_replay_protection() {
    let bridge = IpcBridge::default();

    // 1. Calling stopRecording without startRecording must fail
    let err1 = bridge
        .invoke("stopRecording", None)
        .expect_err("stopRecording on empty buffer must fail");
    assert!(matches!(err1, IpcError::ExecutionFailed(_)));

    // 2. Cycle 1: Start -> Stop
    bridge.invoke("startRecording", None).unwrap();
    let res1 = bridge.invoke("stopRecording", None).unwrap();
    assert_eq!(res1["status"], "TranscriptionReady");

    // 3. Immediate Second Stop without Start:
    // Buffer was drained by previous stop, so it MUST FAIL!
    // This prevents accidental replay of previous recorded audio.
    let err2 = bridge
        .invoke("stopRecording", None)
        .expect_err("Second stopRecording without startRecording must fail due to buffer drain");
    assert!(matches!(err2, IpcError::ExecutionFailed(_)));

    // 4. Consecutive Cycles: Start -> Stop -> Stop (fails) -> Start -> Stop -> Stop (fails)
    for _ in 0..5 {
        bridge.invoke("startRecording", None).unwrap();
        let res = bridge.invoke("stopRecording", None).unwrap();
        assert_eq!(res["status"], "TranscriptionReady");

        let err = bridge.invoke("stopRecording", None).unwrap_err();
        assert!(matches!(err, IpcError::ExecutionFailed(_)));
    }
}

// ============================================================================
// 4. Multi-Threaded Concurrent Burst Stress
// ============================================================================

#[test]
fn test_ipc_multi_threaded_concurrent_burst_hammer_20_threads() {
    let bridge = Arc::new(IpcBridge::default());
    let num_threads = 20;
    let calls_per_thread = 100;
    let mut handles = Vec::with_capacity(num_threads);

    for thread_id in 0..num_threads {
        let b = Arc::clone(&bridge);
        handles.push(thread::spawn(move || {
            for i in 0..calls_per_thread {
                match (thread_id + i) % 7 {
                    0 => {
                        let _ = b.invoke("getSettings", None);
                    }
                    1 => {
                        let _ = b.invoke("toggleRadialDial", None);
                    }
                    2 => {
                        let _ = b.invoke(
                            "triggerAction",
                            Some(json!({ "action": "WindowSnap", "direction": "right" })),
                        );
                    }
                    3 => {
                        let _ = b.invoke("getMeetingStatus", None);
                    }
                    4 => {
                        let _ = b.invoke("startRecording", None);
                        let _ = b.invoke("stopRecording", None);
                    }
                    5 => {
                        let _ = b.invoke(
                            "updateSettings",
                            Some(json!({
                                "hotkey_stt": "Ctrl+Spacebar",
                                "radial_dial_sectors": (4 + (i % 5))
                            })),
                        );
                    }
                    6 => {
                        let _ = b.invoke("triggerCameraKillSwitch", None);
                    }
                    _ => unreachable!(),
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("Worker thread panicked in IPC concurrency hammer");
    }

    // Verify bridge is in consistent state post-stress
    let final_settings = bridge.invoke("getSettings", None).unwrap();
    let sectors = final_settings["radial_dial_sectors"].as_u64().unwrap();
    assert!((4..=8).contains(&sectors));
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), sectors as usize);
}

// ============================================================================
// 5. Static Frontend Build Fallback in `ui/dist/index.html`
// ============================================================================

#[test]
fn test_static_frontend_build_fallback_integrity() {
    let html_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/dist/index.html");
    assert!(
        html_path.exists(),
        "ui/dist/index.html MUST exist as the static frontend fallback artifact"
    );

    let html_content = fs::read_to_string(&html_path)
        .expect("Failed to read ui/dist/index.html fallback file");

    // 1. Verify HTML5 Standard Structure
    assert!(html_content.contains("<!DOCTYPE html>"));
    assert!(html_content.contains("<html lang=\"en\" class=\"dark\">"));
    assert!(html_content.contains("<title>airctrl Desktop HUD</title>"));

    // 2. Verify Embedded Mock Tauri IPC Bridge
    assert!(
        html_content.contains("window.__TAURI_INVOKE__"),
        "index.html must provide window.__TAURI_INVOKE__ bridge fallback"
    );

    // 3. Verify All Required Handlers are implemented in mockHandlers
    let required_handlers = [
        "startRecording",
        "stopRecording",
        "triggerAction",
        "toggleRadialDial",
        "getSettings",
        "updateSettings",
        "getMeetingStatus",
        "triggerCameraKillSwitch",
    ];
    for handler in required_handlers {
        assert!(
            html_content.contains(handler),
            "mockHandlers in ui/dist/index.html must implement '{}'",
            handler
        );
    }

    // 4. Verify HUD DOM Elements & Controls
    assert!(html_content.contains("Speech-to-Text"));
    assert!(html_content.contains("Radial Dial & Gestures"));
    assert!(html_content.contains("Meeting Privacy"));
    assert!(html_content.contains("LAN Deployment"));
    assert!(html_content.contains("opacitySlider"));
    assert!(html_content.contains("Ctrl+Spacebar"));
    assert!(html_content.contains("Ctrl+Alt"));
}
