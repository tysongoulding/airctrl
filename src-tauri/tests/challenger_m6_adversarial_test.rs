//! challenger_m6_adversarial_test.rs - Empirical Adversarial Test Suite for Milestone M6
//!
//! Adversarially validates:
//! 1. Radial Dial Component Specifications & In-Memory State Machine
//!    - Sector inventory: exactly 6 sectors (IDs 0..5)
//!    - Inactivity auto-dismiss: exactly 3.0 seconds
//!    - Deadband boundary: r < 0.10 activates deadzone (active_sector is None)
//!    - Angular hysteresis: 0.05 rad suppressing boundary jitter
//! 2. UI Settings Contracts & Validation:
//!    - Settings keys: hotkey_stt: "Ctrl+Spacebar", hotkey_dial: "Ctrl+Alt", push_to_talk: true,
//!      camera_auto_kill: true, compliance_audio_enabled: true
//!    - Bounds validation: radial_dial_sectors strictly 4..=8, hotkeys prefixed with "Ctrl+"
//! 3. Concurrency Stress:
//!    - Multi-threaded Arc<IpcBridge> concurrent access across 16 worker threads

use airctrl_actions::hotkey::RadialDialOverlay;
use airctrl_gestures::mapper::SpatialCoordinateMapper;
use airctrl_shell::{IpcBridge, IpcError};
use serde_json::json;
use std::sync::Arc;
use std::thread;

// ===========================================================================
// 1. Radial Dial Component Specifications & In-Memory State Machine
// ===========================================================================

#[test]
fn test_adversarial_radial_dial_overlay_contracts() {
    let mut dial = RadialDialOverlay::new();

    // 1. Sector inventory & timeout specifications
    assert_eq!(dial.sector_count(), 6, "Default sector count must be exactly 6");
    assert_eq!(dial.timeout_secs(), 3.0, "Inactivity timeout must be exactly 3.0 seconds");
    assert!(!dial.is_visible(), "Initial state must be hidden");
    assert_eq!(dial.active_sector(), None, "Initial active sector must be None");

    // 2. Toggle visibility
    dial.toggle();
    assert!(dial.is_visible(), "Toggle from hidden must make visible");

    // 3. Sector boundaries [0..5]
    dial.hover_sector(0);
    assert_eq!(dial.active_sector(), Some(0), "Sector 0 must be valid");

    dial.hover_sector(5);
    assert_eq!(dial.active_sector(), Some(5), "Sector 5 must be valid");

    // 4. Out-of-bounds rejection (negative and >= 6)
    dial.hover_sector(-1);
    assert_eq!(dial.active_sector(), Some(5), "Negative sector index must be ignored");

    dial.hover_sector(6);
    assert_eq!(dial.active_sector(), Some(5), "Sector index 6 must be ignored");

    dial.hover_sector(999);
    assert_eq!(dial.active_sector(), Some(5), "Out-of-bounds sector 999 must be ignored");

    // 5. Inactivity timeout dismiss
    dial.on_inactivity_timeout();
    assert!(!dial.is_visible(), "Inactivity timeout must hide dial");
    assert_eq!(dial.active_sector(), None, "Inactivity timeout must reset active sector");
}

#[test]
fn test_adversarial_spatial_deadband_and_hysteresis() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10).with_hysteresis(0.05);

    // 1. Center deadband (r = 0.0 < 0.10)
    let center = mapper.map_coords(0.5, 0.5);
    assert!(center.in_deadzone, "Center must be in deadzone");
    assert_eq!(center.active_sector, None, "Center active_sector must be None");
    assert_eq!(center.normalized_radius, 0.0);

    // 2. Deadband boundary: r = 0.098 (< 0.10)
    let under = mapper.map_coords(0.50, 0.549); // dy = 0.049 -> r = 0.098
    assert!(under.in_deadzone, "r = 0.098 must be in deadzone");
    assert_eq!(under.active_sector, None);

    // 3. Deadband activation: r = 0.104 (>= 0.10)
    let over = mapper.map_coords(0.50, 0.552); // dy = 0.052 -> r = 0.104
    assert!(!over.in_deadzone, "r = 0.104 must NOT be in deadzone");
    assert!(over.active_sector.is_some(), "r = 0.104 must activate a sector");

    // 4. Angular hysteresis at sector 0 <-> 1 boundary (theta = pi / 3 ~ 1.047 rad)
    mapper.reset();
    // Establish sector 0
    let s0 = mapper.map_coords(0.5 + 0.35 * 1.00_f32.cos(), 0.5 + 0.35 * 1.00_f32.sin());
    assert_eq!(s0.active_sector, Some(0));

    // Flutter past boundary (1.06 rad): delta ~ 0.013 rad < 0.05 rad hysteresis -> STAYS sector 0!
    let s0_flutter = mapper.map_coords(0.5 + 0.35 * 1.06_f32.cos(), 0.5 + 0.35 * 1.06_f32.sin());
    assert_eq!(
        s0_flutter.active_sector,
        Some(0),
        "0.05 rad hysteresis must suppress boundary flutter into sector 1"
    );

    // Cross firmly past hysteresis (1.15 rad) -> transitions to sector 1
    let s1 = mapper.map_coords(0.5 + 0.35 * 1.15_f32.cos(), 0.5 + 0.35 * 1.15_f32.sin());
    assert_eq!(s1.active_sector, Some(1));

    // Reverse flutter back across boundary (1.04 rad) -> STAYS sector 1 due to hysteresis!
    let s1_flutter = mapper.map_coords(0.5 + 0.35 * 1.04_f32.cos(), 0.5 + 0.35 * 1.04_f32.sin());
    assert_eq!(
        s1_flutter.active_sector,
        Some(1),
        "0.05 rad hysteresis must suppress reverse boundary flutter into sector 0"
    );
}

// ===========================================================================
// 2. UI Settings Contracts & Validation
// ===========================================================================

#[test]
fn test_adversarial_ipc_settings_keys_and_values() {
    let bridge = IpcBridge::default();
    let settings = bridge
        .invoke("getSettings", None)
        .expect("getSettings failed");

    // Assert exact key contracts
    assert_eq!(
        settings["hotkey_stt"], "Ctrl+Spacebar",
        "STT hotkey must normalize to Ctrl+Spacebar"
    );
    assert_eq!(settings["hotkey_dial"], "Ctrl+Alt");
    assert_eq!(settings["push_to_talk"], true);
    assert_eq!(settings["camera_auto_kill"], true);
    assert_eq!(settings["compliance_audio_enabled"], true);
    assert_eq!(settings["radial_dial_sectors"], 6);
    assert_eq!(settings["theme"], "dark");
    assert_eq!(settings["hud_opacity"], 0.95);
}

#[test]
fn test_adversarial_ipc_settings_validation_boundaries() {
    let bridge = IpcBridge::default();

    // 1. Hotkey prefix boundary attacks: must start with "Ctrl+"
    let invalid_prefixes = vec![
        "Alt+Space",
        "Shift+Ctrl+X",
        "Space",
        "ctrl+alt", // lowercase
        "",
        "Super+Key",
        "Fn+F1",
    ];
    for hk in invalid_prefixes {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "hotkey_stt": hk })))
            .unwrap_err();
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Expected InvalidArguments for hotkey '{}'",
            hk
        );
    }

    // 2. Valid hotkey updates
    let valid_hotkeys = vec!["Ctrl+Spacebar", "Ctrl+Shift+A", "Ctrl+Alt+D"];
    for hk in valid_hotkeys {
        let res = bridge
            .invoke("updateSettings", Some(json!({ "hotkey_stt": hk })))
            .unwrap_or_else(|e| panic!("Hotkey '{}' should be accepted: {:?}", hk, e));
        assert_eq!(res["success"], true);
    }

    // 3. Sector count bounds: strictly [4..=8]
    let invalid_sectors = vec![0, 1, 2, 3, 9, 10, 100];
    for sec in invalid_sectors {
        let err = bridge
            .invoke("updateSettings", Some(json!({ "radial_dial_sectors": sec })))
            .unwrap_err();
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    let valid_sectors = vec![4, 5, 6, 7, 8];
    for sec in valid_sectors {
        let res = bridge
            .invoke("updateSettings", Some(json!({ "radial_dial_sectors": sec })))
            .unwrap_or_else(|_| panic!("Sector count {} should be accepted", sec));
        assert_eq!(res["success"], true);
        assert_eq!(res["settings"]["radial_dial_sectors"], sec);
    }
}

// ===========================================================================
// 3. Multi-Threaded Concurrency Hammer
// ===========================================================================

#[test]
fn test_adversarial_concurrent_ipc_bridge_hammer() {
    let bridge = Arc::new(IpcBridge::default());
    let mut handles = vec![];

    // 16 concurrent threads each performing 50 operations
    for thread_idx in 0..16 {
        let b = Arc::clone(&bridge);
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                match (thread_idx + i) % 4 {
                    0 => {
                        let res = b.invoke("getSettings", None).unwrap();
                        assert!(res["hotkey_stt"].is_string());
                    }
                    1 => {
                        let _ = b.invoke("toggleRadialDial", None).unwrap();
                    }
                    2 => {
                        let _ = b.invoke("triggerAction", Some(json!({ "action": "WindowFocus" }))).unwrap();
                    }
                    3 => {
                        let _ = b.invoke("getMeetingStatus", None).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("Worker thread panicked under concurrency stress");
    }
}
