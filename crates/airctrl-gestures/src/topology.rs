//! 21 MediaPipe Hand Landmark Topology, Indices, and Named Constants.

use airctrl_core::types::gesture::Landmark3D;
use serde::{Deserialize, Serialize};
use crate::error::{GestureError, Result};

pub const WRIST: usize = 0;
pub const THUMB_CMC: usize = 1;
pub const THUMB_MCP: usize = 2;
pub const THUMB_IP: usize = 3;
pub const THUMB_TIP: usize = 4;

pub const INDEX_MCP: usize = 5;
pub const INDEX_PIP: usize = 6;
pub const INDEX_DIP: usize = 7;
pub const INDEX_TIP: usize = 8;

pub const MIDDLE_MCP: usize = 9;
pub const MIDDLE_PIP: usize = 10;
pub const MIDDLE_DIP: usize = 11;
pub const MIDDLE_TIP: usize = 12;

pub const RING_MCP: usize = 13;
pub const RING_PIP: usize = 14;
pub const RING_DIP: usize = 15;
pub const RING_TIP: usize = 16;

pub const PINKY_MCP: usize = 17;
pub const PINKY_PIP: usize = 18;
pub const PINKY_DIP: usize = 19;
pub const PINKY_TIP: usize = 20;

pub const TOTAL_LANDMARKS: usize = 21;
pub const FINGERTIP_INDICES: [usize; 5] = [THUMB_TIP, INDEX_TIP, MIDDLE_TIP, RING_TIP, PINKY_TIP];
pub const MCP_INDICES: [usize; 5] = [THUMB_CMC, INDEX_MCP, MIDDLE_MCP, RING_MCP, PINKY_MCP];

/// Standard MediaPipe hand skeletal joint pairs for bone connectivity and rendering.
pub const SKELETON_CONNECTIONS: &[(usize, usize)] = &[
    (WRIST, THUMB_CMC),
    (THUMB_CMC, THUMB_MCP),
    (THUMB_MCP, THUMB_IP),
    (THUMB_IP, THUMB_TIP),
    (WRIST, INDEX_MCP),
    (INDEX_MCP, INDEX_PIP),
    (INDEX_PIP, INDEX_DIP),
    (INDEX_DIP, INDEX_TIP),
    (INDEX_MCP, MIDDLE_MCP),
    (MIDDLE_MCP, MIDDLE_PIP),
    (MIDDLE_PIP, MIDDLE_DIP),
    (MIDDLE_DIP, MIDDLE_TIP),
    (MIDDLE_MCP, RING_MCP),
    (RING_MCP, RING_PIP),
    (RING_PIP, RING_DIP),
    (RING_DIP, RING_TIP),
    (RING_MCP, PINKY_MCP),
    (WRIST, PINKY_MCP),
    (PINKY_MCP, PINKY_PIP),
    (PINKY_PIP, PINKY_DIP),
    (PINKY_DIP, PINKY_TIP),
];

/// Helper extension trait for geometric computations on `Landmark3D`.
pub trait LandmarkOps {
    fn euclidean_dist_2d(&self, other: &Self) -> f32;
    fn euclidean_dist_3d(&self, other: &Self) -> f32;
    fn in_bounds(&self) -> bool;
    fn is_valid(&self) -> bool;
}

impl LandmarkOps for Landmark3D {
    #[inline]
    fn euclidean_dist_2d(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    #[inline]
    fn euclidean_dist_3d(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[inline]
    fn in_bounds(&self) -> bool {
        self.x >= 0.0 && self.x <= 1.0 && self.y >= 0.0 && self.y <= 1.0
    }

    #[inline]
    fn is_valid(&self) -> bool {
        !self.x.is_nan()
            && !self.y.is_nan()
            && !self.z.is_nan()
            && !self.x.is_infinite()
            && !self.y.is_infinite()
            && !self.z.is_infinite()
    }
}

/// Normalized list of 21 3D hand landmarks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedLandmarkList {
    pub landmarks: Vec<Landmark3D>,
}

impl NormalizedLandmarkList {
    pub fn new(landmarks: Vec<Landmark3D>) -> Result<Self> {
        if landmarks.len() != TOTAL_LANDMARKS {
            return Err(GestureError::InsufficientLandmarks(landmarks.len()));
        }
        Ok(Self { landmarks })
    }

    pub fn from_array(arr: [Landmark3D; TOTAL_LANDMARKS]) -> Self {
        Self {
            landmarks: arr.to_vec(),
        }
    }

    pub fn to_array(&self) -> Result<[Landmark3D; TOTAL_LANDMARKS]> {
        if self.landmarks.len() != TOTAL_LANDMARKS {
            return Err(GestureError::InsufficientLandmarks(self.landmarks.len()));
        }
        let mut arr = [Landmark3D::default(); TOTAL_LANDMARKS];
        arr.copy_from_slice(&self.landmarks[..TOTAL_LANDMARKS]);
        Ok(arr)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.landmarks.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.landmarks.is_empty()
    }

    #[inline]
    pub fn get(&self, idx: usize) -> Option<&Landmark3D> {
        self.landmarks.get(idx)
    }

    #[inline]
    pub fn wrist(&self) -> Option<&Landmark3D> {
        self.get(WRIST)
    }

    #[inline]
    pub fn thumb_tip(&self) -> Option<&Landmark3D> {
        self.get(THUMB_TIP)
    }

    #[inline]
    pub fn index_tip(&self) -> Option<&Landmark3D> {
        self.get(INDEX_TIP)
    }

    #[inline]
    pub fn middle_tip(&self) -> Option<&Landmark3D> {
        self.get(MIDDLE_TIP)
    }

    #[inline]
    pub fn ring_tip(&self) -> Option<&Landmark3D> {
        self.get(RING_TIP)
    }

    #[inline]
    pub fn pinky_tip(&self) -> Option<&Landmark3D> {
        self.get(PINKY_TIP)
    }

    /// Approximates the geometric center of the palm by averaging Wrist and MCP joints.
    pub fn palm_center(&self) -> Option<Landmark3D> {
        if self.landmarks.len() < TOTAL_LANDMARKS {
            return None;
        }
        let indices = [WRIST, INDEX_MCP, MIDDLE_MCP, RING_MCP, PINKY_MCP];
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;
        for &idx in &indices {
            let lm = &self.landmarks[idx];
            sx += lm.x;
            sy += lm.y;
            sz += lm.z;
        }
        let count = indices.len() as f32;
        Some(Landmark3D {
            x: sx / count,
            y: sy / count,
            z: sz / count,
        })
    }
}
