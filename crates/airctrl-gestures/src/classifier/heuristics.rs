//! Heuristic Hand Gesture Classification Rules and Geometric Calculations.

use crate::topology::*;
use airctrl_core::types::gesture::{GestureType, Landmark3D};
use serde::{Deserialize, Serialize};

/// Pointing ray computed from index finger directional vector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointingRay {
    pub origin: Landmark3D,
    pub direction: Landmark3D,
    pub pitch_rad: f32,
    pub yaw_rad: f32,
}

/// 3D Euclidean distance between thumb tip and index tip.
#[inline]
pub fn pinch_distance(thumb_tip: &Landmark3D, index_tip: &Landmark3D) -> f32 {
    let dx = thumb_tip.x - index_tip.x;
    let dy = thumb_tip.y - index_tip.y;
    let dz = thumb_tip.z - index_tip.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Checks if pinch distance satisfies the threshold with 5-decimal rounding parity.
#[inline]
pub fn is_pinch_at_threshold(dist: f32, threshold: f32) -> bool {
    let rounded = (dist * 100_000.0).round() / 100_000.0;
    rounded <= threshold
}

/// Computes the pointing ray from the index finger chain.
pub fn compute_pointing_ray(landmarks: &[Landmark3D; 21]) -> PointingRay {
    let origin = landmarks[INDEX_TIP];
    let pip = landmarks[INDEX_PIP];

    let mut dx = origin.x - pip.x;
    let mut dy = origin.y - pip.y;
    let mut dz = origin.z - pip.z;
    let len = (dx * dx + dy * dy + dz * dz).sqrt();

    let direction = if len > 1e-5 {
        dx /= len;
        dy /= len;
        dz /= len;
        Landmark3D {
            x: dx,
            y: dy,
            z: dz,
        }
    } else {
        Landmark3D {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }
    };

    let pitch_rad = (-direction.y).clamp(-1.0, 1.0).asin();
    let yaw_rad = direction.x.atan2(direction.z);

    PointingRay {
        origin,
        direction,
        pitch_rad,
        yaw_rad,
    }
}

/// Calculates the center of the palm.
pub fn calculate_palm_center(landmarks: &[Landmark3D; 21]) -> Landmark3D {
    let indices = [WRIST, INDEX_MCP, MIDDLE_MCP, PINKY_MCP];
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sz = 0.0;
    for &idx in &indices {
        let lm = &landmarks[idx];
        sx += lm.x;
        sy += lm.y;
        sz += lm.z;
    }
    let count = indices.len() as f32;
    Landmark3D {
        x: sx / count,
        y: sy / count,
        z: sz / count,
    }
}

/// Calculates the palm normal vector via cross product of palm vectors.
pub fn calculate_palm_normal(landmarks: &[Landmark3D; 21]) -> Landmark3D {
    let wrist = landmarks[WRIST];
    let index_mcp = landmarks[INDEX_MCP];
    let pinky_mcp = landmarks[PINKY_MCP];

    let u_x = index_mcp.x - wrist.x;
    let u_y = index_mcp.y - wrist.y;
    let u_z = index_mcp.z - wrist.z;

    let v_x = pinky_mcp.x - wrist.x;
    let v_y = pinky_mcp.y - wrist.y;
    let v_z = pinky_mcp.z - wrist.z;

    // Cross product u x v
    let mut nx = u_y * v_z - u_z * v_y;
    let mut ny = u_z * v_x - u_x * v_z;
    let mut nz = u_x * v_y - u_y * v_x;

    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len > 1e-5 {
        nx /= len;
        ny /= len;
        nz /= len;
    }

    Landmark3D {
        x: nx,
        y: ny,
        z: nz,
    }
}

/// Pure stateless classification logic matching test oracle requirements.
pub fn classify_heuristics(landmarks: &[Landmark3D]) -> GestureType {
    if landmarks.len() < 21 {
        return GestureType::Neutral;
    }

    // 1. Pinch: Thumb tip (#4) and index tip (#8) distance <= 0.05
    let pinch_dist = pinch_distance(&landmarks[THUMB_TIP], &landmarks[INDEX_TIP]);
    if is_pinch_at_threshold(pinch_dist, 0.050) {
        return GestureType::Pinch;
    }

    // 2. OpenPalm: All 5 fingertips (4, 8, 12, 16, 20) y < 0.45
    let tips_extended = [
        THUMB_TIP, INDEX_TIP, MIDDLE_TIP, RING_TIP, PINKY_TIP,
    ]
    .iter()
    .all(|&idx| landmarks[idx].y < 0.45);

    if tips_extended {
        return GestureType::OpenPalm;
    }

    // 3. Pointing: Index tip (#8) extended (y < 0.35) while middle (#12) and ring (#16) curled (y > 0.55)
    if landmarks[INDEX_TIP].y < 0.35
        && landmarks[MIDDLE_TIP].y > 0.55
        && landmarks[RING_TIP].y > 0.55
    {
        return GestureType::Pointing;
    }

    // 4. Fist: All 5 fingertips curled downward (y > 0.55)
    let fist_curled = [
        THUMB_TIP, INDEX_TIP, MIDDLE_TIP, RING_TIP, PINKY_TIP,
    ]
    .iter()
    .all(|&idx| landmarks[idx].y > 0.55);

    if fist_curled {
        return GestureType::Fist;
    }

    GestureType::Neutral
}

/// State machine managing Pinch enter/exit hysteresis to avoid border jitter.
#[derive(Debug, Clone)]
pub struct PinchHysteresis {
    enter_threshold: f32,
    exit_threshold: f32,
    is_pinching: bool,
}

impl PinchHysteresis {
    pub fn new(enter_threshold: f32, exit_threshold: f32) -> Self {
        Self {
            enter_threshold,
            exit_threshold,
            is_pinching: false,
        }
    }

    pub fn update(&mut self, dist: f32) -> bool {
        if self.is_pinching {
            if dist > self.exit_threshold {
                self.is_pinching = false;
            }
        } else if is_pinch_at_threshold(dist, self.enter_threshold) {
            self.is_pinching = true;
        }
        self.is_pinching
    }

    pub fn is_active(&self) -> bool {
        self.is_pinching
    }

    pub fn reset(&mut self) {
        self.is_pinching = false;
    }
}
