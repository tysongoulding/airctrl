use airctrl_core::platform::mock::MockWindowOps;
use airctrl_core::platform::traits::PlatformWindowOps;
use airctrl_core::types::{WindowInfo, WindowRect};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

fn make_window(id: u64, focused: bool) -> WindowInfo {
    WindowInfo {
        id,
        title: format!("Window-{}", id),
        process_name: format!("app_{}.exe", id),
        process_id: 1000 + id as u32,
        bounds: WindowRect::default(),
        is_focused: focused,
        is_minimized: false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_focus_invariant_no_duplicate_focused_windows() {
    let ops = MockWindowOps::new();
    let num_windows: usize = 20;

    // Seed 20 windows, initially none focused
    for id in 1..=num_windows {
        ops.add_window(make_window(id as u64, false));
    }

    let num_workers: usize = 30;
    let ops_per_worker: usize = 200;
    let stop_observer = Arc::new(AtomicBool::new(false));
    let duplicate_focused_violations = Arc::new(AtomicUsize::new(0));
    let cross_call_mismatches = Arc::new(AtomicUsize::new(0));

    // Observer thread continuously inspects windows during the stress storm
    let observer_ops = ops.clone();
    let observer_stop = stop_observer.clone();
    let dup_count = duplicate_focused_violations.clone();
    let mismatch_count = cross_call_mismatches.clone();
    let observer_handle = tokio::spawn(async move {
        while !observer_stop.load(Ordering::Relaxed) {
            let windows = observer_ops.enumerate_windows().unwrap();
            let focused_count = windows.iter().filter(|w| w.is_focused).count();
            if focused_count > 1 {
                dup_count.fetch_add(1, Ordering::SeqCst);
            }
            if let Ok(active) = observer_ops.get_active_window() {
                if !windows.iter().any(|w| w.id == active.id && w.is_focused) {
                    mismatch_count.fetch_add(1, Ordering::SeqCst);
                }
            }
            tokio::task::yield_now().await;
        }
    });

    // 30 worker threads pounding focus_window concurrently
    let mut handles = Vec::new();
    for worker_idx in 0..num_workers {
        let worker_ops = ops.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..ops_per_worker {
                // Focus pseudo-random / cycled window
                let target_id = (((worker_idx * 17 + i) % num_windows) + 1) as u64;
                let res = worker_ops.focus_window(target_id);
                assert!(res.is_ok(), "focus_window on existing ID must succeed");
                if i % 10 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    stop_observer.store(true, Ordering::Relaxed);
    observer_handle.await.unwrap();

    // Check invariants
    let dups = duplicate_focused_violations.load(Ordering::SeqCst);
    let mismatches = cross_call_mismatches.load(Ordering::SeqCst);
    println!("Observer recorded dups: {}, cross-call interleavings: {}", dups, mismatches);
    assert_eq!(
        dups,
        0,
        "Observer detected duplicate focused windows within a single state snapshot!"
    );

    let final_windows = ops.enumerate_windows().unwrap();
    let focused_windows: Vec<_> = final_windows.iter().filter(|w| w.is_focused).collect();
    assert_eq!(
        focused_windows.len(),
        1,
        "Exactly one window must be focused after concurrent execution"
    );

    let active_window = ops.get_active_window().expect("Active window must exist");
    assert_eq!(active_window.id, focused_windows[0].id);

    // Verify focus history length and valid IDs only
    let history = ops.get_focus_history();
    assert_eq!(
        history.len(),
        num_workers * ops_per_worker,
        "Focus history must record every successful focus operation"
    );
    for &hist_id in &history {
        assert!(
            hist_id >= 1 && hist_id <= num_windows as u64,
            "History contains invalid window ID: {}",
            hist_id
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_focus_adversarial_invalid_ids_isolation() {
    let ops = MockWindowOps::new();

    // Seed 5 valid windows (IDs: 10, 20, 30, 40, 50)
    for id in &[10u64, 20, 30, 40, 50] {
        ops.add_window(make_window(*id, false));
    }
    // Set initial focus to 10
    ops.focus_window(10).unwrap();

    let invalid_ids = vec![0u64, 999, 1000, 424242, u64::MAX, u64::MAX - 1];
    let valid_ids = vec![10u64, 20, 30, 40, 50];

    let invalid_err_count = Arc::new(AtomicUsize::new(0));
    let valid_ok_count = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    // 15 threads attacking with invalid IDs
    for thread_idx in 0..15 {
        let worker_ops = ops.clone();
        let err_counter = invalid_err_count.clone();
        let invalids = invalid_ids.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..100 {
                let bad_id = invalids[(thread_idx + i) % invalids.len()];
                let res = worker_ops.focus_window(bad_id);
                match res {
                    Err(airctrl_core::error::CoreError::WindowNotFound(id)) => {
                        assert_eq!(id, bad_id);
                        err_counter.fetch_add(1, Ordering::Relaxed);
                    }
                    other => panic!("Expected WindowNotFound for bad ID {}, got {:?}", bad_id, other),
                }
            }
        }));
    }

    // 15 threads operating with valid IDs
    for thread_idx in 0..15 {
        let worker_ops = ops.clone();
        let ok_counter = valid_ok_count.clone();
        let valids = valid_ids.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..100 {
                let good_id = valids[(thread_idx + i) % valids.len()];
                let res = worker_ops.focus_window(good_id);
                assert!(res.is_ok());
                ok_counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(invalid_err_count.load(Ordering::Relaxed), 15 * 100);
    assert_eq!(valid_ok_count.load(Ordering::Relaxed), 15 * 100);

    // Audit history: strictly only valid IDs present, count matches valid_ok + initial 1
    let history = ops.get_focus_history();
    assert_eq!(history.len(), 1 + 15 * 100);

    for &id in &history {
        assert!(
            valid_ids.contains(&id),
            "CRITICAL BUG: Invalid ID {} leaked into focus_history!",
            id
        );
    }

    // Invariant: exactly one focused window
    let windows = ops.enumerate_windows().unwrap();
    let focused: Vec<_> = windows.iter().filter(|w| w.is_focused).collect();
    assert_eq!(focused.len(), 1, "Must have exactly 1 focused window");
    assert!(valid_ids.contains(&focused[0].id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_focus_churn_with_add_and_close_window() {
    let ops = MockWindowOps::new();

    // Start with 10 windows
    for id in 1..=10 {
        ops.add_window(make_window(id, id == 1));
    }

    let end_time = Instant::now() + Duration::from_millis(500);
    let mut handles = Vec::new();

    // 4 focusers
    for f_idx in 0..4 {
        let worker_ops = ops.clone();
        handles.push(tokio::spawn(async move {
            let mut i = 0;
            while Instant::now() < end_time {
                let target_id = ((f_idx * 13 + i) % 30 + 1) as u64;
                let _ = worker_ops.focus_window(target_id);
                i += 1;
                tokio::task::yield_now().await;
            }
        }));
    }

    // 2 window adders
    for a_idx in 0..2 {
        let worker_ops = ops.clone();
        handles.push(tokio::spawn(async move {
            let mut i = 11 + a_idx * 100;
            while Instant::now() < end_time {
                worker_ops.add_window(make_window(i as u64, false));
                i += 1;
                tokio::task::yield_now().await;
            }
        }));
    }

    // 2 window closers
    for c_idx in 0..2 {
        let worker_ops = ops.clone();
        handles.push(tokio::spawn(async move {
            let mut i = 0;
            while Instant::now() < end_time {
                let target_id = ((c_idx * 7 + i) % 30 + 1) as u64;
                let _ = worker_ops.close_window(target_id);
                i += 1;
                tokio::task::yield_now().await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // At the end of the chaotic churn, verify invariants
    let windows = ops.enumerate_windows().unwrap();
    let focused_count = windows.iter().filter(|w| w.is_focused).count();
    assert!(
        focused_count <= 1,
        "Duplicate focused windows detected after churn: {} focused windows",
        focused_count
    );

    if focused_count == 1 {
        let active = ops.get_active_window().expect("Active window should exist");
        assert_eq!(
            windows.iter().find(|w| w.is_focused).unwrap().id,
            active.id,
            "Active window must match the single focused window in list"
        );
    } else {
        // If 0 focused, active_window must be None / return Err
        assert!(
            ops.get_active_window().is_err(),
            "When 0 windows are focused, active_window must return Err"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_hammer_two_windows_ping_pong() {
    let ops = MockWindowOps::new();
    let win1 = 101u64;
    let win2 = 202u64;

    ops.add_window(make_window(win1, false));
    ops.add_window(make_window(win2, false));

    let num_threads = 40;
    let calls_per_thread = 500;
    let mut handles = Vec::new();

    // 20 threads focusing win1, 20 threads focusing win2
    for t_idx in 0..num_threads {
        let worker_ops = ops.clone();
        let target = if t_idx % 2 == 0 { win1 } else { win2 };
        handles.push(tokio::spawn(async move {
            for _ in 0..calls_per_thread {
                worker_ops.focus_window(target).unwrap();
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let history = ops.get_focus_history();
    assert_eq!(history.len(), num_threads * calls_per_thread);

    // Verify all history entries are strictly either win1 or win2
    for &id in &history {
        assert!(id == win1 || id == win2);
    }

    // Verify exactly 1 window is focused
    let windows = ops.enumerate_windows().unwrap();
    let focused: Vec<_> = windows.iter().filter(|w| w.is_focused).collect();
    assert_eq!(focused.len(), 1);

    let active = ops.get_active_window().unwrap();
    assert_eq!(active.id, focused[0].id);
    assert_eq!(*history.last().unwrap(), active.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_clear_and_focus_safety() {
    let ops = MockWindowOps::new();
    for id in 1..=5 {
        ops.add_window(make_window(id, false));
    }

    let end_time = Instant::now() + Duration::from_millis(300);
    let mut handles = Vec::new();

    // Clearer thread
    let clear_ops = ops.clone();
    handles.push(tokio::spawn(async move {
        while Instant::now() < end_time {
            clear_ops.clear();
            // Re-add windows
            for id in 1..=5 {
                clear_ops.add_window(make_window(id, false));
            }
            tokio::task::yield_now().await;
        }
    }));

    // Focus threads
    for _t in 0..4 {
        let worker_ops = ops.clone();
        handles.push(tokio::spawn(async move {
            let mut i = 0;
            while Instant::now() < end_time {
                let id = (i % 5 + 1) as u64;
                // focus may succeed or return WindowNotFound if cleared
                let _ = worker_ops.focus_window(id);
                i += 1;
                tokio::task::yield_now().await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // After threads stop, verify state consistency
    let windows = ops.enumerate_windows().unwrap();
    let focused_count = windows.iter().filter(|w| w.is_focused).count();
    assert!(focused_count <= 1, "Duplicate focused windows: {}", focused_count);

    let history = ops.get_focus_history();
    for &id in &history {
        assert!((1..=5).contains(&id), "Invalid window ID in history: {}", id);
    }
}

