use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::platform::create_mock_platform;
use airctrl_core::types::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_adversarial_action_command_serialization_invariants() {
    let variants = vec![
        ActionCommand::WindowFocus {
            window_id: u64::MAX,
            query: "  Complex Window Title \u{1F916}\t\n  ".to_string(),
        },
        ActionCommand::WindowClose { window_id: 0 },
        ActionCommand::WindowMinimize { window_id: 42 },
        ActionCommand::WindowSnap {
            direction: SnapDirection::Maximize,
        },
        ActionCommand::WindowSnap {
            direction: SnapDirection::Left,
        },
        ActionCommand::WindowSnap {
            direction: SnapDirection::Center,
        },
        ActionCommand::LaunchApp {
            app_name: "C:\\Program Files\\App\\app.exe".to_string(),
        },
        ActionCommand::CloseApp {
            app_name: "slack".to_string(),
        },
        ActionCommand::CenterApp {
            app_name: "slack".to_string(),
        },
        ActionCommand::AgentDispatch(AgentPayload {
            target: AgentTarget::Antigravity,
            prompt: "Test prompt with quote ' and double quote \" and backslash \\".to_string(),
            context_window: Some("Large context window".to_string()),
            timestamp: "2026-10-11T12:00:00Z".to_string(),
        }),
        ActionCommand::ReaInspect {
            target_pid: u32::MAX,
            binary_path: Some("/opt/bin/test_app".to_string()),
        },
        ActionCommand::RadialDialToggle {},
        ActionCommand::SystemInput(SystemInputAction::Scroll {
            direction: ScrollDirection::Down,
            amount: 5,
        }),
        ActionCommand::SystemInput(SystemInputAction::Click {
            position: Some(ClickPosition::Coordinates { x: -1920, y: 1080 }),
            button: ClickAction::RightClick,
        }),
        ActionCommand::SystemInput(SystemInputAction::KeyCombination {
            modifiers: vec![ModifierKey::Control, ModifierKey::Alt],
            key: InputKey::Space,
        }),
        ActionCommand::Unknown {
            query: "\0\r\n\x1b[31mRedText\x1b[0m".to_string(),
        },
    ];

    for cmd in &variants {
        let json = serde_json::to_string(cmd).expect("Serialization failed");
        let decoded: ActionCommand =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(cmd, &decoded, "Roundtrip mismatch for command {:?}", cmd);
    }
}

#[test]
fn test_adversarial_action_command_malformed_json_rejections() {
    let malformed_cases = [
        r#"{"type": "NonExistentCommand", "payload": {}}"#,
        r#"{"type": "WindowFocus", "payload": {"window_id": "not_a_number", "query": "test"}}"#,
        r#"{"type": "WindowFocus", "payload": {"query": "missing_id"}}"#,
        r#"{"type": "WindowSnap", "payload": {"direction": "InvalidDirection"}}"#,
        r#"{"type": "AgentDispatch", "payload": {"target": "InvalidAgent", "prompt": "p", "timestamp": "t"}}"#,
        r#"{"type": 12345}"#,
        r#"[]"#,
        r#""just_a_string""#,
        r#"{"type": null}"#,
    ];

    for malformed in &malformed_cases {
        let result: Result<ActionCommand, _> = serde_json::from_str(malformed);
        assert!(
            result.is_err(),
            "Expected malformed JSON to be rejected, but it parsed: {}",
            malformed
        );
    }
}

#[test]
fn test_adversarial_domain_event_serialization_roundtrip() {
    let events = vec![
        DomainEvent::SttStateChanged(SttState::Recording),
        DomainEvent::TranscriptionCompleted(TranscriptionPayload {
            text: "Hello world transcribed text".to_string(),
            duration_ms: 1250,
            confidence: 0.98,
            target_window_title: Some("Editor".to_string()),
            target_process_name: Some("code.exe".to_string()),
        }),
        DomainEvent::SpeechStateChanged(SpeechState::Listening),
        DomainEvent::ActionTriggered(ActionCommand::RadialDialToggle {}),
        DomainEvent::GestureRecognized {
            gesture: GestureType::Pointing,
            dial: DialCoordinates {
                angle_rad: std::f32::consts::PI,
                normalized_radius: 0.75,
                active_sector: 3,
                is_selected: true,
            },
        },
        DomainEvent::MeetingStatusChanged(MeetingStatus::Active),
        DomainEvent::MeetingDetected(MeetingInfo {
            platform: MeetingPlatform::MicrosoftTeams,
            process_id: 12345,
            process_name: "teams.exe".to_string(),
            window_title: "Teams Standup Meeting".to_string(),
            detected_at: "2026-10-11T12:00:00Z".to_string(),
        }),
        DomainEvent::CameraKillSwitchTriggered {
            meeting: MeetingInfo {
                platform: MeetingPlatform::Zoom,
                process_id: 54321,
                process_name: "zoom.exe".to_string(),
                window_title: "Zoom Call".to_string(),
                detected_at: "2026-10-11T12:05:00Z".to_string(),
            },
            camera_state: CameraState::KilledByMeeting,
        },
        DomainEvent::ComplianceAudioInjected(ComplianceInjectionEvent {
            meeting_info: MeetingInfo {
                platform: MeetingPlatform::GoogleMeet,
                process_id: 9999,
                process_name: "chrome.exe".to_string(),
                window_title: "Google Meet".to_string(),
                detected_at: "2026-10-11T12:10:00Z".to_string(),
            },
            message: "This meeting is being recorded for legal compliance.".to_string(),
            audio_duration_ms: 2200,
            status: "Injected".to_string(),
        }),
        DomainEvent::LanServerStarted(ServerMetadata {
            server_ip: "192.168.1.100".to_string(),
            port: 8080,
            version: "0.1.0".to_string(),
            binary_name: "airctrl.exe".to_string(),
            sha256: Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()),
            build_timestamp: "2026-10-11T12:00:00Z".to_string(),
        }),
    ];

    for event in &events {
        let json = serde_json::to_string(event).expect("Event serialization failed");
        let decoded: DomainEvent =
            serde_json::from_str(&json).expect("Event deserialization failed");
        assert_eq!(event, &decoded, "Event roundtrip mismatch for {:?}", event);
    }
}

#[tokio::test]
async fn test_adversarial_event_bus_zero_subscribers_and_lag_overflow() {
    let bus = EventBus::new(8); // Small buffer for testing overflow

    // Invariant: publishing with 0 subscribers returns Ok(0)
    let res = bus.publish(DomainEvent::SttStateChanged(SttState::Idle));
    assert_eq!(res.unwrap(), 0, "Publish with 0 subscribers must return Ok(0)");

    let mut rx = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 1);

    // Publish 1 event and read it
    bus.publish(DomainEvent::SttStateChanged(SttState::Recording)).unwrap();
    let received = rx.recv().await.unwrap();
    assert_eq!(received, DomainEvent::SttStateChanged(SttState::Recording));

    // Overfill the buffer beyond capacity (capacity is 8, publish 16 events)
    for i in 0..16 {
        bus.publish(DomainEvent::SttStateChanged(if i % 2 == 0 {
            SttState::Idle
        } else {
            SttState::Processing
        }))
        .unwrap();
    }

    // Next recv must report Lagged
    let lag_result = rx.recv().await;
    match lag_result {
        Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
            assert!(
                missed > 0,
                "Expected non-zero missed messages on lag overflow, got {}",
                missed
            );
        }
        other => panic!("Expected RecvError::Lagged, got {:?}", other),
    }
}

#[tokio::test]
async fn test_adversarial_event_bus_concurrent_publisher_subscriber_hammer() {
    let bus = EventBus::new(1024);
    let total_publishers = 10;
    let events_per_publisher = 50;

    let received_count = Arc::new(AtomicUsize::new(0));

    // Spawn 2 subscribers
    let mut sub_handles = Vec::new();
    for _ in 0..2 {
        let mut rx = bus.subscribe();
        let counter = received_count.clone();
        sub_handles.push(tokio::spawn(async move {
            while rx.recv().await.is_ok() {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Spawn 10 publishers
    let mut pub_handles = Vec::new();
    for pub_id in 0..total_publishers {
        let bus_clone = bus.clone();
        pub_handles.push(tokio::spawn(async move {
            for i in 0..events_per_publisher {
                let _ = bus_clone.publish(DomainEvent::ActionTriggered(
                    ActionCommand::LaunchApp {
                        app_name: format!("app_{}_{}", pub_id, i),
                    },
                ));
                tokio::task::yield_now().await;
            }
        }));
    }

    for p in pub_handles {
        p.await.unwrap();
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Drop bus to let subscribers finish
    drop(bus);

    for s in sub_handles {
        s.abort();
    }

    let count = received_count.load(Ordering::Relaxed);
    assert!(
        count >= (total_publishers * events_per_publisher),
        "Expected at least {} events received across subscribers, got {}",
        total_publishers * events_per_publisher,
        count
    );
}

#[test]
fn test_adversarial_mock_platform_concurrency_and_boundaries() {
    let (provider, controllers) = create_mock_platform();

    // Verify initial state has no active window
    assert!(provider.window.get_active_window().is_err());

    // Add windows into mock manager
    controllers.window.add_window(WindowInfo {
        id: 101,
        process_id: 1000,
        process_name: "code.exe".to_string(),
        title: "VS Code".to_string(),
        bounds: WindowRect::default(),
        is_focused: true,
        is_minimized: false,
    });
    controllers.window.add_window(WindowInfo {
        id: 102,
        process_id: 1001,
        process_name: "chrome.exe".to_string(),
        title: "Google Chrome".to_string(),
        bounds: WindowRect::default(),
        is_focused: false,
        is_minimized: false,
    });

    let active = provider.window.get_active_window().unwrap();
    assert_eq!(active.id, 101);

    // Test text typing accumulation
    provider.input.type_text("First batch: ").unwrap();
    provider.input.type_text("Second batch 🚀").unwrap();
    let typed = controllers.input.get_typed_strings();
    assert_eq!(typed, vec!["First batch: ", "Second batch 🚀"]);

    // Test clipboard paste
    provider.input.paste_text("Pasted content").unwrap();
    let pasted = controllers.input.get_pasted_strings();
    assert_eq!(pasted, vec!["Pasted content"]);

    // Test focus non-existent window
    let focus_res = provider.window.focus_window(999999);
    assert!(
        focus_res.is_err(),
        "Focusing non-existent window must return error"
    );

    // Test focus existing window
    provider.window.focus_window(102).unwrap();
    let new_active = provider.window.get_active_window().unwrap();
    assert_eq!(new_active.id, 102);

    // Verify focus history in controller
    let history = controllers.window.get_focus_history();
    assert!(history.contains(&102));

    // Audio stream playback
    let samples = vec![0.5f32; 1600];
    provider.audio.play_audio_stream(&samples, 16000).unwrap();
    let played = controllers.audio.get_played_buffers();
    assert_eq!(played.len(), 1);
    assert_eq!(played[0].0.len(), 1600);
    assert_eq!(played[0].1, 16000);
}
