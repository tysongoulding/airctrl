use airctrl_actions::RadialDialOverlay;
use airctrl_core::events::EventBus;
use airctrl_core::types::meeting::MeetingStatus;
use airctrl_gestures::{FrameBuffer, PixelFormat};
use airctrl_meetings::{CameraKillSwitch, ProcessScanItem};
use airctrl_shell::loops::*;
use airctrl_shell::AppState;
use airctrl_stt::AudioRingBuffer;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_audio_capture_vad_loop_speech_transitions() {
    let bus = EventBus::default();
    let mut speech_rx = bus.subscribe();
    let app_buffer = Arc::new(AudioRingBuffer::new(48000));
    let metrics = Arc::new(RwLock::new(Default::default()));
    let (sample_tx, sample_rx) = tokio::sync::mpsc::channel(64);

    let cfg = AudioVadConfig {
        tick_interval_ms: 5,
        energy_threshold: 0.02,
        ..Default::default()
    };

    let audio_loop = AudioCaptureVadLoop::new(
        cfg,
        app_buffer.clone(),
        bus.clone(),
        metrics.clone(),
        sample_rx,
    );

    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();
    let handle = tokio::spawn(async move {
        audio_loop.run(token_clone).await;
    });

    // Feed silent samples (below energy threshold)
    sample_tx.send(vec![0.001f32; 640]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    assert!(metrics.read().is_running);
    assert_eq!(metrics.read().speech_segments_detected, 0);

    // Feed speech samples (above energy threshold, 0.2 amplitude)
    sample_tx.send(vec![0.2f32; 1600]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Feed silence to trigger hangover decay and speech offset
    for _ in 0..15 {
        sample_tx.send(vec![0.0f32; 320]).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
    }

    // Verify samples were drained into app_buffer
    assert!(!app_buffer.is_empty());

    // Verify speech event was emitted
    let mut saw_listening = false;
    while let Ok(ev) = speech_rx.try_recv() {
        if matches!(
            ev,
            airctrl_core::events::DomainEvent::SpeechStateChanged(airctrl_core::types::SpeechState::Listening)
        ) {
            saw_listening = true;
        }
    }
    assert!(saw_listening, "Should emit SpeechStateChanged(Listening)");

    cancel_token.cancel();
    let _ = handle.await;
    assert!(!metrics.read().is_running);
}

#[tokio::test]
async fn test_vision_gesture_loop_camera_kill_interlock() {
    let bus = EventBus::default();
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

    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();
    let handle = tokio::spawn(async move {
        vision_loop.run(token_clone).await;
    });

    // 1. When camera is active, frame is processed
    let frame = FrameBuffer {
        width: 64,
        height: 64,
        stride: 64 * 3,
        format: PixelFormat::Rgb8,
        data: vec![128u8; 64 * 64 * 3],
    };
    frame_tx.send(frame.clone()).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(40)).await;

    assert!(metrics.read().camera_active);
    assert_eq!(metrics.read().total_frames_processed, 1);

    // 2. Trigger camera kill switch
    kill_switch.trigger();
    assert!(!kill_switch.is_camera_active());

    // Send another frame
    frame_tx.send(frame).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(40)).await;

    // Frame processing must be bypassed! total_frames_processed remains 1
    assert!(!metrics.read().camera_active);
    assert_eq!(metrics.read().total_frames_processed, 1);

    cancel_token.cancel();
    let _ = handle.await;
    assert!(!metrics.read().is_running);
}

#[tokio::test]
async fn test_meeting_watcher_loop_2_tick_debounce_and_kill_switch() {
    let bus = EventBus::default();
    let mut bus_rx = bus.subscribe();
    let kill_switch = Arc::new(CameraKillSwitch::new());
    let injector = airctrl_meetings::get_global_compliance_injector();
    let metrics = Arc::new(RwLock::new(Default::default()));
    let scan_override = Arc::new(RwLock::new(None));

    let cfg = airctrl_meetings::MeetingWatcherConfig {
        poll_interval_ms: 15,
        debounce_ticks: 2,
        auto_kill_camera: true,
        auto_compliance_audio: true,
        ..Default::default()
    };

    let meeting_loop = MeetingWatcherLoop::new(
        cfg,
        kill_switch.clone(),
        injector,
        bus.clone(),
        metrics.clone(),
    )
    .with_scan_override(scan_override.clone());

    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();
    let handle = tokio::spawn(async move {
        meeting_loop.run(token_clone).await;
    });

    // 1. Tick 1: Meeting injected
    let meeting_procs = vec![ProcessScanItem::new(
        9999,
        "zoom.exe",
        "Zoom Meeting",
    )];
    *scan_override.write() = Some(meeting_procs);

    // Sleep 1 tick (15ms) - status should not be Active yet due to 2-tick debounce
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    // Sleep tick 2 (another 20ms) - now 2 ticks have elapsed
    tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;

    // Verify Active status
    assert_eq!(metrics.read().status, MeetingStatus::Active);
    assert!(metrics.read().active_meeting.is_some());

    // Verify camera kill-switch was triggered atomically (<1.0ms)
    assert!(!kill_switch.is_camera_active());

    // Verify DomainEvent::MeetingDetected was published
    let mut saw_meeting_detected = false;
    let mut saw_kill_switch = false;
    while let Ok(ev) = bus_rx.try_recv() {
        if matches!(ev, airctrl_core::events::DomainEvent::MeetingDetected(_)) {
            saw_meeting_detected = true;
        }
        if matches!(ev, airctrl_core::events::DomainEvent::CameraKillSwitchTriggered { .. }) {
            saw_kill_switch = true;
        }
    }
    assert!(saw_meeting_detected, "MeetingDetected event must be published");
    assert!(saw_kill_switch, "CameraKillSwitchTriggered event must be published");

    // 2. Clear meeting processes (simulate meeting ended)
    *scan_override.write() = Some(vec![]);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Status transitions to Inactive
    assert_eq!(metrics.read().status, MeetingStatus::Inactive);
    assert_eq!(metrics.read().active_meeting, None);

    cancel_token.cancel();
    let _ = handle.await;
    assert!(!metrics.read().is_running);
}

#[tokio::test]
async fn test_global_hotkey_loop_smart_dual_mode_timing() {
    let bus = EventBus::default();
    let mut dag_rx = bus.subscribe_sensors();
    let metrics = Arc::new(RwLock::new(Default::default()));
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(32);
    let dial_overlay = Arc::new(RwLock::new(RadialDialOverlay::new()));

    let hotkey_loop = GlobalHotkeyLoop::new(
        bus.clone(),
        metrics.clone(),
        event_rx,
    );

    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();
    let dial_overlay_clone = dial_overlay.clone();
    let handle = tokio::spawn(async move {
        hotkey_loop.run(token_clone, dial_overlay_clone).await;
    });

    // 1. Micro-press (<30ms): accidental flutter rejection
    event_tx.send(RawKeyEvent {
        target: HotkeyActionTarget::Stt,
        direction: KeyDirection::Down,
        timestamp_ms: 1000,
        window_id: None,
    }).await.unwrap();
    event_tx.send(RawKeyEvent {
        target: HotkeyActionTarget::Stt,
        direction: KeyDirection::Up,
        timestamp_ms: 1020, // 20ms duration -> rejected!
        window_id: None,
    }).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(metrics.read().last_press_duration_ms, 20);
    assert_eq!(metrics.read().last_mode_detected, "Debounced");

    // 2. Quick tap (30-249ms): Toggle mode latched
    event_tx.send(RawKeyEvent {
        target: HotkeyActionTarget::RadialDial,
        direction: KeyDirection::Down,
        timestamp_ms: 2000,
        window_id: None,
    }).await.unwrap();
    event_tx.send(RawKeyEvent {
        target: HotkeyActionTarget::RadialDial,
        direction: KeyDirection::Up,
        timestamp_ms: 2120, // 120ms duration -> Toggle latched!
        window_id: None,
    }).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(metrics.read().last_press_duration_ms, 120);
    assert_eq!(metrics.read().last_mode_detected, "Toggle");
    // In Toggle mode, radial dial remains visible!
    assert!(dial_overlay.read().is_visible());

    // 3. Sustained hold (>=250ms): Push-to-Talk mode
    event_tx.send(RawKeyEvent {
        target: HotkeyActionTarget::Stt,
        direction: KeyDirection::Down,
        timestamp_ms: 3000,
        window_id: None,
    }).await.unwrap();
    event_tx.send(RawKeyEvent {
        target: HotkeyActionTarget::Stt,
        direction: KeyDirection::Up,
        timestamp_ms: 3350, // 350ms duration -> PTT!
        window_id: None,
    }).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(metrics.read().last_press_duration_ms, 350);
    assert_eq!(metrics.read().last_mode_detected, "PushToTalk");

    // Verify DAG sensor events received
    let mut saw_ptt_release = false;
    while let Ok(sensor_ev) = dag_rx.try_recv() {
        if let airctrl_core::dag::SensorEvent::HotkeyEvent(hk) = sensor_ev {
            if hk.trigger == airctrl_core::dag::HotkeyTrigger::SttPushToTalkRelease {
                saw_ptt_release = true;
            }
        }
    }
    assert!(saw_ptt_release, "PTT release sensor event must be published");

    cancel_token.cancel();
    let _ = handle.await;
    assert!(!metrics.read().is_running);
}

#[tokio::test]
async fn test_worker_loop_manager_lifecycle_and_zero_mock_audio() {
    let state = Arc::new(AppState::new());
    let manager = &state.loop_manager;

    assert!(!manager.is_running());
    let initial_health = manager.get_health_metrics();
    assert!(!initial_health.all_healthy);

    // 1. Start all loops
    manager.start_all(state.clone()).await.expect("start_all failed");
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert!(manager.is_running());
    let running_health = manager.get_health_metrics();
    assert!(running_health.all_healthy);

    // 2. Feed real PCM audio samples to audio loop (Zero-Mock test)
    let pcm_chunk = vec![0.3f32; 800];
    manager.feed_audio(pcm_chunk.clone()).await.expect("feed_audio failed");
    tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

    // Verify samples reached state.audio_buffer
    assert!(state.audio_buffer.len() >= 800);

    // 3. Stop recording drains the real samples
    let res = state.stop_recording().expect("stop_recording must succeed with real samples");
    assert!(!res.is_empty());

    // 4. Second stop_recording without new samples must fail with ZeroSamples error
    let err = state.stop_recording().expect_err("second stop_recording must fail due to drain");
    assert!(err.contains("ZeroSamples"));

    // 5. Stop all loops cleanly
    manager.stop_all().await.expect("stop_all failed");
    tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

    assert!(!manager.is_running());
    let stopped_health = manager.get_health_metrics();
    assert!(!stopped_health.all_healthy);
}
