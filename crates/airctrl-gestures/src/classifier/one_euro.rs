//! Adaptive One-Euro Filter for Hand Landmark Jitter Elimination (Casiez et al., CHI 2012).

use airctrl_core::types::gesture::Landmark3D;
use std::f32::consts::PI;

/// Low-pass filter component with cutoff frequency calculation.
#[derive(Debug, Clone)]
struct LowPassFilter {
    hat_x_prev: Option<f32>,
}

impl LowPassFilter {
    fn new() -> Self {
        Self { hat_x_prev: None }
    }

    fn filter(&mut self, x: f32, alpha: f32) -> f32 {
        let hat_x = match self.hat_x_prev {
            Some(prev) => alpha * x + (1.0 - alpha) * prev,
            None => x,
        };
        self.hat_x_prev = Some(hat_x);
        hat_x
    }

    fn reset(&mut self) {
        self.hat_x_prev = None;
    }
}

/// One-Euro adaptive filter for a single 1D scalar signal.
#[derive(Debug, Clone)]
pub struct OneEuroFilter1D {
    min_cutoff: f32,
    beta: f32,
    d_cutoff: f32,
    x_filter: LowPassFilter,
    dx_filter: LowPassFilter,
    last_time_ms: Option<u64>,
}

impl OneEuroFilter1D {
    pub fn new(min_cutoff: f32, beta: f32, d_cutoff: f32) -> Self {
        Self {
            min_cutoff,
            beta,
            d_cutoff,
            x_filter: LowPassFilter::new(),
            dx_filter: LowPassFilter::new(),
            last_time_ms: None,
        }
    }

    fn alpha(rate: f32, cutoff: f32) -> f32 {
        let tau = 1.0 / (2.0 * PI * cutoff);
        let te = 1.0 / rate;
        1.0 / (1.0 + tau / te)
    }

    pub fn filter(&mut self, x: f32, timestamp_ms: u64) -> f32 {
        let dt = match self.last_time_ms {
            Some(last_ms) => {
                let diff = (timestamp_ms.saturating_sub(last_ms)) as f32 / 1000.0;
                if diff <= 0.0 {
                    1.0 / 60.0
                } else {
                    diff
                }
            }
            None => 1.0 / 60.0,
        };
        self.last_time_ms = Some(timestamp_ms);
        let rate = 1.0 / dt;

        let dx = match self.x_filter.hat_x_prev {
            Some(prev) => (x - prev) * rate,
            None => 0.0,
        };

        let edx = self.dx_filter.filter(dx, Self::alpha(rate, self.d_cutoff));
        let cutoff = self.min_cutoff + self.beta * edx.abs();
        self.x_filter.filter(x, Self::alpha(rate, cutoff))
    }

    pub fn reset(&mut self) {
        self.x_filter.reset();
        self.dx_filter.reset();
        self.last_time_ms = None;
    }
}

/// One-Euro adaptive filter for 3D landmark points.
#[derive(Debug, Clone)]
pub struct OneEuroFilter3D {
    x_filter: OneEuroFilter1D,
    y_filter: OneEuroFilter1D,
    z_filter: OneEuroFilter1D,
}

impl OneEuroFilter3D {
    pub fn new(min_cutoff: f32, beta: f32, d_cutoff: f32) -> Self {
        Self {
            x_filter: OneEuroFilter1D::new(min_cutoff, beta, d_cutoff),
            y_filter: OneEuroFilter1D::new(min_cutoff, beta, d_cutoff),
            z_filter: OneEuroFilter1D::new(min_cutoff, beta, d_cutoff),
        }
    }

    pub fn filter(&mut self, p: &Landmark3D, timestamp_ms: u64) -> Landmark3D {
        Landmark3D {
            x: self.x_filter.filter(p.x, timestamp_ms),
            y: self.y_filter.filter(p.y, timestamp_ms),
            z: self.z_filter.filter(p.z, timestamp_ms),
        }
    }

    pub fn reset(&mut self) {
        self.x_filter.reset();
        self.y_filter.reset();
        self.z_filter.reset();
    }
}

/// One-Euro adaptive filter array for 21 MediaPipe hand landmarks.
#[derive(Debug, Clone)]
pub struct OneEuroLandmarksFilter {
    filters: Vec<OneEuroFilter3D>,
}

impl OneEuroLandmarksFilter {
    pub fn new(min_cutoff: f32, beta: f32, d_cutoff: f32) -> Self {
        let filters = (0..21)
            .map(|_| OneEuroFilter3D::new(min_cutoff, beta, d_cutoff))
            .collect();
        Self { filters }
    }

    pub fn filter(&mut self, landmarks: &[Landmark3D; 21], timestamp_ms: u64) -> [Landmark3D; 21] {
        let mut smoothed = *landmarks;
        for i in 0..21 {
            smoothed[i] = self.filters[i].filter(&landmarks[i], timestamp_ms);
        }
        smoothed
    }

    pub fn reset(&mut self) {
        for f in &mut self.filters {
            f.reset();
        }
    }
}
