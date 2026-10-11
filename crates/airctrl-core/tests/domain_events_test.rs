use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::*;

#[tokio::test]
async fn test_domain_events_serialization_and_pubsub() {
    let bus = EventBus::new(16);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    assert_eq!(bus.subscriber_count(), 2);

    let event = DomainEvent::SttStateChanged(SttState::Recording);

    // Verify JSON serialization round-trip
    let json = serde_json::to_string(&event).expect("serialize event");
    let deserialized: DomainEvent = serde_json::from_str(&json).expect("deserialize event");
    assert_eq!(event, deserialized);

    // Publish event
    bus.publish(event.clone()).expect("publish event");

    let received1 = rx1.recv().await.expect("rx1 receive");
    let received2 = rx2.recv().await.expect("rx2 receive");

    assert_eq!(received1, event);
    assert_eq!(received2, event);
}

#[test]
fn test_meeting_and_gesture_events_serialization() {
    let meeting_event = DomainEvent::MeetingDetected(MeetingInfo {
        platform: MeetingPlatform::Zoom,
        process_id: 9999,
        process_name: "zoom.exe".into(),
        window_title: "Zoom Meeting".into(),
        detected_at: "2026-10-11T07:25:00Z".into(),
    });

    let json = serde_json::to_string(&meeting_event).expect("serialize meeting event");
    let decoded: DomainEvent = serde_json::from_str(&json).expect("deserialize meeting event");
    assert_eq!(meeting_event, decoded);
}

#[tokio::test]
async fn test_event_bus_zero_subscribers_then_dynamic_subscribe() {
    let bus = EventBus::new(16);
    assert_eq!(bus.subscriber_count(), 0);

    // Publish with 0 subscribers returns Ok(0)
    let count0 = bus
        .publish(DomainEvent::SttStateChanged(SttState::Idle))
        .expect("publish with 0 subscribers");
    assert_eq!(count0, 0);

    // Dynamically register subscriber
    let mut rx = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 1);

    // Subsequent publish returns Ok(1) and is received
    let count1 = bus
        .publish(DomainEvent::SttStateChanged(SttState::Recording))
        .expect("publish with 1 subscriber");
    assert_eq!(count1, 1);

    let received = rx.recv().await.expect("receive event");
    assert_eq!(received, DomainEvent::SttStateChanged(SttState::Recording));
}

