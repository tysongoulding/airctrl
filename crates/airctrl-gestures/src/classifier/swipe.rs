//! Spatio-Temporal Dynamic Swipe Detector.

use airctrl_core::types::gesture::{GestureType, Landmark3D};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct TrajectorySample {
    point: Landmark3D,
    timestamp_ms: u64,
}

/// Dynamic Swipe gesture detector tracking palm/wrist centroid over a sliding window.
#[derive(Debug, Clone)]
pub struct SwipeDetector {
    window_ms: u64,
    min_displacement: f32,
    min_velocity: f32,
    cooldown_ms: u64,
    history: VecDeque<TrajectorySample>,
    last_trigger_ms: Option<u64>,
}

impl SwipeDetector {
    pub fn new(
        window_ms: u64,
        min_displacement: f32,
        min_velocity: f32,
        cooldown_ms: u64,
    ) -> Self {
        Self {
            window_ms,
            min_displacement,
            min_velocity,
            cooldown_ms,
            history: VecDeque::with_capacity(64),
            last_trigger_ms: None,
        }
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.last_trigger_ms = None;
    }

    /// Ingests a new centroid sample and checks for directional swipe gestures.
    pub fn update(&mut self, centroid: &Landmark3D, timestamp_ms: u64) -> Option<GestureType> {
        // Enforce cooldown if a swipe was previously triggered
        if let Some(last_ms) = self.last_trigger_ms {
            if timestamp_ms < last_ms + self.cooldown_ms {
                return None;
            }
        }

        // Add current sample
        self.history.push_back(TrajectorySample {
            point: *centroid,
            timestamp_ms,
        });

        // Prune samples older than window_ms
        while let Some(front) = self.history.front() {
            if timestamp_ms.saturating_sub(front.timestamp_ms) > self.window_ms {
                self.history.pop_front();
            } else {
                break;
            }
        }

        if self.history.len() < 3 {
            return None;
        }

        let oldest = self.history.front().unwrap();
        let newest = self.history.back().unwrap();

        let dt_sec = (newest.timestamp_ms.saturating_sub(oldest.timestamp_ms)) as f32 / 1000.0;
        if dt_sec < 0.05 {
            return None;
        }

        let dx = newest.point.x - oldest.point.x;
        let dy = newest.point.y - oldest.point.y;
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        let max_disp = abs_dx.max(abs_dy);
        let velocity = max_disp / dt_sec;

        if max_disp < self.min_displacement || velocity < self.min_velocity {
            return None;
        }

        let gesture = if abs_dx >= 1.5 * abs_dy {
            // Dominant horizontal
            if dx < 0.0 {
                Some(GestureType::SwipeLeft)
            } else {
                Some(GestureType::SwipeRight)
            }
        } else if abs_dy >= 1.5 * abs_dx {
            // Dominant vertical
            if dy < 0.0 {
                Some(GestureType::SwipeUp)
            } else {
                Some(GestureType::SwipeDown)
            }
        } else {
            None
        };

        if gesture.is_some() {
            self.last_trigger_ms = Some(timestamp_ms);
            self.history.clear();
        }

        gesture
    }
}
