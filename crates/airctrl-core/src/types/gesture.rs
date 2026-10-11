use serde::{Deserialize, Serialize};

/// 3D landmark coordinate in normalized coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Landmark3D {
    pub x: f32, // Normalized [0.0, 1.0] across image width
    pub y: f32, // Normalized [0.0, 1.0] across image height
    pub z: f32, // Relative depth coordinate
}

impl Landmark3D {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Handedness {
    Left,
    Right,
}

/// 21 MediaPipe hand landmarks topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandLandmarks {
    pub landmarks: [Landmark3D; 21],
    pub handedness: Handedness,
    pub confidence: f32,
}

impl HandLandmarks {
    pub fn new(landmarks: [Landmark3D; 21], handedness: Handedness, confidence: f32) -> Self {
        Self {
            landmarks,
            handedness,
            confidence,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GestureType {
    Neutral,
    Pointing,
    Pinch,
    OpenPalm,
    Fist,
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
}

/// Polar coordinates mapped to the radial dial overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialCoordinates {
    pub angle_rad: f32,         // Angle in radians [0, 2pi)
    pub normalized_radius: f32, // Radial distance from center [0.0, 1.0]
    pub active_sector: usize,   // Active radial sector index 0..N
    pub is_selected: bool,      // Whether selection is confirmed via pinch or dwell
}
