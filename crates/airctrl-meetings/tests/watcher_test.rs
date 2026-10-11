use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_meetings::config::MeetingWatcherConfig;
use airctrl_meetings::detector::MeetingDetector;
use airctrl_meetings::error::MeetingError;
use airctrl_meetings::privacy::CameraKillSwitch;
use airctrl_meetings::types::{
    ActiveMeetingInfo, MeetingApp, MeetingStatus, ProcessScanItem,
};
use airctrl_meetings::watcher::MeetingWatcher;
use chrono::Utc;
use std::sync::{Arc, RwLock};
use std::time::Duration;

struct MockDynamicDetector {
    meeting: Arc<RwLock<Option<ActiveMeetingInfo>>>,
}

impl MeetingDetector for MockDynamicDetector {
    fn detect_meeting(&self, _items: &[ProcessScanItem]) -> Option<ActiveMeetingInfo> {
        self.meeting.read().unwrap().clone()
    }

    fn scan_system(&self) -> Result<Option<ActiveMeetingInfo>, MeetingError> {
        Ok(self.meeting.read().unwrap().clone())
    }
}

#[test]
fn test_sync_scan_processes_triggers_kill_switch() {
    let watcher = MeetingWatcher::default();
    assert!(watcher.is_camera_active());

    let procs = vec![ProcessScanItem::new(100, "zoom.exe", "Zoom")];
    let meeting = watcher.scan_processes(&procs);

    assert!(meeting.is_some());
    let m = meeting.unwrap();
    assert_eq!(m.process_name, "zoom");
    assert!(!watcher.is_camera_active());
}

#[tokio::test]
async fn test_background_watcher_lifecycle_and_debounce() {
    let mock_meeting = Arc::new(RwLock::new(None));
    let detector = Arc::new(MockDynamicDetector {
        meeting: mock_meeting.clone(),
    });

    let kill_switch = Arc::new(CameraKillSwitch::new());
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();

    let config = MeetingWatcherConfig {
        poll_interval_ms: 30, // fast poll for test speed
        debounce_ticks: 2,
        auto_kill_camera: true,
        auto_compliance_audio: false,
        ..Default::default()
    };

    let watcher = MeetingWatcher::new(config)
        .with_detector(detector)
        .with_kill_switch(kill_switch.clone())
        .with_event_bus(bus);

    let handle = watcher.start().expect("Watcher start failed");
    assert!(handle.is_running());
    assert_eq!(handle.get_status(), MeetingStatus::Inactive);

    // Initial state: no meeting
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(handle.get_status(), MeetingStatus::Inactive);

    // Set active meeting in detector
    *mock_meeting.write().unwrap() = Some(ActiveMeetingInfo {
        app: MeetingApp::Zoom,
        window_id: Some(101),
        pid: 202,
        title: "Zoom Call".to_string(),
        start_time: Utc::now(),
    });

    // Wait for 2 ticks to trigger debounce transition (30ms * 3 = 90ms)
    tokio::time::sleep(Duration::from_millis(120)).await;

    assert_eq!(handle.get_status(), MeetingStatus::Active);
    assert!(handle.get_active_meeting().is_some());
    assert!(!kill_switch.is_camera_active());

    // Collect published events
    let mut received_events = Vec::new();
    while let Ok(evt) = rx.try_recv() {
        received_events.push(evt);
    }

    assert!(received_events.iter().any(|e| matches!(
        e,
        DomainEvent::MeetingStatusChanged(MeetingStatus::Detected)
    )));
    assert!(received_events
        .iter()
        .any(|e| matches!(e, DomainEvent::MeetingDetected(_))));
    assert!(received_events.iter().any(|e| matches!(
        e,
        DomainEvent::MeetingStatusChanged(MeetingStatus::Active)
    )));
    assert!(received_events
        .iter()
        .any(|e| matches!(e, DomainEvent::CameraKillSwitchTriggered { .. })));

    // Now remove meeting from detector
    *mock_meeting.write().unwrap() = None;

    // Wait for 2 ticks disappearance debounce
    tokio::time::sleep(Duration::from_millis(120)).await;

    assert_eq!(handle.get_status(), MeetingStatus::Inactive);
    assert!(handle.get_active_meeting().is_none());

    let mut end_events = Vec::new();
    while let Ok(evt) = rx.try_recv() {
        end_events.push(evt);
    }

    assert!(end_events.iter().any(|e| matches!(
        e,
        DomainEvent::MeetingStatusChanged(MeetingStatus::Ended)
    )));
    assert!(end_events
        .iter()
        .any(|e| matches!(e, DomainEvent::MeetingEnded(_))));
    assert!(end_events.iter().any(|e| matches!(
        e,
        DomainEvent::MeetingStatusChanged(MeetingStatus::Inactive)
    )));

    // Stop watcher
    handle.stop().expect("Failed to stop watcher");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!handle.is_running());
}
