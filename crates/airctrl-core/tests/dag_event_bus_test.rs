use airctrl_core::dag::*;
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::action::{ActionCommand, AgentPayload, AgentTarget};
use airctrl_core::types::meeting::{CameraState, MeetingApp, MeetingInfo};
use airctrl_core::types::stt::TranscriptionPayload;
use std::sync::Arc;
use tokio::sync::broadcast::error::TryRecvError;

#[tokio::test]
async fn test_dag_stage_isolation_and_unified_stream() {
    let dag = DagEventBus::new(32);

    let mut sensor_rx = dag.subscribe_sensors();
    let mut inference_rx = dag.subscribe_inferences();
    let mut sink_rx = dag.subscribe_sinks();
    let mut unified_rx = dag.subscribe();

    assert_eq!(dag.sensor_subscriber_count(), 1);
    assert_eq!(dag.inference_subscriber_count(), 1);
    assert_eq!(dag.sink_subscriber_count(), 1);
    assert_eq!(dag.unified_subscriber_count(), 1);

    // 1. Publish Sensor Event
    let sensor_event = SensorEvent::RawAudioChunk(RawAudioChunkPayload {
        samples: vec![0.1, 0.2, 0.3],
        sample_rate: 16000,
        channels: 1,
        timestamp_ns: 1000,
    });
    let subs = dag.publish_sensor(sensor_event.clone()).unwrap();
    assert_eq!(subs, 1);

    // Sensor receiver got it
    let received_sensor = sensor_rx.try_recv().expect("Sensor event expected");
    assert_eq!(received_sensor, sensor_event);

    // Inference & Sink receivers got nothing
    assert_eq!(inference_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(sink_rx.try_recv(), Err(TryRecvError::Empty));

    // Unified receiver got it wrapped
    let received_unified = unified_rx.try_recv().expect("Unified event expected");
    assert_eq!(received_unified, DagEvent::Sensor(sensor_event));

    // 2. Publish Inference Event
    let inference_event = InferenceEvent::SpeechDetected(SpeechDetectedPayload {
        is_speech: true,
        energy_dbfs: -22.5,
        hangover_active: false,
        samples: vec![0.5; 160],
        timestamp_ns: 2000,
    });
    dag.publish_inference(inference_event.clone()).unwrap();

    let received_inf = inference_rx.try_recv().expect("Inference event expected");
    assert_eq!(received_inf, inference_event);
    assert_eq!(sensor_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(sink_rx.try_recv(), Err(TryRecvError::Empty));

    let received_unified2 = unified_rx.try_recv().expect("Unified event expected");
    assert_eq!(received_unified2, DagEvent::Inference(inference_event));

    // 3. Publish Sink Action
    let sink_action = SinkAction::ToggleOverlay(ToggleOverlayPayload {
        overlay: OverlayType::RadialDial,
        visible: Some(true),
        active_sector: Some(2),
    });
    dag.publish_sink(sink_action.clone()).unwrap();

    let received_sink = sink_rx.try_recv().expect("Sink action expected");
    assert_eq!(received_sink, sink_action);
    assert_eq!(sensor_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(inference_rx.try_recv(), Err(TryRecvError::Empty));

    let received_unified3 = unified_rx.try_recv().expect("Unified event expected");
    assert_eq!(received_unified3, DagEvent::Sink(sink_action));
}

#[tokio::test]
async fn test_dag_zero_subscriber_returns_ok_zero() {
    let dag = DagEventBus::new(32);

    let res1 = dag.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
        trigger: HotkeyTrigger::SttToggle,
        raw_combination: "Ctrl+Spacebar".to_string(),
        duration_ms: 50,
        timestamp_ns: 12345,
    }));
    assert_eq!(res1.unwrap(), 0);

    let res2 = dag.publish_inference(InferenceEvent::MeetingDetected(MeetingInfo {
        platform: MeetingApp::Zoom.into(),
        process_id: 1234,
        process_name: "zoom.exe".into(),
        window_title: "Zoom Meeting".into(),
        detected_at: "2026-10-12T12:00:00Z".into(),
    }));
    assert_eq!(res2.unwrap(), 0);

    let res3 = dag.publish_sink(SinkAction::InjectText(InjectTextPayload {
        text: "hello".into(),
        target_window_id: None,
        force_clipboard: false,
    }));
    assert_eq!(res3.unwrap(), 0);
}

#[tokio::test]
async fn test_dag_causal_propagation_transcription_to_inject() {
    let dag = DagEventBus::new(32);
    let mut sink_rx = dag.subscribe_sinks();

    let payload = TranscriptionPayload {
        text: "open browser and search rust documentation".to_string(),
        duration_ms: 1200,
        confidence: 0.95,
        target_window_title: Some("Terminal".into()),
        target_process_name: Some("alacritty".into()),
    };

    let downstream = dag
        .propagate_and_publish(DagEvent::Inference(InferenceEvent::TranscriptionCompleted(
            payload.clone(),
        )))
        .expect("Propagation failed");

    assert_eq!(downstream.len(), 1);
    match &downstream[0] {
        DagEvent::Sink(SinkAction::InjectText(inj)) => {
            assert_eq!(inj.text, payload.text);
            assert!(!inj.force_clipboard);
        }
        other => panic!("Expected InjectText sink, got {:?}", other),
    }

    let sink_event = sink_rx.try_recv().expect("Sink action must be published");
    match sink_event {
        SinkAction::InjectText(inj) => {
            assert_eq!(inj.text, payload.text);
        }
        other => panic!("Expected InjectText, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dag_causal_propagation_meeting_to_kill_camera_and_audio() {
    let dag = DagEventBus::new(32);
    let mut sink_rx = dag.subscribe_sinks();

    let meeting = MeetingInfo {
        platform: MeetingApp::Teams.into(),
        process_id: 5678,
        process_name: "teams.exe".into(),
        window_title: "Microsoft Teams Call".into(),
        detected_at: "2026-10-12T13:00:00Z".into(),
    };

    let downstream = dag
        .propagate_and_publish(DagEvent::Inference(InferenceEvent::MeetingDetected(
            meeting.clone(),
        )))
        .expect("Propagation failed");

    assert_eq!(downstream.len(), 2);
    let mut saw_kill_camera = false;
    let mut saw_play_disclosure = false;

    for event in downstream {
        match event {
            DagEvent::Sink(SinkAction::KillCamera(k)) => {
                assert_eq!(k.meeting.process_id, 5678);
                saw_kill_camera = true;
            }
            DagEvent::Sink(SinkAction::PlayDisclosure(p)) => {
                assert!(p.meeting.is_some());
                assert_eq!(
                    p.spec.announcement_text,
                    "This meeting is being recorded"
                );
                saw_play_disclosure = true;
            }
            other => panic!("Unexpected sink action {:?}", other),
        }
    }

    assert!(saw_kill_camera);
    assert!(saw_play_disclosure);

    // Verify channel received both
    let ev1 = sink_rx.try_recv().expect("First sink action expected");
    let ev2 = sink_rx.try_recv().expect("Second sink action expected");
    assert!(matches!(ev1, SinkAction::KillCamera(_)));
    assert!(matches!(ev2, SinkAction::PlayDisclosure(_)));
}

#[tokio::test]
async fn test_dag_causal_propagation_command_parsed_agent_dispatch() {
    let dag = DagEventBus::new(32);
    let mut sink_rx = dag.subscribe_sinks();

    let agent_payload = AgentPayload {
        target: AgentTarget::Claude,
        prompt: "Refactor error handling".into(),
        context_window: Some("crates/airctrl-core/src/error.rs".into()),
        timestamp: "2026-10-12T14:00:00Z".into(),
    };

    let downstream = dag
        .propagate_and_publish(DagEvent::Inference(InferenceEvent::CommandParsed(
            CommandParsedPayload {
                utterance: "ask claude to refactor error handling".into(),
                command: ActionCommand::AgentDispatch(agent_payload.clone()),
                confidence_score: 95,
                timestamp_ns: 4000,
            },
        )))
        .expect("Propagation failed");

    assert_eq!(downstream.len(), 1);
    assert_eq!(
        downstream[0],
        DagEvent::Sink(SinkAction::DispatchAgent(agent_payload.clone()))
    );

    let sink_ev = sink_rx.try_recv().expect("Sink action expected");
    assert_eq!(sink_ev, SinkAction::DispatchAgent(agent_payload));
}

#[tokio::test]
async fn test_dag_event_bus_metrics() {
    let dag = DagEventBus::new(128);

    dag.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
        trigger: HotkeyTrigger::SttToggle,
        raw_combination: "Ctrl+Space".into(),
        duration_ms: 100,
        timestamp_ns: 1,
    }))
    .unwrap();

    dag.publish_inference(InferenceEvent::SpeechDetected(SpeechDetectedPayload {
        is_speech: true,
        energy_dbfs: -10.0,
        hangover_active: false,
        samples: vec![],
        timestamp_ns: 2,
    }))
    .unwrap();

    dag.publish_sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
        overlay: OverlayType::RadialDial,
        visible: Some(true),
        active_sector: Some(0),
    }))
    .unwrap();

    let metrics = dag.metrics();
    assert!(metrics.active);
    assert_eq!(metrics.capacity, 128);
    assert_eq!(metrics.events_routed_sensors, 1);
    assert_eq!(metrics.events_routed_inferences, 1);
    assert_eq!(metrics.events_routed_sinks, 1);
}

#[tokio::test]
async fn test_backward_compatible_event_bus_facade() {
    let bus = EventBus::new(64);

    let mut legacy_rx = bus.subscribe();
    let mut dag_rx = bus.subscribe_dag();
    let mut sink_rx = bus.subscribe_sinks();

    assert_eq!(bus.subscriber_count(), 1);

    // 1. Publishing legacy DomainEvent delivers to legacy receiver and translates to DAG
    let meeting_info = MeetingInfo {
        platform: MeetingApp::Zoom.into(),
        process_id: 1111,
        process_name: "zoom.exe".into(),
        window_title: "Meeting".into(),
        detected_at: "2026-10-12T12:00:00Z".into(),
    };

    let delivered = bus
        .publish(DomainEvent::CameraKillSwitchTriggered {
            meeting: meeting_info.clone(),
            camera_state: CameraState::KilledByMeeting,
        })
        .unwrap();

    // Exact count returned
    assert_eq!(delivered, 1);

    let legacy_ev = legacy_rx.try_recv().expect("Legacy event expected");
    assert!(matches!(
        legacy_ev,
        DomainEvent::CameraKillSwitchTriggered { .. }
    ));

    // DAG receiver got translated event
    let dag_ev = dag_rx.try_recv().expect("DAG event expected");
    match dag_ev {
        DagEvent::Sink(SinkAction::KillCamera(k)) => {
            assert_eq!(k.meeting.process_id, 1111);
        }
        other => panic!("Expected KillCamera sink in DAG, got {:?}", other),
    }

    // Typed sink channel got it as well
    let sink_ev = sink_rx.try_recv().expect("Sink action expected");
    assert!(matches!(sink_ev, SinkAction::KillCamera(_)));

    // 2. Zero-subscriber publishing returns Ok(0)
    drop(legacy_rx);
    assert_eq!(bus.subscriber_count(), 0);
    let zero_res = bus.publish(DomainEvent::MeetingStatusChanged(
        airctrl_core::types::MeetingStatus::Active,
    ));
    assert_eq!(zero_res.unwrap(), 0);
}

#[tokio::test]
async fn test_dag_concurrent_stress_multi_publisher() {
    let dag = Arc::new(DagEventBus::new(1024));
    let mut unified_rx = dag.subscribe();

    let num_publishers = 10;
    let events_per_publisher = 50;
    let mut handles = Vec::new();

    for p in 0..num_publishers {
        let bus = dag.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..events_per_publisher {
                let _ = bus.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
                    trigger: HotkeyTrigger::SttToggle,
                    raw_combination: format!("P{}_E{}", p, i),
                    duration_ms: (p * 10 + i) as u64,
                    timestamp_ns: i as u64,
                }));
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let total_expected = num_publishers * events_per_publisher;
    let mut count = 0;
    while unified_rx.try_recv().is_ok() {
        count += 1;
    }

    assert_eq!(count, total_expected);
    let metrics = dag.metrics();
    assert_eq!(metrics.events_routed_sensors, total_expected);
}
