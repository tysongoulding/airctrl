use airctrl_core::types::gesture::{GestureTrackingMode, HandLandmarks, Handedness, Landmark3D};
use airctrl_gestures::classifier::{
    compute_pointing_ray, PinchHysteresis, TwoFingerDetector, TwoFingerGestureEvent,
};
use airctrl_gestures::engine::{HandLandmarkEngine, MockMediaPipeEngine, MockPose};
use airctrl_gestures::spatial::SpatialCoordinateMapper;
use airctrl_gestures::{FrameBuffer, PixelFormat};

#[test]
fn test_mock_engine_process_frame_multi_single_hand() {
    let engine = MockMediaPipeEngine::new();
    let frame = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    let view = frame.as_ref();

    let multi = engine.process_frame_multi(&view).expect("tracking failed");
    assert_eq!(multi.hand_count(), 1);
    assert!(multi.right_hand.is_some());
    assert_eq!(multi.left_hand, None);
    assert_eq!(multi.total_landmarks_count(), 21);
}

#[test]
fn test_mock_engine_process_frame_multi_dual_hand() {
    let engine = MockMediaPipeEngine::with_dual_poses(MockPose::Pinch, MockPose::Pointing);
    let frame = FrameBuffer::allocate(64, 64, PixelFormat::Rgb8);
    let view = frame.as_ref();

    let multi = engine.process_frame_multi(&view).expect("tracking failed");
    assert_eq!(multi.hand_count(), 2);
    assert!(multi.left_hand.is_some());
    assert!(multi.right_hand.is_some());
    assert_eq!(multi.total_landmarks_count(), 42);

    let left = multi.left_hand.unwrap();
    let right = multi.right_hand.unwrap();

    assert_eq!(left.handedness, Handedness::Left);
    assert_eq!(right.handedness, Handedness::Right);

    // Left hand in left half x in [0.10, 0.50]
    for pt in &left.landmarks {
        assert!(pt.x >= 0.10 && pt.x <= 0.50, "Left hand landmark x: {}", pt.x);
        assert!(pt.y >= 0.0 && pt.y <= 1.0);
    }

    // Right hand in right half x in [0.50, 0.90]
    for pt in &right.landmarks {
        assert!(pt.x >= 0.50 && pt.x <= 0.90, "Right hand landmark x: {}", pt.x);
        assert!(pt.y >= 0.0 && pt.y <= 1.0);
    }
}

#[test]
fn test_two_finger_scroll_detector() {
    let mut detector = TwoFingerDetector::new();

    // Create a mock HandLandmarks with index and middle close together (d < 0.08)
    let mut hand = HandLandmarks::default();
    // Index extended: TIP y < PIP y
    hand.landmarks[6] = Landmark3D::new(0.50, 0.50, 0.0); // Index PIP
    hand.landmarks[8] = Landmark3D::new(0.50, 0.40, 0.0); // Index TIP
    // Middle extended: TIP y < PIP y
    hand.landmarks[10] = Landmark3D::new(0.52, 0.50, 0.0); // Middle PIP
    hand.landmarks[12] = Landmark3D::new(0.52, 0.40, 0.0); // Middle TIP
    // Ring curled: TIP y >= PIP y
    hand.landmarks[14] = Landmark3D::new(0.55, 0.50, 0.0); // Ring PIP
    hand.landmarks[16] = Landmark3D::new(0.55, 0.60, 0.0); // Ring TIP
    // Pinky curled: TIP y >= PIP y
    hand.landmarks[18] = Landmark3D::new(0.58, 0.50, 0.0); // Pinky PIP
    hand.landmarks[20] = Landmark3D::new(0.58, 0.60, 0.0); // Pinky TIP

    // Initial frame establishes baseline centroid
    let event1 = detector.update(&hand, 1_000_000);
    match event1 {
        TwoFingerGestureEvent::Scroll { delta_y, position, .. } => {
            assert_eq!(delta_y, 0.0);
            assert!((position - 0.40).abs() < 1e-4);
        }
        other => panic!("Expected baseline Scroll event, got {:?}", other),
    }

    // Second frame moves fingers downward: y from 0.40 to 0.45 (delta_y = +0.05)
    hand.landmarks[8].y = 0.45;
    hand.landmarks[12].y = 0.45;
    let event2 = detector.update(&hand, 2_000_000);
    match event2 {
        TwoFingerGestureEvent::Scroll { delta_y, position, .. } => {
            assert!((delta_y - 0.05).abs() < 1e-4);
            assert!((position - 0.45).abs() < 1e-4);
        }
        other => panic!("Expected Scroll event, got {:?}", other),
    }
}

#[test]
fn test_two_finger_scroll_rejected_when_fingers_apart() {
    let mut detector = TwoFingerDetector::new();
    let mut hand = HandLandmarks::default();
    // Index and middle far apart (dx = 0.15 >= 0.080)
    hand.landmarks[6] = Landmark3D::new(0.40, 0.50, 0.0);
    hand.landmarks[8] = Landmark3D::new(0.40, 0.40, 0.0);
    hand.landmarks[10] = Landmark3D::new(0.55, 0.50, 0.0);
    hand.landmarks[12] = Landmark3D::new(0.55, 0.40, 0.0);
    hand.landmarks[14] = Landmark3D::new(0.60, 0.50, 0.0);
    hand.landmarks[16] = Landmark3D::new(0.60, 0.60, 0.0);
    hand.landmarks[18] = Landmark3D::new(0.65, 0.50, 0.0);
    hand.landmarks[20] = Landmark3D::new(0.65, 0.60, 0.0);

    let event = detector.update(&hand, 1_000_000);
    assert!(!matches!(event, TwoFingerGestureEvent::Scroll { .. }));
}

#[test]
fn test_two_finger_resize_deadband() {
    let mut detector = TwoFingerDetector::new();
    let mut hand = HandLandmarks::default();
    // Neutral fingers: index curled, middle curled to avoid triggering scroll
    hand.landmarks[6] = Landmark3D::new(0.50, 0.40, 0.0);
    hand.landmarks[8] = Landmark3D::new(0.50, 0.50, 0.0); // Index curled
    hand.landmarks[10] = Landmark3D::new(0.55, 0.40, 0.0);
    hand.landmarks[12] = Landmark3D::new(0.55, 0.50, 0.0); // Middle curled

    // Thumb tip at (0.50, 0.50, 0.0), Index tip at (0.54, 0.50, 0.0) -> initial dist = 0.04
    hand.landmarks[4] = Landmark3D::new(0.50, 0.50, 0.0); // Thumb tip
    hand.landmarks[8] = Landmark3D::new(0.54, 0.50, 0.0); // Index tip

    // Frame 1 initializes baseline pinch distance
    let _ = detector.update(&hand, 1_000_000);

    // Frame 2: Move index tip slightly: dx = 0.050 (dist = 0.050, delta_dist = 0.010 < 0.020 deadband)
    hand.landmarks[8].x = 0.55;
    let event1 = detector.update(&hand, 2_000_000);
    assert_eq!(event1, TwoFingerGestureEvent::None);

    // Frame 3: Move further: dx = 0.075 (dist = 0.075, delta_dist = 0.035 >= 0.020 deadband)
    hand.landmarks[8].x = 0.575;
    let event2 = detector.update(&hand, 3_000_000);
    match event2 {
        TwoFingerGestureEvent::Resize { delta_dist, scale_factor } => {
            assert!(delta_dist > 0.020);
            assert!(scale_factor > 1.0);
        }
        other => panic!("Expected Resize event, got {:?}", other),
    }
}

#[test]
fn test_hysteresis_pinch_engage_and_release() {
    let mut hysteresis = PinchHysteresis::new(0.050, 0.070);

    // Distance = 0.04 <= 0.050 -> engages
    assert!(hysteresis.update(0.040));
    assert!(hysteresis.is_active());

    // Distance = 0.060 (between 0.050 and 0.070) -> stays active
    assert!(hysteresis.update(0.060));
    assert!(hysteresis.is_active());

    // Distance = 0.080 > 0.070 -> releases
    assert!(!hysteresis.update(0.080));
    assert!(!hysteresis.is_active());

    // Distance = 0.060 when inactive -> stays inactive (needs <= 0.050 to engage)
    assert!(!hysteresis.update(0.060));
    assert!(!hysteresis.is_active());
}

#[test]
fn test_pointing_ray_computation() {
    let mut hand = HandLandmarks::default();

    // Index PIP at (0.50, 0.50, 0.0), Tip at (0.50, 0.20, 0.0) -> pointing directly up (-y)
    hand.landmarks[6] = Landmark3D::new(0.50, 0.50, 0.0);
    hand.landmarks[8] = Landmark3D::new(0.50, 0.20, 0.0);

    let ray = compute_pointing_ray(&hand.landmarks);
    assert!((ray.direction.x).abs() < 1e-4);
    assert!((ray.direction.y - (-1.0)).abs() < 1e-4);
    assert!((ray.direction.z).abs() < 1e-4);
}

#[test]
fn test_spatial_mapper_polar_coordinates() {
    let mut mapper = SpatialCoordinateMapper::new(8, (0.5, 0.5), 0.1);
    let state = mapper.map_coords(0.7, 0.5);
    assert!(!state.in_deadzone);
    assert!(state.normalized_radius > 0.0);
}

#[test]
fn test_gesture_tracking_mode_string_parsing() {
    assert_eq!("SingleHand".parse::<GestureTrackingMode>().unwrap(), GestureTrackingMode::SingleHand);
    assert_eq!("single".parse::<GestureTrackingMode>().unwrap(), GestureTrackingMode::SingleHand);
    assert_eq!("DualHand".parse::<GestureTrackingMode>().unwrap(), GestureTrackingMode::DualHand);
    assert_eq!("dual_hand".parse::<GestureTrackingMode>().unwrap(), GestureTrackingMode::DualHand);
    assert!("invalid_mode".parse::<GestureTrackingMode>().is_err());
}
