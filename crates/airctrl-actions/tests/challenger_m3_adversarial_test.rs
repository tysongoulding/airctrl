//! challenger_m3_adversarial_test.rs
//!
//! Empirical adversarial test suite for Milestone M3:
//! 1. ActionHotkeyStateMachine fuzzing:
//!    - Rapid key fluttering (1ms, 50ms, 99ms -> cancel/drop to Idle)
//!    - Valid presses (100ms, 150ms -> transition to ActionParsing)
//!    - Clock jitter / backwards timestamp handling
//!    - 5,000-cycle rapid flutter hammer
//!    - Toggle mode state transitions
//!    - Concurrent multi-threaded stress
//!    - DomainEvent::ActionTriggered broadcast behavior
//! 2. RadialDialOverlay boundary stress:
//!    - Sector boundaries for 6 sectors: 0 and 5 valid; -1, 6, -1000, 1000 rejected
//!    - Custom sector configurations (1, 8, 12)
//!    - Inactivity timeout boundary: exactly 3.0s timeout dismisses overlay and clears active sector
//! 3. WindowContextMap concurrency and cache stress:
//!    - Multi-threaded concurrent reads and invalidations
//!    - TTL (500ms) and debounce interval (150ms) invariants
//!    - Verification that caching prevents excessive system process enumeration

use airctrl_actions::*;
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::platform::PlatformWindowOps;
use airctrl_core::types::{
    ActionCommand, HotkeyMode, SnapDirection, WindowInfo, WindowRect,
};
use airctrl_core::CoreError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use sysinfo::System;

// ---------------------------------------------------------------------------
// 1. ActionHotkeyStateMachine Fuzzing & Stress Tests
// ---------------------------------------------------------------------------

#[test]
fn test_hotkey_flutter_micro_press_boundaries() {
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // Micro-press 0ms (same millisecond) -> must cancel to Idle
    let _ = sm.on_key_down(1000, Some(10));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    assert!(sm.dial().is_visible());
    let up0 = sm.on_key_up(1000);
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert_eq!(sm.target_window_id(), None);
    assert!(matches!(
        up0,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
            ..
        }
    ));

    // Micro-press 1ms -> must cancel to Idle
    let _ = sm.on_key_down(2000, Some(20));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    let up1 = sm.on_key_up(2001);
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert!(matches!(
        up1,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
            ..
        }
    ));

    // Micro-press 50ms -> must cancel to Idle
    let _ = sm.on_key_down(3000, Some(30));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    let up50 = sm.on_key_up(3050);
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert!(matches!(
        up50,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
            ..
        }
    ));

    // Micro-press 99ms (1ms below 100ms threshold) -> must cancel to Idle
    let _ = sm.on_key_down(4000, Some(40));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    let up99 = sm.on_key_up(4099);
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert!(matches!(
        up99,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
            ..
        }
    ));

    // Clock skew / backwards timestamp (key up timestamp < key down timestamp)
    let _ = sm.on_key_down(5000, Some(50));
    let up_skew = sm.on_key_up(4900); // 100ms in the past
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert!(matches!(
        up_skew,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
            ..
        }
    ));
}

#[test]
fn test_hotkey_valid_press_boundaries() {
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // Exact threshold boundary: 100ms (1000 to 1100) -> must transition to ActionParsing
    sm.on_key_down(1000, Some(101));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    let up100 = sm.on_key_up(1100);
    assert_eq!(*sm.state(), ActionState::ActionParsing);
    assert!(matches!(
        up100,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::ActionParsing,
            instruction: ActionMachineInstruction::StopListeningAndParse,
            ..
        }
    ));

    // Reset to Idle for next test
    sm.reset();
    assert_eq!(*sm.state(), ActionState::Idle);

    // 150ms press (1000 to 1150) -> must transition to ActionParsing
    sm.on_key_down(1000, Some(102));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    let up150 = sm.on_key_up(1150);
    assert_eq!(*sm.state(), ActionState::ActionParsing);
    assert!(matches!(
        up150,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::ActionParsing,
            instruction: ActionMachineInstruction::StopListeningAndParse,
            ..
        }
    ));

    // Reset to Idle
    sm.reset();

    // Long press: 10,000ms press -> must transition to ActionParsing
    sm.on_key_down(1000, Some(103));
    let up_long = sm.on_key_up(11000);
    assert_eq!(*sm.state(), ActionState::ActionParsing);
    assert!(matches!(
        up_long,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::ActionParsing,
            instruction: ActionMachineInstruction::StopListeningAndParse,
            ..
        }
    ));
}

#[test]
fn test_hotkey_5000_cycles_rapid_flutter_fuzz() {
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    let mut current_time_ms = 10_000u64;
    for i in 0..5_000 {
        // Micro duration between 1ms and 99ms
        let micro_duration = (i % 99) + 1;

        let res_down = sm.on_key_down(current_time_ms, Some(i));
        assert_eq!(*sm.state(), ActionState::ActionListening);
        assert!(sm.dial().is_visible());
        assert!(matches!(res_down, ActionStateTransitionResult::Transitioned { .. }));

        current_time_ms += micro_duration;
        let res_up = sm.on_key_up(current_time_ms);
        assert_eq!(*sm.state(), ActionState::Idle);
        assert!(!sm.dial().is_visible());
        assert_eq!(sm.target_window_id(), None);
        assert!(matches!(
            res_up,
            ActionStateTransitionResult::Transitioned {
                to: ActionState::Idle,
                instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
                ..
            }
        ));

        // Advance inter-keystroke interval
        current_time_ms += 10;
    }
}

#[test]
fn test_hotkey_toggle_mode_complete_flow_and_flutter() {
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::Toggle, None);
    assert_eq!(sm.mode(), HotkeyMode::Toggle);

    // Initial state: Idle
    assert_eq!(*sm.state(), ActionState::Idle);

    // First key down: begins listening
    let res1 = sm.on_key_down(1000, Some(200));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    assert!(sm.dial().is_visible());
    assert!(matches!(
        res1,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::ActionListening,
            ..
        }
    ));

    // In Toggle mode, key up is ignored (does not cancel or stop listening)
    let res_up1 = sm.on_key_up(1010);
    assert_eq!(res_up1, ActionStateTransitionResult::NoChange);
    assert_eq!(*sm.state(), ActionState::ActionListening);
    assert!(sm.dial().is_visible());

    // Subsequent unsolicited key up is also ignored
    let res_up2 = sm.on_key_up(1500);
    assert_eq!(res_up2, ActionStateTransitionResult::NoChange);
    assert_eq!(*sm.state(), ActionState::ActionListening);

    // Second key down: triggers StopListeningAndParse -> ActionParsing
    let res2 = sm.on_key_down(2000, Some(200));
    assert_eq!(*sm.state(), ActionState::ActionParsing);
    assert!(matches!(
        res2,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::ActionParsing,
            instruction: ActionMachineInstruction::StopListeningAndParse,
            ..
        }
    ));

    // While in ActionParsing, further key down or key up events are ignored
    assert_eq!(sm.on_key_down(2100, Some(200)), ActionStateTransitionResult::NoChange);
    assert_eq!(sm.on_key_up(2150), ActionStateTransitionResult::NoChange);
    assert_eq!(*sm.state(), ActionState::ActionParsing);

    // Parse command
    let cmd = ActionCommand::WindowFocus {
        window_id: 10,
        query: "firefox".to_string(),
    };
    let res3 = sm.on_command_parsed(cmd.clone());
    assert_eq!(*sm.state(), ActionState::ActionExecuting);
    assert!(matches!(
        res3,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::ActionExecuting,
            instruction: ActionMachineInstruction::ExecuteCommand(_),
            ..
        }
    ));

    // While in ActionExecuting, key presses are ignored
    assert_eq!(sm.on_key_down(2200, Some(200)), ActionStateTransitionResult::NoChange);
    assert_eq!(sm.on_key_up(2250), ActionStateTransitionResult::NoChange);

    // Execution completes -> returns to Idle
    let res4 = sm.on_execution_completed();
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert_eq!(sm.target_window_id(), None);
    assert!(matches!(
        res4,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::ReturnToIdle,
            ..
        }
    ));
}

#[test]
fn test_hotkey_concurrent_multi_threaded_hammer() {
    let bus = EventBus::new(128);
    let sm = Arc::new(Mutex::new(ActionHotkeyStateMachine::new(
        HotkeyMode::PushToTalk,
        Some(bus),
    )));

    let mut handles = Vec::new();
    let thread_count = 16;
    let iterations_per_thread = 200;

    for tid in 0..thread_count {
        let sm_clone = Arc::clone(&sm);
        handles.push(thread::spawn(move || {
            for i in 0..iterations_per_thread {
                let timestamp = (tid * 10_000 + i * 50) as u64;
                let mut machine = sm_clone.lock().unwrap();

                match i % 6 {
                    0 => {
                        let _ = machine.on_key_down(timestamp, Some(tid as u64));
                    }
                    1 => {
                        let _ = machine.on_key_up(timestamp + (i % 150) as u64);
                    }
                    2 => {
                        let _ = machine.on_command_parsed(ActionCommand::WindowSnap {
                            direction: SnapDirection::Maximize,
                        });
                    }
                    3 => {
                        let _ = machine.on_execution_completed();
                    }
                    4 => {
                        let _ = machine.on_inactivity_timeout();
                    }
                    _ => {
                        let _ = machine.reset();
                    }
                }

                // Invariant: If state is Idle, dial is hidden
                if *machine.state() == ActionState::Idle {
                    assert!(!machine.dial().is_visible());
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("Worker thread panicked under concurrency hammer");
    }

    // Final state check: reset to Idle cleanly
    let mut final_sm = sm.lock().unwrap();
    final_sm.reset();
    assert_eq!(*final_sm.state(), ActionState::Idle);
    assert!(!final_sm.dial().is_visible());
}

#[test]
fn test_hotkey_domain_event_broadcast_behavior() {
    let bus = EventBus::new(64);
    assert_eq!(bus.subscriber_count(), 0);

    // Subscribe 5 independent listeners
    let mut sub1 = bus.subscribe();
    let mut sub2 = bus.subscribe();
    let mut sub3 = bus.subscribe();
    let mut sub4 = bus.subscribe();
    let mut sub5 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 5);

    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, Some(bus.clone()));

    // 1. Negative test: calling on_command_parsed while Idle must NOT broadcast
    let spurious_cmd = ActionCommand::WindowSnap { direction: SnapDirection::Left };
    let res_idle = sm.on_command_parsed(spurious_cmd);
    assert_eq!(res_idle, ActionStateTransitionResult::NoChange);
    assert!(sub1.try_recv().is_err());
    assert!(sub2.try_recv().is_err());

    // 2. Negative test: calling on_command_parsed while Listening must NOT broadcast
    sm.on_key_down(1000, Some(999));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    let res_listening = sm.on_command_parsed(ActionCommand::WindowSnap { direction: SnapDirection::Right });
    assert_eq!(res_listening, ActionStateTransitionResult::NoChange);
    assert!(sub1.try_recv().is_err());

    // 3. Move to ActionParsing
    sm.on_key_up(1200); // 200ms duration -> ActionParsing
    assert_eq!(*sm.state(), ActionState::ActionParsing);

    // 4. Positive test: parsing valid command MUST broadcast to all 5 subscribers
    let valid_cmd = ActionCommand::WindowSnap { direction: SnapDirection::Maximize };
    let res_parsed = sm.on_command_parsed(valid_cmd.clone());
    assert_eq!(*sm.state(), ActionState::ActionExecuting);
    assert!(matches!(
        res_parsed,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::ActionExecuting,
            instruction: ActionMachineInstruction::ExecuteCommand(_),
            ..
        }
    ));

    // Verify all 5 subscribers received the broadcast event
    let ev1 = sub1.try_recv().expect("sub1 missed broadcast");
    let ev2 = sub2.try_recv().expect("sub2 missed broadcast");
    let ev3 = sub3.try_recv().expect("sub3 missed broadcast");
    let ev4 = sub4.try_recv().expect("sub4 missed broadcast");
    let ev5 = sub5.try_recv().expect("sub5 missed broadcast");

    let expected = DomainEvent::ActionTriggered(valid_cmd.clone());
    assert_eq!(ev1, expected);
    assert_eq!(ev2, expected);
    assert_eq!(ev3, expected);
    assert_eq!(ev4, expected);
    assert_eq!(ev5, expected);

    // 5. Subsequent calls while Executing do not re-broadcast
    let res_exec = sm.on_command_parsed(valid_cmd);
    assert_eq!(res_exec, ActionStateTransitionResult::NoChange);
    assert!(sub1.try_recv().is_err());

    // Complete execution
    sm.on_execution_completed();
    assert_eq!(*sm.state(), ActionState::Idle);

    // 6. Zero subscribers edge case: EventBus with 0 subscribers does not error or panic
    let bus_zero = EventBus::new(16);
    let mut sm_zero = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, Some(bus_zero));
    sm_zero.on_key_down(1000, None);
    sm_zero.on_key_up(1200);
    let res_zero = sm_zero.on_command_parsed(ActionCommand::WindowMinimize { window_id: 42 });
    assert_eq!(*sm_zero.state(), ActionState::ActionExecuting);
    assert!(matches!(res_zero, ActionStateTransitionResult::Transitioned { .. }));
}

// ---------------------------------------------------------------------------
// 2. RadialDialOverlay Boundary Stress Tests
// ---------------------------------------------------------------------------

#[test]
fn test_radial_dial_sector_boundaries_6_sectors() {
    let mut dial = RadialDialOverlay::new();
    assert_eq!(dial.sector_count(), 6);
    assert_eq!(dial.timeout_secs(), 3.0);
    assert!(!dial.is_visible());
    assert_eq!(dial.active_sector(), None);

    // While hidden, hovering any sector is ignored
    dial.hover_sector(0);
    assert_eq!(dial.active_sector(), None);
    dial.hover_sector(5);
    assert_eq!(dial.active_sector(), None);

    dial.show();
    assert!(dial.is_visible());

    // Boundary: sector 0 is valid lower bound
    dial.hover_sector(0);
    assert_eq!(dial.active_sector(), Some(0));

    // Boundary: sector 5 is valid upper bound
    dial.hover_sector(5);
    assert_eq!(dial.active_sector(), Some(5));

    // Boundary: sector -1 is strictly rejected (ignored, retains previous selection)
    dial.hover_sector(-1);
    assert_eq!(dial.active_sector(), Some(5));

    // Boundary: sector 6 is strictly rejected (ignored, retains previous selection)
    dial.hover_sector(6);
    assert_eq!(dial.active_sector(), Some(5));

    // Extreme boundaries: i32::MIN, -9999, 7, 9999, i32::MAX
    dial.hover_sector(i32::MIN);
    assert_eq!(dial.active_sector(), Some(5));
    dial.hover_sector(-9999);
    assert_eq!(dial.active_sector(), Some(5));
    dial.hover_sector(7);
    assert_eq!(dial.active_sector(), Some(5));
    dial.hover_sector(9999);
    assert_eq!(dial.active_sector(), Some(5));
    dial.hover_sector(i32::MAX);
    assert_eq!(dial.active_sector(), Some(5));

    // Interior valid sectors
    for s in 0..6 {
        dial.hover_sector(s);
        assert_eq!(dial.active_sector(), Some(s as usize));
    }

    // Hide dial clears active sector
    dial.hide();
    assert!(!dial.is_visible());
    assert_eq!(dial.active_sector(), None);
}

#[test]
fn test_radial_dial_custom_sector_counts() {
    // 1-sector dial (sector 0 valid; -1 and 1 rejected)
    let mut dial1 = RadialDialOverlay::with_sectors(1);
    dial1.show();
    dial1.hover_sector(0);
    assert_eq!(dial1.active_sector(), Some(0));
    dial1.hover_sector(-1);
    assert_eq!(dial1.active_sector(), Some(0));
    dial1.hover_sector(1);
    assert_eq!(dial1.active_sector(), Some(0));

    // 8-sector dial (sectors 0..=7 valid; -1 and 8 rejected)
    let mut dial8 = RadialDialOverlay::with_sectors(8);
    dial8.show();
    dial8.hover_sector(7);
    assert_eq!(dial8.active_sector(), Some(7));
    dial8.hover_sector(8);
    assert_eq!(dial8.active_sector(), Some(7));
    dial8.hover_sector(-1);
    assert_eq!(dial8.active_sector(), Some(7));

    // 12-sector dial (sectors 0..=11 valid; 12 rejected)
    let mut dial12 = RadialDialOverlay::with_sectors(12);
    dial12.show();
    dial12.hover_sector(11);
    assert_eq!(dial12.active_sector(), Some(11));
    dial12.hover_sector(12);
    assert_eq!(dial12.active_sector(), Some(11));
}

#[test]
fn test_radial_dial_inactivity_timeout_boundary() {
    let mut dial = RadialDialOverlay::new();
    assert_eq!(dial.timeout_secs(), 3.0);

    dial.show();
    dial.hover_sector(4);
    assert!(dial.is_visible());
    assert_eq!(dial.active_sector(), Some(4));

    // Inactivity timeout triggers dismissal
    dial.on_inactivity_timeout();
    assert!(!dial.is_visible(), "Inactivity timeout must dismiss overlay");
    assert_eq!(
        dial.active_sector(),
        None,
        "Inactivity timeout must clear active sector"
    );

    // Snapshot verifies cleared state
    let snap = dial.snapshot();
    assert!(!snap.is_visible);
    assert_eq!(snap.active_sector, None);
    assert_eq!(snap.timeout_secs, 3.0);

    // State machine inactivity timeout interaction
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, None);
    sm.on_key_down(1000, Some(555));
    sm.dial_mut().hover_sector(3);
    assert_eq!(*sm.state(), ActionState::ActionListening);
    assert!(sm.dial().is_visible());
    assert_eq!(sm.dial().active_sector(), Some(3));

    let res_timeout = sm.on_inactivity_timeout();
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert_eq!(sm.dial().active_sector(), None);
    assert_eq!(sm.target_window_id(), None);
    assert!(matches!(
        res_timeout,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::ReturnToIdle,
            ..
        }
    ));

    // Subsequent timeout when already Idle returns NoChange
    assert_eq!(sm.on_inactivity_timeout(), ActionStateTransitionResult::NoChange);
}

// ---------------------------------------------------------------------------
// 3. WindowContextMap Concurrency and Cache Stress Tests
// ---------------------------------------------------------------------------

struct MockWindowPlatform {
    call_count: Arc<AtomicUsize>,
}

impl MockWindowPlatform {
    fn new(call_count: Arc<AtomicUsize>) -> Self {
        Self { call_count }
    }
}

impl PlatformWindowOps for MockWindowPlatform {
    fn get_active_window(&self) -> std::result::Result<WindowInfo, CoreError> {
        Ok(WindowInfo {
            id: 10,
            process_id: 100,
            process_name: "cursor.exe".into(),
            title: "airctrl - Cursor".into(),
            bounds: WindowRect { x: 0, y: 0, width: 1920, height: 1080 },
            is_focused: true,
            is_minimized: false,
        })
    }

    fn focus_window(&self, _id: u64) -> std::result::Result<(), CoreError> {
        Ok(())
    }

    fn enumerate_windows(&self) -> std::result::Result<Vec<WindowInfo>, CoreError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(vec![
            WindowInfo {
                id: 10,
                process_id: 100,
                process_name: "cursor.exe".into(),
                title: "airctrl - Cursor".into(),
                bounds: WindowRect { x: 0, y: 0, width: 1920, height: 1080 },
                is_focused: true,
                is_minimized: false,
            },
            WindowInfo {
                id: 20,
                process_id: 200,
                process_name: "chrome.exe".into(),
                title: "Google Chrome".into(),
                bounds: WindowRect { x: 100, y: 100, width: 1280, height: 720 },
                is_focused: false,
                is_minimized: false,
            },
        ])
    }

    fn minimize_window(&self, _id: u64) -> std::result::Result<(), CoreError> {
        Ok(())
    }

    fn close_window(&self, _id: u64) -> std::result::Result<(), CoreError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_window_context_map_concurrency_reads_and_invalidations() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let platform = Arc::new(MockWindowPlatform::new(Arc::clone(&call_count)));

    let config = CrawlerConfig {
        ttl: Duration::from_millis(500),
        debounce_interval: Duration::from_millis(150),
        fuzzy_threshold: 0.65,
    };

    let crawler = WindowProcessCrawler::new(platform, config);

    // Initial query refreshes once
    let initial_windows = crawler.get_open_windows().await.expect("Failed to get windows");
    assert!(!initial_windows.is_empty());
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Spawn 10 concurrent reader tasks and 5 concurrent invalidator tasks
    let mut handles = Vec::new();
    let reader_iterations = 100;

    for _ in 0..10 {
        let c = crawler.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..reader_iterations {
                let wins = c.get_open_windows().await.expect("Reader failed get_open_windows");
                assert!(!wins.is_empty());
                let procs = c.get_running_processes().await.expect("Reader failed get_running_processes");
                let _ = procs.len();
                let _ = c.find_window("cursor").await;
            }
        }));
    }

    for _ in 0..5 {
        let c = crawler.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..reader_iterations {
                c.invalidate_cache().await;
                tokio::task::yield_now().await;
            }
        }));
    }

    for h in handles {
        h.await.expect("Task failed under concurrent reader/invalidator stress");
    }

    // Invalidation and debounce prevented excessive queries:
    // Even under 1,000 read iterations and 500 invalidations, debounce throttles refreshes
    let total_calls = call_count.load(Ordering::SeqCst);
    assert!(
        total_calls < 50,
        "Debounce interval failed to throttle refresh calls; total calls was {}",
        total_calls
    );
}

#[tokio::test]
async fn test_crawler_ttl_and_debounce_throttling_invariants() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let platform = Arc::new(MockWindowPlatform::new(Arc::clone(&call_count)));

    let config = CrawlerConfig {
        ttl: Duration::from_millis(500),
        debounce_interval: Duration::from_millis(150),
        fuzzy_threshold: 0.65,
    };

    let crawler = WindowProcessCrawler::new(platform, config);

    // 1. Initial call triggers refresh
    let _ = crawler.get_open_windows().await.unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1, "Initial call must refresh");

    // 2. 100 immediate repeated calls in a tight loop must be served 100% from cache
    for _ in 0..100 {
        let _ = crawler.get_open_windows().await.unwrap();
        let _ = crawler.get_running_processes().await.unwrap();
    }
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        1,
        "Tight loop calls within TTL/debounce must not re-enumerate processes"
    );

    // 3. Explicit invalidation within debounce window (< 150ms)
    crawler.invalidate_cache().await;
    // Call immediately after invalidation: should_debounce() is true, so refresh must be DEBOUNCED
    let _ = crawler.get_open_windows().await.unwrap();
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        1,
        "Immediate call following invalidate() within 150ms debounce window must NOT re-enumerate"
    );

    // 4. Wait for debounce interval (160ms > 150ms)
    tokio::time::sleep(Duration::from_millis(160)).await;

    // Invalidate cache again
    crawler.invalidate_cache().await;
    // Now debounce window has elapsed and cache is dirty -> must refresh!
    let _ = crawler.get_open_windows().await.unwrap();
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        2,
        "Call after debounce interval with dirty cache must re-enumerate"
    );

    // 5. Test TTL expiry without explicit invalidation:
    // Wait for TTL (500ms)
    tokio::time::sleep(Duration::from_millis(520)).await;
    // Now cache has expired (elapsed > 500ms) -> must refresh!
    let _ = crawler.get_open_windows().await.unwrap();
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        3,
        "Call after TTL expiry (500ms) must re-enumerate"
    );
}

#[test]
fn test_window_context_map_direct_indexing_and_freshness() {
    let config = CrawlerConfig {
        ttl: Duration::from_millis(50),
        debounce_interval: Duration::from_millis(20),
        fuzzy_threshold: 0.65,
    };

    let mut map = WindowContextMap::new(config);
    // Initially dirty
    assert!(!map.is_fresh());

    let call_count = Arc::new(AtomicUsize::new(0));
    let platform = MockWindowPlatform::new(call_count);
    let mut sys = System::new();

    // Refresh map
    map.refresh(&platform, &mut sys).expect("Refresh failed");
    assert!(map.is_fresh(), "Map should be fresh immediately after refresh");
    assert!(map.should_debounce(), "Should debounce immediately after refresh");

    // Test lookup by process name (case-insensitive)
    let cursor_wins = map.get_windows_by_process("CURSOR.EXE");
    assert_eq!(cursor_wins.len(), 1);
    assert_eq!(cursor_wins[0].id, 10);

    let chrome_wins = map.get_windows_by_process("Chrome.exe");
    assert_eq!(chrome_wins.len(), 1);
    assert_eq!(chrome_wins[0].id, 20);

    // Invalidation immediately marks dirty
    map.invalidate();
    assert!(!map.is_fresh(), "Map must not be fresh after invalidate()");

    // Refresh again
    map.refresh(&platform, &mut sys).expect("Refresh failed");
    assert!(map.is_fresh());

    // Wait past TTL (50ms)
    thread::sleep(Duration::from_millis(60));
    assert!(!map.is_fresh(), "Map must not be fresh after TTL elapsed");
}
