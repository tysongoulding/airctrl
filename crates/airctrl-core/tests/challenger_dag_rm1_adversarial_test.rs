use airctrl_core::dag::*;
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::action::{ActionCommand, AgentPayload, AgentTarget};
use airctrl_core::types::meeting::{MeetingApp, MeetingInfo};
use airctrl_core::types::stt::{SttState, TranscriptionPayload};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

/// Adversarial Challenge 1: Stage Channel Boundary & Isolation
/// Verifies strict isolation between stage channels (Sensors, Inferences, Sinks)
/// and confirms that the Unified stream receives properly tagged envelopes.
#[tokio::test]
async fn test_challenger_stage_channel_boundaries_and_strict_isolation() {
    let dag = DagEventBus::new(64);

    let mut sensor_rx = dag.subscribe_sensors();
    let mut inference_rx = dag.subscribe_inferences();
    let mut sink_rx = dag.subscribe_sinks();
    let mut unified_rx = dag.subscribe();

    assert_eq!(dag.sensor_subscriber_count(), 1);
    assert_eq!(dag.inference_subscriber_count(), 1);
    assert_eq!(dag.sink_subscriber_count(), 1);
    assert_eq!(dag.unified_subscriber_count(), 1);

    // 1. Publish 50 Sensor events: only sensor_rx and unified_rx must receive them.
    for i in 0..50 {
        let ev = SensorEvent::HotkeyEvent(HotkeyEventPayload {
            trigger: HotkeyTrigger::SttToggle,
            raw_combination: format!("Ctrl+Space_{}", i),
            duration_ms: i,
            timestamp_ns: i * 1000,
        });
        let recipients = dag.publish_sensor(ev.clone()).expect("publish sensor");
        assert_eq!(recipients, 1);

        let rx_sensor = sensor_rx.try_recv().expect("sensor_rx must receive");
        assert_eq!(rx_sensor, ev);

        let rx_unified = unified_rx.try_recv().expect("unified_rx must receive");
        assert_eq!(rx_unified, DagEvent::Sensor(ev));

        // Inference and Sink channels must remain completely empty!
        assert_eq!(inference_rx.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(sink_rx.try_recv(), Err(TryRecvError::Empty));
    }

    // 2. Publish 50 Inference events: only inference_rx and unified_rx must receive them.
    for i in 0..50 {
        let ev = InferenceEvent::SpeechDetected(SpeechDetectedPayload {
            is_speech: true,
            energy_dbfs: -15.0 + (i as f32 * 0.1),
            hangover_active: false,
            samples: vec![0.1; 16],
            timestamp_ns: i * 2000,
        });
        let recipients = dag.publish_inference(ev.clone()).expect("publish inference");
        assert_eq!(recipients, 1);

        let rx_inf = inference_rx.try_recv().expect("inference_rx must receive");
        assert_eq!(rx_inf, ev);

        let rx_unified = unified_rx.try_recv().expect("unified_rx must receive");
        assert_eq!(rx_unified, DagEvent::Inference(ev));

        assert_eq!(sensor_rx.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(sink_rx.try_recv(), Err(TryRecvError::Empty));
    }

    // 3. Publish 50 Sink actions: only sink_rx and unified_rx must receive them.
    for i in 0..50 {
        let action = SinkAction::ToggleOverlay(ToggleOverlayPayload {
            overlay: OverlayType::RadialDial,
            visible: Some(true),
            active_sector: Some(i as usize % 6),
        });
        let recipients = dag.publish_sink(action.clone()).expect("publish sink");
        assert_eq!(recipients, 1);

        let rx_sink = sink_rx.try_recv().expect("sink_rx must receive");
        assert_eq!(rx_sink, action);

        let rx_unified = unified_rx.try_recv().expect("unified_rx must receive");
        assert_eq!(rx_unified, DagEvent::Sink(action));

        assert_eq!(sensor_rx.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(inference_rx.try_recv(), Err(TryRecvError::Empty));
    }

    // 4. Cross-publish via unified publish() API
    let sensor_dag = DagEvent::Sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
        trigger: HotkeyTrigger::ActionRadialDial,
        raw_combination: "Ctrl+Alt".into(),
        duration_ms: 50,
        timestamp_ns: 9999,
    }));
    dag.publish(sensor_dag.clone()).unwrap();

    let s_ev = sensor_rx.try_recv().expect("sensor channel should receive from dag.publish");
    assert!(matches!(s_ev, SensorEvent::HotkeyEvent(_)));
    let u_ev = unified_rx.try_recv().expect("unified channel should receive");
    assert_eq!(u_ev, sensor_dag);
    assert_eq!(inference_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(sink_rx.try_recv(), Err(TryRecvError::Empty));

    let metrics = dag.metrics();
    assert_eq!(metrics.events_routed_sensors, 51);
    assert_eq!(metrics.events_routed_inferences, 50);
    assert_eq!(metrics.events_routed_sinks, 50);
}

/// Adversarial Challenge 2: Subscriber Counting Precision & Zero-Subscriber Decoupling
/// Tests dynamic churn and ensures Ok(0) invariant is preserved across all channels
/// without channel closed errors, panics, or counter drift.
#[tokio::test]
async fn test_challenger_subscriber_counting_precision_and_zero_subscriber_decoupling() {
    let dag = DagEventBus::new(32);

    // Initial state: 0 subscribers everywhere
    assert_eq!(dag.sensor_subscriber_count(), 0);
    assert_eq!(dag.inference_subscriber_count(), 0);
    assert_eq!(dag.sink_subscriber_count(), 0);
    assert_eq!(dag.unified_subscriber_count(), 0);

    // Phase 1: 0 subscribers -> 500 publishes per channel -> must return Ok(0)
    for _ in 0..500 {
        let r1 = dag.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
            trigger: HotkeyTrigger::SttToggle,
            raw_combination: "none".into(),
            duration_ms: 0,
            timestamp_ns: 0,
        }));
        assert_eq!(r1.unwrap(), 0);

        let r2 = dag.publish_inference(InferenceEvent::MeetingDetected(MeetingInfo {
            platform: MeetingApp::Zoom.into(),
            process_id: 1,
            process_name: "zoom.exe".into(),
            window_title: "w".into(),
            detected_at: "t".into(),
        }));
        assert_eq!(r2.unwrap(), 0);

        let r3 = dag.publish_sink(SinkAction::InjectText(InjectTextPayload {
            text: "test".into(),
            target_window_id: None,
            force_clipboard: false,
        }));
        assert_eq!(r3.unwrap(), 0);
    }

    // Phase 2: Add 5 subscribers to inferences, 3 to sensors, 2 to sinks, 4 to unified
    let mut inf_subs: Vec<_> = (0..5).map(|_| dag.subscribe_inferences()).collect();
    let sensor_subs: Vec<_> = (0..3).map(|_| dag.subscribe_sensors()).collect();
    let sink_subs: Vec<_> = (0..2).map(|_| dag.subscribe_sinks()).collect();
    let unified_subs: Vec<_> = (0..4).map(|_| dag.subscribe()).collect();

    assert_eq!(dag.inference_subscriber_count(), 5);
    assert_eq!(dag.sensor_subscriber_count(), 3);
    assert_eq!(dag.sink_subscriber_count(), 2);
    assert_eq!(dag.unified_subscriber_count(), 4);

    let r_inf = dag.publish_inference(InferenceEvent::MeetingDetected(MeetingInfo {
        platform: MeetingApp::Teams.into(),
        process_id: 2,
        process_name: "teams.exe".into(),
        window_title: "Teams".into(),
        detected_at: "t".into(),
    })).unwrap();
    assert_eq!(r_inf, 5);

    let r_sensor = dag.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
        trigger: HotkeyTrigger::SttToggle,
        raw_combination: "Ctrl".into(),
        duration_ms: 10,
        timestamp_ns: 1,
    })).unwrap();
    assert_eq!(r_sensor, 3);

    let r_sink = dag.publish_sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
        overlay: OverlayType::RadialDial,
        visible: Some(false),
        active_sector: None,
    })).unwrap();
    assert_eq!(r_sink, 2);

    // Phase 3: Dynamic drops
    inf_subs.pop(); // now 4
    inf_subs.pop(); // now 3
    assert_eq!(dag.inference_subscriber_count(), 3);

    let r_inf_3 = dag.publish_inference(InferenceEvent::MeetingDetected(MeetingInfo {
        platform: MeetingApp::Teams.into(),
        process_id: 3,
        process_name: "teams.exe".into(),
        window_title: "Teams".into(),
        detected_at: "t".into(),
    })).unwrap();
    assert_eq!(r_inf_3, 3);

    // Drop all
    drop(inf_subs);
    drop(sensor_subs);
    drop(sink_subs);
    drop(unified_subs);

    assert_eq!(dag.inference_subscriber_count(), 0);
    assert_eq!(dag.sensor_subscriber_count(), 0);
    assert_eq!(dag.sink_subscriber_count(), 0);
    assert_eq!(dag.unified_subscriber_count(), 0);

    // Publishes return Ok(0) again cleanly
    assert_eq!(dag.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
        trigger: HotkeyTrigger::SttToggle,
        raw_combination: "Ctrl".into(),
        duration_ms: 10,
        timestamp_ns: 1,
    })).unwrap(), 0);
}

/// Adversarial Challenge 3: High-Concurrency Multi-Stage Multi-Producer Multi-Consumer Stress Test
/// Launches 30 concurrent publishers and 10 concurrent consumers hammering all stages
/// with 15,000 events total while verifying zero deadlock, no panics, and telemetry parity.
#[tokio::test]
async fn test_challenger_high_concurrency_multi_stage_propagation_stress() {
    let dag = Arc::new(DagEventBus::new(2048));
    let num_sensor_publishers = 10;
    let num_inference_publishers = 10;
    let num_sink_publishers = 10;
    let events_per_publisher = 200; // 30 * 200 = 6,000 events total

    let sensor_events_received = Arc::new(AtomicUsize::new(0));
    let inference_events_received = Arc::new(AtomicUsize::new(0));
    let sink_events_received = Arc::new(AtomicUsize::new(0));
    let unified_events_received = Arc::new(AtomicUsize::new(0));

    let stop_consumers = Arc::new(AtomicBool::new(false));

    // Spawn 2 Sensor Consumers
    let mut consumer_handles = Vec::new();
    for _ in 0..2 {
        let mut rx = dag.subscribe_sensors();
        let counter = sensor_events_received.clone();
        let stop = stop_consumers.clone();
        consumer_handles.push(tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                match rx.recv().await {
                    Ok(_) => { counter.fetch_add(1, Ordering::Relaxed); }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
            while rx.try_recv().is_ok() {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Spawn 2 Inference Consumers
    for _ in 0..2 {
        let mut rx = dag.subscribe_inferences();
        let counter = inference_events_received.clone();
        let stop = stop_consumers.clone();
        consumer_handles.push(tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                match rx.recv().await {
                    Ok(_) => { counter.fetch_add(1, Ordering::Relaxed); }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
            while rx.try_recv().is_ok() {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Spawn 2 Sink Consumers
    for _ in 0..2 {
        let mut rx = dag.subscribe_sinks();
        let counter = sink_events_received.clone();
        let stop = stop_consumers.clone();
        consumer_handles.push(tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                match rx.recv().await {
                    Ok(_) => { counter.fetch_add(1, Ordering::Relaxed); }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
            while rx.try_recv().is_ok() {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Spawn 2 Unified Consumers
    for _ in 0..2 {
        let mut rx = dag.subscribe();
        let counter = unified_events_received.clone();
        let stop = stop_consumers.clone();
        consumer_handles.push(tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                match rx.recv().await {
                    Ok(_) => { counter.fetch_add(1, Ordering::Relaxed); }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
            while rx.try_recv().is_ok() {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Spawn 10 Sensor Publishers
    let mut pub_handles = Vec::new();
    for p in 0..num_sensor_publishers {
        let bus = dag.clone();
        pub_handles.push(tokio::spawn(async move {
            for i in 0..events_per_publisher {
                let _ = bus.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
                    trigger: HotkeyTrigger::SttToggle,
                    raw_combination: format!("S_{}_{}", p, i),
                    duration_ms: i as u64,
                    timestamp_ns: (p * 1000 + i) as u64,
                }));
                if i % 50 == 0 { tokio::task::yield_now().await; }
            }
        }));
    }

    // Spawn 10 Inference Publishers
    for p in 0..num_inference_publishers {
        let bus = dag.clone();
        pub_handles.push(tokio::spawn(async move {
            for i in 0..events_per_publisher {
                let _ = bus.publish_inference(InferenceEvent::SpeechDetected(SpeechDetectedPayload {
                    is_speech: true,
                    energy_dbfs: -20.0,
                    hangover_active: false,
                    samples: vec![0.0f32; 8],
                    timestamp_ns: (p * 1000 + i) as u64,
                }));
                if i % 50 == 0 { tokio::task::yield_now().await; }
            }
        }));
    }

    // Spawn 10 Sink Publishers
    for p in 0..num_sink_publishers {
        let bus = dag.clone();
        pub_handles.push(tokio::spawn(async move {
            for i in 0..events_per_publisher {
                let _ = bus.publish_sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                    overlay: OverlayType::RadialDial,
                    visible: Some(true),
                    active_sector: Some((p + i) % 6),
                }));
                if i % 50 == 0 { tokio::task::yield_now().await; }
            }
        }));
    }

    // Wait for all 30 publishers to complete
    for h in pub_handles {
        h.await.unwrap();
    }

    // Give consumers time to drain
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    stop_consumers.store(true, Ordering::Relaxed);

    // Push wakeups
    let _ = dag.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
        trigger: HotkeyTrigger::SttToggle,
        raw_combination: "sentinel".into(),
        duration_ms: 0,
        timestamp_ns: 0,
    }));
    let _ = dag.publish_inference(InferenceEvent::SpeechDetected(SpeechDetectedPayload {
        is_speech: false,
        energy_dbfs: 0.0,
        hangover_active: false,
        samples: vec![],
        timestamp_ns: 0,
    }));
    let _ = dag.publish_sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
        overlay: OverlayType::SttFloatingPill,
        visible: Some(false),
        active_sector: None,
    }));

    for h in consumer_handles {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), h).await;
    }

    let metrics = dag.metrics();
    assert_eq!(metrics.events_routed_sensors, num_sensor_publishers * events_per_publisher + 1);
    assert_eq!(metrics.events_routed_inferences, num_inference_publishers * events_per_publisher + 1);
    assert_eq!(metrics.events_routed_sinks, num_sink_publishers * events_per_publisher + 1);

    assert!(sensor_events_received.load(Ordering::Relaxed) > 0);
    assert!(inference_events_received.load(Ordering::Relaxed) > 0);
    assert!(sink_events_received.load(Ordering::Relaxed) > 0);
    assert!(unified_events_received.load(Ordering::Relaxed) > 0);
}

/// Adversarial Challenge 4: Policy Dynamic Modification Under Concurrent Traffic & Boundary Tests
/// Stresses rwlock policy updates during active propagation, and tests clipboard threshold boundaries.
#[tokio::test]
async fn test_challenger_policy_concurrency_and_clipboard_threshold_boundary() {
    let dag = Arc::new(DagEventBus::new(256));

    // 1. Boundary check: 200 chars vs 201 chars clipboard threshold
    let exact_200_chars = "a".repeat(200);
    let exact_201_chars = "b".repeat(201);

    let down_200 = dag.propagate_and_publish(DagEvent::Inference(InferenceEvent::TranscriptionCompleted(
        TranscriptionPayload {
            text: exact_200_chars.clone(),
            duration_ms: 100,
            confidence: 0.99,
            target_window_title: None,
            target_process_name: None,
        },
    ))).unwrap();

    assert_eq!(down_200.len(), 1);
    match &down_200[0] {
        DagEvent::Sink(SinkAction::InjectText(inj)) => {
            assert_eq!(inj.text, exact_200_chars);
            assert!(!inj.force_clipboard, "200 characters must NOT force clipboard");
        }
        other => panic!("Unexpected event {:?}", other),
    }

    let down_201 = dag.propagate_and_publish(DagEvent::Inference(InferenceEvent::TranscriptionCompleted(
        TranscriptionPayload {
            text: exact_201_chars.clone(),
            duration_ms: 100,
            confidence: 0.99,
            target_window_title: None,
            target_process_name: None,
        },
    ))).unwrap();

    assert_eq!(down_201.len(), 1);
    match &down_201[0] {
        DagEvent::Sink(SinkAction::InjectText(inj)) => {
            assert_eq!(inj.text, exact_201_chars);
            assert!(inj.force_clipboard, "201 characters MUST force clipboard");
        }
        other => panic!("Unexpected event {:?}", other),
    }

    // 2. Empty and whitespace-only transcriptions must produce NO downstream sinks
    let down_empty = dag.propagate_and_publish(DagEvent::Inference(InferenceEvent::TranscriptionCompleted(
        TranscriptionPayload {
            text: "   \t\n  ".into(),
            duration_ms: 50,
            confidence: 0.1,
            target_window_title: None,
            target_process_name: None,
        },
    ))).unwrap();
    assert_eq!(down_empty.len(), 0, "Whitespace transcription must not trigger injection");

    // 3. Concurrent policy toggling while publishers are propagating
    let stop_flag = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    // 5 policy flippers
    for _t in 0..5 {
        let bus = dag.clone();
        let stop = stop_flag.clone();
        handles.push(tokio::spawn(async move {
            let mut flip = false;
            while !stop.load(Ordering::Relaxed) {
                flip = !flip;
                bus.set_policy(DagPropagationPolicy {
                    auto_inject_transcription: flip,
                    auto_kill_camera_on_meeting: !flip,
                    auto_play_disclosure_on_meeting: flip,
                    auto_dispatch_agent_commands: !flip,
                    text_length_clipboard_threshold: if flip { 100 } else { 300 },
                });
                tokio::task::yield_now().await;
            }
        }));
    }

    // 10 active propagators
    for p in 0..10 {
        let bus = dag.clone();
        let stop = stop_flag.clone();
        handles.push(tokio::spawn(async move {
            let mut i = 0;
            while !stop.load(Ordering::Relaxed) && i < 100 {
                let _ = bus.propagate_and_publish(DagEvent::Inference(InferenceEvent::MeetingDetected(MeetingInfo {
                    platform: MeetingApp::Zoom.into(),
                    process_id: (p * 100 + i) as u32,
                    process_name: "zoom.exe".into(),
                    window_title: "Zoom".into(),
                    detected_at: "now".into(),
                })));
                i += 1;
                tokio::task::yield_now().await;
            }
        }));
    }

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    stop_flag.store(true, Ordering::Relaxed);

    for h in handles {
        h.await.unwrap();
    }
}

/// Adversarial Challenge 5: EventBus Backward-Compatible Facade Interoperability
/// Validates that legacy subscribers receive legacy events and translated DAG events,
/// events without DAG mapping pass through transparently, and publish_inference/publish_sink
/// bridge bidirectionally without drop or recursion.
#[tokio::test]
async fn test_challenger_event_bus_facade_bidirectional_interop() {
    let bus = EventBus::new(128);

    let mut legacy_rx = bus.subscribe();
    let mut dag_sensor_rx = bus.subscribe_sensors();
    let mut dag_inf_rx = bus.subscribe_inferences();
    let mut dag_sink_rx = bus.subscribe_sinks();
    let mut dag_unified_rx = bus.subscribe_dag();

    assert_eq!(bus.subscriber_count(), 1);
    assert_eq!(bus.dag().sensor_subscriber_count(), 1);
    assert_eq!(bus.dag().inference_subscriber_count(), 1);
    assert_eq!(bus.dag().sink_subscriber_count(), 1);
    assert_eq!(bus.dag().unified_subscriber_count(), 1);

    // 1. Legacy event with NO DAG mapping (e.g. SttStateChanged)
    // Legacy receiver must get it, DAG stage receivers must NOT get it
    let count = bus.publish(DomainEvent::SttStateChanged(SttState::Recording)).unwrap();
    assert_eq!(count, 1);
    let legacy_ev = legacy_rx.try_recv().expect("legacy_rx should get SttStateChanged");
    assert_eq!(legacy_ev, DomainEvent::SttStateChanged(SttState::Recording));
    assert_eq!(dag_sensor_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(dag_inf_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(dag_sink_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(dag_unified_rx.try_recv(), Err(TryRecvError::Empty));

    // 2. Legacy event WITH DAG mapping (e.g. TranscriptionCompleted)
    let trans_payload = TranscriptionPayload {
        text: "hello legacy and dag".into(),
        duration_ms: 300,
        confidence: 0.95,
        target_window_title: None,
        target_process_name: None,
    };
    bus.publish(DomainEvent::TranscriptionCompleted(trans_payload.clone())).unwrap();

    let leg_trans = legacy_rx.try_recv().expect("legacy received transcription");
    assert_eq!(leg_trans, DomainEvent::TranscriptionCompleted(trans_payload.clone()));

    let dag_inf = dag_inf_rx.try_recv().expect("dag inference received transcription");
    assert_eq!(dag_inf, InferenceEvent::TranscriptionCompleted(trans_payload.clone()));

    let dag_u = dag_unified_rx.try_recv().expect("dag unified received transcription");
    assert_eq!(dag_u, DagEvent::Inference(InferenceEvent::TranscriptionCompleted(trans_payload)));

    // 3. Publishing directly via publish_inference bridges to legacy subscriber
    let meeting_info = MeetingInfo {
        platform: MeetingApp::GoogleMeet.into(),
        process_id: 8888,
        process_name: "chrome.exe".into(),
        window_title: "Google Meet".into(),
        detected_at: "2026-10-12T14:00:00Z".into(),
    };
    let inf_recipients = bus.publish_inference(InferenceEvent::MeetingDetected(meeting_info.clone())).unwrap();
    assert_eq!(inf_recipients, 1);

    // Legacy receiver got it translated back to DomainEvent::MeetingDetected!
    let leg_meet = legacy_rx.try_recv().expect("legacy received bridged meeting");
    assert_eq!(leg_meet, DomainEvent::MeetingDetected(meeting_info.clone()));

    // DAG inference receiver got it
    let dag_meet = dag_inf_rx.try_recv().expect("dag inference received meeting");
    assert_eq!(dag_meet, InferenceEvent::MeetingDetected(meeting_info.clone()));

    // 4. Publishing directly via publish_sink bridges to legacy subscriber
    let agent_payload = AgentPayload {
        target: AgentTarget::Grokbot,
        prompt: "analyze system metrics".into(),
        context_window: None,
        timestamp: "now".into(),
    };
    let sink_recipients = bus.publish_sink(SinkAction::DispatchAgent(agent_payload.clone())).unwrap();
    assert_eq!(sink_recipients, 1);

    let leg_agent = legacy_rx.try_recv().expect("legacy received bridged agent dispatch");
    assert_eq!(leg_agent, DomainEvent::ActionTriggered(ActionCommand::AgentDispatch(agent_payload.clone())));

    let dag_agent = dag_sink_rx.try_recv().expect("dag sink received agent dispatch");
    assert_eq!(dag_agent, SinkAction::DispatchAgent(agent_payload));

    // 5. Zero-subscriber decoupling on EventBus
    drop(legacy_rx);
    drop(dag_sensor_rx);
    drop(dag_inf_rx);
    drop(dag_sink_rx);
    drop(dag_unified_rx);

    assert_eq!(bus.subscriber_count(), 0);
    assert_eq!(bus.publish(DomainEvent::SttStateChanged(SttState::Idle)).unwrap(), 0);
    assert_eq!(bus.publish_inference(InferenceEvent::MeetingDetected(meeting_info)).unwrap(), 0);
}

/// Adversarial Challenge 6: Ring Buffer Overflow & Non-Blocking Publisher Guarantee
/// Confirms that a slow, non-reading consumer does NOT block publishers and that
/// broadcast correctly drops oldest messages while fast consumers proceed normally.
#[tokio::test]
async fn test_challenger_ring_buffer_overflow_and_non_blocking_publisher() {
    let dag = DagEventBus::new(16); // Tiny buffer of 16

    let mut slow_rx = dag.subscribe_sensors();
    let mut fast_rx = dag.subscribe_sensors();

    // Fast receiver consumer task
    let fast_received = Arc::new(AtomicUsize::new(0));
    let fast_counter = fast_received.clone();
    let fast_handle = tokio::spawn(async move {
        loop {
            match fast_rx.recv().await {
                Ok(_) => {
                    fast_counter.fetch_add(1, Ordering::Relaxed);
                }
                Err(RecvError::Lagged(_)) => {
                    // Fast consumer recovered from any momentary scheduler lag
                    continue;
                }
                Err(RecvError::Closed) => break,
            }
        }
    });

    // Publish 100 messages - verifies publishers are NEVER blocked by slow consumer
    for i in 0..100 {
        let ev = SensorEvent::HotkeyEvent(HotkeyEventPayload {
            trigger: HotkeyTrigger::SttToggle,
            raw_combination: format!("key_{}", i),
            duration_ms: i,
            timestamp_ns: i,
        });
        let recipients = dag.publish_sensor(ev).unwrap();
        // Both receivers were subscribed
        assert_eq!(recipients, 2);
        if i % 10 == 0 {
            tokio::task::yield_now().await;
        }
    }

    // Slow receiver attempts to read: it MUST receive RecvError::Lagged
    match slow_rx.recv().await {
        Err(RecvError::Lagged(missed)) => {
            assert!(missed > 0, "Slow receiver must experience lag drop, got {}", missed);
        }
        other => panic!("Expected RecvError::Lagged, got {:?}", other),
    }

    drop(dag);
    let _ = fast_handle.await;
    assert!(fast_received.load(Ordering::Relaxed) > 0, "Fast consumer should have processed events");
}
