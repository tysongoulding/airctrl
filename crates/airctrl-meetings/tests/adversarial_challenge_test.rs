use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_meetings::config::MeetingWatcherConfig;
use airctrl_meetings::detector::{DefaultMeetingDetector, MeetingDetector};
use airctrl_meetings::error::MeetingError;
use airctrl_meetings::privacy::CameraKillSwitch;
use airctrl_meetings::types::{
    ActiveMeetingInfo, MeetingApp, MeetingStatus, ProcessScanItem,
};
use airctrl_meetings::watcher::MeetingWatcher;
use chrono::Utc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// 1. Detection Adversarial Fuzzing Tests
// ============================================================================

#[test]
fn test_slack_huddle_boundary_adversarial() {
    let detector = DefaultMeetingDetector::new();

    // Verification 1: slack.exe with normal chat title MUST BE IGNORED
    let slack_general = vec![ProcessScanItem::new(101, "slack.exe", "Slack | General")];
    assert!(
        detector.detect_meeting(&slack_general).is_none(),
        "Slack | General must be IGNORED"
    );

    let slack_dev = vec![ProcessScanItem::new(102, "slack.exe", "#dev-team - Slack")];
    assert!(
        detector.detect_meeting(&slack_dev).is_none(),
        "Regular channel chat must be IGNORED"
    );

    // Verification 2: slack.exe with Huddle in title MUST BE DETECTED
    let slack_huddle = vec![ProcessScanItem::new(
        103,
        "slack.exe",
        "Slack | Huddle - Product Sync",
    )];
    let detected = detector.detect_meeting(&slack_huddle);
    assert!(
        detected.is_some(),
        "Slack | Huddle - Product Sync must be DETECTED"
    );
    let m = detected.unwrap();
    assert_eq!(m.app, MeetingApp::SlackHuddle);
    assert_eq!(m.pid, 103);
}

#[test]
fn test_browser_title_fuzzing_valid_meetings() {
    let detector = DefaultMeetingDetector::new();

    // 1. Uppercase GOOGLE MEET in browser
    let upper_meet = vec![ProcessScanItem::new(
        201,
        "chrome.exe",
        "PROJECT REVIEW - GOOGLE MEET",
    )];
    let res_meet = detector.detect_meeting(&upper_meet);
    assert!(res_meet.is_some(), "Uppercase GOOGLE MEET must be detected");
    assert_eq!(res_meet.unwrap().app, MeetingApp::GoogleMeet);

    // 2. Mixed case Zoom Meeting in browser
    let mixed_zoom = vec![ProcessScanItem::new(
        202,
        "firefox.exe",
        "Zoom Meeting - Team Standup",
    )];
    let res_zoom = detector.detect_meeting(&mixed_zoom);
    assert!(res_zoom.is_some(), "Mixed case Zoom Meeting must be detected");
    assert_eq!(res_zoom.unwrap().app, MeetingApp::Zoom);
}

#[test]
fn test_title_case_sensitivity_and_unicode() {
    let detector = DefaultMeetingDetector::new();

    // Unicode em-dash in Slack Huddle title
    let unicode_slack = vec![ProcessScanItem::new(
        203,
        "slack.exe",
        "Slack | Huddle \u{2014} Product Sync",
    )];
    assert!(
        detector.detect_meeting(&unicode_slack).is_some(),
        "Slack Huddle with em-dash must be detected"
    );

    // Unicode bullet in Zoom Meeting
    let unicode_zoom = vec![ProcessScanItem::new(
        204,
        "chrome.exe",
        "Zoom Meeting \u{2022} Standup",
    )];
    assert!(
        detector.detect_meeting(&unicode_zoom).is_some(),
        "Zoom Meeting with bullet must be detected"
    );

    // Mixed/upper case process and title
    let mixed_teams = vec![ProcessScanItem::new(
        205,
        "TEAMS.EXE",
        "MICROSOFT TEAMS CALL",
    )];
    assert!(
        detector.detect_meeting(&mixed_teams).is_some(),
        "Uppercase TEAMS.EXE must be detected"
    );
}

// BUG FINDING TEST 1: Non-meeting browser titles falsely detected as meetings
#[test]
fn test_bug_finding_non_meeting_browser_titles_falsely_detected() {
    let detector = DefaultMeetingDetector::new();

    let how_to_zoom = vec![ProcessScanItem::new(
        301,
        "chrome.exe",
        "How to use Zoom - Article - Google Chrome",
    )];
    let search_webex = vec![ProcessScanItem::new(
        302,
        "chrome.exe",
        "Google Search: Webex",
    )];

    let res_how_to = detector.detect_meeting(&how_to_zoom);
    let res_search = detector.detect_meeting(&search_webex);

    println!("[FIX VERIFIED] 'How to use Zoom' detected as: {:?}", res_how_to);
    println!("[FIX VERIFIED] 'Google Search: Webex' detected as: {:?}", res_search);

    // Non-meeting browser titles must cleanly return None
    assert!(
        res_how_to.is_none(),
        "Non-meeting browser title 'How to use Zoom' must NOT be detected as a meeting"
    );
    assert!(
        res_search.is_none(),
        "Non-meeting browser title 'Google Search: Webex' must NOT be detected as a meeting"
    );
}

// BUG FINDING TEST 2: Process table stress with 500+ processes and decoys
#[test]
fn test_process_table_stress_500_with_decoys() {
    let detector = DefaultMeetingDetector::new();

    let mut procs = Vec::with_capacity(510);
    // 500 benign processes
    for i in 0..500 {
        procs.push(ProcessScanItem::new(
            1000 + i,
            format!("background_svc_{i}.exe"),
            format!("Background Worker {i}"),
        ));
    }

    // Add decoys:
    // 1. slack_helper.exe (Slack helper daemon)
    procs.push(ProcessScanItem::new(
        2001,
        "slack_helper.exe",
        "Slack Helper Process",
    ));
    // 2. teamviewer.exe (Remote control tool, not MS Teams)
    procs.push(ProcessScanItem::new(
        2002,
        "teamviewer.exe",
        "TeamViewer Remote Session",
    ));
    // 3. zoombot.exe (Bot process, not Zoom client)
    procs.push(ProcessScanItem::new(
        2003,
        "zoombot.exe",
        "ZoomBot Headless Automation",
    ));

    let detected = detector.detect_meeting(&procs);
    println!(
        "[FIX VERIFIED] Process table with 500+ procs and decoys detected: {:?}",
        detected
    );

    // In a clean table with only decoys and NO active meeting, detection must be None.
    assert!(
        detected.is_none(),
        "Decoy processes (zoombot.exe, slack_helper.exe, teamviewer.exe) must NOT cause false positive"
    );
}

// ============================================================================
// 2. Watcher Debounce Hysteresis Stress Tests
// ============================================================================

/// Step-synchronous mock detector for deterministic tick-by-tick testing.
struct SynchronizedMockDetector {
    sequence: Mutex<Vec<Option<ActiveMeetingInfo>>>,
    tick_count: AtomicUsize,
}

impl SynchronizedMockDetector {
    fn new(sequence: Vec<Option<ActiveMeetingInfo>>) -> Self {
        Self {
            sequence: Mutex::new(sequence),
            tick_count: AtomicUsize::new(0),
        }
    }

    fn ticks_executed(&self) -> usize {
        self.tick_count.load(Ordering::SeqCst)
    }
}

impl MeetingDetector for SynchronizedMockDetector {
    fn detect_meeting(&self, _items: &[ProcessScanItem]) -> Option<ActiveMeetingInfo> {
        let mut seq = self.sequence.lock().unwrap();
        self.tick_count.fetch_add(1, Ordering::SeqCst);
        if seq.is_empty() {
            None
        } else {
            seq.remove(0)
        }
    }

    fn scan_system(&self) -> Result<Option<ActiveMeetingInfo>, MeetingError> {
        let mut seq = self.sequence.lock().unwrap();
        self.tick_count.fetch_add(1, Ordering::SeqCst);
        if seq.is_empty() {
            Ok(None)
        } else {
            Ok(seq.remove(0))
        }
    }
}

#[tokio::test]
async fn test_watcher_rapid_alternation_no_flapping() {
    let sample_meeting = ActiveMeetingInfo {
        app: MeetingApp::Zoom,
        window_id: Some(1),
        pid: 100,
        title: "Zoom Call".to_string(),
        start_time: Utc::now(),
    };

    // Alternate: Some, None, Some, None, Some, None, Some, None (8 ticks)
    // No two consecutive ticks are ever identical!
    let mut pattern = Vec::new();
    for _ in 0..6 {
        pattern.push(Some(sample_meeting.clone()));
        pattern.push(None);
    }

    let detector = Arc::new(SynchronizedMockDetector::new(pattern));
    let det_clone = detector.clone();

    let config = MeetingWatcherConfig {
        poll_interval_ms: 15,
        debounce_ticks: 2,
        auto_kill_camera: false,
        auto_compliance_audio: false,
        ..Default::default()
    };

    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();

    let watcher = MeetingWatcher::new(config)
        .with_detector(detector)
        .with_event_bus(bus);

    let handle = watcher.start().expect("Watcher start failed");

    // Wait until at least 10 ticks have run
    for _ in 0..50 {
        if det_clone.ticks_executed() >= 10 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Because each state only lasted 1 tick, the 2-tick debounce threshold was never reached.
    // The watcher must NOT flap to Active or Ended!
    assert_eq!(
        handle.get_status(),
        MeetingStatus::Inactive,
        "Rapid alternation must NOT flap out of Inactive (status: {:?})",
        handle.get_status()
    );

    // Verify no Active or Ended events were emitted
    let mut events = Vec::new();
    while let Ok(evt) = rx.try_recv() {
        events.push(evt);
    }

    let flapped = events.iter().any(|e| {
        matches!(
            e,
            DomainEvent::MeetingStatusChanged(MeetingStatus::Active)
                | DomainEvent::MeetingStatusChanged(MeetingStatus::Ended)
        )
    });
    assert!(!flapped, "Watcher must not publish Active or Ended events on rapid alternation");

    handle.stop().unwrap();
}

#[tokio::test]
async fn test_watcher_2_tick_requirement() {
    let sample_meeting = ActiveMeetingInfo {
        app: MeetingApp::Teams,
        window_id: Some(2),
        pid: 200,
        title: "Teams Sync".to_string(),
        start_time: Utc::now(),
    };

    // Sequence:
    // Tick 1: Some (tick 1 of appearance) -> must NOT transition to Active
    // Tick 2: Some (tick 2 of appearance) -> MUST transition to Active
    // Tick 3: Some (stay Active)
    // Tick 4: None (tick 1 of disappearance) -> must NOT transition to Inactive
    // Tick 5: None (tick 2 of disappearance) -> MUST transition to Inactive
    let pattern = vec![
        Some(sample_meeting.clone()),
        Some(sample_meeting.clone()),
        Some(sample_meeting.clone()),
        None,
        None,
        None,
    ];

    let detector = Arc::new(SynchronizedMockDetector::new(pattern));
    let det_clone = detector.clone();

    let config = MeetingWatcherConfig {
        poll_interval_ms: 20,
        debounce_ticks: 2,
        auto_kill_camera: false,
        auto_compliance_audio: false,
        ..Default::default()
    };

    let watcher = MeetingWatcher::new(config).with_detector(detector);
    let handle = watcher.start().expect("Watcher start failed");

    // --- PHASE A: Check after EXACTLY 1 tick ---
    // Wait until tick 1 completes
    for _ in 0..50 {
        if det_clone.ticks_executed() >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // Immediately after tick 1 (before tick 2 runs), status must NOT be Active
    // Note: It might be Inactive or Detected, but NEVER Active
    let status_after_1_tick = handle.get_status();
    println!("Status after 1 tick: {:?}", status_after_1_tick);
    assert_ne!(
        status_after_1_tick,
        MeetingStatus::Active,
        "Single appearance tick must NOT transition to Active"
    );

    // --- PHASE B: Wait for tick 2 & 3 ---
    for _ in 0..50 {
        if det_clone.ticks_executed() >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        handle.get_status(),
        MeetingStatus::Active,
        "2 consecutive appearance ticks MUST transition to Active"
    );

    // --- PHASE C: Check after 1 disappearance tick (tick 4) ---
    for _ in 0..50 {
        if det_clone.ticks_executed() >= 4 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let status_after_1_disappear_tick = handle.get_status();
    println!("Status after 1 disappear tick: {:?}", status_after_1_disappear_tick);
    assert_eq!(
        status_after_1_disappear_tick,
        MeetingStatus::Active,
        "Single absent tick must NOT transition to Ended or Inactive"
    );

    // --- PHASE D: Wait for 2nd disappearance tick (tick 5) ---
    for _ in 0..50 {
        if det_clone.ticks_executed() >= 5 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // Give brief moment for state write
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(
        handle.get_status(),
        MeetingStatus::Inactive,
        "2 consecutive absent ticks MUST transition back to Inactive"
    );

    handle.stop().unwrap();
}

// ============================================================================
// 3. Camera Kill Switch Latency & Concurrency Attack Tests
// ============================================================================

#[test]
fn test_kill_switch_sub_millisecond_latency() {
    let switch = CameraKillSwitch::new();

    // Warm-up iteration
    let _ = switch.trigger();
    switch.reset();

    let iterations = 1000;
    let mut max_duration = Duration::ZERO;
    let mut total_duration = Duration::ZERO;

    for _ in 0..iterations {
        switch.reset();
        let start = Instant::now();
        let status = switch.trigger();
        let elapsed = start.elapsed();

        assert_eq!(status.status, "KilledByMeeting");
        assert!(status.camera_handle_closed);

        total_duration += elapsed;
        if elapsed > max_duration {
            max_duration = elapsed;
        }
    }

    let avg_duration_micros = total_duration.as_micros() as f64 / iterations as f64;
    let max_duration_micros = max_duration.as_micros() as f64;

    println!(
        "[LATENCY BENCHMARK] Average: {:.3} µs ({:.4} ms), Max: {:.3} µs ({:.4} ms)",
        avg_duration_micros,
        avg_duration_micros / 1000.0,
        max_duration_micros,
        max_duration_micros / 1000.0
    );

    // Must be strictly sub-millisecond (< 1.0 ms = < 1000 µs)
    assert!(
        avg_duration_micros < 1000.0,
        "Average latency must be < 1.0ms, got {} µs",
        avg_duration_micros
    );
    assert!(
        max_duration_micros < 1000.0,
        "Max latency must be < 1.0ms, got {} µs",
        max_duration_micros
    );
}

#[test]
fn test_kill_switch_concurrency_race_10_threads() {
    let switch = Arc::new(CameraKillSwitch::new());
    let barrier = Arc::new(Barrier::new(10));
    let handles_closed_count = Arc::new(AtomicUsize::new(0));
    let already_released_count = Arc::new(AtomicUsize::new(0));

    let mut thread_handles = Vec::with_capacity(10);

    for _ in 0..10 {
        let sw = switch.clone();
        let b = barrier.clone();
        let closed = handles_closed_count.clone();
        let released = already_released_count.clone();

        let handle = thread::spawn(move || {
            // Synchronize all 10 threads to line up at the barrier before racing
            b.wait();

            let status = sw.trigger();
            if status.camera_handle_closed {
                closed.fetch_add(1, Ordering::SeqCst);
            } else {
                released.fetch_add(1, Ordering::SeqCst);
            }
        });
        thread_handles.push(handle);
    }

    for h in thread_handles {
        h.join().expect("Thread panicked during kill switch race");
    }

    let closed = handles_closed_count.load(Ordering::SeqCst);
    let released = already_released_count.load(Ordering::SeqCst);

    println!(
        "[CONCURRENCY ATTACK] Closed: {}, Released: {}",
        closed, released
    );

    // Strict requirement: EXACTLY 1 returns camera_handle_closed: true,
    // EXACTLY 9 return camera_handle_closed: false
    assert_eq!(
        closed, 1,
        "Exactly 1 thread must return camera_handle_closed: true, got {}",
        closed
    );
    assert_eq!(
        released, 9,
        "Exactly 9 threads must return camera_handle_closed: false, got {}",
        released
    );
}

#[test]
fn test_global_kill_switch_concurrency_race_10_threads() {
    // Also test via public airctrl_meetings::trigger_camera_kill_switch()
    let global_switch = airctrl_meetings::get_global_camera_kill_switch();
    global_switch.reset(); // ensure fresh active state
    assert!(global_switch.is_camera_active());

    let barrier = Arc::new(Barrier::new(10));
    let handles_closed_count = Arc::new(AtomicUsize::new(0));
    let already_released_count = Arc::new(AtomicUsize::new(0));

    let mut thread_handles = Vec::with_capacity(10);

    for _ in 0..10 {
        let b = barrier.clone();
        let closed = handles_closed_count.clone();
        let released = already_released_count.clone();

        let handle = thread::spawn(move || {
            b.wait();

            let status = airctrl_meetings::trigger_camera_kill_switch().expect("Kill switch error");
            if status.camera_handle_closed {
                closed.fetch_add(1, Ordering::SeqCst);
            } else {
                released.fetch_add(1, Ordering::SeqCst);
            }
        });
        thread_handles.push(handle);
    }

    for h in thread_handles {
        h.join().expect("Thread panicked during global kill switch race");
    }

    let closed = handles_closed_count.load(Ordering::SeqCst);
    let released = already_released_count.load(Ordering::SeqCst);

    println!(
        "[GLOBAL CONCURRENCY ATTACK] Closed: {}, Released: {}",
        closed, released
    );

    assert_eq!(
        closed, 1,
        "Exactly 1 thread must close global camera handle, got {}",
        closed
    );
    assert_eq!(
        released, 9,
        "Exactly 9 threads must see already released global camera handle, got {}",
        released
    );
}
