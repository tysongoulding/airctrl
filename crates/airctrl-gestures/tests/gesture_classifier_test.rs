//! Hand Gesture Classification Test Suite (Heuristics, Boundary Values, Swipes, Filters).

use airctrl_core::types::gesture::{GestureType, HandLandmarks, Handedness, Landmark3D};
use airctrl_gestures::classifier::heuristics::{is_pinch_at_threshold, pinch_distance, PinchHysteresis};
use airctrl_gestures::classifier::{
    FilterType, GestureClassifier, GestureClassifierConfig, SwipeDetector,
};
use airctrl_gestures::engine::mock::MockPose;
use airctrl_gestures::engine::MockMediaPipeEngine;
use airctrl_gestures::error::GestureError;
use airctrl_gestures::topology::*;

#[test]
fn test_pinch_boundary_precision_and_hysteresis() {
    let p_thumb = Landmark3D::new(0.50, 0.50, 0.0);

    // Exactly 0.050 distance
    let p_index_exact = Landmark3D::new(0.55, 0.50, 0.0);
    let dist_exact = pinch_distance(&p_thumb, &p_index_exact);
    assert!((dist_exact - 0.050).abs() < 1e-6);
    assert!(is_pinch_at_threshold(dist_exact, 0.050));

    // 0.049 distance
    let p_index_under = Landmark3D::new(0.549, 0.50, 0.0);
    let dist_under = pinch_distance(&p_thumb, &p_index_under);
    assert!(is_pinch_at_threshold(dist_under, 0.050));

    // 0.051 distance
    let p_index_over = Landmark3D::new(0.551, 0.50, 0.0);
    let dist_over = pinch_distance(&p_thumb, &p_index_over);
    assert!(!is_pinch_at_threshold(dist_over, 0.050));

    // Hysteresis enter / exit state machine
    let mut hyst = PinchHysteresis::new(0.050, 0.070);
    assert!(!hyst.is_active());

    // Enter pinch at 0.045
    assert!(hyst.update(0.045));
    assert!(hyst.is_active());

    // In intermediate zone (0.060), remains active
    assert!(hyst.update(0.060));
    assert!(hyst.is_active());

    // Exceed exit threshold (0.075) -> releases
    assert!(!hyst.update(0.075));
    assert!(!hyst.is_active());

    // Remains released at 0.060
    assert!(!hyst.update(0.060));
    assert!(!hyst.is_active());
}

#[test]
fn test_pointing_gesture_and_ray_calculation() {
    let pointing_lms = MockMediaPipeEngine::synthesize_landmarks(MockPose::Pointing).unwrap();
    let gesture = GestureClassifier::classify(&pointing_lms);
    assert_eq!(gesture, GestureType::Pointing);

    let mut classifier = GestureClassifier::default();
    let hand = HandLandmarks {
        landmarks: pointing_lms,
        handedness: Handedness::Right,
        confidence: 0.95,
    };

    let recognized = classifier.process_frame(&hand, 100).unwrap();
    assert_eq!(recognized.gesture, GestureType::Pointing);
    assert!(recognized.pointing_ray.is_some());

    let ray = recognized.pointing_ray.unwrap();
    assert_eq!(ray.origin, pointing_lms[INDEX_TIP]);
    // Pointing direction should have negative y component (upward in screen coordinates)
    assert!(ray.direction.y < 0.0);
    assert!(ray.pitch_rad > 0.0);
}

#[test]
fn test_open_palm_and_fist_boundary_values() {
    // OpenPalm boundary (all tips y < 0.45)
    let mut palm_lms = [Landmark3D::new(0.5, 0.8, 0.0); 21];
    palm_lms[THUMB_TIP] = Landmark3D::new(0.30, 0.449, 0.0);
    palm_lms[INDEX_TIP] = Landmark3D::new(0.40, 0.449, 0.0);
    palm_lms[MIDDLE_TIP] = Landmark3D::new(0.50, 0.449, 0.0);
    palm_lms[RING_TIP] = Landmark3D::new(0.60, 0.449, 0.0);
    palm_lms[PINKY_TIP] = Landmark3D::new(0.70, 0.449, 0.0);

    assert_eq!(
        GestureClassifier::classify(&palm_lms),
        GestureType::OpenPalm
    );

    // Tip at 0.451 invalidates OpenPalm
    palm_lms[INDEX_TIP] = Landmark3D::new(0.40, 0.451, 0.0);
    assert_ne!(
        GestureClassifier::classify(&palm_lms),
        GestureType::OpenPalm
    );

    // Fist boundary (all tips y > 0.55)
    let mut fist_lms = [Landmark3D::new(0.5, 0.8, 0.0); 21];
    fist_lms[THUMB_TIP] = Landmark3D::new(0.35, 0.65, 0.0);
    fist_lms[INDEX_TIP] = Landmark3D::new(0.48, 0.65, 0.0);
    fist_lms[MIDDLE_TIP] = Landmark3D::new(0.55, 0.65, 0.0);
    fist_lms[RING_TIP] = Landmark3D::new(0.62, 0.65, 0.0);
    fist_lms[PINKY_TIP] = Landmark3D::new(0.68, 0.65, 0.0);
    assert_eq!(GestureClassifier::classify(&fist_lms), GestureType::Fist);
}

#[test]
fn test_directional_swipes_and_cooldown() {
    let mut detector = SwipeDetector::new(250, 0.15, 0.50, 400);

    // 1. SwipeRight: x: 0.2 -> 0.45 over 200ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.20, 0.50, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.32, 0.50, 0.0), 200),
        None
    );
    let right_res = detector.update(&Landmark3D::new(0.45, 0.50, 0.0), 300);
    assert_eq!(right_res, Some(GestureType::SwipeRight));

    // Immediate movement within 400ms cooldown is suppressed
    assert_eq!(
        detector.update(&Landmark3D::new(0.60, 0.50, 0.0), 400),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.70, 0.50, 0.0), 500),
        None
    );

    // 2. SwipeLeft after cooldown: x: 0.8 -> 0.55 at t = 800ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.80, 0.50, 0.0), 800),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.68, 0.50, 0.0), 900),
        None
    );
    let left_res = detector.update(&Landmark3D::new(0.55, 0.50, 0.0), 1000);
    assert_eq!(left_res, Some(GestureType::SwipeLeft));

    // 3. SwipeUp: y: 0.70 -> 0.45 at t = 1500ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.70, 0.0), 1500),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.58, 0.0), 1600),
        None
    );
    let up_res = detector.update(&Landmark3D::new(0.50, 0.45, 0.0), 1700);
    assert_eq!(up_res, Some(GestureType::SwipeUp));

    // 4. SwipeDown: y: 0.30 -> 0.55 at t = 2200ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.30, 0.0), 2200),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.42, 0.0), 2300),
        None
    );
    let down_res = detector.update(&Landmark3D::new(0.50, 0.55, 0.0), 2400);
    assert_eq!(down_res, Some(GestureType::SwipeDown));
}

#[test]
fn test_temporal_smoothing_one_euro_and_ema() {
    let config = GestureClassifierConfig {
        filter_type: FilterType::OneEuro,
        ..Default::default()
    };
    let mut classifier = GestureClassifier::new(config);

    let lms = MockMediaPipeEngine::synthesize_landmarks(MockPose::Pointing).unwrap();
    let hand = HandLandmarks {
        landmarks: lms,
        handedness: Handedness::Right,
        confidence: 0.95,
    };

    // Frame 1
    let f1 = classifier.process_frame(&hand, 0).unwrap();
    // Frame 2 with small jitter (+0.02 on index tip)
    let mut jitter_lms = lms;
    jitter_lms[INDEX_TIP].x += 0.02;
    let hand_jitter = HandLandmarks {
        landmarks: jitter_lms,
        handedness: Handedness::Right,
        confidence: 0.95,
    };
    let f2 = classifier.process_frame(&hand_jitter, 33).unwrap();

    // Smoothed index tip should filter out high-frequency delta
    let delta = (f2.pointing_ray.unwrap().origin.x - f1.pointing_ray.unwrap().origin.x).abs();
    assert!(delta < 0.02, "Expected jitter filtering, got delta {delta}");
}

#[test]
fn test_edge_cases_and_error_handling() {
    // 1. Truncated landmarks returns Neutral safely
    let truncated = vec![Landmark3D::default(); 12];
    assert_eq!(
        GestureClassifier::classify(&truncated),
        GestureType::Neutral
    );

    let empty: Vec<Landmark3D> = Vec::new();
    assert_eq!(GestureClassifier::classify(&empty), GestureType::Neutral);

    // 2. Low confidence rejected
    let mut classifier = GestureClassifier::default();
    let lms = MockMediaPipeEngine::synthesize_landmarks(MockPose::Pointing).unwrap();
    let low_conf_hand = HandLandmarks {
        landmarks: lms,
        handedness: Handedness::Right,
        confidence: 0.20,
    };
    let conf_err = classifier.process_frame(&low_conf_hand, 0).unwrap_err();
    assert!(matches!(conf_err, GestureError::LowConfidence { .. }));

    // 3. NaN coordinate rejected
    let mut nan_lms = lms;
    nan_lms[INDEX_TIP].x = f32::NAN;
    let nan_hand = HandLandmarks {
        landmarks: nan_lms,
        handedness: Handedness::Right,
        confidence: 0.95,
    };
    let nan_err = classifier.process_frame(&nan_hand, 0).unwrap_err();
    assert!(matches!(nan_err, GestureError::InvalidCoordinate { .. }));
}
