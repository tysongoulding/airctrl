//! Milestone M11 Adversarial Stress Test Suite
//!
//! Validates:
//! 1. `MultiHandLandmarks`: zero hands, single hand (left only, right only),
//!    dual hands, overlapping hands, out-of-bounds coordinates, NaN/infinite float inputs.
//! 2. `TwoFingerDetector`: noisy jittering landmarks near 0.080 boundary,
//!    rapid scrolling direction reversals, pinch-to-resize deadband thresholding (<0.020 vs >=0.020).
//! 3. Pinch hysteresis: distance cycling 0.049 -> 0.055 -> 0.069 -> 0.071,
//!    verifying pinch remains engaged until strictly exceeding 0.070.
//! 4. High-throughput CPU benchmark: `process_frame_multi` sustaining >= 60 FPS across 1,000 continuous frames.

use airctrl_core::types::gesture::{
    GestureTrackingMode, HandLandmarks, Handedness, Landmark3D, MultiHandLandmarks,
};
use airctrl_gestures::classifier::{
    compute_pointing_ray, is_pinch_at_threshold, pinch_distance, FilterType, GestureClassifier,
    GestureClassifierConfig, PinchHysteresis, TwoFingerDetector, TwoFingerGestureEvent,
};
use airctrl_gestures::engine::{HandLandmarkEngine, MockMediaPipeEngine, MockPose};
use airctrl_gestures::error::GestureError;
use airctrl_gestures::frame::{FrameBuffer, PixelFormat};
use airctrl_gestures::mapper::SpatialCoordinateMapper;
use airctrl_gestures::topology::{INDEX_PIP, INDEX_TIP, MIDDLE_PIP, MIDDLE_TIP};
use std::time::Instant;

// ============================================================================
// 1. MultiHandLandmarks Stress Tests
// ============================================================================

#[test]
fn test_multi_hand_landmarks_zero_hands_invariants() {
    // 1. Default construct
    let default_multi = MultiHandLandmarks::default();
    assert_eq!(default_multi.hand_count(), 0);
    assert!(default_multi.is_empty());
    assert_eq!(default_multi.total_landmarks_count(), 0);
    assert!(default_multi.primary_hand().is_none());
    assert!(default_multi.left_hand.is_none());
    assert!(default_multi.right_hand.is_none());
    assert!(default_multi.hands.is_empty());

    // 2. MultiHandLandmarks::new(None, None)
    let explicit_none = MultiHandLandmarks::new(None, None);
    assert_eq!(explicit_none.hand_count(), 0);
    assert!(explicit_none.is_empty());
    assert_eq!(explicit_none.total_landmarks_count(), 0);
    assert!(explicit_none.primary_hand().is_none());

    // 3. MultiHandLandmarks::from_hands(vec![])
    let empty_vec = MultiHandLandmarks::from_hands(vec![]);
    assert_eq!(empty_vec.hand_count(), 0);
    assert!(empty_vec.is_empty());
    assert_eq!(empty_vec.total_landmarks_count(), 0);
    assert!(empty_vec.primary_hand().is_none());

    // 4. JSON serialization roundtrip
    let serialized = serde_json::to_string(&default_multi).expect("serialize zero hands");
    let deserialized: MultiHandLandmarks =
        serde_json::from_str(&serialized).expect("deserialize zero hands");
    assert_eq!(deserialized.hand_count(), 0);
    assert!(deserialized.is_empty());
    assert_eq!(deserialized.total_landmarks_count(), 0);
}

#[test]
fn test_multi_hand_landmarks_single_hand_left_and_right() {
    let left_hand = HandLandmarks {
        landmarks: [Landmark3D::new(0.25, 0.50, 0.0); 21],
        handedness: Handedness::Left,
        confidence: 0.92,
    };
    let right_hand = HandLandmarks {
        landmarks: [Landmark3D::new(0.75, 0.50, 0.0); 21],
        handedness: Handedness::Right,
        confidence: 0.98,
    };

    // Left Only
    let left_only = MultiHandLandmarks::new(Some(left_hand.clone()), None);
    assert_eq!(left_only.hand_count(), 1);
    assert!(!left_only.is_empty());
    assert_eq!(left_only.total_landmarks_count(), 21);
    assert!(left_only.left_hand.is_some());
    assert!(left_only.right_hand.is_none());
    assert_eq!(left_only.primary_hand().unwrap().handedness, Handedness::Left);

    // Right Only
    let right_only = MultiHandLandmarks::new(None, Some(right_hand.clone()));
    assert_eq!(right_only.hand_count(), 1);
    assert!(!right_only.is_empty());
    assert_eq!(right_only.total_landmarks_count(), 21);
    assert!(right_only.left_hand.is_none());
    assert!(right_only.right_hand.is_some());
    assert_eq!(right_only.primary_hand().unwrap().handedness, Handedness::Right);

    // Precedence test: When both present, primary hand prefers Right hand
    let dual = MultiHandLandmarks::new(Some(left_hand), Some(right_hand));
    assert_eq!(dual.hand_count(), 2);
    assert_eq!(dual.total_landmarks_count(), 42);
    assert_eq!(dual.primary_hand().unwrap().handedness, Handedness::Right);
}

#[test]
fn test_multi_hand_landmarks_dual_hands_and_from_hands_sorting() {
    let left = HandLandmarks {
        landmarks: [Landmark3D::new(0.20, 0.50, 0.0); 21],
        handedness: Handedness::Left,
        confidence: 0.88,
    };
    let right = HandLandmarks {
        landmarks: [Landmark3D::new(0.80, 0.50, 0.0); 21],
        handedness: Handedness::Right,
        confidence: 0.96,
    };

    // from_hands with reversed order [Right, Left]
    let multi = MultiHandLandmarks::from_hands(vec![right.clone(), left.clone()]);
    assert_eq!(multi.hand_count(), 2);
    assert_eq!(multi.total_landmarks_count(), 42);
    assert!(multi.left_hand.is_some());
    assert!(multi.right_hand.is_some());
    assert_eq!(multi.left_hand.as_ref().unwrap().handedness, Handedness::Left);
    assert_eq!(multi.right_hand.as_ref().unwrap().handedness, Handedness::Right);

    // JSON roundtrip
    let json_str = serde_json::to_string(&multi).expect("serialize dual");
    let restored: MultiHandLandmarks = serde_json::from_str(&json_str).expect("deserialize dual");
    assert_eq!(restored.hand_count(), 2);
    assert_eq!(restored.total_landmarks_count(), 42);
    assert_eq!(restored.left_hand.unwrap().handedness, Handedness::Left);
    assert_eq!(restored.right_hand.unwrap().handedness, Handedness::Right);
}

#[test]
fn test_multi_hand_landmarks_overlapping_hands() {
    // Both hands at the exact same spatial location
    let identical_landmarks = [Landmark3D::new(0.50, 0.50, 0.0); 21];
    let left_hand = HandLandmarks {
        landmarks: identical_landmarks,
        handedness: Handedness::Left,
        confidence: 0.90,
    };
    let right_hand = HandLandmarks {
        landmarks: identical_landmarks,
        handedness: Handedness::Right,
        confidence: 0.90,
    };

    let multi = MultiHandLandmarks::new(Some(left_hand), Some(right_hand));
    assert_eq!(multi.hand_count(), 2);
    assert_eq!(multi.total_landmarks_count(), 42);
    assert_eq!(multi.primary_hand().unwrap().handedness, Handedness::Right);

    // Inter-hand distance between wrists is 0.0
    let l_wrist = multi.left_hand.as_ref().unwrap().landmarks[0];
    let r_wrist = multi.right_hand.as_ref().unwrap().landmarks[0];
    let dist = ((l_wrist.x - r_wrist.x).powi(2) + (l_wrist.y - r_wrist.y).powi(2)).sqrt();
    assert_eq!(dist, 0.0);
}

#[test]
fn test_multi_hand_landmarks_out_of_bounds_coordinates() {
    // Coordinates outside standard normalized [0.0, 1.0] viewport
    let out_of_bounds_coords = [
        Landmark3D::new(-1.50, -0.80, -2.0),
        Landmark3D::new(2.75, 3.50, 5.0),
        Landmark3D::new(0.50, 1.25, 0.0),
    ];

    let mut landmarks = [Landmark3D::default(); 21];
    for (i, lm) in landmarks.iter_mut().enumerate() {
        *lm = out_of_bounds_coords[i % out_of_bounds_coords.len()];
    }

    let hand = HandLandmarks {
        landmarks,
        handedness: Handedness::Right,
        confidence: 0.95,
    };

    let multi = MultiHandLandmarks::new(None, Some(hand.clone()));
    assert_eq!(multi.hand_count(), 1);
    assert_eq!(multi.total_landmarks_count(), 21);

    // SpatialCoordinateMapper handles out-of-bounds coords with radius clamping
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);
    let state = mapper.map_coords(-1.50, -0.80);
    assert!(state.normalized_radius <= 1.0, "Radius clamped at boundary");

    // Pointing ray computation on out-of-bounds coordinates normalizes safely
    let ray = compute_pointing_ray(&hand.landmarks);
    let dir_len = (ray.direction.x.powi(2) + ray.direction.y.powi(2) + ray.direction.z.powi(2)).sqrt();
    assert!((dir_len - 1.0).abs() < 1e-4, "Direction vector must be normalized unit length");
}

#[test]
fn test_multi_hand_landmarks_nan_and_infinite_rejection() {
    let bad_coords = [
        (f32::NAN, 0.5, 0.0),
        (0.5, f32::NAN, 0.0),
        (0.5, 0.5, f32::NAN),
        (f32::INFINITY, 0.5, 0.0),
        (0.5, f32::INFINITY, 0.0),
        (0.5, 0.5, f32::INFINITY),
        (f32::NEG_INFINITY, 0.5, 0.0),
        (0.5, f32::NEG_INFINITY, 0.0),
        (0.5, 0.5, f32::NEG_INFINITY),
    ];

    let mut classifier = GestureClassifier::new(GestureClassifierConfig {
        filter_type: FilterType::None,
        ..Default::default()
    });

    for (x, y, z) in bad_coords {
        let mut landmarks = [Landmark3D::new(0.5, 0.5, 0.0); 21];
        landmarks[8] = Landmark3D::new(x, y, z); // Corrupt index tip

        let hand = HandLandmarks {
            landmarks,
            handedness: Handedness::Right,
            confidence: 0.95,
        };

        // GestureClassifier::process_frame must reject NaN/infinite with GestureError::InvalidCoordinate
        let result = classifier.process_frame(&hand, 100);
        assert!(
            matches!(result, Err(GestureError::InvalidCoordinate { index: 8, .. })),
            "Expected InvalidCoordinate error for coord ({}, {}, {}), got {:?}",
            x,
            y,
            z,
            result
        );
    }
}

// ============================================================================
// 2. TwoFingerDetector Stress Tests
// ============================================================================

#[test]
fn test_two_finger_detector_boundary_jitter_near_0_080() {
    let mut detector = TwoFingerDetector::new();
    let mut hand = HandLandmarks::default();

    // Index & middle extended, ring & pinky curled
    hand.landmarks[INDEX_PIP] = Landmark3D::new(0.50, 0.50, 0.0);
    hand.landmarks[INDEX_TIP] = Landmark3D::new(0.50, 0.40, 0.0);
    hand.landmarks[MIDDLE_PIP] = Landmark3D::new(0.50, 0.50, 0.0);
    hand.landmarks[MIDDLE_TIP] = Landmark3D::new(0.50, 0.40, 0.0);

    // Ring & pinky curled down
    hand.landmarks[14] = Landmark3D::new(0.60, 0.50, 0.0);
    hand.landmarks[16] = Landmark3D::new(0.60, 0.60, 0.0);
    hand.landmarks[18] = Landmark3D::new(0.65, 0.50, 0.0);
    hand.landmarks[20] = Landmark3D::new(0.65, 0.60, 0.0);

    // Sequence of distances oscillating right around 0.080 threshold:
    // (x_offset, expected_in_threshold)
    let jitter_sequence = [
        (0.0750, true),  // 0.0750 < 0.080 -> in threshold
        (0.0810, false), // 0.0810 > 0.080 -> out
        (0.0795, true),  // 0.0795 < 0.080 -> in
        (0.0820, false), // 0.0820 > 0.080 -> out
        (0.0798, true),  // 0.0798 < 0.080 -> in
        (0.0802, false), // 0.0802 > 0.080 -> out (strictly > 0.080)
        (0.0790, true),  // 0.0790 < 0.080 -> in
    ];

    for (step, &(offset, expected_in_threshold)) in jitter_sequence.iter().enumerate() {
        hand.landmarks[MIDDLE_TIP].x = 0.50 + offset;
        let p_scroll = TwoFingerDetector::evaluate_scroll_position(
            &hand.landmarks[INDEX_TIP],
            &hand.landmarks[MIDDLE_TIP],
        );

        if expected_in_threshold {
            assert!(
                p_scroll > 0.0,
                "Step {}: Expected active scroll position for offset {}, got 0.0",
                step,
                offset
            );
        } else {
            assert_eq!(
                p_scroll, 0.0,
                "Step {}: Expected 0.0 scroll position for offset {}, got {}",
                step, offset, p_scroll
            );
        }

        let event = detector.update(&hand, (step as u64 + 1) * 100);
        if !expected_in_threshold {
            // When fingers are apart (> 0.080), scroll must NOT be produced
            assert!(
                !matches!(event, TwoFingerGestureEvent::Scroll { .. }),
                "Step {}: Scroll produced when fingers apart: {:?}",
                step,
                event
            );
        }
    }
}

#[test]
fn test_two_finger_detector_rapid_scrolling_direction_reversals() {
    let mut detector = TwoFingerDetector::new();
    let mut hand = HandLandmarks::default();

    // Set up valid two-finger scroll pose
    hand.landmarks[INDEX_PIP] = Landmark3D::new(0.50, 0.50, 0.0);
    hand.landmarks[MIDDLE_PIP] = Landmark3D::new(0.54, 0.50, 0.0);
    hand.landmarks[14] = Landmark3D::new(0.60, 0.50, 0.0);
    hand.landmarks[16] = Landmark3D::new(0.60, 0.60, 0.0);
    hand.landmarks[18] = Landmark3D::new(0.65, 0.50, 0.0);
    hand.landmarks[20] = Landmark3D::new(0.65, 0.60, 0.0);

    // Initial baseline frame at y = 0.30
    hand.landmarks[INDEX_TIP] = Landmark3D::new(0.50, 0.30, 0.0);
    hand.landmarks[MIDDLE_TIP] = Landmark3D::new(0.54, 0.30, 0.0);
    let init_event = detector.update(&hand, 1_000);
    assert!(matches!(init_event, TwoFingerGestureEvent::Scroll { delta_y, .. } if delta_y == 0.0));

    // Rapid alternating reversal frames:
    // (target_y, expected_sign: +1 for down, -1 for up, 0 for deadband None)
    let movements: &[(f32, i32)] = &[
        (0.35, 1),   // delta = +0.05 -> Scroll Down
        (0.28, -1),  // delta = -0.07 -> Scroll Up
        (0.36, 1),   // delta = +0.08 -> Scroll Down
        (0.25, -1),  // delta = -0.11 -> Scroll Up
        (0.255, 0),  // delta = +0.005 < 0.010 threshold -> Deadband (None)
        (0.32, 1),   // delta = +0.065 relative to 0.255 -> Scroll Down
        (0.20, -1),  // delta = -0.12 relative to 0.320 -> Scroll Up
    ];

    let mut current_y = 0.30;
    for (i, &(target_y, expected_sign)) in movements.iter().enumerate() {
        hand.landmarks[INDEX_TIP].y = target_y;
        hand.landmarks[MIDDLE_TIP].y = target_y;
        let event = detector.update(&hand, (i as u64 + 2) * 1_000);

        match expected_sign {
            1 => match event {
                TwoFingerGestureEvent::Scroll { delta_y, position, .. } => {
                    assert!(delta_y > 0.0, "Expected positive delta_y at frame {}", i);
                    let expected_delta = target_y - current_y;
                    assert!(
                        (delta_y - expected_delta).abs() < 1e-4,
                        "Frame {}: delta_y {} != expected {}",
                        i,
                        delta_y,
                        expected_delta
                    );
                    assert!((position - target_y).abs() < 1e-4);
                    current_y = target_y;
                }
                other => panic!("Frame {}: expected Scroll with delta_y > 0, got {:?}", i, other),
            },
            -1 => match event {
                TwoFingerGestureEvent::Scroll { delta_y, position, .. } => {
                    assert!(delta_y < 0.0, "Expected negative delta_y at frame {}", i);
                    let expected_delta = target_y - current_y;
                    assert!(
                        (delta_y - expected_delta).abs() < 1e-4,
                        "Frame {}: delta_y {} != expected {}",
                        i,
                        delta_y,
                        expected_delta
                    );
                    assert!((position - target_y).abs() < 1e-4);
                    current_y = target_y;
                }
                other => panic!("Frame {}: expected Scroll with delta_y < 0, got {:?}", i, other),
            },
            0 => {
                assert_eq!(
                    event,
                    TwoFingerGestureEvent::None,
                    "Frame {}: expected None for sub-threshold motion, got {:?}",
                    i,
                    event
                );
                // TwoFingerDetector updates last_scroll_centroid on every frame to prevent drift
                current_y = target_y;
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_two_finger_resize_deadband_thresholding_sub_vs_super() {
    let mut detector = TwoFingerDetector::new();
    let mut hand = HandLandmarks::default();

    // Curl index & middle so scroll does not take priority
    hand.landmarks[INDEX_PIP] = Landmark3D::new(0.50, 0.40, 0.0);
    hand.landmarks[INDEX_TIP] = Landmark3D::new(0.50, 0.50, 0.0); // curled
    hand.landmarks[MIDDLE_PIP] = Landmark3D::new(0.55, 0.40, 0.0);
    hand.landmarks[MIDDLE_TIP] = Landmark3D::new(0.55, 0.50, 0.0); // curled

    // Thumb tip at (0.50, 0.50, 0.0), Index tip at (0.60, 0.50, 0.0) -> dist = 0.100
    hand.landmarks[4] = Landmark3D::new(0.50, 0.50, 0.0); // Thumb tip
    hand.landmarks[8] = Landmark3D::new(0.60, 0.50, 0.0); // Index tip

    // Baseline frame initializes pinch distance to 0.100
    let init_event = detector.update(&hand, 1_000);
    assert_eq!(init_event, TwoFingerGestureEvent::None);

    // Test cases: (new_index_x, expected_event_type, description)
    // 1. Move to 0.610 -> delta = 0.010 < 0.020 deadband -> None
    hand.landmarks[8].x = 0.610;
    assert_eq!(
        detector.update(&hand, 2_000),
        TwoFingerGestureEvent::None,
        "Sub-threshold delta 0.010 must yield None"
    );

    // 2. Move to 0.618 -> delta = 0.018 from baseline 0.100 < 0.020 deadband -> None
    hand.landmarks[8].x = 0.618;
    assert_eq!(
        detector.update(&hand, 3_000),
        TwoFingerGestureEvent::None,
        "Sub-threshold delta 0.018 must yield None"
    );

    // 3. Move to 0.625 -> delta = 0.025 >= 0.020 deadband -> Resize expand!
    hand.landmarks[8].x = 0.625;
    let event_expand = detector.update(&hand, 4_000);
    match event_expand {
        TwoFingerGestureEvent::Resize { delta_dist, scale_factor } => {
            assert!(delta_dist >= 0.020, "delta_dist should be >= 0.020, got {}", delta_dist);
            assert!(scale_factor > 1.0, "Expanding pinch must have scale_factor > 1.0");
            assert!((scale_factor - (0.125 / 0.100)).abs() < 1e-4);
        }
        other => panic!("Expected Resize expand event, got {:?}", other),
    }

    // Now current tracked distance is 0.120.
    // 4. Move to 0.625 -> delta = +0.005 from 0.120 < 0.020 deadband -> None
    hand.landmarks[8].x = 0.625;
    assert_eq!(
        detector.update(&hand, 5_000),
        TwoFingerGestureEvent::None,
        "Sub-threshold delta 0.005 from new base must yield None"
    );

    // 5. Shrink back to 0.595 -> dist = 0.095, delta = 0.095 - 0.120 = -0.025 (abs >= 0.020) -> Resize shrink!
    hand.landmarks[8].x = 0.595;
    let event_shrink = detector.update(&hand, 6_000);
    match event_shrink {
        TwoFingerGestureEvent::Resize { delta_dist, scale_factor } => {
            assert!(delta_dist < -0.020, "delta_dist should be < -0.020, got {}", delta_dist);
            assert!(scale_factor < 1.0, "Shrinking pinch must have scale_factor < 1.0");
        }
        other => panic!("Expected Resize shrink event, got {:?}", other),
    }

    // 6. Test scale_factor clamping to [0.5, 2.0] on extreme jumps
    // Current dist is 0.095. Move index tip far away: x = 0.90 -> dist = 0.40 -> scale = 0.40 / 0.095 ~ 4.21 -> clamped to 2.0
    hand.landmarks[8].x = 0.90;
    let event_extreme_expand = detector.update(&hand, 7_000);
    match event_extreme_expand {
        TwoFingerGestureEvent::Resize { scale_factor, .. } => {
            assert_eq!(scale_factor, 2.0, "Scale factor must clamp to upper limit 2.0");
        }
        other => panic!("Expected Resize event with clamped scale, got {:?}", other),
    }

    // Extreme shrink: from 0.40 to 0.505 -> dist = 0.005 -> scale = 0.005 / 0.40 = 0.0125 -> clamped to 0.5
    hand.landmarks[8].x = 0.505;
    let event_extreme_shrink = detector.update(&hand, 8_000);
    match event_extreme_shrink {
        TwoFingerGestureEvent::Resize { scale_factor, .. } => {
            assert_eq!(scale_factor, 0.5, "Scale factor must clamp to lower limit 0.5");
        }
        other => panic!("Expected Resize event with clamped scale, got {:?}", other),
    }
}

// ============================================================================
// 3. Pinch Hysteresis Stress Tests
// ============================================================================

#[test]
fn test_pinch_hysteresis_exact_sequence_0_049_to_0_055_to_0_069_to_0_071() {
    let mut hysteresis = PinchHysteresis::new(0.050, 0.070);

    // Initial state: inactive
    assert!(!hysteresis.is_active(), "Pinch must start inactive");

    // Step 1: Distance = 0.049 <= 0.050 -> Engages
    let active_1 = hysteresis.update(0.049);
    assert!(active_1, "Step 1: Distance 0.049 must engage pinch");
    assert!(hysteresis.is_active());

    // Step 2: Distance = 0.055 > 0.050 but <= 0.070 -> Must remain engaged
    let active_2 = hysteresis.update(0.055);
    assert!(
        active_2,
        "Step 2: Distance 0.055 must maintain pinch engagement via hysteresis"
    );
    assert!(hysteresis.is_active());

    // Step 3: Distance = 0.069 <= 0.070 -> Must remain engaged
    let active_3 = hysteresis.update(0.069);
    assert!(
        active_3,
        "Step 3: Distance 0.069 must maintain pinch engagement below exit threshold 0.070"
    );
    assert!(hysteresis.is_active());

    // Step 4: Distance = 0.071 > 0.070 -> Must strictly release
    let active_4 = hysteresis.update(0.071);
    assert!(
        !active_4,
        "Step 4: Distance 0.071 strictly exceeds 0.070 exit threshold and must release"
    );
    assert!(!hysteresis.is_active());

    // Step 5: Distance returns to 0.060 while inactive -> Must NOT re-engage (requires <= 0.050)
    let active_5 = hysteresis.update(0.060);
    assert!(
        !active_5,
        "Step 5: Distance 0.060 must NOT re-engage pinch when currently inactive"
    );
    assert!(!hysteresis.is_active());

    // Step 6: Exact boundary test: distance = 0.05000 -> Engages
    let active_6 = hysteresis.update(0.050);
    assert!(active_6, "Step 6: Distance exactly 0.050 must engage pinch");
    assert!(hysteresis.is_active());

    // Step 7: Exact boundary test: distance = 0.07000 -> Must NOT release (exit condition is dist > 0.070)
    let active_7 = hysteresis.update(0.070);
    assert!(
        active_7,
        "Step 7: Distance exactly 0.070 must remain engaged (only strictly > 0.070 releases)"
    );
    assert!(hysteresis.is_active());

    // Step 8: Exceed by epsilon: 0.07001 -> Releases
    let active_8 = hysteresis.update(0.07001);
    assert!(!active_8, "Step 8: Distance 0.07001 > 0.070 must release");
    assert!(!hysteresis.is_active());
}

#[test]
fn test_pinch_distance_and_threshold_helper_functions() {
    let p1 = Landmark3D::new(0.50, 0.50, 0.0);
    let p2 = Landmark3D::new(0.53, 0.54, 0.0); // dx = 0.03, dy = 0.04 -> dist = 0.0500

    let dist = pinch_distance(&p1, &p2);
    assert!((dist - 0.050).abs() < 1e-5);
    assert!(is_pinch_at_threshold(dist, 0.050));
    assert!(is_pinch_at_threshold(0.049999, 0.050));
    assert!(!is_pinch_at_threshold(0.05001, 0.050));
}

// ============================================================================
// 4. High-Throughput CPU Benchmark: >= 60 FPS Across 1,000 Continuous Frames
// ============================================================================

#[test]
fn test_high_throughput_benchmark_process_frame_multi_1000_frames_sustains_60_fps() {
    let engine = MockMediaPipeEngine::with_dual_poses(MockPose::Pinch, MockPose::Pointing);
    let frame = FrameBuffer::allocate(320, 240, PixelFormat::Rgb8);
    let view = frame.as_ref();

    // 1. Warmup phase (50 frames)
    for _ in 0..50 {
        let multi = engine.process_frame_multi(&view).expect("warmup failed");
        assert_eq!(multi.hand_count(), 2);
    }

    // 2. High-throughput measurement phase: 1,000 continuous frames
    const TOTAL_FRAMES: usize = 1_000;
    let mut latencies_us: Vec<f64> = Vec::with_capacity(TOTAL_FRAMES);

    let start_total = Instant::now();

    for _ in 0..TOTAL_FRAMES {
        let frame_start = Instant::now();
        let multi = engine.process_frame_multi(&view).expect("dual inference failed");
        let elapsed_us = frame_start.elapsed().as_micros() as f64;
        latencies_us.push(elapsed_us);

        // Invariant checks on processed frame
        assert_eq!(multi.hand_count(), 2);
        assert_eq!(multi.total_landmarks_count(), 42);
        assert!(multi.left_hand.is_some());
        assert!(multi.right_hand.is_some());
    }

    let total_duration = start_total.elapsed();
    let total_duration_secs = total_duration.as_secs_f64();
    let effective_fps = (TOTAL_FRAMES as f64) / total_duration_secs;

    latencies_us.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean_us: f64 = latencies_us.iter().sum::<f64>() / (TOTAL_FRAMES as f64);
    let mean_ms = mean_us / 1000.0;
    let p50_ms = latencies_us[TOTAL_FRAMES * 50 / 100] / 1000.0;
    let p95_ms = latencies_us[TOTAL_FRAMES * 95 / 100] / 1000.0;
    let p99_ms = latencies_us[TOTAL_FRAMES * 99 / 100] / 1000.0;
    let max_ms = latencies_us[TOTAL_FRAMES - 1] / 1000.0;

    println!(
        "\n--- High-Throughput 1,000-Frame CPU Benchmark Results --- \
         \nTotal Frames:       {} \
         \nTotal Time:         {:.4} s \
         \nEffective FPS:      {:.2} FPS (Target: >= 60.0 FPS) \
         \nMean Latency:       {:.4} ms (Budget: <= 16.67 ms) \
         \nP50 Latency:        {:.4} ms \
         \nP95 Latency:        {:.4} ms (Budget: <= 20.00 ms) \
         \nP99 Latency:        {:.4} ms (Budget: <= 25.00 ms) \
         \nMax Latency:        {:.4} ms \
         \n---------------------------------------------------------",
        TOTAL_FRAMES,
        total_duration_secs,
        effective_fps,
        mean_ms,
        p50_ms,
        p95_ms,
        p99_ms,
        max_ms
    );

    // Strict empirical assertions:
    assert!(
        effective_fps >= 60.0,
        "Failed 60 FPS requirement: achieved only {:.2} FPS",
        effective_fps
    );
    assert!(
        mean_ms <= 16.67,
        "Mean latency {:.4} ms exceeds 60 FPS budget (16.67 ms)",
        mean_ms
    );
    assert!(
        p95_ms <= 20.0,
        "P95 latency {:.4} ms exceeds budget (20.0 ms)",
        p95_ms
    );
    assert!(
        p99_ms <= 25.0,
        "P99 latency {:.4} ms exceeds budget (25.0 ms)",
        p99_ms
    );
}

#[test]
fn test_tracking_mode_conversions_and_display() {
    assert_eq!(GestureTrackingMode::SingleHand.to_string(), "SingleHand");
    assert_eq!(GestureTrackingMode::DualHand.to_string(), "DualHand");

    let modes = ["SingleHand", "single", "DualHand", "dual", "DUAL_HAND"];
    for mode_str in modes {
        assert!(mode_str.parse::<GestureTrackingMode>().is_ok());
    }

    assert!("three_hands".parse::<GestureTrackingMode>().is_err());
}
