use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::platform::mock::{create_mock_platform, MockWindowOps};
use airctrl_core::platform::traits::{PlatformAudioOps, PlatformInputOps, PlatformWindowOps};
use airctrl_core::types::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn test_event_bus_high_concurrency_stress() {
    let bus = EventBus::new(512);
    let mut rx1 = bus.subscribe();
    let _rx2 = bus.subscribe();

    assert_eq!(bus.subscriber_count(), 2);

    let num_publishers = 10;
    let events_per_publisher = 100;
    let mut pub_handles = Vec::new();

    for _p in 0..num_publishers {
        let bus_clone = bus.clone();
        pub_handles.push(tokio::spawn(async move {
            for i in 0..events_per_publisher {
                let event = DomainEvent::SttStateChanged(SttState::Processing);
                let _ = bus_clone.publish(event);
                if i % 10 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        }));
    }

    let received_count = Arc::new(AtomicUsize::new(0));
    let counter_clone = received_count.clone();

    let sub_handle = tokio::spawn(async move {
        while let Ok(_event) = rx1.recv().await {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            if counter_clone.load(Ordering::Relaxed) >= (num_publishers * events_per_publisher) {
                break;
            }
        }
    });

    for h in pub_handles {
        h.await.unwrap();
    }

    // Wait a brief moment for subscriber to drain
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), sub_handle).await;

    println!(
        "EventBus received: {} / {}",
        received_count.load(Ordering::Relaxed),
        num_publishers * events_per_publisher
    );
}

#[tokio::test]
async fn test_event_bus_zero_subscribers_failure_mode() {
    let bus = EventBus::new(64);
    assert_eq!(bus.subscriber_count(), 0);

    // When there are no active subscribers, publish() gracefully returns Ok(0)
    let result = bus.publish(DomainEvent::SttStateChanged(SttState::Idle));
    assert_eq!(
        result.expect("publish with 0 subscribers succeeds"),
        0,
        "Expected publish() with 0 subscribers to return Ok(0)"
    );
}

#[tokio::test]
async fn test_mock_window_ops_concurrency_and_edge_conditions() {
    let window_ops = MockWindowOps::new();

    // 1. Concurrency stress test: 20 threads simultaneously modifying and querying
    let mut handles = Vec::new();
    for thread_id in 0..20 {
        let ops = window_ops.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..50 {
                let win_id = (thread_id * 100 + i) as u64;
                ops.add_window(WindowInfo {
                    id: win_id,
                    title: format!("Window {}", win_id),
                    process_name: "test.exe".to_string(),
                    process_id: 1000 + win_id as u32,
                    bounds: WindowRect::default(),
                    is_focused: false,
                    is_minimized: false,
                });

                let _ = ops.focus_window(win_id);
                let _ = ops.get_active_window();
                let _ = ops.enumerate_windows();
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // Ensure state remains intact without deadlock or corruption
    let windows = window_ops.enumerate_windows().unwrap();
    assert_eq!(windows.len(), 20 * 50);

    let history = window_ops.get_focus_history();
    assert_eq!(history.len(), 20 * 50);

    // 2. Empirical edge condition test:
    // When focus_window is called on a non-existent window:
    // focus_history must NOT record the invalid ID
    let non_existent_id = 999_999_u64;
    let focus_res = window_ops.focus_window(non_existent_id);
    assert!(focus_res.is_err(), "Expected focus_window on nonexistent ID to fail");

    let history_after = window_ops.get_focus_history();
    let records_failed_attempt = history_after.contains(&non_existent_id);
    assert!(
        !records_failed_attempt,
        "focus_history must NOT record ID when window does not exist"
    );

    // 3. Multi-window focus exclusivity test:
    // When window A is focused, and then window B is focused:
    // Window A must be cleared to is_focused = false in the windows list
    let win_a = 10_u64;
    let win_b = 20_u64;
    window_ops.clear();
    window_ops.add_window(WindowInfo {
        id: win_a,
        title: "Window A".into(),
        process_name: "a.exe".into(),
        process_id: 1,
        bounds: WindowRect::default(),
        is_focused: false,
        is_minimized: false,
    });
    window_ops.add_window(WindowInfo {
        id: win_b,
        title: "Window B".into(),
        process_name: "b.exe".into(),
        process_id: 2,
        bounds: WindowRect::default(),
        is_focused: false,
        is_minimized: false,
    });

    window_ops.focus_window(win_a).unwrap();
    window_ops.focus_window(win_b).unwrap();

    let all_windows = window_ops.enumerate_windows().unwrap();
    let win_a_state = all_windows.iter().find(|w| w.id == win_a).unwrap();
    let win_b_state = all_windows.iter().find(|w| w.id == win_b).unwrap();

    assert!(
        !win_a_state.is_focused,
        "Previous window A must be cleared to is_focused == false"
    );
    assert!(
        win_b_state.is_focused,
        "Newly focused window B must have is_focused == true"
    );

    // Active window points to B
    assert_eq!(window_ops.get_active_window().unwrap().id, win_b);
}

#[tokio::test]
async fn test_mock_input_ops_concurrency_no_lost_updates() {
    let (_provider, controllers) = create_mock_platform();
    let input_ops = controllers.input;

    let num_threads = 20;
    let actions_per_thread = 50;
    let mut handles = Vec::new();

    for t in 0..num_threads {
        let ops = input_ops.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..actions_per_thread {
                ops.type_text(&format!("t{}_i{}", t, i)).unwrap();
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let recorded = input_ops.get_recorded_actions();
    assert_eq!(
        recorded.len(),
        num_threads * actions_per_thread,
        "Expected exactly {} actions recorded without race conditions or lost updates",
        num_threads * actions_per_thread
    );
}

#[tokio::test]
async fn test_mock_audio_ops_concurrency_and_underflow() {
    let (_provider, controllers) = create_mock_platform();
    let audio_ops = controllers.audio;

    // 1. Underflow handling
    let underflow_result = audio_ops.capture_audio_chunk();
    assert!(underflow_result.is_err(), "Expected underflow error when queue is empty");
    assert!(underflow_result.unwrap_err().to_string().contains("underflow"));

    // 2. High concurrency enqueue and capture
    let mut push_handles = Vec::new();
    for _ in 0..10 {
        let ops = audio_ops.clone();
        push_handles.push(tokio::spawn(async move {
            for _ in 0..20 {
                ops.enqueue_capture_samples(vec![0.1, 0.2, 0.3]);
            }
        }));
    }

    for h in push_handles {
        h.await.unwrap();
    }

    // Now capture all 200 chunks
    let mut captured = 0;
    while audio_ops.capture_audio_chunk().is_ok() {
        captured += 1;
    }
    assert_eq!(captured, 200);
}
