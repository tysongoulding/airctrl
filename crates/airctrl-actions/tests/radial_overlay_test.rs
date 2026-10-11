use airctrl_actions::*;

#[test]
fn test_radial_overlay_initial_state_and_toggle() {
    let mut dial = RadialDialOverlay::new();

    // Starts hidden
    assert!(!dial.is_visible());
    assert_eq!(dial.active_sector(), None);
    assert_eq!(dial.sector_count(), 6);
    assert_eq!(dial.timeout_secs(), 3.0);

    // Toggle on
    dial.toggle();
    assert!(dial.is_visible());

    // Toggle off
    dial.toggle();
    assert!(!dial.is_visible());
    assert_eq!(dial.active_sector(), None);
}

#[test]
fn test_radial_overlay_sector_hover_boundaries() {
    let mut dial = RadialDialOverlay::new();
    dial.show();

    // Hover sector 0 (valid lower boundary)
    dial.hover_sector(0);
    assert_eq!(dial.active_sector(), Some(0));

    // Hover sector 2
    dial.hover_sector(2);
    assert_eq!(dial.active_sector(), Some(2));

    // Hover sector 5 (valid upper boundary for 6 sectors)
    dial.hover_sector(5);
    assert_eq!(dial.active_sector(), Some(5));

    // Negative sector: rejected, remains at previous valid sector
    dial.hover_sector(-1);
    assert_eq!(dial.active_sector(), Some(5));

    // Sector 6 (out of range for [0, 5]): rejected, remains at previous valid sector
    dial.hover_sector(6);
    assert_eq!(dial.active_sector(), Some(5));

    // Sector 10 (far out of range): rejected
    dial.hover_sector(10);
    assert_eq!(dial.active_sector(), Some(5));
}

#[test]
fn test_radial_overlay_inactivity_timeout() {
    let mut dial = RadialDialOverlay::new();
    dial.show();
    dial.hover_sector(3);
    assert!(dial.is_visible());
    assert_eq!(dial.active_sector(), Some(3));

    // Inactivity timeout dismisses overlay
    dial.on_inactivity_timeout();
    assert!(!dial.is_visible());
    assert_eq!(dial.active_sector(), None);
}

#[test]
fn test_radial_overlay_snapshot() {
    let mut dial = RadialDialOverlay::new();
    dial.show();
    dial.hover_sector(4);

    let snap = dial.snapshot();
    assert!(snap.is_visible);
    assert_eq!(snap.active_sector, Some(4));
    assert_eq!(snap.sector_count, 6);
    assert_eq!(snap.timeout_secs, 3.0);
}
