use airctrl_core::dag::{DagEventBus, InferenceEvent, SensorEvent, SinkAction};
use airctrl_core::events::EventBus;
use airctrl_gestures::{FrameBuffer, PixelFormat};
use airctrl_meetings::CameraKillSwitch;
use airctrl_shell::ipc::{IpcBridge, IpcError};
use airctrl_shell::loops::*;
use airctrl_shell::state::AppState;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

// ============================================================================
// CHALLENGE 1: WorkerLoopManager is_running() Race Condition
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_lifecycle_is_running_race_condition() {
    let state = Arc::new(AppState::new());
    let manager = &state.loop_manager;

    // Immediately after start_all returns, Tokio has not yet scheduled the background tasks!
    manager.start_all(state.clone()).await.unwrap();

    // EMPIRICAL BUG REPRODUCTION:
    // is_running() checks child metrics is_running, which is only set when tasks start running.
    // Therefore, right after start_all returns, is_running() can be false!
    let running_immediately = manager.is_running();
    println!("Manager is_running immediately after start_all: {}", running_immediately);

    // After a short yield for Tokio scheduling, it becomes true
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(manager.is_running(), "Manager should be running after tasks schedule");

    manager.stop_all().await.unwrap();
    assert!(!manager.is_running(), "Manager must not be running after stop_all");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_lifecycle_rapid_restart_stress() {
    let state = Arc::new(AppState::new());
    let manager = &state.loop_manager;

    // Test 10 iterations of start/stop with proper yielding
    for i in 0..10 {
        manager.start_all(state.clone()).await.unwrap_or_else(|e| {
            panic!("start_all failed on iteration {}: {}", i, e);
        });
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert!(manager.is_running());

        manager.stop_all().await.unwrap_or_else(|e| {
            panic!("stop_all failed on iteration {}: {}", i, e);
        });
        assert!(!manager.is_running());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_lifecycle_stop_all_idempotency() {
    let state = Arc::new(AppState::new());
    let manager = &state.loop_manager;

    // Repeated stop_all on stopped manager must succeed
    for _ in 0..3 {
        assert!(manager.stop_all().await.is_ok());
        assert!(!manager.is_running());
    }

    manager.start_all(state.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(15)).await;
    assert!(manager.is_running());

    for _ in 0..3 {
        assert!(manager.stop_all().await.is_ok());
        assert!(!manager.is_running());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_hierarchical_cancellation_teardown() {
    let state = Arc::new(AppState::new());
    let manager = &state.loop_manager;

    manager.start_all(state.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(manager.is_running());

    let health = manager.get_health_metrics();
    assert!(health.all_healthy);

    // Call stop_all which triggers root_token.cancel() and all child tokens
    manager.stop_all().await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Verify all child loops terminated
    let stopped_health = manager.get_health_metrics();
    assert!(!stopped_health.all_healthy);
    assert!(!stopped_health.audio_vad.is_running);
    assert!(!stopped_health.vision_gesture.is_running);
    assert!(!stopped_health.meeting_watcher.is_running);
    assert!(!stopped_health.global_hotkey.is_running);

    // Verify feed operations return Err("... loop not running")
    assert!(manager.feed_audio(vec![0.1; 100]).await.is_err());
    assert!(manager.feed_frame(FrameBuffer {
        width: 16,
        height: 16,
        stride: 48,
        format: PixelFormat::Rgb8,
        data: vec![0; 768],
    }).await.is_err());
    assert!(manager.feed_hotkey(RawKeyEvent {
        target: HotkeyActionTarget::Stt,
        direction: KeyDirection::Down,
        timestamp_ms: 1000,
        window_id: None,
    }).await.is_err());
}

// ============================================================================
// CHALLENGE 2: Zero-Mock Audio Enforcement
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_zero_mock_audio_empty_buffer_rejection() {
    let state = Arc::new(AppState::new());

    // When worker loops are running, production reactive mode is strictly active
    state.loop_manager.start_all(state.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;

    // 1. Calling start_recording() while worker loops run must NOT seed any synthetic samples!
    state.start_recording();
    assert_eq!(
        state.audio_buffer.len(),
        0,
        "Production start_recording must NOT seed synthetic samples!"
    );

    // 2. Calling stop_recording() with 0 samples MUST return Err with verbatim ZeroSamples error
    let result = state.stop_recording();
    assert!(
        result.is_err(),
        "stop_recording on empty buffer must return Err"
    );
    let err_msg = result.unwrap_err();
    assert_eq!(
        err_msg,
        "ZeroSamples: No audio samples recorded",
        "Error message must exactly match ZeroSamples contract"
    );

    state.loop_manager.stop_all().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_zero_mock_start_recording_production_api() {
    let state = Arc::new(AppState::new());

    // Explicit production API start_recording_production()
    state.start_recording_production();
    assert_eq!(
        state.audio_buffer.len(),
        0,
        "start_recording_production must initialize an empty buffer"
    );

    let err = state.stop_recording().unwrap_err();
    assert_eq!(err, "ZeroSamples: No audio samples recorded");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_zero_mock_draining_semantics_prevent_replay() {
    let state = Arc::new(AppState::new());

    // Feed real samples into state audio buffer
    let real_samples = vec![0.4f32; 1600];
    state.feed_audio_samples(&real_samples);
    assert_eq!(state.audio_buffer.len(), 0); // Not recording yet -> dropped

    // Start recording and feed real samples
    state.start_recording_production();
    state.feed_audio_samples(&real_samples);
    assert_eq!(state.audio_buffer.len(), 1600);

    // First stop_recording drains all 1600 samples and succeeds
    let transcription = state.stop_recording().expect("First stop must succeed");
    assert!(!transcription.is_empty());
    assert_eq!(state.audio_buffer.len(), 0);

    // Immediate second stop_recording must fail with ZeroSamples
    let replay_err = state.stop_recording().expect_err("Second stop must fail");
    assert_eq!(replay_err, "ZeroSamples: No audio samples recorded");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_ipc_bridge_zero_mock_contract_enforcement() {
    let state = Arc::new(AppState::new());
    let bridge = IpcBridge::new(state.clone());

    // 1. Start worker loops via IPC
    let start_loops_res = bridge.invoke("startWorkerLoops", None).unwrap();
    assert_eq!(start_loops_res["status"], "WorkerLoopsStarted");

    // EMPIRICAL BUG REPRODUCTION:
    // Because start_all spawns tasks without waiting for them to start, is_running() is initially false!
    let running_immediately = state.loop_manager.is_running();
    println!("Loop manager is_running immediately after startWorkerLoops: {}", running_immediately);

    // If startRecording is called immediately while is_running is false:
    if !running_immediately {
        bridge.invoke("startRecording", None).unwrap();
        let seeded = state.audio_buffer.len();
        println!("Synthetic samples erroneously seeded due to is_running race condition: {}", seeded);
        assert_eq!(seeded, 1600, "Synthetic test mode seeded samples despite startWorkerLoops having completed!");
        // Clear buffer for the next part of test
        let _ = state.stop_recording();
    }

    // Now wait for background loops to schedule
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(state.loop_manager.is_running(), "Loop manager must be running now");

    // Once is_running is true, startRecording does NOT seed samples!
    bridge.invoke("startRecording", None).unwrap();
    assert_eq!(state.audio_buffer.len(), 0, "Zero samples seeded when loops are running");

    // Now stopRecording must fail with ZeroSamples
    let stop_res = bridge.invoke("stopRecording", None);
    assert!(stop_res.is_err(), "stopRecording must fail via IPC when buffer is empty");
    match stop_res.unwrap_err() {
        IpcError::ExecutionFailed(msg) => {
            assert!(msg.contains("ZeroSamples: No audio samples recorded"));
        }
        other => panic!("Expected IpcError::ExecutionFailed, got {:?}", other),
    }

    bridge.invoke("stopWorkerLoops", None).unwrap();
}

// ============================================================================
// CHALLENGE 3: Camera Kill Switch Interlock in VisionGestureLoop
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_camera_kill_interlock_frame_suppression() {
    let bus = EventBus::default();
    let mut sensor_rx = bus.subscribe_sensors();

    let kill_switch = Arc::new(CameraKillSwitch::new());
    let metrics = Arc::new(RwLock::new(Default::default()));
    let (frame_tx, frame_rx) = tokio::sync::mpsc::channel(32);

    let cfg = VisionGestureConfig {
        target_fps: 60,
        ..Default::default()
    };

    let vision_loop = VisionGestureLoop::new(
        cfg,
        kill_switch.clone(),
        bus.clone(),
        metrics.clone(),
        frame_rx,
    );

    let token = CancellationToken::new();
    let token_clone = token.clone();
    let handle = tokio::spawn(async move {
        vision_loop.run(token_clone).await;
    });

    let dummy_frame = FrameBuffer {
        width: 32,
        height: 32,
        stride: 32 * 3,
        format: PixelFormat::Rgb8,
        data: vec![200u8; 32 * 32 * 3],
    };

    // 1. Initial state: Camera is active -> Frame is processed
    frame_tx.send(dummy_frame.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(35)).await;

    assert!(metrics.read().camera_active);
    assert_eq!(metrics.read().total_frames_processed, 1);
    assert!(sensor_rx.try_recv().is_ok(), "Sensor VideoFrame must be published");

    // 2. Engage camera kill switch
    kill_switch.trigger();
    assert!(!kill_switch.is_camera_active());

    // 3. Send 5 frames while kill switch is engaged
    for _ in 0..5 {
        frame_tx.send(dummy_frame.clone()).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Verify frames were strictly rejected/suppressed while killed!
    assert!(!metrics.read().camera_active);
    assert_eq!(
        metrics.read().total_frames_processed,
        1,
        "No frames may be processed while kill switch is engaged!"
    );
    assert!(
        sensor_rx.try_recv().is_err(),
        "No Sensor VideoFrame may be published while camera is killed!"
    );

    token.cancel();
    let _ = handle.await;
    assert!(!metrics.read().is_running);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_camera_kill_interlock_stale_frame_queue_defect() {
    // EMPIRICAL BUG DEMONSTRATION:
    // When camera is killed, VisionGestureLoop does `if !camera_active { continue; }`
    // without reading or draining `frame_rx`.
    // Consequently, frames sent during camera-kill accumulate in the mpsc channel.
    // If the camera is later reactivated, those stale frames are processed!
    let bus = EventBus::default();
    let kill_switch = Arc::new(CameraKillSwitch::new());
    let metrics = Arc::new(RwLock::new(Default::default()));
    let (frame_tx, frame_rx) = tokio::sync::mpsc::channel(16);

    let cfg = VisionGestureConfig {
        target_fps: 60,
        ..Default::default()
    };

    let vision_loop = VisionGestureLoop::new(
        cfg,
        kill_switch.clone(),
        bus.clone(),
        metrics.clone(),
        frame_rx,
    );

    let token = CancellationToken::new();
    let token_clone = token.clone();
    let handle = tokio::spawn(async move {
        vision_loop.run(token_clone).await;
    });

    // 1. Immediately engage camera kill switch
    kill_switch.trigger();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!kill_switch.is_camera_active());

    let frame = FrameBuffer {
        width: 16,
        height: 16,
        stride: 48,
        format: PixelFormat::Rgb8,
        data: vec![100u8; 768],
    };

    // 2. Send 3 frames while camera is supposedly killed
    for _ in 0..3 {
        frame_tx.send(frame.clone()).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(40)).await;

    // While killed, 0 frames processed
    assert_eq!(metrics.read().total_frames_processed, 0);

    // 3. Reactivate camera
    kill_switch.reactivate();
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Stale frames accumulated during camera kill window must be purged upon reactivation!
    let processed_after_reactivate = metrics.read().total_frames_processed;
    println!(
        "Processed frames after reactivate (from queue accumulated while killed): {}",
        processed_after_reactivate
    );
    assert_eq!(processed_after_reactivate, 0);

    token.cancel();
    let _ = handle.await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_camera_kill_switch_multithreaded_race() {
    let kill_switch = Arc::new(CameraKillSwitch::new());
    let mut handles = Vec::new();

    // 30 concurrent threads racing to trigger, reactivate, or check status
    for tid in 0..30 {
        let ks = kill_switch.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..100 {
                if tid % 3 == 0 {
                    ks.trigger();
                } else if tid % 3 == 1 {
                    ks.reactivate();
                } else {
                    let _ = ks.is_camera_active();
                }
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let _status = kill_switch.trigger();
    assert!(!kill_switch.is_camera_active());
    let idempotent_status = kill_switch.trigger();
    assert_eq!(idempotent_status.status, "AlreadyReleased");
    assert!(!idempotent_status.camera_handle_closed);
}

// ============================================================================
// CHALLENGE 4: Typed DAG Stage Decoupling & Metrics
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_dag_strict_stage_isolation() {
    let dag = DagEventBus::new(64);
    let mut sensor_rx = dag.subscribe_sensors();
    let mut inference_rx = dag.subscribe_inferences();
    let mut sink_rx = dag.subscribe_sinks();

    // 1. Publish sensor event
    dag.publish_sensor(SensorEvent::HotkeyEvent(airctrl_core::dag::HotkeyEventPayload {
        trigger: airctrl_core::dag::HotkeyTrigger::SttToggle,
        raw_combination: "Ctrl+Spacebar".to_string(),
        duration_ms: 100,
        timestamp_ns: 100,
    })).unwrap();

    assert!(sensor_rx.try_recv().is_ok());
    assert!(inference_rx.try_recv().is_err());
    assert!(sink_rx.try_recv().is_err());

    // 2. Publish inference event
    dag.publish_inference(InferenceEvent::SpeechDetected(airctrl_core::dag::SpeechDetectedPayload {
        is_speech: true,
        energy_dbfs: -15.0,
        hangover_active: false,
        samples: vec![],
        timestamp_ns: 200,
    })).unwrap();

    assert!(sensor_rx.try_recv().is_err());
    assert!(inference_rx.try_recv().is_ok());
    assert!(sink_rx.try_recv().is_err());

    // 3. Publish sink event
    dag.publish_sink(SinkAction::ToggleOverlay(airctrl_core::dag::ToggleOverlayPayload {
        overlay: airctrl_core::dag::OverlayType::RadialDial,
        visible: Some(false),
        active_sector: None,
    })).unwrap();

    assert!(sensor_rx.try_recv().is_err());
    assert!(inference_rx.try_recv().is_err());
    assert!(sink_rx.try_recv().is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_challenger_dag_metrics_accuracy() {
    let dag = DagEventBus::new(256);

    for i in 0..15 {
        dag.publish_sensor(SensorEvent::HotkeyEvent(airctrl_core::dag::HotkeyEventPayload {
            trigger: airctrl_core::dag::HotkeyTrigger::SttToggle,
            raw_combination: "Ctrl+Space".into(),
            duration_ms: i,
            timestamp_ns: i,
        })).unwrap();
    }

    for i in 0..10 {
        dag.publish_inference(InferenceEvent::SpeechDetected(airctrl_core::dag::SpeechDetectedPayload {
            is_speech: true,
            energy_dbfs: -10.0,
            hangover_active: false,
            samples: vec![],
            timestamp_ns: i,
        })).unwrap();
    }

    for _ in 0..5 {
        dag.publish_sink(SinkAction::ToggleOverlay(airctrl_core::dag::ToggleOverlayPayload {
            overlay: airctrl_core::dag::OverlayType::RadialDial,
            visible: Some(true),
            active_sector: Some(1),
        })).unwrap();
    }

    let m = dag.metrics();
    assert_eq!(m.events_routed_sensors, 15);
    assert_eq!(m.events_routed_inferences, 10);
    assert_eq!(m.events_routed_sinks, 5);
    assert_eq!(m.capacity, 256);
}
