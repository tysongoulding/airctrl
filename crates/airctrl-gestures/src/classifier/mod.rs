//! Hand Gesture Classification Module (Pointing, Pinch, OpenPalm, Fist, Swipes).

pub mod ema;
pub mod heuristics;
pub mod one_euro;
pub mod swipe;

pub use ema::EmaLandmarksFilter;
pub use heuristics::{
    calculate_palm_center, calculate_palm_normal, classify_heuristics, compute_pointing_ray,
    is_pinch_at_threshold, pinch_distance, PinchHysteresis, PointingRay,
};
pub use one_euro::OneEuroLandmarksFilter;
pub use swipe::SwipeDetector;

use crate::error::{GestureError, Result};
use airctrl_core::types::gesture::{GestureType, HandLandmarks, Handedness, Landmark3D};
use serde::{Deserialize, Serialize};

/// Type of temporal smoothing filter applied to hand landmarks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    None,
    Ema,
    OneEuro,
}

/// Configuration settings for `GestureClassifier`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureClassifierConfig {
    pub filter_type: FilterType,
    pub pinch_enter_threshold: f32,
    pub pinch_exit_threshold: f32,
    pub swipe_enabled: bool,
    pub swipe_window_ms: u64,
    pub swipe_min_displacement: f32,
    pub swipe_min_velocity: f32,
    pub swipe_cooldown_ms: u64,
    pub ema_alpha: f32,
    pub one_euro_min_cutoff: f32,
    pub one_euro_beta: f32,
    pub one_euro_d_cutoff: f32,
    pub min_confidence: f32,
}

impl Default for GestureClassifierConfig {
    fn default() -> Self {
        Self {
            filter_type: FilterType::OneEuro,
            pinch_enter_threshold: 0.050,
            pinch_exit_threshold: 0.070,
            swipe_enabled: true,
            swipe_window_ms: 250,
            swipe_min_displacement: 0.15,
            swipe_min_velocity: 0.50,
            swipe_cooldown_ms: 400,
            ema_alpha: 0.65,
            one_euro_min_cutoff: 1.0,
            one_euro_beta: 0.005,
            one_euro_d_cutoff: 1.0,
            min_confidence: 0.40,
        }
    }
}

/// Rich recognized gesture output produced by the classification engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecognizedGesture {
    pub gesture: GestureType,
    pub confidence: f32,
    pub pinch_distance: Option<f32>,
    pub is_pinching: bool,
    pub pointing_ray: Option<PointingRay>,
    pub palm_center: Landmark3D,
    pub palm_normal: Landmark3D,
    pub handedness: Handedness,
    pub timestamp_ms: u64,
}

/// Stateful and stateless hand gesture classifier.
pub struct GestureClassifier {
    config: GestureClassifierConfig,
    pinch_hysteresis: PinchHysteresis,
    swipe_detector: SwipeDetector,
    one_euro_filter: Option<OneEuroLandmarksFilter>,
    ema_filter: Option<EmaLandmarksFilter>,
}

impl Default for GestureClassifier {
    fn default() -> Self {
        Self::new(GestureClassifierConfig::default())
    }
}

impl GestureClassifier {
    pub fn new(config: GestureClassifierConfig) -> Self {
        let pinch_hysteresis = PinchHysteresis::new(
            config.pinch_enter_threshold,
            config.pinch_exit_threshold,
        );
        let swipe_detector = SwipeDetector::new(
            config.swipe_window_ms,
            config.swipe_min_displacement,
            config.swipe_min_velocity,
            config.swipe_cooldown_ms,
        );
        let one_euro_filter = if config.filter_type == FilterType::OneEuro {
            Some(OneEuroLandmarksFilter::new(
                config.one_euro_min_cutoff,
                config.one_euro_beta,
                config.one_euro_d_cutoff,
            ))
        } else {
            None
        };
        let ema_filter = if config.filter_type == FilterType::Ema {
            Some(EmaLandmarksFilter::new(config.ema_alpha))
        } else {
            None
        };

        Self {
            config,
            pinch_hysteresis,
            swipe_detector,
            one_euro_filter,
            ema_filter,
        }
    }

    /// Pure stateless classifier matching exact test oracle specifications.
    /// Safely handles empty or truncated landmark arrays by returning GestureType::Neutral.
    pub fn classify(landmarks: &[Landmark3D]) -> GestureType {
        classify_heuristics(landmarks)
    }

    /// Full stateful processing pipeline with temporal smoothing, hysteresis, and swipe tracking.
    pub fn process_frame(
        &mut self,
        hand: &HandLandmarks,
        timestamp_ms: u64,
    ) -> Result<RecognizedGesture> {
        if hand.confidence < self.config.min_confidence {
            return Err(GestureError::LowConfidence {
                confidence: hand.confidence,
                threshold: self.config.min_confidence,
            });
        }

        // Validate landmark coordinates
        for (i, lm) in hand.landmarks.iter().enumerate() {
            if lm.x.is_nan()
                || lm.y.is_nan()
                || lm.z.is_nan()
                || lm.x.is_infinite()
                || lm.y.is_infinite()
                || lm.z.is_infinite()
            {
                return Err(GestureError::InvalidCoordinate {
                    index: i,
                    x: lm.x,
                    y: lm.y,
                    z: lm.z,
                });
            }
        }

        // Temporal smoothing
        let smoothed_landmarks = if let Some(filter) = &mut self.one_euro_filter {
            filter.filter(&hand.landmarks, timestamp_ms)
        } else if let Some(filter) = &mut self.ema_filter {
            filter.filter(&hand.landmarks)
        } else {
            hand.landmarks
        };

        // Geometric evaluations
        let p_thumb = smoothed_landmarks[crate::topology::THUMB_TIP];
        let p_index = smoothed_landmarks[crate::topology::INDEX_TIP];
        let p_dist = pinch_distance(&p_thumb, &p_index);
        let is_pinching = self.pinch_hysteresis.update(p_dist);
        let palm_center = calculate_palm_center(&smoothed_landmarks);
        let palm_normal = calculate_palm_normal(&smoothed_landmarks);

        // Check dynamic swipe if enabled and hand is not actively pinching
        let swipe_gesture = if self.config.swipe_enabled && !is_pinching {
            self.swipe_detector.update(&palm_center, timestamp_ms)
        } else {
            None
        };

        // Determine final gesture enum
        let gesture = if let Some(swipe) = swipe_gesture {
            swipe
        } else if is_pinching {
            GestureType::Pinch
        } else {
            Self::classify(&smoothed_landmarks)
        };

        // Compute pointing ray if pointing
        let pointing_ray = if gesture == GestureType::Pointing {
            Some(compute_pointing_ray(&smoothed_landmarks))
        } else {
            None
        };

        Ok(RecognizedGesture {
            gesture,
            confidence: hand.confidence,
            pinch_distance: Some(p_dist),
            is_pinching,
            pointing_ray,
            palm_center,
            palm_normal,
            handedness: hand.handedness,
            timestamp_ms,
        })
    }

    pub fn reset(&mut self) {
        self.pinch_hysteresis.reset();
        self.swipe_detector.reset();
        if let Some(filter) = &mut self.one_euro_filter {
            filter.reset();
        }
        if let Some(filter) = &mut self.ema_filter {
            filter.reset();
        }
    }
}
