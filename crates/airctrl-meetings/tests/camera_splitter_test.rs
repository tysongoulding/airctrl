use airctrl_core::types::camera::{FrameBuffer, FrameFormat, PrivacyMuteState};
use airctrl_meetings::camera::{PrivacySlateGenerator, VirtualCameraSplitter, ZeroLockVideoRingBuffer};
use std::sync::Arc;
use std::thread;

#[test]
fn test_continuous_virtual_camera_startup_and_status() {
    let splitter = VirtualCameraSplitter::with_default_config();
    assert!(splitter.is_running());
    assert_eq!(splitter.device_name(), "AirCtrl Virtual Camera");
    assert!(!splitter.is_privacy_muted());
    assert!(!splitter.is_dual_muted());
    assert!(splitter.is_consumer_1_active());

    let status = splitter.status();
    assert!(status.is_running);
    assert_eq!(status.device_name, "AirCtrl Virtual Camera");
    assert_eq!(status.privacy_mode, PrivacyMuteState::Active);
    assert_eq!(status.consumer_count, 2);
    assert_eq!(status.frames_fanned_out, 0);
}

#[test]
fn test_custom_device_name() {
    let splitter = VirtualCameraSplitter::with_device_name("AirCtrl Studio Cam 4K");
    assert_eq!(splitter.device_name(), "AirCtrl Studio Cam 4K");
}

#[test]
fn test_concurrent_fanout_to_both_consumers() {
    let splitter = VirtualCameraSplitter::with_default_config();

    let out = splitter.feed_camera_frame(42).expect("feed failed");
    assert_eq!(out.device, "AirCtrl Virtual Camera");
    assert_eq!(out.frame_id, 42);
    assert_eq!(out.consumer_1_mediapipe, Some("raw_frame_42".to_string()));
    assert_eq!(out.consumer_2_meeting, "outbound_frame_42");
    assert!(!out.privacy_muted);
    assert!(!out.dual_muted);
    assert!(out.consumer_1_frame.is_some());
    assert!(out.consumer_2_frame.is_some());
    assert_eq!(splitter.frames_fanned_out(), 1);
}

#[test]
fn test_independent_privacy_mute_toggle() {
    let splitter = VirtualCameraSplitter::with_default_config();

    // Engage privacy mute
    splitter.set_privacy_mute(true);
    assert!(splitter.is_privacy_muted());
    assert!(!splitter.is_dual_muted());
    // Consumer 1 (MediaPipe) REMAINS ACTIVE!
    assert!(splitter.is_consumer_1_active());

    let out1 = splitter.feed_camera_frame(5).expect("feed failed");
    assert_eq!(out1.consumer_1_mediapipe, Some("raw_frame_5".to_string()));
    assert_eq!(out1.consumer_2_meeting, "privacy_slate_frame");
    assert!(out1.privacy_muted);
    assert!(!out1.dual_muted);
    assert!(out1.consumer_1_frame.is_some());
    assert!(out1.consumer_2_frame.is_some());

    // Disengage privacy mute
    splitter.set_privacy_mute(false);
    assert!(!splitter.is_privacy_muted());

    let out2 = splitter.feed_camera_frame(6).expect("feed failed");
    assert_eq!(out2.consumer_1_mediapipe, Some("raw_frame_6".to_string()));
    assert_eq!(out2.consumer_2_meeting, "outbound_frame_6");
    assert!(!out2.privacy_muted);
}

#[test]
fn test_dual_mute_silences_both() {
    let splitter = VirtualCameraSplitter::with_default_config();

    splitter.set_dual_mute(true);
    assert!(splitter.is_dual_muted());
    assert!(splitter.is_privacy_muted());
    assert!(!splitter.is_consumer_1_active());

    let out = splitter.feed_camera_frame(20).expect("feed failed");
    assert_eq!(out.consumer_1_mediapipe, None);
    assert_eq!(out.consumer_2_meeting, "privacy_slate_frame");
    assert!(out.consumer_1_frame.is_none());
    assert!(out.consumer_2_frame.is_some());
    assert!(out.privacy_muted);
    assert!(out.dual_muted);

    // Unmute restores Active
    splitter.set_dual_mute(false);
    assert!(!splitter.is_dual_muted());
    assert!(!splitter.is_privacy_muted());
    assert!(splitter.is_consumer_1_active());
}

#[test]
fn test_stopped_splitter_error() {
    let splitter = VirtualCameraSplitter::with_default_config();
    splitter.set_running(false);
    assert!(!splitter.is_running());

    let err = splitter.feed_camera_frame(10).unwrap_err();
    assert_eq!(err, airctrl_meetings::MeetingError::SplitterNotRunning);
}

#[test]
fn test_zero_lock_ring_buffer_concurrency() {
    let ring = Arc::new(ZeroLockVideoRingBuffer::new());
    let ring_producer = ring.clone();

    let producer = thread::spawn(move || {
        for i in 1..=100 {
            let frame = Arc::new(FrameBuffer::allocate(64, 64, FrameFormat::Rgb8));
            ring_producer.publish_frame(frame, i);
        }
    });

    let ring_consumer = ring.clone();
    let consumer = thread::spawn(move || {
        let mut last_seen = 0;
        let mut frames_read = 0;
        while last_seen < 100 && frames_read < 200 {
            if let Some((_frame, id)) = ring_consumer.get_latest_frame(last_seen) {
                assert!(id > last_seen);
                last_seen = id;
                frames_read += 1;
            }
            thread::yield_now();
        }
        frames_read
    });

    producer.join().unwrap();
    let frames_read = consumer.join().unwrap();
    assert!(frames_read > 0);
    assert_eq!(ring.total_published(), 100);
}

#[test]
fn test_privacy_slate_watermark_pulsing() {
    let slate_gen = PrivacySlateGenerator::new(640, 480);
    let frame1 = slate_gen.generate_slate_frame(0, 0);
    let frame2 = slate_gen.generate_slate_frame(30, 0);

    // Check pixel at corner watermark (40, 40)
    let p1 = frame1.get_pixel(40, 40).unwrap();
    let p2 = frame2.get_pixel(40, 40).unwrap();

    // At frame 0, pulse is sin(0) = 0
    // At frame 30, pulse is sin(pi/2) = 1.0, so color should be noticeably brighter!
    assert!(p2.0 >= p1.0);
    assert!(p2.1 >= p1.1);
    assert!(p2.2 >= p1.2);
}

#[test]
fn test_set_privacy_mode_direct_transitions() {
    let splitter = VirtualCameraSplitter::with_default_config();
    assert_eq!(splitter.status().privacy_mode, PrivacyMuteState::Active);

    // 1. Transition Active -> DualMute
    splitter.set_privacy_mode(PrivacyMuteState::DualMute);
    assert!(splitter.is_dual_muted());
    assert!(splitter.is_privacy_muted());
    assert!(!splitter.is_consumer_1_active());

    // 2. Transition DualMute -> PrivacySlate
    splitter.set_privacy_mode(PrivacyMuteState::PrivacySlate);
    assert!(!splitter.is_dual_muted());
    assert!(splitter.is_privacy_muted());
    assert!(splitter.is_consumer_1_active());

    let out_slate = splitter.feed_camera_frame(300).expect("feed failed");
    assert_eq!(out_slate.consumer_1_mediapipe, Some("raw_frame_300".to_string()));
    assert_eq!(out_slate.consumer_2_meeting, "privacy_slate_frame");

    // 3. Toggle DualMute from PrivacySlate and verify PrivacySlate is restored on unmute
    splitter.set_dual_mute(true);
    assert!(splitter.is_dual_muted());
    splitter.set_dual_mute(false);
    assert!(!splitter.is_dual_muted());
    assert!(splitter.is_privacy_muted());
    assert!(splitter.is_consumer_1_active());

    // 4. Transition PrivacySlate -> Active
    splitter.set_privacy_mode(PrivacyMuteState::Active);
    assert!(!splitter.is_dual_muted());
    assert!(!splitter.is_privacy_muted());
    assert!(splitter.is_consumer_1_active());

    let out_active = splitter.feed_camera_frame(301).expect("feed failed");
    assert_eq!(out_active.consumer_1_mediapipe, Some("raw_frame_301".to_string()));
    assert_eq!(out_active.consumer_2_meeting, "outbound_frame_301");

    // 5. Toggle DualMute from Active and verify Active is restored on unmute
    splitter.set_dual_mute(true);
    assert!(splitter.is_dual_muted());
    splitter.set_dual_mute(false);
    assert!(!splitter.is_dual_muted());
    assert!(!splitter.is_privacy_muted());
    assert!(splitter.is_consumer_1_active());
}
