//! Real-Time CPU Benchmark Test Suite and Public Facade Verification.

use airctrl_core::types::gesture::GestureType;
use airctrl_gestures::benchmark::{
    BenchmarkBudget, BenchmarkRunner, HandPath, SyntheticVideoStream,
};
use airctrl_gestures::engine::MockMediaPipeEngine;
use airctrl_gestures::frame::PixelFormat;
use airctrl_gestures::synthetic::SyntheticFrameGenerator;
use airctrl_gestures::*;

#[test]
fn test_real_time_30_fps_and_60_fps_latency_budget() {
    let engine = MockMediaPipeEngine::new();
    let runner_30 = BenchmarkRunner::new(BenchmarkBudget::fps_30());
    let mut stream_orbit =
        SyntheticVideoStream::new(320, 240, 30.0, HandPath::CircularOrbit, 100);

    let report_30 = runner_30.run(&engine, &mut stream_orbit).unwrap();

    assert!(
        report_30.satisfies_30_fps,
        "Failed 30 FPS budget: mean latency = {} ms, effective FPS = {}",
        report_30.mean_latency_ms, report_30.effective_fps
    );
    assert!(report_30.meets_budget);
    assert_eq!(report_30.total_frames, 100);

    // Verify percentile distribution ordering
    assert!(report_30.min_latency_ms <= report_30.p50_latency_ms);
    assert!(report_30.p50_latency_ms <= report_30.p90_latency_ms);
    assert!(report_30.p90_latency_ms <= report_30.p95_latency_ms);
    assert!(report_30.p95_latency_ms <= report_30.p99_latency_ms);
    assert!(report_30.p99_latency_ms <= report_30.max_latency_ms);

    // 60 FPS budget test
    let runner_60 = BenchmarkRunner::new(BenchmarkBudget::fps_60());
    let mut stream_sweep =
        SyntheticVideoStream::new(320, 240, 60.0, HandPath::LinearSweep, 100);

    let report_60 = runner_60.run(&engine, &mut stream_sweep).unwrap();

    assert!(
        report_60.satisfies_60_fps,
        "Failed 60 FPS budget: mean latency = {} ms, effective FPS = {}",
        report_60.mean_latency_ms, report_60.effective_fps
    );
    assert!(report_60.meets_budget);
}

#[test]
fn test_all_synthetic_stream_trajectory_paths() {
    let engine = MockMediaPipeEngine::new();
    let runner = BenchmarkRunner::new(BenchmarkBudget::fps_30());

    let paths = [
        HandPath::CircularOrbit,
        HandPath::LinearSweep,
        HandPath::RadialPulse,
        HandPath::BoundaryFlutter,
        HandPath::StaticPose,
    ];

    for path in paths {
        let mut stream = SyntheticVideoStream::new(160, 120, 30.0, path, 50);
        let report = runner.run(&engine, &mut stream).unwrap();
        assert_eq!(report.total_frames, 50);
        assert!(report.satisfies_30_fps);
    }
}

#[test]
fn test_lib_public_facade_convenience_functions() {
    // 1. process_frame_landmarks
    let frame = SyntheticFrameGenerator::create_hand_frame(
        160,
        120,
        MockPose::Pointing,
        Handedness::Right,
        PixelFormat::Rgb8,
    );
    let hand_landmarks = process_frame_landmarks(&frame).unwrap();
    assert_eq!(hand_landmarks.landmarks.len(), 21);

    // 2. classify_gesture
    let recognized = classify_gesture(&hand_landmarks);
    assert_eq!(recognized.gesture, GestureType::Pointing);
    assert!(recognized.pointing_ray.is_some());

    // 3. map_pointing_to_radial_dial
    let index_tip = hand_landmarks.landmarks[INDEX_TIP];
    let dial_state = map_pointing_to_radial_dial(&index_tip, 6);
    assert!(!dial_state.in_deadzone);
    assert!(dial_state.active_sector.is_some());

    // 4. Factory functions
    let landmark_engine = create_landmark_engine();
    assert!(landmark_engine.is_ready());

    let mut gesture_classifier = create_gesture_classifier();
    let reg2 = gesture_classifier
        .process_frame(&hand_landmarks, 100)
        .unwrap();
    assert_eq!(reg2.gesture, GestureType::Pointing);

    let mut spatial_mapper = create_spatial_mapper();
    let s_mapped = spatial_mapper.map_coords(0.85, 0.50);
    assert_eq!(s_mapped.active_sector, Some(0));
}
