use airctrl_meetings::privacy::CameraKillSwitch;
use airctrl_meetings::types::CameraState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[test]
fn test_direct_kill_switch_invocation() {
    let switch = CameraKillSwitch::new();
    assert!(switch.is_camera_active());
    assert_eq!(switch.get_camera_state(), CameraState::Active);

    let res = switch.trigger();
    assert_eq!(res.status, "KilledByMeeting");
    assert!(res.camera_handle_closed);
    assert!(!switch.is_camera_active());
    assert_eq!(switch.get_camera_state(), CameraState::KilledByMeeting);
}

#[test]
fn test_kill_switch_idempotent_when_already_killed() {
    let switch = CameraKillSwitch::new();
    let res1 = switch.trigger();
    assert_eq!(res1.status, "KilledByMeeting");
    assert!(res1.camera_handle_closed);

    let res2 = switch.trigger();
    assert_eq!(res2.status, "AlreadyReleased");
    assert!(!res2.camera_handle_closed);
    assert!(!switch.is_camera_active());
}

#[test]
fn test_kill_switch_repeated_10_times_stability() {
    let switch = CameraKillSwitch::new();
    let res_first = switch.trigger();
    assert_eq!(res_first.status, "KilledByMeeting");

    for _ in 0..10 {
        let res = switch.trigger();
        assert_eq!(res.status, "AlreadyReleased");
        assert!(!res.camera_handle_closed);
        assert!(!switch.is_camera_active());
    }
}

#[test]
fn test_kill_switch_latency_sub_millisecond() {
    let switch = CameraKillSwitch::new();
    let start = Instant::now();
    let res = switch.trigger();
    let elapsed = start.elapsed();

    assert_eq!(res.status, "KilledByMeeting");
    let elapsed_ms = elapsed.as_micros() as f64 / 1000.0;
    assert!(
        elapsed_ms < 1.0,
        "Kill switch trigger took {} ms (must be < 1.0 ms)",
        elapsed_ms
    );
}

#[test]
fn test_camera_reactivate_and_reset() {
    let switch = CameraKillSwitch::new();
    switch.trigger();
    assert!(!switch.is_camera_active());

    switch.reactivate();
    assert!(switch.is_camera_active());
    assert_eq!(switch.get_camera_state(), CameraState::Active);

    // After reactivating, can be triggered again
    let res = switch.trigger();
    assert_eq!(res.status, "KilledByMeeting");
    assert!(res.camera_handle_closed);
    assert!(!switch.is_camera_active());

    // Reset alias
    switch.reset();
    assert!(switch.is_camera_active());
}

#[test]
fn test_release_hooks_executed_on_first_trigger_only() {
    let switch = CameraKillSwitch::new();
    let hook_counter = Arc::new(AtomicUsize::new(0));

    let counter_clone = hook_counter.clone();
    switch.register_release_hook(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(hook_counter.load(Ordering::SeqCst), 0);

    // First trigger invokes the hook
    let res1 = switch.trigger();
    assert!(res1.camera_handle_closed);
    assert_eq!(hook_counter.load(Ordering::SeqCst), 1);

    // Repeated trigger does NOT re-invoke the hook
    let res2 = switch.trigger();
    assert!(!res2.camera_handle_closed);
    assert_eq!(hook_counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_pause_by_user() {
    let switch = CameraKillSwitch::new();
    let res = switch.pause_by_user();
    assert_eq!(res.status, "PausedByUser");
    assert!(res.camera_handle_closed);
    assert_eq!(switch.get_camera_state(), CameraState::PausedByUser);
    assert!(!switch.is_camera_active());
}
