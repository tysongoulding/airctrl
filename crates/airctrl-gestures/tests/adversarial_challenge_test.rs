//! Adversarial Challenge Test Suite for Milestone M4 Gesture Classification
//!
//! Empirical validation of:
//! 1. Pinch boundary attacks & hysteresis (fuzzing at d=0.049, d=0.050, d=0.051; sustain/release).
//! 2. Pointing ray & finger curl fuzzing (borderlines, degenerate vectors, pitch/yaw, rock-on/gun poses).
//! 3. Swipe dynamic gestures (sub-threshold disp/vel, 4 directions, 400ms refractory cooldown).

use airctrl_core::types::gesture::{GestureType, HandLandmarks, Handedness, Landmark3D};
use airctrl_gestures::classifier::heuristics::{
    compute_pointing_ray, is_pinch_at_threshold, pinch_distance, PinchHysteresis,
};
use airctrl_gestures::classifier::{
    FilterType, GestureClassifier, GestureClassifierConfig, SwipeDetector,
};
use airctrl_gestures::topology::*;
use std::f32::consts::PI;

// ============================================================================
// Helper Utilities for Adversarial Landmark Synthesis
// ============================================================================

fn make_neutral_landmarks() -> [Landmark3D; 21] {
    let mut lms = [Landmark3D::new(0.5, 0.5, 0.0); 21];
    lms[WRIST] = Landmark3D::new(0.5, 0.8, 0.0);
    lms[INDEX_MCP] = Landmark3D::new(0.45, 0.55, 0.0);
    lms[INDEX_PIP] = Landmark3D::new(0.45, 0.45, 0.0);
    lms[INDEX_DIP] = Landmark3D::new(0.45, 0.40, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.45, 0.38, 0.0);

    lms[MIDDLE_MCP] = Landmark3D::new(0.50, 0.55, 0.0);
    lms[MIDDLE_PIP] = Landmark3D::new(0.50, 0.48, 0.0);
    lms[MIDDLE_DIP] = Landmark3D::new(0.50, 0.45, 0.0);
    lms[MIDDLE_TIP] = Landmark3D::new(0.50, 0.42, 0.0);

    lms[RING_MCP] = Landmark3D::new(0.55, 0.56, 0.0);
    lms[RING_PIP] = Landmark3D::new(0.55, 0.50, 0.0);
    lms[RING_DIP] = Landmark3D::new(0.55, 0.46, 0.0);
    lms[RING_TIP] = Landmark3D::new(0.55, 0.43, 0.0);

    lms[PINKY_MCP] = Landmark3D::new(0.60, 0.58, 0.0);
    lms[PINKY_PIP] = Landmark3D::new(0.60, 0.52, 0.0);
    lms[PINKY_DIP] = Landmark3D::new(0.60, 0.48, 0.0);
    lms[PINKY_TIP] = Landmark3D::new(0.60, 0.45, 0.0);

    lms[THUMB_CMC] = Landmark3D::new(0.40, 0.70, 0.0);
    lms[THUMB_MCP] = Landmark3D::new(0.35, 0.62, 0.0);
    lms[THUMB_IP] = Landmark3D::new(0.32, 0.55, 0.0);
    lms[THUMB_TIP] = Landmark3D::new(0.30, 0.50, 0.0);

    lms
}

// ============================================================================
// 1. PINCH BOUNDARY ATTACKS & HYSTERESIS
// ============================================================================

#[test]
fn test_pinch_boundary_3d_euclidean_fuzzing() {
    let p_thumb = Landmark3D::new(0.50, 0.50, 0.50);

    // 1. Axis-aligned boundary tests
    let p_index_under = Landmark3D::new(0.50 + 0.049, 0.50, 0.50);
    let p_index_exact = Landmark3D::new(0.50 + 0.050, 0.50, 0.50);
    let p_index_over = Landmark3D::new(0.50 + 0.051, 0.50, 0.50);

    let d_under = pinch_distance(&p_thumb, &p_index_under);
    let d_exact = pinch_distance(&p_thumb, &p_index_exact);
    let d_over = pinch_distance(&p_thumb, &p_index_over);

    assert!((d_under - 0.049).abs() < 1e-6);
    assert!((d_exact - 0.050).abs() < 1e-6);
    assert!((d_over - 0.051).abs() < 1e-6);

    assert!(is_pinch_at_threshold(d_under, 0.050));
    assert!(is_pinch_at_threshold(d_exact, 0.050));
    assert!(!is_pinch_at_threshold(d_over, 0.050));

    // 2. 3D Diagonal vector: dx = dy = dz = d / sqrt(3)
    let diag_comp_under = 0.049 / (3.0f32).sqrt();
    let diag_comp_exact = 0.050 / (3.0f32).sqrt();
    let diag_comp_over = 0.051 / (3.0f32).sqrt();

    let p_diag_under = Landmark3D::new(
        0.50 + diag_comp_under,
        0.50 + diag_comp_under,
        0.50 + diag_comp_under,
    );
    let p_diag_exact = Landmark3D::new(
        0.50 + diag_comp_exact,
        0.50 + diag_comp_exact,
        0.50 + diag_comp_exact,
    );
    let p_diag_over = Landmark3D::new(
        0.50 + diag_comp_over,
        0.50 + diag_comp_over,
        0.50 + diag_comp_over,
    );

    let d_diag_under = pinch_distance(&p_thumb, &p_diag_under);
    let d_diag_exact = pinch_distance(&p_thumb, &p_diag_exact);
    let d_diag_over = pinch_distance(&p_thumb, &p_diag_over);

    assert!((d_diag_under - 0.049).abs() < 1e-5);
    assert!((d_diag_exact - 0.050).abs() < 1e-5);
    assert!((d_diag_over - 0.051).abs() < 1e-5);

    assert!(is_pinch_at_threshold(d_diag_under, 0.050));
    assert!(is_pinch_at_threshold(d_diag_exact, 0.050));
    assert!(!is_pinch_at_threshold(d_diag_over, 0.050));

    // 3. Spherical angles fuzzing: 360 orientations around unit sphere
    for angle_deg in (0..360).step_by(10) {
        let theta = (angle_deg as f32) * PI / 180.0;
        for phi_deg in (10..170).step_by(20) {
            let phi = (phi_deg as f32) * PI / 180.0;

            let dir_x = phi.sin() * theta.cos();
            let dir_y = phi.sin() * theta.sin();
            let dir_z = phi.cos();

            // At r = 0.049
            let p_u = Landmark3D::new(
                p_thumb.x + dir_x * 0.049,
                p_thumb.y + dir_y * 0.049,
                p_thumb.z + dir_z * 0.049,
            );
            assert!(
                is_pinch_at_threshold(pinch_distance(&p_thumb, &p_u), 0.050),
                "Fuzzing failed at angle ({theta}, {phi}) for d=0.049"
            );

            // At r = 0.050
            let p_e = Landmark3D::new(
                p_thumb.x + dir_x * 0.050,
                p_thumb.y + dir_y * 0.050,
                p_thumb.z + dir_z * 0.050,
            );
            assert!(
                is_pinch_at_threshold(pinch_distance(&p_thumb, &p_e), 0.050),
                "Fuzzing failed at angle ({theta}, {phi}) for d=0.050"
            );

            // At r = 0.051
            let p_o = Landmark3D::new(
                p_thumb.x + dir_x * 0.051,
                p_thumb.y + dir_y * 0.051,
                p_thumb.z + dir_z * 0.051,
            );
            assert!(
                !is_pinch_at_threshold(pinch_distance(&p_thumb, &p_o), 0.050),
                "Fuzzing failed at angle ({theta}, {phi}) for d=0.051"
            );
        }
    }
}

#[test]
fn test_pinch_hysteresis_state_machine_and_flutter() {
    let mut hyst = PinchHysteresis::new(0.050, 0.070);
    assert!(!hyst.is_active());

    // 1. Inactive outside threshold
    assert!(!hyst.update(0.080));
    assert!(!hyst.is_active());

    assert!(!hyst.update(0.055));
    assert!(!hyst.is_active());

    assert!(!hyst.update(0.0501));
    assert!(!hyst.is_active());

    // 2. Enter pinch at <= 0.050
    assert!(hyst.update(0.0500));
    assert!(hyst.is_active(), "Should activate at exact 0.050 boundary");

    // 3. Sustain in hysteresis band [0.050, 0.070]
    assert!(hyst.update(0.051));
    assert!(hyst.is_active());

    assert!(hyst.update(0.060));
    assert!(hyst.is_active());

    assert!(hyst.update(0.0699));
    assert!(hyst.is_active(), "Should sustain at 0.0699");

    // Empirical verification of exact exit boundary (0.070 vs >0.070)
    // Code has: if dist > self.exit_threshold { is_pinching = false; }
    // At exactly 0.07000, 0.07000 > 0.070 is false -> sustains.
    assert!(hyst.update(0.0700));
    assert!(
        hyst.is_active(),
        "At exact exit boundary 0.070, hysteresis sustains until strictly exceeded"
    );

    // 4. Release when exceeding exit threshold (> 0.070)
    assert!(
        !hyst.update(0.0701),
        "Must release when exceeding exit threshold"
    );
    assert!(!hyst.is_active());

    assert!(!hyst.update(0.075));
    assert!(!hyst.is_active());

    // 5. Does not re-enter in intermediate band
    assert!(!hyst.update(0.060));
    assert!(!hyst.is_active());

    // 6. Rapid boundary flutter resistance across enter boundary (0.049 <-> 0.051)
    assert!(hyst.update(0.049)); // enters
    assert!(hyst.is_active());
    for _ in 0..100 {
        assert!(
            hyst.update(0.051),
            "Must NOT drop pinch during jitter across enter threshold"
        );
        assert!(hyst.update(0.049));
    }

    // 7. Reset clears active state
    hyst.reset();
    assert!(!hyst.is_active());
}

// ============================================================================
// 2. POINTING RAY & FINGER CURL FUZZING
// ============================================================================

#[test]
fn test_pointing_heuristics_finger_curl_boundaries() {
    let mut lms = make_neutral_landmarks();

    // Pointing requirement: Index tip (#8) extended (y < 0.35)
    // while middle (#12) and ring (#16) curled (y > 0.55)

    // Baseline Pointing pose
    lms[INDEX_TIP] = Landmark3D::new(0.45, 0.30, 0.0);
    lms[MIDDLE_TIP] = Landmark3D::new(0.50, 0.60, 0.0);
    lms[RING_TIP] = Landmark3D::new(0.55, 0.60, 0.0);
    assert_eq!(GestureClassifier::classify(&lms), GestureType::Pointing);

    // 1. Index tip boundary attacks (0.349 vs 0.350 vs 0.351)
    lms[INDEX_TIP].y = 0.349;
    assert_eq!(GestureClassifier::classify(&lms), GestureType::Pointing);

    lms[INDEX_TIP].y = 0.350;
    assert_ne!(
        GestureClassifier::classify(&lms),
        GestureType::Pointing,
        "y=0.350 must not trigger Pointing (< 0.35 is strict)"
    );

    lms[INDEX_TIP].y = 0.351;
    assert_ne!(GestureClassifier::classify(&lms), GestureType::Pointing);

    // Reset index tip
    lms[INDEX_TIP].y = 0.30;

    // 2. Middle tip curl boundary attacks (0.549 vs 0.550 vs 0.551)
    lms[MIDDLE_TIP].y = 0.551;
    assert_eq!(GestureClassifier::classify(&lms), GestureType::Pointing);

    lms[MIDDLE_TIP].y = 0.550;
    assert_ne!(
        GestureClassifier::classify(&lms),
        GestureType::Pointing,
        "Middle y=0.550 must not trigger Pointing (> 0.55 is strict)"
    );

    lms[MIDDLE_TIP].y = 0.549;
    assert_ne!(GestureClassifier::classify(&lms), GestureType::Pointing);

    // Reset middle tip
    lms[MIDDLE_TIP].y = 0.60;

    // 3. Ring tip curl boundary attacks (0.549 vs 0.550 vs 0.551)
    lms[RING_TIP].y = 0.551;
    assert_eq!(GestureClassifier::classify(&lms), GestureType::Pointing);

    lms[RING_TIP].y = 0.550;
    assert_ne!(
        GestureClassifier::classify(&lms),
        GestureType::Pointing,
        "Ring y=0.550 must not trigger Pointing (> 0.55 is strict)"
    );

    lms[RING_TIP].y = 0.549;
    assert_ne!(GestureClassifier::classify(&lms), GestureType::Pointing);

    // 4. "Rock-on" gesture attack: pinky extended with index extended
    lms[RING_TIP].y = 0.60;
    lms[PINKY_TIP].y = 0.25; // Pinky fully extended!
    let rock_on_gesture = GestureClassifier::classify(&lms);
    // Because classify_heuristics only checks index, middle, and ring,
    // rock-on is evaluated as Pointing! This matches the Python test oracle.
    assert_eq!(
        rock_on_gesture,
        GestureType::Pointing,
        "Oracle contract: pinky is unconstrained in Pointing heuristic"
    );

    // 5. OpenPalm priority attack: if all fingers extended (< 0.45), OpenPalm takes precedence
    lms[MIDDLE_TIP].y = 0.30;
    lms[RING_TIP].y = 0.30;
    lms[THUMB_TIP].y = 0.30;
    assert_eq!(
        GestureClassifier::classify(&lms),
        GestureType::OpenPalm,
        "OpenPalm must take precedence over Pointing when all fingers extended"
    );

    // 6. Pinch priority attack: if thumb and index are close, Pinch takes precedence over Pointing
    lms[THUMB_TIP] = lms[INDEX_TIP]; // dist = 0.0
    lms[MIDDLE_TIP].y = 0.60;
    lms[RING_TIP].y = 0.60;
    assert_eq!(
        GestureClassifier::classify(&lms),
        GestureType::Pinch,
        "Pinch must take precedence over Pointing when thumb and index tips are together"
    );
}

#[test]
fn test_pointing_ray_direction_vectors_and_angles() {
    let mut lms = make_neutral_landmarks();

    // 1. Ray pointing straight UP (-Y direction in screen coordinates)
    lms[INDEX_PIP] = Landmark3D::new(0.50, 0.40, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.50, 0.20, 0.0);

    let ray_up = compute_pointing_ray(&lms);
    assert_eq!(ray_up.origin, lms[INDEX_TIP]);
    assert!((ray_up.direction.x).abs() < 1e-6);
    assert!((ray_up.direction.y - (-1.0)).abs() < 1e-6);
    assert!((ray_up.direction.z).abs() < 1e-6);

    let norm_up = (ray_up.direction.x.powi(2)
        + ray_up.direction.y.powi(2)
        + ray_up.direction.z.powi(2))
    .sqrt();
    assert!((norm_up - 1.0).abs() < 1e-6, "Unit vector must have norm 1.0");

    assert!(
        (ray_up.pitch_rad - PI / 2.0).abs() < 1e-5,
        "Pointing UP pitch should be +PI/2 rad (+90 deg), got {}",
        ray_up.pitch_rad
    );
    assert!(
        ray_up.yaw_rad.abs() < 1e-5,
        "Pointing UP yaw should be 0.0 rad, got {}",
        ray_up.yaw_rad
    );

    // 2. Ray pointing straight DOWN (+Y direction)
    lms[INDEX_PIP] = Landmark3D::new(0.50, 0.20, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.50, 0.40, 0.0);

    let ray_down = compute_pointing_ray(&lms);
    assert!((ray_down.direction.y - 1.0).abs() < 1e-6);
    assert!(
        (ray_down.pitch_rad - (-PI / 2.0)).abs() < 1e-5,
        "Pointing DOWN pitch should be -PI/2 rad (-90 deg), got {}",
        ray_down.pitch_rad
    );

    // 3. Ray pointing RIGHT (+X direction)
    lms[INDEX_PIP] = Landmark3D::new(0.40, 0.30, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.60, 0.30, 0.0);

    let ray_right = compute_pointing_ray(&lms);
    assert!((ray_right.direction.x - 1.0).abs() < 1e-6);
    assert!(ray_right.pitch_rad.abs() < 1e-5);
    assert!(
        (ray_right.yaw_rad - PI / 2.0).abs() < 1e-5,
        "Pointing RIGHT yaw should be +PI/2 rad, got {}",
        ray_right.yaw_rad
    );

    // 4. Ray pointing LEFT (-X direction)
    lms[INDEX_PIP] = Landmark3D::new(0.60, 0.30, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.40, 0.30, 0.0);

    let ray_left = compute_pointing_ray(&lms);
    assert!((ray_left.direction.x - (-1.0)).abs() < 1e-6);
    assert!(ray_left.pitch_rad.abs() < 1e-5);
    assert!(
        (ray_left.yaw_rad - (-PI / 2.0)).abs() < 1e-5,
        "Pointing LEFT yaw should be -PI/2 rad, got {}",
        ray_left.yaw_rad
    );

    // 5. Ray pointing FORWARD into depth (+Z direction)
    lms[INDEX_PIP] = Landmark3D::new(0.50, 0.30, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.50, 0.30, 0.20);

    let ray_forward = compute_pointing_ray(&lms);
    assert!((ray_forward.direction.z - 1.0).abs() < 1e-6);
    assert!(ray_forward.pitch_rad.abs() < 1e-5);
    assert!(
        ray_forward.yaw_rad.abs() < 1e-5,
        "Pointing FORWARD yaw should be 0.0 rad, got {}",
        ray_forward.yaw_rad
    );

    // 6. Degenerate index chain (TIP == PIP, distance = 0)
    lms[INDEX_PIP] = Landmark3D::new(0.50, 0.30, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.50, 0.30, 0.0);

    let ray_degen = compute_pointing_ray(&lms);
    assert_eq!(ray_degen.origin, lms[INDEX_TIP]);
    // Degenerate vector safely falls back to (0.0, -1.0, 0.0) without panic
    assert_eq!(ray_degen.direction, Landmark3D::new(0.0, -1.0, 0.0));
    assert!(!ray_degen.pitch_rad.is_nan());
    assert!(!ray_degen.yaw_rad.is_nan());
}

// ============================================================================
// 3. SWIPE DYNAMIC GESTURES & COOLDOWN REFACTORING
// ============================================================================

#[test]
fn test_swipe_sub_threshold_displacement_and_velocity_rejection() {
    let mut detector = SwipeDetector::new(250, 0.15, 0.50, 400);

    // 1. Sub-threshold displacement (< 0.15): displacement = 0.140 in 100ms (velocity = 1.40 >= 0.50)
    assert_eq!(
        detector.update(&Landmark3D::new(0.30, 0.50, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.37, 0.50, 0.0), 150),
        None
    );
    let sub_disp_res = detector.update(&Landmark3D::new(0.44, 0.50, 0.0), 200);
    assert_eq!(
        sub_disp_res, None,
        "Displacement 0.140 < 0.150 must be rejected"
    );

    detector.reset();

    // 2. Sub-threshold velocity (< 0.50): displacement = 0.20 in 450ms (velocity = 0.444 < 0.50)
    // Wait, sliding window is 250ms, so samples older than 250ms get pruned!
    // Let's test displacement = 0.16 in 350ms with samples at 100ms, 250ms, 400ms:
    // Window from 400ms keeps samples back to 150ms -> oldest is at 250ms.
    // Let's do within a 200ms window: displacement = 0.08 in 200ms (v = 0.40)
    assert_eq!(
        detector.update(&Landmark3D::new(0.30, 0.50, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.34, 0.50, 0.0), 200),
        None
    );
    let sub_vel_res = detector.update(&Landmark3D::new(0.38, 0.50, 0.0), 300);
    assert_eq!(
        sub_vel_res, None,
        "Velocity 0.08 / 0.20s = 0.40 < 0.50 must be rejected"
    );

    detector.reset();

    // 3. Diagonal non-dominant axis movement (ratio < 1.5):
    // dx = 0.20, dy = 0.18 over 150ms (both > 0.15, v > 0.50, but neither dominant)
    assert_eq!(
        detector.update(&Landmark3D::new(0.30, 0.30, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.40, 0.39, 0.0), 180),
        None
    );
    let diag_res = detector.update(&Landmark3D::new(0.50, 0.48, 0.0), 250);
    assert_eq!(
        diag_res, None,
        "Diagonal movement with ratio 0.20 / 0.18 = 1.11 < 1.5 must be rejected"
    );
}

#[test]
fn test_fast_valid_swipes_all_cardinal_directions() {
    let mut detector = SwipeDetector::new(250, 0.15, 0.50, 400);

    // 1. Fast SwipeRight: dx = +0.25 in 150ms (v = 1.67)
    assert_eq!(
        detector.update(&Landmark3D::new(0.20, 0.50, 0.0), 100),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.32, 0.50, 0.0), 180),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.45, 0.50, 0.0), 250),
        Some(GestureType::SwipeRight)
    );

    // 2. Fast SwipeLeft (after 400ms cooldown, t >= 650): dx = -0.25 in 150ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.80, 0.50, 0.0), 700),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.68, 0.50, 0.0), 780),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.55, 0.50, 0.0), 850),
        Some(GestureType::SwipeLeft)
    );

    // 3. Fast SwipeUp (after cooldown, t >= 1250): dy = -0.25 in 150ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.80, 0.0), 1300),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.68, 0.0), 1380),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.55, 0.0), 1450),
        Some(GestureType::SwipeUp)
    );

    // 4. Fast SwipeDown (after cooldown, t >= 1850): dy = +0.25 in 150ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.20, 0.0), 1900),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.32, 0.0), 1980),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.45, 0.0), 2050),
        Some(GestureType::SwipeDown)
    );
}

#[test]
fn test_swipe_cooldown_refractory_period_enforcement() {
    let mut detector = SwipeDetector::new(250, 0.15, 0.50, 400);

    // Initial Swipe triggers at t = 200ms
    assert_eq!(
        detector.update(&Landmark3D::new(0.20, 0.50, 0.0), 50),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.32, 0.50, 0.0), 120),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.45, 0.50, 0.0), 200),
        Some(GestureType::SwipeRight)
    );

    // Cooldown window is [200, 600ms)
    // Attack: inject massive fast swipes during refractory period
    for t in (210..600).step_by(30) {
        let res = detector.update(&Landmark3D::new(0.10, 0.50, 0.0), t);
        assert_eq!(
            res, None,
            "Movement at t={t} during 400ms cooldown must return None"
        );
    }

    // Borderline t = 599ms (1ms before cooldown expiration)
    assert_eq!(
        detector.update(&Landmark3D::new(0.90, 0.50, 0.0), 599),
        None,
        "t=599ms is within 400ms cooldown and must be rejected"
    );

    // Now at t = 600ms, cooldown has elapsed (600 >= 200 + 400)
    // History was empty during cooldown, so new swipe needs >= 3 samples
    assert_eq!(
        detector.update(&Landmark3D::new(0.80, 0.50, 0.0), 600),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.65, 0.50, 0.0), 680),
        None
    );
    assert_eq!(
        detector.update(&Landmark3D::new(0.50, 0.50, 0.0), 750),
        Some(GestureType::SwipeLeft),
        "Swipe after 400ms cooldown must trigger successfully"
    );
}

#[test]
fn test_gesture_classifier_full_stateful_pinch_and_swipe_interaction() {
    let mut classifier = GestureClassifier::new(GestureClassifierConfig {
        filter_type: FilterType::None, // disable filter for raw coordinate testing
        swipe_enabled: true,
        pinch_enter_threshold: 0.050,
        pinch_exit_threshold: 0.070,
        swipe_cooldown_ms: 400,
        ..Default::default()
    });

    let mut lms = make_neutral_landmarks();

    // 1. Hand starts neutral
    let hand1 = HandLandmarks {
        landmarks: lms,
        handedness: Handedness::Right,
        confidence: 0.95,
    };
    let r1 = classifier.process_frame(&hand1, 100).unwrap();
    assert_eq!(r1.gesture, GestureType::Neutral);
    assert!(!r1.is_pinching);

    // 2. Hand pinches (thumb and index tips close: d = 0.040)
    lms[THUMB_TIP] = Landmark3D::new(0.50, 0.50, 0.0);
    lms[INDEX_TIP] = Landmark3D::new(0.53, 0.50, 0.0); // d = 0.030
    let hand2 = HandLandmarks {
        landmarks: lms,
        handedness: Handedness::Right,
        confidence: 0.95,
    };
    let r2 = classifier.process_frame(&hand2, 130).unwrap();
    assert_eq!(r2.gesture, GestureType::Pinch);
    assert!(r2.is_pinching);

    // 3. While pinching, hand moves rapidly across screen (simulating dragging)
    // Swipe detector should be suppressed while hand is pinching!
    lms[THUMB_TIP].x += 0.30;
    lms[INDEX_TIP].x += 0.30;
    lms[WRIST].x += 0.30;
    lms[INDEX_MCP].x += 0.30;
    lms[MIDDLE_MCP].x += 0.30;
    lms[PINKY_MCP].x += 0.30;

    let hand3 = HandLandmarks {
        landmarks: lms,
        handedness: Handedness::Right,
        confidence: 0.95,
    };
    let r3 = classifier.process_frame(&hand3, 220).unwrap();
    assert_eq!(
        r3.gesture,
        GestureType::Pinch,
        "Pinch must remain active and suppress accidental swipe during drag"
    );
    assert!(r3.is_pinching);
}

#[test]
fn test_pinch_hysteresis_random_walk_fuzzing() {
    let mut hyst = PinchHysteresis::new(0.050, 0.070);

    // Random walk with 1,000 steps across [0.030, 0.090]
    let mut current_dist = 0.080f32;
    // Simple LCG pseudo-random generator for deterministic, repeatable fuzzing
    let mut rng_state: u64 = 0xDEADBEEFCAFE;
    let mut next_f32 = || -> f32 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((rng_state >> 32) as u32 as f32) / (u32::MAX as f32)
    };

    for step in 0..1000 {
        let delta = (next_f32() - 0.5) * 0.005; // +/- 0.0025 step
        current_dist = (current_dist + delta).clamp(0.030, 0.090);

        let was_pinching = hyst.is_active();
        let is_pinching = hyst.update(current_dist);

        if was_pinching {
            if current_dist > 0.070 {
                assert!(
                    !is_pinching,
                    "Step {step}: dist={current_dist} > 0.070 but failed to release pinch!"
                );
            } else {
                assert!(
                    is_pinching,
                    "Step {step}: dist={current_dist} <= 0.070 but dropped pinch prematurely!"
                );
            }
        } else {
            // Rounded check matches 5-decimal parity
            let rounded = (current_dist * 100_000.0).round() / 100_000.0;
            if rounded <= 0.050 {
                assert!(
                    is_pinching,
                    "Step {step}: dist={current_dist} (rounded {rounded}) <= 0.050 but failed to enter pinch!"
                );
            } else {
                assert!(
                    !is_pinching,
                    "Step {step}: dist={current_dist} (rounded {rounded}) > 0.050 but falsely entered pinch!"
                );
            }
        }
    }
}

#[test]
fn test_pointing_ray_fuzzing_1000_orientations() {
    let mut lms = make_neutral_landmarks();
    let pip = Landmark3D::new(0.50, 0.50, 0.50);
    lms[INDEX_PIP] = pip;

    let mut rng_state: u64 = 0xCAFEBABEDEAD;
    let mut next_f32 = || -> f32 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((rng_state >> 32) as u32 as f32) / (u32::MAX as f32)
    };

    for i in 0..1000 {
        // Generate random vector with length between 0.001 and 0.50
        let rx = (next_f32() - 0.5) * 2.0;
        let ry = (next_f32() - 0.5) * 2.0;
        let rz = (next_f32() - 0.5) * 2.0;
        let r_len = (rx * rx + ry * ry + rz * rz).sqrt();
        if r_len < 1e-4 {
            continue;
        }

        let scale = 0.001 + next_f32() * 0.499;
        let tip = Landmark3D::new(
            pip.x + (rx / r_len) * scale,
            pip.y + (ry / r_len) * scale,
            pip.z + (rz / r_len) * scale,
        );
        lms[INDEX_TIP] = tip;

        let ray = compute_pointing_ray(&lms);

        // 1. Origin must be tip
        assert_eq!(ray.origin, tip);

        // 2. Direction must be normalized unit vector
        let dir_norm = (ray.direction.x.powi(2) + ray.direction.y.powi(2) + ray.direction.z.powi(2)).sqrt();
        assert!(
            (dir_norm - 1.0).abs() < 1e-4,
            "Iteration {i}: dir_norm={dir_norm} is not unit length!"
        );

        // 3. Angles must be valid real numbers (not NaN, not Inf)
        assert!(!ray.pitch_rad.is_nan(), "Iteration {i}: pitch is NaN");
        assert!(!ray.pitch_rad.is_infinite(), "Iteration {i}: pitch is Infinite");
        assert!(!ray.yaw_rad.is_nan(), "Iteration {i}: yaw is NaN");
        assert!(!ray.yaw_rad.is_infinite(), "Iteration {i}: yaw is Infinite");

        // 4. Pitch must be in [-PI/2, PI/2]
        assert!(
            ray.pitch_rad >= -PI / 2.0 - 1e-5 && ray.pitch_rad <= PI / 2.0 + 1e-5,
            "Iteration {i}: pitch {} out of bounds [-pi/2, pi/2]",
            ray.pitch_rad
        );

        // 5. Yaw must be in [-PI, PI]
        assert!(
            ray.yaw_rad >= -PI - 1e-5 && ray.yaw_rad <= PI + 1e-5,
            "Iteration {i}: yaw {} out of bounds [-pi, pi]",
            ray.yaw_rad
        );
    }
}

#[test]
fn test_multi_swipe_sequential_cadence_stress() {
    let mut detector = SwipeDetector::new(250, 0.15, 0.50, 400);

    let directions = [
        GestureType::SwipeRight,
        GestureType::SwipeLeft,
        GestureType::SwipeUp,
        GestureType::SwipeDown,
    ];

    let mut current_t = 100u64;

    for (cycle, &expected_gesture) in directions.iter().cycle().take(20).enumerate() {
        // Complete a fast valid swipe in 150ms
        let (p1, p2, p3) = match expected_gesture {
            GestureType::SwipeRight => (
                Landmark3D::new(0.20, 0.50, 0.0),
                Landmark3D::new(0.35, 0.50, 0.0),
                Landmark3D::new(0.50, 0.50, 0.0),
            ),
            GestureType::SwipeLeft => (
                Landmark3D::new(0.80, 0.50, 0.0),
                Landmark3D::new(0.65, 0.50, 0.0),
                Landmark3D::new(0.50, 0.50, 0.0),
            ),
            GestureType::SwipeUp => (
                Landmark3D::new(0.50, 0.80, 0.0),
                Landmark3D::new(0.50, 0.65, 0.0),
                Landmark3D::new(0.50, 0.50, 0.0),
            ),
            GestureType::SwipeDown => (
                Landmark3D::new(0.50, 0.20, 0.0),
                Landmark3D::new(0.50, 0.35, 0.0),
                Landmark3D::new(0.50, 0.50, 0.0),
            ),
            _ => unreachable!(),
        };

        assert_eq!(detector.update(&p1, current_t), None);
        assert_eq!(detector.update(&p2, current_t + 75), None);
        let trigger_t = current_t + 150;
        let res = detector.update(&p3, trigger_t);
        assert_eq!(
            res,
            Some(expected_gesture),
            "Cycle {cycle}: Expected {expected_gesture:?} at t={trigger_t}"
        );

        // Immediate movement during 400ms cooldown must be suppressed
        let supp_res = detector.update(&p1, trigger_t + 200);
        assert_eq!(supp_res, None, "Cycle {cycle}: suppression failed at trigger_t + 200");

        // Advance to after 400ms cooldown (e.g. +450ms)
        current_t = trigger_t + 450;
    }
}

