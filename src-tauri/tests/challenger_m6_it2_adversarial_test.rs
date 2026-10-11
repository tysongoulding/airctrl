//! challenger_m6_it2_adversarial_test.rs
//!
//! Milestone M6 Iteration 2 Empirical Adversarial Challenge:
//! 1. Radial Dial Dynamic Resizing Stress Test:
//!    - Start with 6 sectors, hover sector 5.
//!    - Resize down to 4 sectors -> active sector resets to None.
//!    - Verify hover works for sectors 0..3, and is rejected for sector >= 4.
//!    - Resize up to 8 sectors -> verify hover works for sectors 0..7, rejected for >= 8.
//!    - Verify visibility flag is preserved across sector count changes (both visible and hidden).
//! 2. Tauri IPC State Synchronization:
//!    - Call updateSettings({"radial_dial_sectors": 8}).
//!    - Verify state.config.read().radial_dial_sectors == 8.
//!    - Verify state.dial_overlay.read().sector_count() == 8.
//!    - Call updateSettings({"radial_dial_sectors": 4}).
//!    - Verify state.dial_overlay.read().sector_count() == 4.
//!    - Verify active sector invalidation through IPC when resizing down while visible.
//!    - Verify transactional safety when invalid sector count is provided.

use airctrl_actions::hotkey::RadialDialOverlay;
use airctrl_shell::{IpcBridge, IpcError};
use serde_json::json;

#[test]
fn test_radial_dial_dynamic_resizing_stress() {
    let mut dial = RadialDialOverlay::new();
    assert_eq!(dial.sector_count(), 6);
    assert!(!dial.is_visible());

    // 1. Start with 6 sectors. Show and hover sector 5.
    dial.show();
    assert!(dial.is_visible());
    dial.hover_sector(5);
    assert_eq!(dial.active_sector(), Some(5), "Sector 5 should be active");

    // 2. Resize down to 4 sectors using set_sector_count(4).
    dial.set_sector_count(4);
    assert_eq!(dial.sector_count(), 4);
    // Verify active sector resets to None (preventing out-of-bounds pointer)
    assert_eq!(
        dial.active_sector(),
        None,
        "Active sector must reset to None when previous index 5 >= 4"
    );
    // Verify visibility flag is preserved across sector count change
    assert!(
        dial.is_visible(),
        "Visibility flag must be preserved after resizing down"
    );

    // 3. Verify hover works for sectors 0..3
    for s in 0..4 {
        dial.hover_sector(s);
        assert_eq!(
            dial.active_sector(),
            Some(s as usize),
            "Hover must work for valid sector {}",
            s
        );
    }

    // Verify hover fails/clamps for sector >= 4 (current active is Some(3))
    dial.hover_sector(4);
    assert_eq!(
        dial.active_sector(),
        Some(3),
        "Sector 4 is out of bounds for sector_count=4 and must be ignored"
    );

    dial.hover_sector(5);
    assert_eq!(
        dial.active_sector(),
        Some(3),
        "Sector 5 is out of bounds for sector_count=4 and must be ignored"
    );

    dial.hover_sector(99);
    assert_eq!(
        dial.active_sector(),
        Some(3),
        "Sector 99 is out of bounds and must be ignored"
    );

    dial.hover_sector(-1);
    assert_eq!(
        dial.active_sector(),
        Some(3),
        "Negative sector index must be ignored"
    );

    // 4. Resize up to 8 sectors using set_sector_count(8).
    dial.set_sector_count(8);
    assert_eq!(dial.sector_count(), 8);
    // Active sector was 3, which is < 8, so it should be preserved
    assert_eq!(
        dial.active_sector(),
        Some(3),
        "Active sector 3 should be preserved when resizing up to 8"
    );
    // Visibility flag preserved
    assert!(
        dial.is_visible(),
        "Visibility flag must be preserved after resizing up"
    );

    // Verify hover works for sectors 0..7
    for s in 0..8 {
        dial.hover_sector(s);
        assert_eq!(
            dial.active_sector(),
            Some(s as usize),
            "Hover must work for sector {} in 8-sector dial",
            s
        );
    }

    // Sector 8 and beyond must be ignored
    dial.hover_sector(8);
    assert_eq!(
        dial.active_sector(),
        Some(7),
        "Sector 8 is out of bounds for 8 sectors and must be ignored"
    );

    dial.hover_sector(12);
    assert_eq!(
        dial.active_sector(),
        Some(7),
        "Sector 12 is out of bounds and must be ignored"
    );

    // 5. Verify visibility preservation when hidden
    dial.hide();
    assert!(!dial.is_visible());
    assert_eq!(dial.active_sector(), None);

    dial.set_sector_count(5);
    assert_eq!(dial.sector_count(), 5);
    assert!(!dial.is_visible(), "Hidden state must be preserved after resize");
    assert_eq!(dial.active_sector(), None);

    // Hover while hidden should NOT activate sectors
    dial.hover_sector(2);
    assert_eq!(
        dial.active_sector(),
        None,
        "Hover while hidden must not activate sector"
    );
}

#[test]
fn test_tauri_ipc_state_synchronization() {
    let bridge = IpcBridge::default();

    // Verify initial state
    assert_eq!(bridge.state.config.read().radial_dial_sectors, 6);
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), 6);

    // 1. Call updateSettings({"radial_dial_sectors": 8})
    let res8 = bridge
        .invoke("updateSettings", Some(json!({ "radial_dial_sectors": 8 })))
        .expect("updateSettings to 8 sectors should succeed");
    assert_eq!(res8["success"], true);

    // Verify state.config.read().radial_dial_sectors == 8
    assert_eq!(
        bridge.state.config.read().radial_dial_sectors,
        8,
        "Config radial_dial_sectors must be 8"
    );

    // Verify state.dial_overlay.read().sector_count == 8
    assert_eq!(
        bridge.state.dial_overlay.read().sector_count(),
        8,
        "Dial overlay sector count must be 8"
    );

    // 2. Call updateSettings({"radial_dial_sectors": 4})
    let res4 = bridge
        .invoke("updateSettings", Some(json!({ "radial_dial_sectors": 4 })))
        .expect("updateSettings to 4 sectors should succeed");
    assert_eq!(res4["success"], true);

    // Verify state.dial_overlay.read().sector_count == 4
    assert_eq!(
        bridge.state.config.read().radial_dial_sectors,
        4,
        "Config radial_dial_sectors must be 4"
    );
    assert_eq!(
        bridge.state.dial_overlay.read().sector_count(),
        4,
        "Dial overlay sector count must be 4"
    );
}

#[test]
fn test_tauri_ipc_active_sector_invalidation_across_resize() {
    let bridge = IpcBridge::default();

    // Resize to 8 sectors and make visible
    bridge
        .invoke("updateSettings", Some(json!({ "radial_dial_sectors": 8 })))
        .unwrap();
    {
        let mut overlay = bridge.state.dial_overlay.write();
        overlay.show();
        overlay.hover_sector(7);
        assert_eq!(overlay.active_sector(), Some(7));
        assert!(overlay.is_visible());
    }

    // Now update settings over IPC to 5 sectors
    let res = bridge
        .invoke("updateSettings", Some(json!({ "radial_dial_sectors": 5 })))
        .unwrap();
    assert_eq!(res["success"], true);

    // Verify overlay: sector 7 was >= 5, so active sector must be reset to None, but remain visible!
    {
        let overlay = bridge.state.dial_overlay.read();
        assert_eq!(overlay.sector_count(), 5);
        assert!(overlay.is_visible(), "Overlay must remain visible after resize");
        assert_eq!(
            overlay.active_sector(),
            None,
            "Out-of-bounds active sector 7 must be reset to None when resized to 5"
        );
    }
}

#[test]
fn test_tauri_ipc_invalid_sector_count_preserves_state() {
    let bridge = IpcBridge::default();
    assert_eq!(bridge.state.config.read().radial_dial_sectors, 6);
    assert_eq!(bridge.state.dial_overlay.read().sector_count(), 6);

    let invalid_sector_payloads = vec![
        json!({ "radial_dial_sectors": 3 }),
        json!({ "radial_dial_sectors": 9 }),
        json!({ "radial_dial_sectors": -1 }),
        json!({ "radial_dial_sectors": 5.5 }),
        json!({ "radial_dial_sectors": "8" }),
        json!({ "radial_dial_sectors": [6] }),
        json!({ "radial_dial_sectors": null }),
    ];

    for payload in invalid_sector_payloads {
        let err = bridge
            .invoke("updateSettings", Some(payload))
            .expect_err("Invalid radial_dial_sectors must return error");
        assert!(matches!(err, IpcError::InvalidArguments(_)));

        // Verify state is completely preserved
        assert_eq!(
            bridge.state.config.read().radial_dial_sectors,
            6,
            "Config must remain unchanged on invalid update"
        );
        assert_eq!(
            bridge.state.dial_overlay.read().sector_count(),
            6,
            "Overlay sector count must remain unchanged on invalid update"
        );
    }
}
