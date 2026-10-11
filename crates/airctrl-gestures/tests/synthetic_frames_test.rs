//! Synthetic Frame Generation Test Suite (Clean, Occluded, Empty, Handedness).

use airctrl_core::types::gesture::{GestureType, Handedness};
use airctrl_gestures::classifier::GestureClassifier;
use airctrl_gestures::engine::mock::MockPose;
use airctrl_gestures::engine::{HandLandmarkEngine, MockMediaPipeEngine};
use airctrl_gestures::frame::PixelFormat;
use airctrl_gestures::synthetic::SyntheticFrameGenerator;

#[test]
fn test_synthetic_empty_frame_yields_no_landmarks() {
    let engine = MockMediaPipeEngine::new();
    let frame = SyntheticFrameGenerator::create_empty_frame(160, 120, PixelFormat::Rgb8);

    let res = engine.process_frame(&frame.as_ref()).unwrap();
    assert!(res.is_none());
}

#[test]
fn test_synthetic_hand_frame_poses_and_classification() {
    let engine = MockMediaPipeEngine::new();

    // 1. Pointing
    let frame_pointing = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::Pointing,
        Handedness::Right,
        PixelFormat::Rgb8,
    );
    let hand_pointing = engine
        .process_frame(&frame_pointing.as_ref())
        .unwrap()
        .unwrap();
    let gesture_pointing = GestureClassifier::classify(&hand_pointing.landmarks);
    assert_eq!(gesture_pointing, GestureType::Pointing);

    // 2. Pinch
    let frame_pinch = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::Pinch,
        Handedness::Right,
        PixelFormat::Rgb8,
    );
    let hand_pinch = engine
        .process_frame(&frame_pinch.as_ref())
        .unwrap()
        .unwrap();
    let gesture_pinch = GestureClassifier::classify(&hand_pinch.landmarks);
    assert_eq!(gesture_pinch, GestureType::Pinch);

    // 3. OpenPalm
    let frame_palm = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::OpenPalm,
        Handedness::Right,
        PixelFormat::Rgb8,
    );
    let hand_palm = engine.process_frame(&frame_palm.as_ref()).unwrap().unwrap();
    let gesture_palm = GestureClassifier::classify(&hand_palm.landmarks);
    assert_eq!(gesture_palm, GestureType::OpenPalm);

    // 4. Fist
    let frame_fist = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::Fist,
        Handedness::Right,
        PixelFormat::Rgb8,
    );
    let hand_fist = engine.process_frame(&frame_fist.as_ref()).unwrap().unwrap();
    let gesture_fist = GestureClassifier::classify(&hand_fist.landmarks);
    assert_eq!(gesture_fist, GestureType::Fist);
}

#[test]
fn test_synthetic_handedness_left_and_right() {
    let frame_right = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::Pointing,
        Handedness::Right,
        PixelFormat::Rgb8,
    );
    let frame_left = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::Pointing,
        Handedness::Left,
        PixelFormat::Rgb8,
    );

    // Marker at (0, 0) should be identical (Pointing)
    assert_eq!(frame_right.get_pixel(0, 0), Some((255, 0, 0)));
    assert_eq!(frame_left.get_pixel(0, 0), Some((255, 0, 0)));

    // Both frames have valid dimensions and stride
    assert!(frame_right.validate().is_ok());
    assert!(frame_left.validate().is_ok());
}

#[test]
fn test_synthetic_occluded_hand_frame() {
    let engine = MockMediaPipeEngine::new();
    let occlusion_rect = (50, 50, 60, 40); // (x, y, w, h)
    let occluded_frame = SyntheticFrameGenerator::create_occluded_hand_frame(
        160,
        120,
        MockPose::Pointing,
        Handedness::Right,
        occlusion_rect,
        PixelFormat::Rgb8,
    );

    // Verify marker pixel was NOT overwritten by occlusion
    assert_eq!(occluded_frame.get_pixel(0, 0), Some((255, 0, 0)));

    // Verify occlusion pixel inside rect
    assert_eq!(occluded_frame.get_pixel(60, 60), Some((40, 40, 40)));

    // Inference succeeds and classifies pointing
    let hand = engine
        .process_frame(&occluded_frame.as_ref())
        .unwrap()
        .unwrap();
    let gesture = GestureClassifier::classify(&hand.landmarks);
    assert_eq!(gesture, GestureType::Pointing);
}

#[test]
fn test_synthetic_pixel_formats() {
    let formats = [
        PixelFormat::Rgb8,
        PixelFormat::Rgba8,
        PixelFormat::Bgr8,
        PixelFormat::Bgra8,
    ];

    for fmt in formats {
        let frame = SyntheticFrameGenerator::create_hand_frame(
            80,
            60,
            MockPose::Pinch,
            Handedness::Right,
            fmt,
        );
        assert_eq!(frame.format, fmt);
        assert_eq!(frame.get_pixel(0, 0), Some((0, 255, 0)));
        assert!(frame.validate().is_ok());
    }
}
