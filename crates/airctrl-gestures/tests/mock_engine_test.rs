//! Mock Engine Determinism and Marker Decoding Test Suite.

use airctrl_core::types::gesture::Handedness;
use airctrl_gestures::engine::{HandLandmarkEngine, MockMediaPipeEngine, MockPose};
use airctrl_gestures::error::GestureError;
use airctrl_gestures::frame::{FrameBuffer, PixelFormat};
use airctrl_gestures::topology::*;
use std::time::Instant;

#[test]
fn test_mock_engine_lifecycle_and_readiness() {
    let engine = MockMediaPipeEngine::new();
    assert!(engine.is_ready());
    assert_eq!(engine.name(), "MockMediaPipeEngine-v1.0");

    let frame = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    let res = engine.process_frame(&frame.as_ref());
    assert!(res.is_ok());

    // Toggle unready
    engine.set_ready(false);
    assert!(!engine.is_ready());
    let unready_res = engine.process_frame(&frame.as_ref());
    assert!(matches!(unready_res, Err(GestureError::EngineNotReady(_))));

    // Toggle back to ready
    engine.set_ready(true);
    assert!(engine.is_ready());
}

#[test]
fn test_mock_engine_confidence_and_handedness() {
    let engine = MockMediaPipeEngine::new();
    engine.set_confidence(0.88);
    engine.set_handedness(Handedness::Left);

    let frame = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    let hand = engine.process_frame(&frame.as_ref()).unwrap().unwrap();
    assert_eq!(hand.confidence, 0.88);
    assert_eq!(hand.handedness, Handedness::Left);

    // Confidence clamping
    engine.set_confidence(1.5);
    let hand_clamped = engine.process_frame(&frame.as_ref()).unwrap().unwrap();
    assert_eq!(hand_clamped.confidence, 1.0);

    engine.set_confidence(-0.5);
    let hand_clamped_low = engine.process_frame(&frame.as_ref()).unwrap().unwrap();
    assert_eq!(hand_clamped_low.confidence, 0.0);
}

#[test]
fn test_mock_pose_synthesis_properties() {
    // 1. Pinch
    let pinch_lms = MockMediaPipeEngine::synthesize_landmarks(MockPose::Pinch).unwrap();
    let dx = pinch_lms[THUMB_TIP].x - pinch_lms[INDEX_TIP].x;
    let dy = pinch_lms[THUMB_TIP].y - pinch_lms[INDEX_TIP].y;
    let dist = (dx * dx + dy * dy).sqrt();
    assert!(dist <= 0.05);

    // 2. Pointing
    let point_lms = MockMediaPipeEngine::synthesize_landmarks(MockPose::Pointing).unwrap();
    assert!(point_lms[INDEX_TIP].y < 0.35);
    assert!(point_lms[MIDDLE_TIP].y > 0.55);
    assert!(point_lms[RING_TIP].y > 0.55);

    // 3. OpenPalm
    let palm_lms = MockMediaPipeEngine::synthesize_landmarks(MockPose::OpenPalm).unwrap();
    for &idx in &FINGERTIP_INDICES {
        assert!(palm_lms[idx].y < 0.45);
    }

    // 4. Fist
    let fist_lms = MockMediaPipeEngine::synthesize_landmarks(MockPose::Fist).unwrap();
    for &idx in &FINGERTIP_INDICES {
        assert!(fist_lms[idx].y > 0.55);
    }

    // 5. Empty
    assert!(MockMediaPipeEngine::synthesize_landmarks(MockPose::Empty).is_none());
}

#[test]
fn test_frame_marker_pixel_decoding() {
    let engine = MockMediaPipeEngine::new();

    // Red -> Pointing
    let mut frame_red = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    frame_red.set_pixel(0, 0, 255, 0, 0);
    let res_red = engine.process_frame(&frame_red.as_ref()).unwrap().unwrap();
    assert!(res_red.landmarks[INDEX_TIP].y < 0.35);

    // Green -> Pinch
    let mut frame_green = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    frame_green.set_pixel(0, 0, 0, 255, 0);
    let res_green = engine
        .process_frame(&frame_green.as_ref())
        .unwrap()
        .unwrap();
    let pinch_dist = ((res_green.landmarks[THUMB_TIP].x - res_green.landmarks[INDEX_TIP].x).powi(2)
        + (res_green.landmarks[THUMB_TIP].y - res_green.landmarks[INDEX_TIP].y).powi(2))
    .sqrt();
    assert!(pinch_dist <= 0.05);

    // Blue -> OpenPalm
    let mut frame_blue = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    frame_blue.set_pixel(0, 0, 0, 0, 255);
    let res_blue = engine.process_frame(&frame_blue.as_ref()).unwrap().unwrap();
    assert!(res_blue.landmarks[MIDDLE_TIP].y < 0.45);

    // Gray (128, 128, 128) -> Fist
    let mut frame_gray = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    frame_gray.set_pixel(0, 0, 128, 128, 128);
    let res_fist = engine.process_frame(&frame_gray.as_ref()).unwrap().unwrap();
    assert!(res_fist.landmarks[INDEX_TIP].y > 0.55);

    // Marker (1, 1, 1) -> Empty
    let mut frame_empty = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    frame_empty.set_pixel(0, 0, 1, 1, 1);
    let res_empty = engine.process_frame(&frame_empty.as_ref()).unwrap();
    assert!(res_empty.is_none());
}

#[test]
fn test_mock_engine_sub_millisecond_latency() {
    let engine = MockMediaPipeEngine::new();
    let frame = FrameBuffer::allocate(320, 240, PixelFormat::Rgb8);

    let start = Instant::now();
    let iterations = 100;
    for _ in 0..iterations {
        let _ = engine.process_frame(&frame.as_ref()).unwrap();
    }
    let elapsed = start.elapsed();
    let per_frame_us = elapsed.as_micros() as f64 / iterations as f64;

    // Per-frame latency should easily be < 100 microseconds (< 0.1ms)
    assert!(
        per_frame_us < 200.0,
        "Mock engine latency exceeded budget: {per_frame_us} us"
    );
}
