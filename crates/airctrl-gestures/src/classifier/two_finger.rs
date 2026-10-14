//! Two-finger gesture heuristics: scrolling and pinch-to-resize.

use airctrl_core::types::gesture::{HandLandmarks, Landmark3D};
use crate::topology::{
    INDEX_PIP, INDEX_TIP, MIDDLE_PIP, MIDDLE_TIP, PINKY_PIP, PINKY_TIP, RING_PIP, RING_TIP,
    THUMB_TIP,
};
use serde::{Deserialize, Serialize};

/// Event produced by two-finger gesture recognition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TwoFingerGestureEvent {
    Scroll {
        delta_x: f32,
        delta_y: f32,
        position: f32,
    },
    Resize {
        delta_dist: f32,
        scale_factor: f32,
    },
    None,
}

/// Detector tracking two-finger scroll and dynamic pinch-to-resize gestures.
#[derive(Debug, Clone)]
pub struct TwoFingerDetector {
    last_scroll_centroid: Option<(f32, f32)>,
    last_pinch_distance: Option<f32>,
    pub last_timestamp_ms: u64,
}

impl Default for TwoFingerDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TwoFingerDetector {
    pub fn new() -> Self {
        Self {
            last_scroll_centroid: None,
            last_pinch_distance: None,
            last_timestamp_ms: 0,
        }
    }

    /// Evaluates static vertical scroll center position of two adjacent fingertips.
    /// If index and middle tips are within 0.080 distance, returns vertical midpoint.
    pub fn evaluate_scroll_position(index_tip: &Landmark3D, middle_tip: &Landmark3D) -> f32 {
        let dx = index_tip.x - middle_tip.x;
        let dy = index_tip.y - middle_tip.y;
        let dz = index_tip.z - middle_tip.z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        if dist < 0.080 {
            (index_tip.y + middle_tip.y) * 0.5
        } else {
            0.0
        }
    }

    /// Evaluates two-finger scroll and pinch-to-resize from hand landmarks over time.
    pub fn update(&mut self, hand: &HandLandmarks, timestamp_ms: u64) -> TwoFingerGestureEvent {
        let lms = &hand.landmarks;

        // 1. Two-Finger Scroll Evaluation
        // Conditions: Index and Middle extended, Ring and Pinky curled.
        let index_extended = lms[INDEX_TIP].y < lms[INDEX_PIP].y;
        let middle_extended = lms[MIDDLE_TIP].y < lms[MIDDLE_PIP].y;
        let ring_curled = lms[RING_TIP].y >= lms[RING_PIP].y - 0.05 || lms[RING_TIP].y > 0.45;
        let pinky_curled = lms[PINKY_TIP].y >= lms[PINKY_PIP].y - 0.05 || lms[PINKY_TIP].y > 0.45;

        if index_extended && middle_extended && ring_curled && pinky_curled {
            let dx = lms[INDEX_TIP].x - lms[MIDDLE_TIP].x;
            let dy = lms[INDEX_TIP].y - lms[MIDDLE_TIP].y;
            let inter_finger_dist = (dx * dx + dy * dy).sqrt();

            if inter_finger_dist < 0.080 {
                let current_centroid = (
                    (lms[INDEX_TIP].x + lms[MIDDLE_TIP].x) * 0.5,
                    (lms[INDEX_TIP].y + lms[MIDDLE_TIP].y) * 0.5,
                );

                let event = if let Some((prev_x, prev_y)) = self.last_scroll_centroid {
                    let delta_x = current_centroid.0 - prev_x;
                    let delta_y = current_centroid.1 - prev_y;
                    if delta_x.abs() >= 0.010 || delta_y.abs() >= 0.010 {
                        TwoFingerGestureEvent::Scroll {
                            delta_x,
                            delta_y,
                            position: current_centroid.1,
                        }
                    } else {
                        TwoFingerGestureEvent::None
                    }
                } else {
                    TwoFingerGestureEvent::Scroll {
                        delta_x: 0.0,
                        delta_y: 0.0,
                        position: current_centroid.1,
                    }
                };

                self.last_scroll_centroid = Some(current_centroid);
                self.last_pinch_distance = None;
                self.last_timestamp_ms = timestamp_ms;
                return event;
            }
        }

        self.last_scroll_centroid = None;

        // 2. Pinch-to-Resize (Dynamic Two-Finger Zoom / Expansion) Evaluation
        // Conditions: Thumb and Index tips extended, tracking inter-digit delta distance.
        let thumb_tip = &lms[THUMB_TIP];
        let index_tip = &lms[INDEX_TIP];
        let p_dx = thumb_tip.x - index_tip.x;
        let p_dy = thumb_tip.y - index_tip.y;
        let p_dz = thumb_tip.z - index_tip.z;
        let dist = (p_dx * p_dx + p_dy * p_dy + p_dz * p_dz).sqrt();

        if let Some(prev_dist) = self.last_pinch_distance {
            let delta_dist = dist - prev_dist;
            // Deadband threshold: 0.020
            if delta_dist.abs() >= 0.020 {
                let scale_factor = (dist / prev_dist.max(1e-4)).clamp(0.5, 2.0);
                self.last_pinch_distance = Some(dist);
                self.last_timestamp_ms = timestamp_ms;
                return TwoFingerGestureEvent::Resize {
                    delta_dist,
                    scale_factor,
                };
            }
        } else {
            self.last_pinch_distance = Some(dist);
        }

        self.last_timestamp_ms = timestamp_ms;
        TwoFingerGestureEvent::None
    }

    /// Resets internal temporal state.
    pub fn reset(&mut self) {
        self.last_scroll_centroid = None;
        self.last_pinch_distance = None;
        self.last_timestamp_ms = 0;
    }
}
