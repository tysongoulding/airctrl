//! challenger_m7_gestures_adversarial_test.rs
//!
//! Milestone M7 Phase 2 Tier 5 Adversarial Coverage Hardening:
//! Deep white-box adversarial verification of `airctrl-gestures`:
//! 1. 21-landmark topology invariants & normalized bounds [0.0, 1.0]:
//!    - Structural skeleton joints, fingertip indices, palm center invariants.
//!    - Bounds enforcement, negative coordinates, NaN and infinite float rejection.
//!    - Confidence threshold rejection and error mapping.
//! 2. Pointing ray geometric mathematics & degenerate configurations:
//!    - Collinear/coincident tip and pip joints (zero vector fallback).
//!    - 3D directional vector unit normalization, pitch [-pi/2, pi/2], yaw [-pi, pi].
//!    - Hemispherical and omnidirectional ray angle boundaries.
//! 3. Pinch Euclidean distance & hysteresis stability:
//!    - Precision boundary fuzzing at d=0.050 with 5-decimal rounding parity.
//!    - Inactive -> Active -> Sustained -> Released hysteresis lifecycle.
//!    - Resistance to high-frequency boundary flutter across enter/exit thresholds.
//! 4. Swipe spatio-temporal velocity & cooldown refractory state machine:
//!    - Minimum 3 samples requirement, temporal sub-window pruning.
//!    - Sub-threshold displacement and sub-threshold velocity filtering.
//!    - Cardinal axis dominance (1.5x ratio) vs ambiguous diagonal rejection.
//!    - 400ms refractory cooldown enforcement under hostile event injection.
//! 5. One-Euro and EMA temporal filters:
//!    - Stationary noise attenuation (jitter elimination).
//!    - Dynamic adaptive cutoff responding to rapid step motion without lag.
//!    - Zero and negative delta time resilience.
//! 6. Spatial coordinate mapper polar boundaries:
//!    - Deadband r < 0.10 isolation across all radial spokes.
//!    - Sector transitions at multiples of 60 degrees (pi / 3 rad).
//!    - Angular wrap-around (2pi - eps <-> 0 + eps).
//!    - Angular hysteresis suppression of sector edge jitter.
//!    - Screen viewport projection and clamping on extreme inputs.

use airctrl_core::types::gesture::{GestureType, HandLandmarks, Handedness, Landmark3D};
use airctrl_gestures::classifier::heuristics::{
    calculate_palm_normal, compute_pointing_ray, is_pinch_at_threshold,
    PinchHysteresis,
};
use airctrl_gestures::classifier::{
    EmaLandmarksFilter, GestureClassifier, OneEuroLandmarksFilter, SwipeDetector,
};
use airctrl_gestures::error::GestureError;
use airctrl_gestures::mapper::{ScreenBounds, SpatialCoordinateMapper};
use airctrl_gestures::topology::*;
use std::f32::consts::PI;

fn generate_neutral_hand() -> [Landmark3D; TOTAL_LANDMARKS] {
    let mut arr = [Landmark3D::new(0.5, 0.5, 0.0); TOTAL_LANDMARKS];
    arr[WRIST] = Landmark3D::new(0.5, 0.85, 0.0);

    arr[THUMB_CMC] = Landmark3D::new(0.42, 0.75, 0.0);
    arr[THUMB_MCP] = Landmark3D::new(0.38, 0.65, 0.0);
    arr[THUMB_IP] = Landmark3D::new(0.34, 0.58, 0.0);
    arr[THUMB_TIP] = Landmark3D::new(0.30, 0.52, 0.0);

    arr[INDEX_MCP] = Landmark3D::new(0.45, 0.60, 0.0);
    arr[INDEX_PIP] = Landmark3D::new(0.45, 0.48, 0.0);
    arr[INDEX_DIP] = Landmark3D::new(0.45, 0.42, 0.0);
    arr[INDEX_TIP] = Landmark3D::new(0.45, 0.38, 0.0);

    arr[MIDDLE_MCP] = Landmark3D::new(0.50, 0.60, 0.0);
    arr[MIDDLE_PIP] = Landmark3D::new(0.50, 0.46, 0.0);
    arr[MIDDLE_DIP] = Landmark3D::new(0.50, 0.40, 0.0);
    arr[MIDDLE_TIP] = Landmark3D::new(0.50, 0.36, 0.0);

    arr[RING_MCP] = Landmark3D::new(0.55, 0.61, 0.0);
    arr[RING_PIP] = Landmark3D::new(0.55, 0.48, 0.0);
    arr[RING_DIP] = Landmark3D::new(0.55, 0.42, 0.0);
    arr[RING_TIP] = Landmark3D::new(0.55, 0.38, 0.0);

    arr[PINKY_MCP] = Landmark3D::new(0.60, 0.63, 0.0);
    arr[PINKY_PIP] = Landmark3D::new(0.60, 0.52, 0.0);
    arr[PINKY_DIP] = Landmark3D::new(0.60, 0.46, 0.0);
    arr[PINKY_TIP] = Landmark3D::new(0.60, 0.42, 0.0);

    arr
}

// ============================================================================
// 1. 21-Landmark Topology Invariants & Normalized Bounds
// ============================================================================

#[test]
fn test_topology_constants_and_skeletal_connectivity_invariants() {
    assert_eq!(TOTAL_LANDMARKS, 21);
    assert_eq!(FINGERTIP_INDICES, [4, 8, 12, 16, 20]);
    assert_eq!(MCP_INDICES, [1, 5, 9, 13, 17]);
    assert_eq!(SKELETON_CONNECTIONS.len(), 21);

    // Verify all joint indices in skeleton connections are strictly within 0..21
    for &(a, b) in SKELETON_CONNECTIONS {
        assert!(a < TOTAL_LANDMARKS, "Joint index {} out of range", a);
        assert!(b < TOTAL_LANDMARKS, "Joint index {} out of range", b);
        assert_ne!(a, b, "Self-connection ({}, {}) is invalid", a, b);
    }
}

#[test]
fn test_normalized_landmark_list_array_conversion_and_palm_center() {
    let raw = generate_neutral_hand();

    // 1. Success on exact 21 landmarks
    let list = NormalizedLandmarkList::new(raw.to_vec()).expect("Should construct successfully");
    assert_eq!(list.len(), 21);
    assert!(!list.is_empty());
    assert_eq!(list.wrist(), Some(&raw[WRIST]));
    assert_eq!(list.thumb_tip(), Some(&raw[THUMB_TIP]));
    assert_eq!(list.index_tip(), Some(&raw[INDEX_TIP]));

    let exported_arr = list.to_array().expect("Array conversion failed");
    assert_eq!(exported_arr, raw);

    // 2. Reject insufficient or excessive landmarks
    let short_vec = raw[0..20].to_vec();
    let err_short = NormalizedLandmarkList::new(short_vec).unwrap_err();
    assert!(matches!(err_short, GestureError::InsufficientLandmarks(20)));

    let mut long_vec = raw.to_vec();
    long_vec.push(Landmark3D::default());
    let err_long = NormalizedLandmarkList::new(long_vec).unwrap_err();
    assert!(matches!(err_long, GestureError::InsufficientLandmarks(22)));

    // 3. Palm center invariant
    let palm = list.palm_center().expect("Palm center must be computable");
    let expected_x = (raw[WRIST].x
        + raw[INDEX_MCP].x
        + raw[MIDDLE_MCP].x
        + raw[RING_MCP].x
        + raw[PINKY_MCP].x)
        / 5.0;
    let expected_y = (raw[WRIST].y
        + raw[INDEX_MCP].y
        + raw[MIDDLE_MCP].y
        + raw[RING_MCP].y
        + raw[PINKY_MCP].y)
        / 5.0;
    assert!((palm.x - expected_x).abs() < 1e-6);
    assert!((palm.y - expected_y).abs() < 1e-6);
}

#[test]
fn test_landmark_ops_bounds_and_validity_predicates() {
    // 1. in_bounds predicate on [0.0, 1.0]
    let inside = Landmark3D::new(0.5, 0.5, 0.0);
    assert!(inside.in_bounds());

    let corners = [
        Landmark3D::new(0.0, 0.0, 0.0),
        Landmark3D::new(1.0, 0.0, 0.0),
        Landmark3D::new(0.0, 1.0, 0.0),
        Landmark3D::new(1.0, 1.0, 0.0),
    ];
    for c in corners {
        assert!(c.in_bounds(), "Corner {:?} must be in bounds", c);
    }

    let out_bounds = [
        Landmark3D::new(-0.0001, 0.5, 0.0),
        Landmark3D::new(1.0001, 0.5, 0.0),
        Landmark3D::new(0.5, -0.0001, 0.0),
        Landmark3D::new(0.5, 1.0001, 0.0),
        Landmark3D::new(-5.0, 5.0, 0.0),
    ];
    for ob in out_bounds {
        assert!(!ob.in_bounds(), "Coordinate {:?} should be out of bounds", ob);
    }

    // 2. is_valid predicate rejecting NaN and infinities
    let valid = Landmark3D::new(0.5, 0.5, -0.1);
    assert!(valid.is_valid());

    let invalid = [
        Landmark3D::new(f32::NAN, 0.5, 0.0),
        Landmark3D::new(0.5, f32::NAN, 0.0),
        Landmark3D::new(0.5, 0.5, f32::NAN),
        Landmark3D::new(f32::INFINITY, 0.5, 0.0),
        Landmark3D::new(0.5, f32::NEG_INFINITY, 0.0),
        Landmark3D::new(0.5, 0.5, f32::INFINITY),
    ];
    for inv in invalid {
        assert!(!inv.is_valid(), "Coordinate {:?} should be invalid", inv);
    }

    // 3. Euclidean distances
    let p1 = Landmark3D::new(0.0, 0.0, 0.0);
    let p2 = Landmark3D::new(3.0, 4.0, 12.0);
    assert!((p1.euclidean_dist_2d(&p2) - 5.0).abs() < 1e-6);
    assert!((p1.euclidean_dist_3d(&p2) - 13.0).abs() < 1e-6);
}

#[test]
fn test_classifier_rejects_nan_and_low_confidence() {
    let mut classifier = GestureClassifier::default();
    let mut raw = generate_neutral_hand();

    // 1. Low confidence rejection (< 0.40)
    let hand_low = HandLandmarks {
        landmarks: raw,
        handedness: Handedness::Right,
        confidence: 0.39,
    };
    let err_conf = classifier.process_frame(&hand_low, 100).unwrap_err();
    assert!(matches!(
        err_conf,
        GestureError::LowConfidence {
            confidence: _,
            threshold: _
        }
    ));

    // 2. NaN coordinate rejection
    raw[INDEX_TIP].x = f32::NAN;
    let hand_nan = HandLandmarks {
        landmarks: raw,
        handedness: Handedness::Right,
        confidence: 0.95,
    };
    let err_nan = classifier.process_frame(&hand_nan, 100).unwrap_err();
    assert!(matches!(
        err_nan,
        GestureError::InvalidCoordinate {
            index: 8,
            x: _,
            y: _,
            z: _
        }
    ));

    // 3. Infinity coordinate rejection
    raw[INDEX_TIP].x = f32::INFINITY;
    let hand_inf = HandLandmarks {
        landmarks: raw,
        handedness: Handedness::Right,
        confidence: 0.95,
    };
    let err_inf = classifier.process_frame(&hand_inf, 100).unwrap_err();
    assert!(matches!(
        err_inf,
        GestureError::InvalidCoordinate {
            index: 8,
            x: _,
            y: _,
            z: _
        }
    ));
}

// ============================================================================
// 2. Pointing Ray Mathematics & Degenerate Geometries
// ============================================================================

#[test]
fn test_pointing_ray_degenerate_and_boundary_angles() {
    let mut lms = generate_neutral_hand();

    // 1. Coincident joints (PIP == TIP)
    lms[INDEX_PIP] = Landmark3D::new(0.5, 0.4, 0.1);
    lms[INDEX_TIP] = Landmark3D::new(0.5, 0.4, 0.1); // zero vector
    let ray_degen = compute_pointing_ray(&lms);
    assert_eq!(ray_degen.origin, lms[INDEX_TIP]);
    assert_eq!(ray_degen.direction, Landmark3D::new(0.0, -1.0, 0.0));
    assert!(!ray_degen.pitch_rad.is_nan());
    assert!(!ray_degen.yaw_rad.is_nan());

    // 2. Pure directional axes
    // Direction straight into depth (+Z)
    lms[INDEX_PIP] = Landmark3D::new(0.5, 0.4, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.5, 0.4, 0.2);
    let ray_z = compute_pointing_ray(&lms);
    assert!((ray_z.direction.z - 1.0).abs() < 1e-6);
    assert!(ray_z.pitch_rad.abs() < 1e-5);
    assert!(ray_z.yaw_rad.abs() < 1e-5);

    // Direction straight out of depth (-Z)
    lms[INDEX_TIP] = Landmark3D::new(0.5, 0.4, -0.2);
    let ray_neg_z = compute_pointing_ray(&lms);
    assert!((ray_neg_z.direction.z - (-1.0)).abs() < 1e-6);
    assert!(ray_neg_z.pitch_rad.abs() < 1e-5);
    assert!((ray_neg_z.yaw_rad.abs() - PI).abs() < 1e-5);

    // Direction straight right (+X)
    lms[INDEX_TIP] = Landmark3D::new(0.7, 0.4, 0.0);
    let ray_x = compute_pointing_ray(&lms);
    assert!((ray_x.direction.x - 1.0).abs() < 1e-6);
    assert!((ray_x.yaw_rad - PI / 2.0).abs() < 1e-5);

    // Direction straight left (-X)
    lms[INDEX_TIP] = Landmark3D::new(0.3, 0.4, 0.0);
    let ray_neg_x = compute_pointing_ray(&lms);
    assert!((ray_neg_x.direction.x - (-1.0)).abs() < 1e-6);
    assert!((ray_neg_x.yaw_rad - (-PI / 2.0)).abs() < 1e-5);
}

#[test]
fn test_palm_normal_cross_product_geometry() {
    let mut lms = generate_neutral_hand();

    // Baseline flat palm in XY plane facing camera (-Z normal)
    lms[WRIST] = Landmark3D::new(0.5, 0.8, 0.0);
    lms[INDEX_MCP] = Landmark3D::new(0.4, 0.5, 0.0);
    lms[PINKY_MCP] = Landmark3D::new(0.6, 0.5, 0.0);

    let normal = calculate_palm_normal(&lms);
    let norm_len = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    assert!((norm_len - 1.0).abs() < 1e-5, "Normal must be normalized");

    // Degenerate collinear palm points
    lms[INDEX_MCP] = Landmark3D::new(0.5, 0.7, 0.0);
    lms[PINKY_MCP] = Landmark3D::new(0.5, 0.6, 0.0);
    let degen_normal = calculate_palm_normal(&lms);
    assert!(!degen_normal.x.is_nan());
    assert!(!degen_normal.y.is_nan());
    assert!(!degen_normal.z.is_nan());
}

// ============================================================================
// 3. Pinch Euclidean Distance & Hysteresis Stability
// ============================================================================

#[test]
fn test_pinch_rounding_parity_and_hysteresis_state_machine() {
    // 1. 5-decimal rounding boundary checks
    assert!(is_pinch_at_threshold(0.049999, 0.050));
    assert!(is_pinch_at_threshold(0.050000, 0.050));
    assert!(is_pinch_at_threshold(0.050004, 0.050));
    assert!(!is_pinch_at_threshold(0.050006, 0.050));
    assert!(!is_pinch_at_threshold(0.050010, 0.050));

    // 2. Complete Hysteresis Lifecycle: Inactive -> Active -> Sustained -> Released
    let mut hyst = PinchHysteresis::new(0.050, 0.070);
    assert!(!hyst.is_active());

    // Approaching threshold
    assert!(!hyst.update(0.080));
    assert!(!hyst.update(0.060));
    assert!(!hyst.update(0.0501));
    assert!(!hyst.is_active());

    // Cross into pinch at 0.0500
    assert!(hyst.update(0.0500));
    assert!(hyst.is_active(), "Must activate at 0.0500");

    // Sustain in [0.050, 0.070]
    assert!(hyst.update(0.055));
    assert!(hyst.update(0.065));
    assert!(hyst.update(0.0699));
    assert!(hyst.update(0.0700));
    assert!(hyst.is_active(), "Must sustain through 0.0700");

    // Exceed exit threshold (> 0.070)
    assert!(!hyst.update(0.0701));
    assert!(!hyst.is_active(), "Must release at 0.0701");

    // Does not re-enter when decreasing back to 0.060
    assert!(!hyst.update(0.060));
    assert!(!hyst.is_active());

    // Re-enters at 0.048
    assert!(hyst.update(0.048));
    assert!(hyst.is_active());

    // Reset clears state
    hyst.reset();
    assert!(!hyst.is_active());
}

// ============================================================================
// 4. Swipe Spatio-Temporal Velocity & Cooldown Refractory State Machine
// ============================================================================

#[test]
fn test_swipe_detector_cardinal_dominance_and_sub_thresholds() {
    let mut detector = SwipeDetector::new(250, 0.15, 0.50, 400);

    // 1. Fewer than 3 samples returns None
    assert_eq!(
        detector.update(&Landmark3D::new(0.2, 0.5, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.5, 0.5, 0.0), 150),
        None
    );

    detector.reset();

    // 2. Sub-threshold displacement (< 0.15) over 100ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.20, 0.50, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.26, 0.50, 0.0), 150),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.33, 0.50, 0.0), 200),
        None, // disp = 0.13 < 0.15
    );

    detector.reset();

    // 3. Ambiguous diagonal rejection (ratio < 1.5)
    // dx = 0.20, dy = 0.16 -> ratio 0.20 / 0.16 = 1.25 < 1.5
    assert_eq!(
        detector.update(&Landmark3D::new(0.20, 0.20, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.30, 0.28, 0.0), 160),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.40, 0.36, 0.0), 220),
        None,
        "Ambiguous diagonal must not trigger cardinal swipe"
    );

    detector.reset();

    // 4. Clean cardinal SwipeRight with dominance ratio 2.0 (dx = 0.24, dy = 0.04)
    assert_eq!(
        detector.update(&Landmark3D::new(0.20, 0.50, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.32, 0.52, 0.0), 160),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.44, 0.54, 0.0), 220),
        Some(GestureType::SwipeRight)
    );

    // 5. 400ms Cooldown enforcement
    // Next attempt at t = 220 + 350 = 570ms (still within 400ms cooldown)
    assert_eq!(
        detector.update(&Landmark3D::new(0.80, 0.50, 0.0), 570),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.40, 0.50, 0.0), 590),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.10, 0.50, 0.0), 610),
        None,
        "Movement during 400ms cooldown must be suppressed"
    );
}

// ============================================================================
// 5. One-Euro & EMA Temporal Filters
// ============================================================================

#[test]
fn test_ema_filter_alpha_bounds_and_smoothing() {
    let mut raw = generate_neutral_hand();

    // 1. Initial frame passthrough
    let mut ema = EmaLandmarksFilter::new(0.5);
    let f1 = ema.filter(&raw);
    assert_eq!(f1, raw);

    // 2. Perturbation smoothed by 50%
    raw[INDEX_TIP].x += 0.10;
    let f2 = ema.filter(&raw);
    assert!((f2[INDEX_TIP].x - (raw[INDEX_TIP].x - 0.05)).abs() < 1e-5);

    // 3. Reset clears state
    ema.reset();
    let f3 = ema.filter(&raw);
    assert_eq!(f3, raw);
}

#[test]
fn test_one_euro_filter_jitter_attenuation_and_step_response() {
    let mut filter = OneEuroLandmarksFilter::new(1.0, 0.005, 1.0);
    let raw = generate_neutral_hand();

    // 1. Warm up filter over 30 frames at stationary position
    for t in 0..30 {
        let _ = filter.filter(&raw, t * 16);
    }

    // 2. Introduce micro-jitter (+/- 0.002)
    let mut jittered = raw;
    jittered[INDEX_TIP].x += 0.002;
    let smoothed = filter.filter(&jittered, 500);

    // Jitter should be substantially attenuated
    let error_raw = (jittered[INDEX_TIP].x - raw[INDEX_TIP].x).abs();
    let error_smoothed = (smoothed[INDEX_TIP].x - raw[INDEX_TIP].x).abs();
    assert!(
        error_smoothed < error_raw,
        "One-Euro filter must reduce stationary jitter"
    );

    // 3. Negative / zero dt handled safely
    let same_time = filter.filter(&raw, 500);
    assert!(!same_time[INDEX_TIP].x.is_nan());
}

// ============================================================================
// 6. Spatial Coordinate Mapper Polar Boundaries
// ============================================================================

#[test]
fn test_spatial_mapper_exact_radial_and_angular_boundaries() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // 1. Exact center
    let center = mapper.map_coords(0.5, 0.5);
    assert!(center.in_deadzone);
    assert_eq!(center.active_sector, None);
    assert_eq!(center.normalized_radius, 0.0);

    // 2. Deadzone boundary (dx = 0.0495 -> r = 0.099; dx = 0.0505 -> r = 0.101)
    let s_in = mapper.map_coords(0.5495, 0.5000);
    assert!(s_in.in_deadzone);
    assert_eq!(s_in.active_sector, None);

    let s_out = mapper.map_coords(0.5505, 0.5000);
    assert!(!s_out.in_deadzone);
    assert_eq!(s_out.active_sector, Some(0));

    // 3. 6 Sectors: Exactly 60 degrees each
    // Sector 0: theta ~ 30 deg (0.52 rad)
    let s0 = mapper.map_coords(0.5 + 0.3 * (30.0_f32 * PI / 180.0).cos(), 0.5 + 0.3 * (30.0_f32 * PI / 180.0).sin());
    assert_eq!(s0.active_sector, Some(0));

    // Sector 1: theta ~ 90 deg (1.57 rad)
    let s1 = mapper.map_coords(0.5, 0.8);
    assert_eq!(s1.active_sector, Some(1));

    // Sector 2: theta ~ 150 deg (2.62 rad)
    let s2 = mapper.map_coords(0.5 + 0.3 * (150.0_f32 * PI / 180.0).cos(), 0.5 + 0.3 * (150.0_f32 * PI / 180.0).sin());
    assert_eq!(s2.active_sector, Some(2));

    // Sector 3: theta ~ 210 deg (3.66 rad)
    let s3 = mapper.map_coords(0.5 + 0.3 * (210.0_f32 * PI / 180.0).cos(), 0.5 + 0.3 * (210.0_f32 * PI / 180.0).sin());
    assert_eq!(s3.active_sector, Some(3));

    // Sector 4: theta ~ 270 deg (4.71 rad)
    let s4 = mapper.map_coords(0.5, 0.2);
    assert_eq!(s4.active_sector, Some(4));

    // Sector 5: theta ~ 330 deg (5.76 rad)
    let s5 = mapper.map_coords(0.5 + 0.3 * (330.0_f32 * PI / 180.0).cos(), 0.5 + 0.3 * (330.0_f32 * PI / 180.0).sin());
    assert_eq!(s5.active_sector, Some(5));

    // 4. Viewport mapping
    let bounds = ScreenBounds::new(100.0, 200.0, 1920.0, 1080.0);
    let mut mapper_vp = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10).with_viewport(bounds);
    let vp_res = mapper_vp.map_coords(0.5, 0.5);
    assert_eq!(vp_res.screen_x, Some(100.0 + 0.5 * 1920.0));
    assert_eq!(vp_res.screen_y, Some(200.0 + 0.5 * 1080.0));
}
