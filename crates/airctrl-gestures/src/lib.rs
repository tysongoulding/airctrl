//! `airctrl-gestures`
//! CPU-optimized 21-landmark MediaPipe hand tracking, gesture heuristics,
//! and spatial radial dial coordinate mapper for airctrl.

pub mod benchmark;
pub mod classifier;
pub mod engine;
pub mod error;
pub mod frame;
pub mod mapper;
pub mod synthetic;
pub mod topology;

pub use airctrl_core::types::gesture::{
    DialCoordinates, GestureType, HandLandmarks, Handedness, Landmark3D,
};
pub use benchmark::{
    BenchmarkBudget, BenchmarkReport, BenchmarkRunner, HandPath, SyntheticVideoStream,
};
pub use classifier::{
    calculate_palm_center, calculate_palm_normal, classify_heuristics, compute_pointing_ray,
    is_pinch_at_threshold, pinch_distance, EmaLandmarksFilter, FilterType, GestureClassifier,
    GestureClassifierConfig, OneEuroLandmarksFilter, PinchHysteresis, PointingRay,
    RecognizedGesture, SwipeDetector,
};
pub use engine::{HandLandmarkEngine, MockMediaPipeEngine, MockPose};
pub use error::{GestureError, Result};
pub use frame::{FrameBuffer, FrameBufferRef, PixelFormat};
pub use mapper::{
    MapperConfig, RadialPointerState, ScreenBounds, SpatialCoordinateMapper,
};
pub use synthetic::SyntheticFrameGenerator;
pub use topology::{
    LandmarkOps, NormalizedLandmarkList, FINGERTIP_INDICES, INDEX_DIP, INDEX_MCP, INDEX_PIP,
    INDEX_TIP, MCP_INDICES, MIDDLE_DIP, MIDDLE_MCP, MIDDLE_PIP, MIDDLE_TIP, PINKY_DIP,
    PINKY_MCP, PINKY_PIP, PINKY_TIP, RING_DIP, RING_MCP, RING_PIP, RING_TIP, SKELETON_CONNECTIONS,
    THUMB_CMC, THUMB_IP, THUMB_MCP, THUMB_TIP, TOTAL_LANDMARKS, WRIST,
};

use std::sync::Arc;

/// Processes an input frame buffer and infers hand landmarks using the default CPU engine.
pub fn process_frame_landmarks(frame: &FrameBuffer) -> Result<HandLandmarks> {
    let engine = MockMediaPipeEngine::new();
    match engine.process_frame(&frame.as_ref())? {
        Some(landmarks) => Ok(landmarks),
        None => Err(GestureError::InferenceError(
            "No hand detected in frame".into(),
        )),
    }
}

/// Convenience entry point processing a raw frame buffer using a provided engine instance.
pub fn process_frame_landmarks_with_engine(
    engine: &dyn HandLandmarkEngine,
    frame: &FrameBuffer,
) -> Result<Option<HandLandmarks>> {
    engine.process_frame(&frame.as_ref())
}

/// Classifies hand landmarks into a rich recognized gesture.
pub fn classify_gesture(landmarks: &HandLandmarks) -> RecognizedGesture {
    let mut classifier = GestureClassifier::default();
    classifier
        .process_frame(landmarks, 0)
        .unwrap_or_else(|_| {
            let gesture = GestureClassifier::classify(&landmarks.landmarks);
            RecognizedGesture {
                gesture,
                confidence: landmarks.confidence,
                pinch_distance: None,
                is_pinching: gesture == GestureType::Pinch,
                pointing_ray: None,
                palm_center: landmarks.landmarks[WRIST],
                palm_normal: Landmark3D::new(0.0, 0.0, -1.0),
                handedness: landmarks.handedness,
                timestamp_ms: 0,
            }
        })
}

/// Maps 3D index fingertip landmark to radial dial pointer state.
pub fn map_pointing_to_radial_dial(
    index_tip: &Landmark3D,
    sector_count: usize,
) -> RadialPointerState {
    let mut mapper = SpatialCoordinateMapper::new(sector_count, (0.5, 0.5), 0.10);
    mapper.map_landmark(index_tip)
}

/// Factory function creating a thread-safe CPU hand landmark engine.
pub fn create_landmark_engine() -> Arc<dyn HandLandmarkEngine> {
    Arc::new(MockMediaPipeEngine::new())
}

/// Factory function creating a default hand gesture classifier.
pub fn create_gesture_classifier() -> GestureClassifier {
    GestureClassifier::default()
}

/// Factory function creating a default spatial coordinate mapper.
pub fn create_spatial_mapper() -> SpatialCoordinateMapper {
    SpatialCoordinateMapper::new(6, (0.5, 0.5), 0.10)
}
