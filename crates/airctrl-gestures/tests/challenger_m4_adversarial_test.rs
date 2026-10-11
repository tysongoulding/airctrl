//! challenger_m4_adversarial_test.rs
//!
//! Empirical adversarial test suite for Milestone M4:
//! 1. Spatial Coordinate Mapper polar boundaries:
//!    - Center coordinate (0.5, 0.5): radius r = 0.0 < 0.10 results in active_sector = None.
//!    - Deadband boundary fuzzing: r = 0.099 (None), r = 0.100 and r = 0.101 (valid sector).
//!    - Sector angle boundaries: exact transitions at multiples of pi / 3 (0°, 60°, 120°, 180°, 240°, 300°).
//!    - Angular wrap-around from 2pi - eps to 0 + eps.
//!    - Screen corner coordinates: (0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), verifying radius clamping.
//! 2. Real-time CPU benchmark stress:
//!    - 100+ frames of synthetic video streams across all trajectories.
//!    - Assert throughput satisfies >= 30 FPS (latency <= 33.3ms) and >= 60 FPS (latency <= 16.6ms).
//!    - High-load 1,000-frame CPU saturation test.
//! 3. Concurrency stress:
//!    - Multi-threaded independent mapper invocations (16 threads, 80,000 calls).
//!    - Shared Arc<Mutex<SpatialCoordinateMapper>> contention hammer.
//!    - Multi-threaded Arc<dyn HandLandmarkEngine> inference hammer (16 threads, 8,000 calls).

use airctrl_core::types::gesture::{GestureType, Handedness, Landmark3D};
use airctrl_gestures::benchmark::{
    BenchmarkBudget, BenchmarkRunner, HandPath, SyntheticVideoStream,
};
use airctrl_gestures::engine::{HandLandmarkEngine, MockMediaPipeEngine, MockPose};
use airctrl_gestures::frame::PixelFormat;
use airctrl_gestures::mapper::{MapperConfig, ScreenBounds, SpatialCoordinateMapper};
use airctrl_gestures::synthetic::SyntheticFrameGenerator;
use airctrl_gestures::{
    classify_gesture, create_landmark_engine, create_spatial_mapper, map_pointing_to_radial_dial,
    process_frame_landmarks, INDEX_TIP, TOTAL_LANDMARKS,
};
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::thread;

// ===========================================================================
// 1. Polar Boundaries & Deadband Fuzzing
// ===========================================================================

#[test]
fn test_adversarial_center_coordinate_boundary_and_perturbations() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // 1. Exact Center Coordinate (0.5, 0.5)
    let center_state = mapper.map_coords(0.5, 0.5);
    assert!(
        center_state.in_deadzone,
        "Center (0.5, 0.5) MUST be in deadzone"
    );
    assert_eq!(
        center_state.active_sector, None,
        "Center (0.5, 0.5) MUST have active_sector = None"
    );
    assert_eq!(
        center_state.normalized_radius, 0.0,
        "Center (0.5, 0.5) MUST have normalized_radius = 0.0"
    );
    assert_eq!(
        center_state.angle_rad, 0.0,
        "Center (0.5, 0.5) MUST have angle_rad = 0.0"
    );
    assert!(
        center_state.to_dial_coordinates().is_none(),
        "to_dial_coordinates() MUST return None when in deadzone"
    );

    // 2. Micro-perturbations around center (sub-millimeter jitter)
    let epsilons = [
        (0.500001, 0.500000),
        (0.499999, 0.500000),
        (0.500000, 0.500001),
        (0.500000, 0.499999),
        (0.500010, 0.500010),
        (0.499990, 0.499990),
        (0.510000, 0.500000), // radius = 0.01 / 0.5 = 0.02 < 0.10
        (0.500000, 0.520000), // radius = 0.02 / 0.5 = 0.04 < 0.10
    ];

    for (px, py) in epsilons {
        let state = mapper.map_coords(px, py);
        assert!(
            state.in_deadzone,
            "Perturbation ({}, {}) MUST be inside deadzone",
            px, py
        );
        assert_eq!(
            state.active_sector, None,
            "Perturbation ({}, {}) MUST yield active_sector = None",
            px, py
        );
        assert!(
            state.normalized_radius < 0.10,
            "Normalized radius at ({}, {}) was {}, expected < 0.10",
            px, py, state.normalized_radius
        );
    }
}

#[test]
fn test_adversarial_deadband_boundary_fuzzing() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // max_radius is 0.50, so radius = normalized_radius * 0.50
    // At r = 0.099: dx = 0.099 * 0.50 = 0.0495 -> x = 0.5495
    // At r = 0.100: dx = 0.100 * 0.50 = 0.0500 -> x = 0.5500
    // At r = 0.101: dx = 0.101 * 0.50 = 0.0505 -> x = 0.5505

    // 1. r = 0.099 -> MUST be in deadzone, active_sector = None
    let s_0099 = mapper.map_coords(0.5495, 0.5000);
    assert!(
        s_0099.in_deadzone,
        "r = 0.099 MUST be inside deadzone (was {})",
        s_0099.normalized_radius
    );
    assert_eq!(
        s_0099.active_sector, None,
        "r = 0.099 MUST have active_sector = None"
    );
    assert_eq!(s_0099.normalized_radius, 0.099);

    // 2. r = 0.100 -> MUST NOT be in deadzone, active_sector is valid (Sector 0)
    let s_0100 = mapper.map_coords(0.5500, 0.5000);
    assert!(
        !s_0100.in_deadzone,
        "r = 0.100 MUST activate sector (was {})",
        s_0100.normalized_radius
    );
    assert_eq!(
        s_0100.active_sector,
        Some(0),
        "r = 0.100 at angle 0 MUST be Sector 0"
    );
    assert_eq!(s_0100.normalized_radius, 0.100);

    // 3. r = 0.101 -> MUST NOT be in deadzone, active_sector is valid (Sector 0)
    let s_0101 = mapper.map_coords(0.5505, 0.5000);
    assert!(
        !s_0101.in_deadzone,
        "r = 0.101 MUST activate sector (was {})",
        s_0101.normalized_radius
    );
    assert_eq!(
        s_0101.active_sector,
        Some(0),
        "r = 0.101 at angle 0 MUST be Sector 0"
    );
    assert_eq!(s_0101.normalized_radius, 0.101);

    // 4. Omnidirectional deadband boundary fuzzing across 16 radial spokes
    for step in 0..16 {
        let angle = step as f32 * (2.0 * PI / 16.0);
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        // Test r = 0.099: inside deadzone
        let x_in = 0.5 + (0.099 * 0.5) * cos_a;
        let y_in = 0.5 + (0.099 * 0.5) * sin_a;
        let res_in = mapper.map_coords(x_in, y_in);
        assert!(
            res_in.in_deadzone,
            "Angle {:.3} rad at r=0.099 MUST be inside deadzone",
            angle
        );
        assert_eq!(
            res_in.active_sector, None,
            "Angle {:.3} rad at r=0.099 MUST have active_sector = None",
            angle
        );

        // Test r = 0.101: outside deadzone
        let x_out = 0.5 + (0.101 * 0.5) * cos_a;
        let y_out = 0.5 + (0.101 * 0.5) * sin_a;
        let res_out = mapper.map_coords(x_out, y_out);
        assert!(
            !res_out.in_deadzone,
            "Angle {:.3} rad at r=0.101 MUST be outside deadzone",
            angle
        );
        assert!(
            res_out.active_sector.is_some(),
            "Angle {:.3} rad at r=0.101 MUST activate a sector",
            angle
        );
    }

    // 5. High-density pseudo-random continuous fuzzing (1,000 points)
    for i in 0..1000 {
        // Deterministic pseudo-random sequence
        let pseudo_t = ((i * 7919 + 104729) % 10000) as f32 / 10000.0;
        let pseudo_angle = (((i * 48271) % 6283) as f32) / 1000.0;

        // Inside deadband: r in [0.001, 0.099]
        let r_dead = 0.001 + 0.098 * pseudo_t;
        let x_dead = 0.5 + (r_dead * 0.5) * pseudo_angle.cos();
        let y_dead = 0.5 + (r_dead * 0.5) * pseudo_angle.sin();
        let s_dead = mapper.map_coords(x_dead, y_dead);
        assert!(
            s_dead.in_deadzone,
            "Fuzz point {} at r={:.4} must be in deadzone",
            i, r_dead
        );
        assert_eq!(s_dead.active_sector, None);

        // Outside deadband: r in [0.101, 0.950]
        let r_active = 0.101 + 0.849 * pseudo_t;
        let x_active = 0.5 + (r_active * 0.5) * pseudo_angle.cos();
        let y_active = 0.5 + (r_active * 0.5) * pseudo_angle.sin();
        let s_active = mapper.map_coords(x_active, y_active);
        assert!(
            !s_active.in_deadzone,
            "Fuzz point {} at r={:.4} must NOT be in deadzone",
            i, r_active
        );
        assert!(s_active.active_sector.is_some());
    }
}

// ===========================================================================
// 2. Sector Angle Boundaries & Angular Wrap-Around
// ===========================================================================

#[test]
fn test_adversarial_sector_angle_exact_transitions_at_multiples_of_pi_over_3() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);
    let r = 0.35; // Inside active radius
    let eps = 1e-4_f32;

    // Boundary multiples of pi/3:
    // Sector 0: [0.0, pi/3)       -> [0.0000, 1.0472) rad (0° to 60°)
    // Sector 1: [pi/3, 2pi/3)     -> [1.0472, 2.0944) rad (60° to 120°)
    // Sector 2: [2pi/3, pi)       -> [2.0944, 3.1416) rad (120° to 180°)
    // Sector 3: [pi, 4pi/3)       -> [3.1416, 4.1888) rad (180° to 240°)
    // Sector 4: [4pi/3, 5pi/3)     -> [4.1888, 5.2360) rad (240° to 300°)
    // Sector 5: [5pi/3, 2pi)       -> [5.2360, 6.2832) rad (300° to 360°)

    let sector_boundaries = [
        (0, 0.0 * PI / 3.0),
        (1, 1.0 * PI / 3.0),
        (2, 2.0 * PI / 3.0),
        (3, 3.0 * PI / 3.0),
        (4, 4.0 * PI / 3.0),
        (5, 5.0 * PI / 3.0),
    ];

    for &(target_sector, boundary_rad) in &sector_boundaries {
        // 1. Immediately BEFORE boundary (boundary_rad - eps)
        let angle_before = if boundary_rad == 0.0 {
            2.0 * PI - eps
        } else {
            boundary_rad - eps
        };
        let expected_before = (target_sector + 5) % 6; // Previous sector

        let x_b = 0.5 + r * angle_before.cos();
        let y_b = 0.5 + r * angle_before.sin();
        let res_b = mapper.map_coords(x_b, y_b);
        assert_eq!(
            res_b.active_sector,
            Some(expected_before),
            "Angle {:.5} rad (boundary - eps at boundary #{}) expected sector {}, got {:?}",
            angle_before, target_sector, expected_before, res_b.active_sector
        );

        // 2. EXACTLY AT boundary (boundary_rad)
        let x_at = 0.5 + r * boundary_rad.cos();
        let y_at = 0.5 + r * boundary_rad.sin();
        let res_at = mapper.map_coords(x_at, y_at);
        assert_eq!(
            res_at.active_sector,
            Some(target_sector),
            "Angle {:.5} rad (exact boundary #{}) expected sector {}, got {:?}",
            boundary_rad, target_sector, target_sector, res_at.active_sector
        );

        // 3. Immediately AFTER boundary (boundary_rad + eps)
        let angle_after = boundary_rad + eps;
        let x_a = 0.5 + r * angle_after.cos();
        let y_a = 0.5 + r * angle_after.sin();
        let res_a = mapper.map_coords(x_a, y_a);
        assert_eq!(
            res_a.active_sector,
            Some(target_sector),
            "Angle {:.5} rad (boundary + eps at boundary #{}) expected sector {}, got {:?}",
            angle_after, target_sector, target_sector, res_a.active_sector
        );
    }
}

#[test]
fn test_adversarial_angular_wrap_around_2pi_minus_eps_to_zero_plus_eps() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);
    let r = 0.35;
    let eps = 1e-4_f32;

    // 1. Angle at 2pi - eps (e.g. 6.28308 rad) -> Sector 5
    let angle_near_2pi = 2.0 * PI - eps;
    let x_5 = 0.5 + r * angle_near_2pi.cos();
    let y_5 = 0.5 + r * angle_near_2pi.sin();
    let res_5 = mapper.map_coords(x_5, y_5);
    assert_eq!(
        res_5.active_sector,
        Some(5),
        "Angle 2pi - eps MUST be Sector 5"
    );

    // 2. Angle at 0 + eps (e.g. 0.00010 rad) -> Sector 0
    let angle_near_zero = eps;
    let x_0 = 0.5 + r * angle_near_zero.cos();
    let y_0 = 0.5 + r * angle_near_zero.sin();
    let res_0 = mapper.map_coords(x_0, y_0);
    assert_eq!(
        res_0.active_sector,
        Some(0),
        "Angle 0 + eps MUST be Sector 0"
    );

    // 3. Rapid back-and-forth oscillation across 2pi <-> 0 branch cut
    for cycle in 0..100 {
        // Step into sector 5
        let s5 = mapper.map_coords(0.5 + r * (2.0 * PI - eps).cos(), 0.5 + r * (2.0 * PI - eps).sin());
        assert_eq!(s5.active_sector, Some(5), "Cycle {} failed sector 5", cycle);

        // Step into sector 0
        let s0 = mapper.map_coords(0.5 + r * eps.cos(), 0.5 + r * eps.sin());
        assert_eq!(s0.active_sector, Some(0), "Cycle {} failed sector 0", cycle);
    }

    // 4. Wrap-around with angular hysteresis enabled (0.05 rad)
    let mut mapper_hyst = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10).with_hysteresis(0.05);

    // Initialize firmly inside sector 5 (angle 5.80 rad)
    let _ = mapper_hyst.map_coords(0.5 + r * 5.80_f32.cos(), 0.5 + r * 5.80_f32.sin());

    // Advance just past boundary 0.0 to 0.02 rad: within 0.05 rad hysteresis margin -> MUST retain Sector 5!
    let hyst_retained = mapper_hyst.map_coords(0.5 + r * 0.02_f32.cos(), 0.5 + r * 0.02_f32.sin());
    assert_eq!(
        hyst_retained.active_sector,
        Some(5),
        "Hysteresis MUST retain Sector 5 across 0.0 within margin"
    );

    // Advance far past hysteresis threshold (0.08 rad) -> MUST transition to Sector 0
    let hyst_transitioned = mapper_hyst.map_coords(0.5 + r * 0.08_f32.cos(), 0.5 + r * 0.08_f32.sin());
    assert_eq!(
        hyst_transitioned.active_sector,
        Some(0),
        "Hysteresis MUST transition to Sector 0 past margin"
    );

    // Retreat slightly backwards past boundary (e.g. 2pi - 0.02 rad) -> MUST retain Sector 0!
    let hyst_back = mapper_hyst.map_coords(0.5 + r * (2.0 * PI - 0.02).cos(), 0.5 + r * (2.0 * PI - 0.02).sin());
    assert_eq!(
        hyst_back.active_sector,
        Some(0),
        "Hysteresis MUST retain Sector 0 backwards crossing within margin"
    );
}

// ===========================================================================
// 3. Screen Corner Coordinates & Radius Clamping
// ===========================================================================

#[test]
fn test_adversarial_screen_corner_coordinates_and_radius_clamping() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // Screen corners in normalized [0, 1] coordinate system:
    // Distance from (0.5, 0.5) is sqrt(0.5^2 + 0.5^2) = sqrt(0.5) ~ 0.7071
    // max_radius = 0.50 -> normalized radius = 0.7071 / 0.50 = 1.414 -> CLAMPED TO 1.0

    // 1. Top-Left: (0.0, 0.0) -> dx = -0.5, dy = -0.5 -> theta = 5pi/4 = 225° -> Sector 3
    let tl = mapper.map_coords(0.0, 0.0);
    assert!(!tl.in_deadzone, "Top-Left (0.0, 0.0) must not be in deadzone");
    assert_eq!(tl.normalized_radius, 1.0, "Top-Left radius MUST clamp to 1.0");
    assert_eq!(tl.active_sector, Some(3), "Top-Left (0.0, 0.0) must map to Sector 3");

    // 2. Top-Right: (1.0, 0.0) -> dx = +0.5, dy = -0.5 -> theta = 7pi/4 = 315° -> Sector 5
    let tr = mapper.map_coords(1.0, 0.0);
    assert!(!tr.in_deadzone, "Top-Right (1.0, 0.0) must not be in deadzone");
    assert_eq!(tr.normalized_radius, 1.0, "Top-Right radius MUST clamp to 1.0");
    assert_eq!(tr.active_sector, Some(5), "Top-Right (1.0, 0.0) must map to Sector 5");

    // 3. Bottom-Left: (0.0, 1.0) -> dx = -0.5, dy = +0.5 -> theta = 3pi/4 = 135° -> Sector 2
    let bl = mapper.map_coords(0.0, 1.0);
    assert!(!bl.in_deadzone, "Bottom-Left (0.0, 1.0) must not be in deadzone");
    assert_eq!(bl.normalized_radius, 1.0, "Bottom-Left radius MUST clamp to 1.0");
    assert_eq!(bl.active_sector, Some(2), "Bottom-Left (0.0, 1.0) must map to Sector 2");

    // 4. Bottom-Right: (1.0, 1.0) -> dx = +0.5, dy = +0.5 -> theta = pi/4 = 45° -> Sector 0
    let br = mapper.map_coords(1.0, 1.0);
    assert!(!br.in_deadzone, "Bottom-Right (1.0, 1.0) must not be in deadzone");
    assert_eq!(br.normalized_radius, 1.0, "Bottom-Right radius MUST clamp to 1.0");
    assert_eq!(br.active_sector, Some(0), "Bottom-Right (1.0, 1.0) must map to Sector 0");

    // 5. Far Out-of-Bounds Stress (Negative and Multifold Coordinates)
    let extremes = [
        (-10.0, -10.0),
        (50.0, 50.0),
        (-1000.0, 1000.0),
        (1_000_000.0, -1_000_000.0),
    ];
    for (ex, ey) in extremes {
        let res = mapper.map_coords(ex, ey);
        assert!(!res.in_deadzone);
        assert_eq!(res.normalized_radius, 1.0, "Extreme ({}, {}) MUST clamp radius to 1.0", ex, ey);
        assert!(res.active_sector.is_some(), "Extreme ({}, {}) MUST yield valid sector", ex, ey);
    }
}

// ===========================================================================
// 4. Real-Time CPU Benchmark Stress (100+ Frames & Throughput Budgets)
// ===========================================================================

#[test]
fn test_adversarial_cpu_benchmark_stress_100_plus_frames_all_trajectories() {
    let engine = MockMediaPipeEngine::new();
    let runner_30 = BenchmarkRunner::new(BenchmarkBudget::fps_30()).with_warmup(15);
    let runner_60 = BenchmarkRunner::new(BenchmarkBudget::fps_60()).with_warmup(15);

    let trajectories = [
        (HandPath::CircularOrbit, 120),
        (HandPath::LinearSweep, 150),
        (HandPath::RadialPulse, 180),
        (HandPath::BoundaryFlutter, 200),
        (HandPath::StaticPose, 100),
    ];

    for (path, frame_count) in trajectories {
        // Assert >= 100 frames condition
        assert!(frame_count >= 100);

        let mut stream = SyntheticVideoStream::new(320, 240, 30.0, path, frame_count);
        let report_30 = runner_30.run(&engine, &mut stream).expect("Benchmark run failed");

        // Verification of 30 FPS criteria (latency <= 33.3ms, effective FPS >= 30.0)
        assert!(
            report_30.satisfies_30_fps,
            "Path {:?} failed 30 FPS: mean_latency = {:.3}ms, FPS = {:.1}",
            path, report_30.mean_latency_ms, report_30.effective_fps
        );
        assert!(
            report_30.mean_latency_ms <= 33.33,
            "Path {:?} mean latency {:.3}ms exceeded 33.33ms",
            path, report_30.mean_latency_ms
        );
        assert!(
            report_30.effective_fps >= 30.0,
            "Path {:?} throughput {:.1} FPS below 30.0 FPS",
            path, report_30.effective_fps
        );
        assert_eq!(report_30.total_frames, frame_count);

        // Verification of 60 FPS criteria (latency <= 16.6ms, effective FPS >= 60.0)
        let mut stream_60 = SyntheticVideoStream::new(320, 240, 60.0, path, frame_count);
        let report_60 = runner_60.run(&engine, &mut stream_60).expect("Benchmark run failed");

        assert!(
            report_60.satisfies_60_fps,
            "Path {:?} failed 60 FPS: mean_latency = {:.3}ms, FPS = {:.1}",
            path, report_60.mean_latency_ms, report_60.effective_fps
        );
        assert!(
            report_60.mean_latency_ms <= 16.67,
            "Path {:?} mean latency {:.3}ms exceeded 16.67ms",
            path, report_60.mean_latency_ms
        );
        assert!(
            report_60.effective_fps >= 60.0,
            "Path {:?} throughput {:.1} FPS below 60.0 FPS",
            path, report_60.effective_fps
        );

        // Monotonic distribution ordering
        assert!(report_60.min_latency_ms <= report_60.p50_latency_ms);
        assert!(report_60.p50_latency_ms <= report_60.p90_latency_ms);
        assert!(report_60.p90_latency_ms <= report_60.p95_latency_ms);
        assert!(report_60.p95_latency_ms <= report_60.p99_latency_ms);
        assert!(report_60.p99_latency_ms <= report_60.p99_9_latency_ms);
        assert!(report_60.p99_9_latency_ms <= report_60.max_latency_ms);
    }
}

#[test]
fn test_adversarial_cpu_benchmark_saturation_1000_frames() {
    let engine = MockMediaPipeEngine::new();
    let runner = BenchmarkRunner::new(BenchmarkBudget::fps_60()).with_warmup(20);
    let mut stream = SyntheticVideoStream::new(640, 480, 60.0, HandPath::CircularOrbit, 1000);

    let report = runner.run(&engine, &mut stream).expect("1000-frame benchmark failed");

    assert_eq!(report.total_frames, 1000);
    assert!(
        report.satisfies_60_fps,
        "1000-frame test must satisfy 60 FPS: mean latency = {:.4}ms, FPS = {:.1}",
        report.mean_latency_ms, report.effective_fps
    );
    assert!(
        report.effective_fps >= 500.0,
        "CPU mock engine should achieve massive throughput: got {:.1} FPS",
        report.effective_fps
    );
    assert!(
        report.mean_latency_ms <= 2.0,
        "CPU mock engine mean latency should be < 2.0ms, got {:.4}ms",
        report.mean_latency_ms
    );
}

// ===========================================================================
// 5. Concurrency Stress
// ===========================================================================

#[test]
fn test_adversarial_concurrency_multi_threaded_independent_mappers() {
    let num_threads = 16;
    let iterations_per_thread = 5_000;
    let mut handles = Vec::with_capacity(num_threads);

    for thread_id in 0..num_threads {
        let handle = thread::spawn(move || {
            let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10)
                .with_hysteresis(0.05);

            for iter in 0..iterations_per_thread {
                let angle = (iter as f32 * 0.01) % (2.0 * PI);
                let radius = 0.05 + ((iter % 100) as f32 / 100.0) * 0.45;
                let x = 0.5 + radius * angle.cos();
                let y = 0.5 + radius * angle.sin();

                let state = mapper.map_coords(x, y);

                if state.in_deadzone {
                    assert_eq!(state.active_sector, None);
                } else {
                    let sector = state.active_sector.unwrap();
                    assert!(sector < 6, "Sector index {} must be < 6", sector);
                }
            }
            thread_id
        });
        handles.push(handle);
    }

    for handle in handles {
        let tid = handle.join().expect("Thread crashed");
        assert!(tid < num_threads);
    }
}

#[test]
fn test_adversarial_concurrency_shared_mutex_mapper_hammer() {
    let shared_mapper = Arc::new(Mutex::new(SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10)));
    let num_threads = 16;
    let iterations_per_thread = 2_000;
    let mut handles = Vec::with_capacity(num_threads);

    for _ in 0..num_threads {
        let mapper_clone = Arc::clone(&shared_mapper);
        let handle = thread::spawn(move || {
            for iter in 0..iterations_per_thread {
                let x = 0.1 + ((iter % 80) as f32 / 100.0);
                let y = 0.5;

                let mut mapper = mapper_clone.lock().expect("Poisoned mutex");
                let state = mapper.map_coords(x, y);
                drop(mapper);

                if !state.in_deadzone {
                    assert!(state.active_sector.is_some());
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Worker thread failed under mutex contention");
    }
}

#[test]
fn test_adversarial_concurrency_multi_threaded_engine_inference_hammer() {
    let engine: Arc<dyn HandLandmarkEngine> = Arc::new(MockMediaPipeEngine::new());
    let num_threads = 16;
    let frames_per_thread = 250;
    let mut handles = Vec::with_capacity(num_threads);

    for _ in 0..num_threads {
        let engine_clone = Arc::clone(&engine);
        let handle = thread::spawn(move || {
            let frame = SyntheticFrameGenerator::create_hand_frame(
                160,
                120,
                MockPose::Pointing,
                Handedness::Right,
                PixelFormat::Rgb8,
            );

            for _ in 0..frames_per_thread {
                let result = engine_clone.process_frame(&frame.as_ref()).expect("Inference error");
                let landmarks = result.expect("Expected detected hand landmarks");
                assert_eq!(landmarks.landmarks.len(), TOTAL_LANDMARKS);
                assert!(landmarks.confidence >= 0.90);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Engine inference thread failed");
    }
}

// ===========================================================================
// 6. Pointing Ray, Viewport, and Custom Sector Stress
// ===========================================================================

#[test]
fn test_adversarial_pointing_ray_and_landmark_mapper_stress() {
    let mut mapper = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10);

    // 1. Valid pointing ray projection
    let mcp = Landmark3D::new(0.5, 0.4, 0.0);
    let tip = Landmark3D::new(0.5, 0.25, 0.0); // Points upward (Sector 4)
    let state_ray = mapper.map_pointing_ray(&mcp, &tip);
    assert!(!state_ray.in_deadzone);
    assert_eq!(state_ray.active_sector, Some(4), "Upward ray should be sector 4");

    // 2. Degenerate ray (MCP and Tip at identical coordinates: zero length)
    let ident_mcp = Landmark3D::new(0.85, 0.5, 0.0);
    let ident_tip = Landmark3D::new(0.85, 0.5, 0.0);
    let state_ident = mapper.map_pointing_ray(&ident_mcp, &ident_tip);
    assert!(!state_ident.in_deadzone);
    assert_eq!(
        state_ident.active_sector,
        Some(0),
        "Degenerate ray MUST cleanly fall back to mapping tip position (Sector 0)"
    );

    // 3. High-DPI screen viewport mapping stress
    let viewport = ScreenBounds::new(0.0, 0.0, 3840.0, 2160.0);
    let mut mapper_4k = SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10).with_viewport(viewport);

    let state_4k = mapper_4k.map_coords(0.75, 0.25);
    assert_eq!(state_4k.screen_x, Some(0.75 * 3840.0));
    assert_eq!(state_4k.screen_y, Some(0.25 * 2160.0));

    // Viewport clamping on extreme normalized values
    let state_clamp = mapper_4k.map_coords(1.5, -0.5);
    assert_eq!(state_clamp.screen_x, Some(3840.0));
    assert_eq!(state_clamp.screen_y, Some(0.0));
}

#[test]
fn test_adversarial_custom_sector_configurations_fuzzing() {
    let sector_counts = [1, 4, 8, 12, 16, 24];

    for &count in &sector_counts {
        let mut mapper = SpatialCoordinateMapper::new(count, (0.5, 0.5), 0.10);

        for step in 0..100 {
            let angle = step as f32 * (2.0 * PI / 100.0);
            let x = 0.5 + 0.35 * angle.cos();
            let y = 0.5 + 0.35 * angle.sin();

            let state = mapper.map_coords(x, y);
            assert!(!state.in_deadzone);
            let sector = state.active_sector.expect("Active sector required");
            assert!(
                sector < count,
                "For count {}, sector {} must be < {}",
                count, sector, count
            );
        }
    }
}

#[test]
fn test_adversarial_facade_and_custom_config() {
    // 1. map_pointing_to_radial_dial facade
    let index_tip = Landmark3D::new(0.85, 0.50, -0.1);
    let state = map_pointing_to_radial_dial(&index_tip, 6);
    assert!(!state.in_deadzone);
    assert_eq!(state.active_sector, Some(0));

    // 2. create_spatial_mapper and custom MapperConfig with mirroring
    let cfg = MapperConfig {
        sector_count: 6,
        center_x: 0.5,
        center_y: 0.5,
        deadzone_radius: 0.10,
        max_radius: 0.50,
        hysteresis_rad: 0.0,
        radial_hysteresis: 0.0,
        mirror_x: true,
        sector_offset_rad: 0.0,
        viewport: None,
    };
    let mut mirrored_mapper = SpatialCoordinateMapper::with_config(cfg);
    // In mirrored mode, physical x=0.85 becomes 1.0 - 0.85 = 0.15 (left side -> Sector 3)
    let s_mirrored = mirrored_mapper.map_coords(0.85, 0.50);
    assert_eq!(s_mirrored.active_sector, Some(3));

    // 3. create_landmark_engine & process_frame_landmarks & classify_gesture
    let engine = create_landmark_engine();
    assert!(engine.is_ready());

    let frame = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::Pointing,
        Handedness::Right,
        PixelFormat::Rgb8,
    );
    let landmarks = process_frame_landmarks(&frame).expect("Failed to process frame");
    assert_eq!(landmarks.landmarks.len(), TOTAL_LANDMARKS);
    assert!(landmarks.landmarks[INDEX_TIP].y < 0.35);

    let recognized = classify_gesture(&landmarks);
    assert_eq!(recognized.gesture, GestureType::Pointing);

    let mut default_mapper = create_spatial_mapper();
    assert_eq!(default_mapper.map_coords(0.5, 0.5).active_sector, None);
}
