//! Exponential Moving Average (EMA) Landmark Smoothing Filter.

use airctrl_core::types::gesture::Landmark3D;

/// Exponential Moving Average filter for 21 3D landmarks.
#[derive(Debug, Clone)]
pub struct EmaLandmarksFilter {
    alpha: f32,
    prev_landmarks: Option<[Landmark3D; 21]>,
}

impl EmaLandmarksFilter {
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha: alpha.clamp(0.01, 1.0),
            prev_landmarks: None,
        }
    }

    pub fn filter(&mut self, landmarks: &[Landmark3D; 21]) -> [Landmark3D; 21] {
        let prev = match self.prev_landmarks {
            Some(p) => p,
            None => {
                self.prev_landmarks = Some(*landmarks);
                return *landmarks;
            }
        };

        let mut smoothed = *landmarks;
        for i in 0..21 {
            smoothed[i] = Landmark3D {
                x: self.alpha * landmarks[i].x + (1.0 - self.alpha) * prev[i].x,
                y: self.alpha * landmarks[i].y + (1.0 - self.alpha) * prev[i].y,
                z: self.alpha * landmarks[i].z + (1.0 - self.alpha) * prev[i].z,
            };
        }

        self.prev_landmarks = Some(smoothed);
        smoothed
    }

    pub fn reset(&mut self) {
        self.prev_landmarks = None;
    }
}
