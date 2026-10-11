use airctrl_core::error::CoreError;
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::*;
use std::io::{Error as IoError, ErrorKind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;

#[test]
fn test_error_conversions_and_display() {
    // 1. std::io::Error conversions across error kinds
    let io_err_not_found = IoError::new(ErrorKind::NotFound, "file not found on disk");
    let core_err_io: CoreError = io_err_not_found.into();
    assert!(matches!(core_err_io, CoreError::Io(_)));
    assert_eq!(core_err_io.to_string(), "I/O error: file not found on disk");

    let io_err_conn_reset = IoError::new(ErrorKind::ConnectionReset, "connection reset by peer");
    let core_err_conn: CoreError = io_err_conn_reset.into();
    assert!(core_err_conn.to_string().contains("connection reset by peer"));

    // 2. serde_json::Error conversions
    let malformed_json = "{ \"category\": \"SttStateChanged\", invalid }";
    let serde_err = serde_json::from_str::<DomainEvent>(malformed_json).unwrap_err();
    let core_err_serde: CoreError = serde_err.into();
    assert!(matches!(core_err_serde, CoreError::Serialization(_)));
    assert!(core_err_serde.to_string().starts_with("Serialization error:"));

    // 3. Trait implementations: Clone, PartialEq, Eq, Display
    let err1 = CoreError::WindowNotFound(1001);
    let err2 = err1.clone();
    assert_eq!(err1, err2);
    assert_eq!(err1.to_string(), "Target window not found (ID: 1001)");

    let proc_err = CoreError::ProcessNotFound(42);
    assert_eq!(proc_err.to_string(), "Process not found (PID: 42)");
}

#[test]
fn test_serialization_corrupted_payloads() {
    // 1. Completely broken JSON syntax
    let broken_syntaxes = vec![
        "",
        "{",
        "{ category: 'SttStateChanged' }",
        "null",
        "DomainEvent",
        "{\"category\": \"NonExistentVariant\", \"data\": {}}",
        "{\"category\": \"SttStateChanged\", \"data\": \"NonExistentState\"}",
        "{\"category\": \"ActionTriggered\", \"data\": {\"type\": \"UnknownType\"}}",
    ];

    for raw in broken_syntaxes {
        let result = serde_json::from_str::<DomainEvent>(raw);
        assert!(result.is_err(), "Payload should have failed parsing: {}", raw);
    }

    // 2. HandLandmarks with invalid array sizes (expected exactly 21)
    let bad_landmarks_json = r#"{
        "landmarks": [{"x": 0.0, "y": 0.0, "z": 0.0}],
        "handedness": "Left",
        "confidence": 0.95
    }"#;
    assert!(serde_json::from_str::<HandLandmarks>(bad_landmarks_json).is_err());

    // 3. Type confusion / mismatch
    let type_confusion_json = r#"{
        "category": "MeetingDetected",
        "data": {
            "platform": "Zoom",
            "process_id": "not-a-number",
            "process_name": "zoom.exe",
            "window_title": "Zoom",
            "detected_at": "2026-10-11T00:00:00Z"
        }
    }"#;
    assert!(serde_json::from_str::<DomainEvent>(type_confusion_json).is_err());
}

#[test]
fn test_serialization_oversized_payloads() {
    // 1. 5MB Prompt in AgentPayload
    let large_string = "A".repeat(5 * 1024 * 1024);
    let event = DomainEvent::ActionTriggered(ActionCommand::AgentDispatch(AgentPayload {
        target: AgentTarget::Claude,
        prompt: large_string.clone(),
        context_window: Some("Large context".to_string()),
        timestamp: "2026-10-11T08:00:00Z".to_string(),
    }));

    let serialized = serde_json::to_string(&event).expect("Serialize 5MB string payload");
    assert!(serialized.len() >= 5 * 1024 * 1024);

    let deserialized: DomainEvent = serde_json::from_str(&serialized).expect("Deserialize 5MB payload");
    assert_eq!(event, deserialized);

    // 2. Massive AudioBuffer (1,000,000 float samples = 4MB)
    let big_audio = AudioBuffer {
        samples: vec![0.123f32; 1_000_000],
        sample_rate: 16000,
        channels: 1,
    };
    let audio_json = serde_json::to_string(&big_audio).expect("Serialize big audio buffer");
    let audio_recovered: AudioBuffer = serde_json::from_str(&audio_json).expect("Deserialize big audio buffer");
    assert_eq!(audio_recovered.samples.len(), 1_000_000);
}

#[tokio::test]
async fn test_event_bus_zero_subscribers_behavior() {
    let bus = EventBus::new(16);
    assert_eq!(bus.subscriber_count(), 0);

    let event = DomainEvent::SttStateChanged(SttState::Idle);
    let res = bus.publish(event);
    assert_eq!(
        res.expect("publishing without subscribers should return Ok(0)"),
        0,
        "Expected 0 receivers to be reported when publishing without subscribers"
    );
}

#[tokio::test]
async fn test_event_bus_overflow_and_lagged_receiver() {
    let capacity = 8;
    let bus = EventBus::new(capacity);
    let mut rx_slow = bus.subscribe();
    let mut rx_fast = bus.subscribe();

    assert_eq!(bus.subscriber_count(), 2);

    // Publish 32 events into an 8-capacity channel
    for i in 0..32 {
        let event = DomainEvent::ActionTriggered(ActionCommand::WindowClose { window_id: i });
        bus.publish(event).expect("publish event");
    }

    // Slow receiver overflow detection
    let slow_recv = rx_slow.recv().await;
    match slow_recv {
        Err(RecvError::Lagged(skipped)) => {
            // Buffer size is 8, 32 sent -> skipped should be 24
            assert_eq!(skipped, 24, "Expected 24 skipped events due to 8-slot ring buffer overflow");
        }
        other => panic!("Expected RecvError::Lagged, got {:?}", other),
    }

    // Fast receiver also experiences Lagged(24)
    let fast_recv = rx_fast.recv().await;
    match fast_recv {
        Err(RecvError::Lagged(skipped)) => {
            assert_eq!(skipped, 24);
        }
        other => panic!("Expected RecvError::Lagged, got {:?}", other),
    }

    // Subsequent read should succeed and yield the 25th event (window_id = 24)
    let next_event = rx_slow.recv().await.expect("receive next valid event");
    assert_eq!(
        next_event,
        DomainEvent::ActionTriggered(ActionCommand::WindowClose { window_id: 24 })
    );
}

#[tokio::test]
async fn test_event_bus_burst_and_lag_accounting() {
    let bus = Arc::new(EventBus::new(1024));
    let message_count = 5000;
    let mut rx = bus.subscribe();

    let received_counter = Arc::new(AtomicUsize::new(0));
    let skipped_counter = Arc::new(AtomicUsize::new(0));
    let r_clone = received_counter.clone();
    let s_clone = skipped_counter.clone();

    // Consumer task
    let consumer_handle = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(_) => {
                    r_clone.fetch_add(1, Ordering::Relaxed);
                }
                Err(RecvError::Lagged(n)) => {
                    s_clone.fetch_add(n as usize, Ordering::Relaxed);
                }
                Err(RecvError::Closed) => break,
            }
            if r_clone.load(Ordering::Relaxed) + s_clone.load(Ordering::Relaxed) >= message_count {
                break;
            }
        }
    });

    // 5 concurrent producers
    let producers: Vec<_> = (0..5)
        .map(|producer_id| {
            let bus = bus.clone();
            tokio::spawn(async move {
                for i in 0..1000 {
                    let event = DomainEvent::ActionTriggered(ActionCommand::WindowFocus {
                        window_id: (producer_id * 1000 + i) as u64,
                        query: format!("test-query-{}-{}", producer_id, i),
                    });
                    let _ = bus.publish(event);
                }
            })
        })
        .collect();

    for p in producers {
        p.await.unwrap();
    }

    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), consumer_handle).await;
    let total_accounted = received_counter.load(Ordering::Relaxed) + skipped_counter.load(Ordering::Relaxed);
    assert_eq!(
        total_accounted, message_count,
        "Every event must be accounted for as either received or lagged/skipped"
    );
}
