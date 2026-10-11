use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;

#[tokio::test]
async fn test_challenger_bus_high_concurrency_multi_producer_multi_consumer() {
    let bus = Arc::new(EventBus::new(2048));
    let num_publishers = 20;
    let events_per_publisher = 250; // 5,000 events total
    let num_subscribers = 5;

    let mut sub_handles = Vec::new();
    let total_received = Arc::new(AtomicUsize::new(0));
    let total_lagged = Arc::new(AtomicUsize::new(0));

    let publishers_done = Arc::new(std::sync::atomic::AtomicBool::new(false));

    for _s in 0..num_subscribers {
        let mut rx = bus.subscribe();
        let counter = total_received.clone();
        let lag_counter = total_lagged.clone();
        let done_flag = publishers_done.clone();
        sub_handles.push(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(_event) => {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        lag_counter.fetch_add(skipped as usize, Ordering::Relaxed);
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                }
                if done_flag.load(Ordering::Relaxed) {
                    // Drain any remaining buffered events before exiting
                    loop {
                        match rx.try_recv() {
                            Ok(_) => {
                                counter.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                                lag_counter.fetch_add(n as usize, Ordering::Relaxed);
                            }
                            Err(tokio::sync::broadcast::error::TryRecvError::Empty) | Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                                break;
                            }
                        }
                    }
                    break;
                }
            }
        }));
    }

    assert_eq!(bus.subscriber_count(), num_subscribers);

    let mut pub_handles = Vec::new();
    for p in 0..num_publishers {
        let bus_clone = bus.clone();
        pub_handles.push(tokio::spawn(async move {
            for i in 0..events_per_publisher {
                let event = DomainEvent::ActionTriggered(ActionCommand::WindowFocus {
                    window_id: (p * 1000 + i) as u64,
                    query: format!("window-{}-{}", p, i),
                });
                let recipients = bus_clone.publish(event).expect("publish must succeed");
                assert_eq!(recipients, num_subscribers);
                if i % 25 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        }));
    }

    for p in pub_handles {
        p.await.unwrap();
    }

    publishers_done.store(true, Ordering::Relaxed);
    // Push sentinel event to wake up subscribers
    let _ = bus.publish(DomainEvent::SttStateChanged(SttState::Idle));

    for s in sub_handles {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(3), s).await;
    }

    let final_received = total_received.load(Ordering::Relaxed);
    let final_lagged = total_lagged.load(Ordering::Relaxed);
    let total_accounted = final_received + final_lagged;
    println!(
        "[+] Multi-producer multi-consumer: {} received, {} lagged (total accounted: {}) across {} subscribers",
        final_received, final_lagged, total_accounted, num_subscribers
    );
    // Each of the 5 subscribers must account for the published events (plus 1 sentinel event)
    let expected_total = num_subscribers * (num_publishers * events_per_publisher + 1);
    assert_eq!(
        total_accounted, expected_total,
        "Every single published event across all subscribers must be 100% accounted for (received or lagged)"
    );
}

#[tokio::test]
async fn test_challenger_bus_dynamic_subscriber_churn() {
    let bus = Arc::new(EventBus::new(512));
    let churn_active = Arc::new(std::sync::atomic::AtomicBool::new(true));

    // Background publisher task
    let bus_pub = bus.clone();
    let active_flag = churn_active.clone();
    let pub_handle = tokio::spawn(async move {
        let mut i = 0u64;
        while active_flag.load(Ordering::Relaxed) {
            let event = DomainEvent::SttStateChanged(SttState::Processing);
            let _ = bus_pub.publish(event);
            i += 1;
            if i.is_multiple_of(50) {
                tokio::task::yield_now().await;
            }
        }
        i
    });

    // 10 concurrent churners: subscribe, read 5 events, drop receiver, repeat
    let mut churn_handles = Vec::new();
    for _ in 0..10 {
        let bus_churn = bus.clone();
        churn_handles.push(tokio::spawn(async move {
            for _ in 0..20 {
                let mut rx = bus_churn.subscribe();
                for _ in 0..5 {
                    let _ = rx.recv().await;
                }
                drop(rx);
                tokio::task::yield_now().await;
            }
        }));
    }

    for h in churn_handles {
        h.await.unwrap();
    }

    churn_active.store(false, Ordering::Relaxed);
    let total_published = pub_handle.await.unwrap();
    println!("[+] Churn test: successfully published {} events during dynamic subscriber churn", total_published);
}

#[tokio::test]
async fn test_challenger_bus_zero_subscriber_transitions() {
    let bus = EventBus::new(64);

    // Phase 1: 0 subscribers -> publish 1000 events -> all Ok(0)
    for _ in 0..1000 {
        let res = bus.publish(DomainEvent::SttStateChanged(SttState::Idle));
        assert_eq!(res.unwrap(), 0);
    }
    assert_eq!(bus.subscriber_count(), 0);

    // Phase 2: Add 3 subscribers -> publish 500 events -> all Ok(3)
    let rx1 = bus.subscribe();
    let rx2 = bus.subscribe();
    let rx3 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 3);

    for _ in 0..500 {
        let res = bus.publish(DomainEvent::SttStateChanged(SttState::Recording));
        assert_eq!(res.unwrap(), 3);
    }

    // Phase 3: Drop all 3 subscribers -> publish 1000 events -> all Ok(0)
    drop(rx1);
    drop(rx2);
    drop(rx3);
    assert_eq!(bus.subscriber_count(), 0);

    for _ in 0..1000 {
        let res = bus.publish(DomainEvent::SttStateChanged(SttState::Idle));
        assert_eq!(res.unwrap(), 0);
    }

    // Phase 4: Dynamic re-subscribe -> publish -> Ok(1)
    let _rx_new = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 1);
    let res = bus.publish(DomainEvent::SttStateChanged(SttState::Injecting));
    assert_eq!(res.unwrap(), 1);
    println!("[+] Zero subscriber transitions passed 100% cleanly without channel error or deadlock");
}

#[tokio::test]
async fn test_challenger_bus_ring_buffer_overflow_lag_accounting() {
    let capacity = 16;
    let bus = EventBus::new(capacity);
    let mut rx = bus.subscribe();

    let total_published = 256;
    for i in 0..total_published {
        let event = DomainEvent::ActionTriggered(ActionCommand::WindowClose { window_id: i as u64 });
        bus.publish(event).unwrap();
    }

    // Receiver was slow: should receive Lagged(240) because buffer holds only 16
    match rx.recv().await {
        Err(RecvError::Lagged(skipped)) => {
            assert_eq!(skipped, 240, "Expected exactly 240 skipped events (256 sent - 16 capacity)");
        }
        other => panic!("Expected RecvError::Lagged, got {:?}", other),
    }

    // Next event received must be the 241st event (window_id = 240)
    let next_event = rx.recv().await.unwrap();
    assert_eq!(
        next_event,
        DomainEvent::ActionTriggered(ActionCommand::WindowClose { window_id: 240 })
    );
    println!("[+] Lag accounting accurately verified: 240 skipped, recovered event #240");
}
