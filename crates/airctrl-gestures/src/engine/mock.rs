//! Deterministic In-Memory Mock MediaPipe Engine (Zero Native Compiler / CMake Dependencies).

use super::HandLandmarkEngine;
use crate::error::{GestureError, Result};
use crate::frame::FrameBufferRef;
use crate::topology::*;
use airctrl_core::types::gesture::{HandLandmarks, Handedness, Landmark3D, MultiHandLandmarks};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

/// Standard predefined mock poses for deterministic testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockPose {
    Neutral,
    Pointing,
    Pinch,
    OpenPalm,
    Fist,
    Empty, // Simulates no hand detected
}

/// Deterministic, zero-dependency in-memory MediaPipe Hand Landmarker engine.
pub struct MockMediaPipeEngine {
    is_ready: AtomicBool,
    current_pose: RwLock<MockPose>,
    left_pose: RwLock<MockPose>,
    dual_hand_mode: AtomicBool,
    handedness: RwLock<Handedness>,
    confidence: RwLock<f32>,
    name: String,
}

impl Default for MockMediaPipeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMediaPipeEngine {
    pub fn new() -> Self {
        Self {
            is_ready: AtomicBool::new(true),
            current_pose: RwLock::new(MockPose::Neutral),
            left_pose: RwLock::new(MockPose::Empty),
            dual_hand_mode: AtomicBool::new(false),
            handedness: RwLock::new(Handedness::Right),
            confidence: RwLock::new(0.95),
            name: "MockMediaPipeEngine-v1.0".to_string(),
        }
    }

    pub fn with_pose(pose: MockPose) -> Self {
        let engine = Self::new();
        *engine.current_pose.write().unwrap() = pose;
        engine
    }

    pub fn with_dual_poses(left: MockPose, right: MockPose) -> Self {
        let engine = Self::new();
        *engine.left_pose.write().unwrap() = left;
        *engine.current_pose.write().unwrap() = right;
        engine.dual_hand_mode.store(true, Ordering::SeqCst);
        engine
    }

    pub fn set_pose(&self, pose: MockPose) {
        *self.current_pose.write().unwrap() = pose;
    }

    pub fn set_left_pose(&self, pose: MockPose) {
        *self.left_pose.write().unwrap() = pose;
    }

    pub fn set_dual_hand_mode(&self, enabled: bool) {
        self.dual_hand_mode.store(enabled, Ordering::SeqCst);
    }

    pub fn is_dual_hand_mode(&self) -> bool {
        self.dual_hand_mode.load(Ordering::SeqCst)
    }

    pub fn set_ready(&self, ready: bool) {
        self.is_ready.store(ready, Ordering::SeqCst);
    }

    pub fn set_confidence(&self, confidence: f32) {
        *self.confidence.write().unwrap() = confidence.clamp(0.0, 1.0);
    }

    pub fn set_handedness(&self, handedness: Handedness) {
        *self.handedness.write().unwrap() = handedness;
    }


    /// Synthesizes 21 3D landmarks for a given mock pose.
    pub fn synthesize_landmarks(pose: MockPose) -> Option<[Landmark3D; 21]> {
        if pose == MockPose::Empty {
            return None;
        }

        let mut landmarks = [Landmark3D { x: 0.5, y: 0.8, z: 0.0 }; 21];

        // Wrist
        landmarks[WRIST] = Landmark3D { x: 0.5, y: 0.8, z: 0.0 };

        // MCP Knuckles for natural palm geometry
        landmarks[THUMB_CMC] = Landmark3D { x: 0.42, y: 0.72, z: 0.0 };
        landmarks[THUMB_MCP] = Landmark3D { x: 0.38, y: 0.68, z: 0.0 };
        landmarks[THUMB_IP] = Landmark3D { x: 0.36, y: 0.64, z: 0.0 };

        landmarks[INDEX_MCP] = Landmark3D { x: 0.46, y: 0.60, z: 0.0 };
        landmarks[INDEX_PIP] = Landmark3D { x: 0.47, y: 0.55, z: 0.0 };
        landmarks[INDEX_DIP] = Landmark3D { x: 0.47, y: 0.52, z: 0.0 };

        landmarks[MIDDLE_MCP] = Landmark3D { x: 0.50, y: 0.58, z: 0.0 };
        landmarks[MIDDLE_PIP] = Landmark3D { x: 0.52, y: 0.55, z: 0.0 };
        landmarks[MIDDLE_DIP] = Landmark3D { x: 0.53, y: 0.53, z: 0.0 };

        landmarks[RING_MCP] = Landmark3D { x: 0.54, y: 0.60, z: 0.0 };
        landmarks[RING_PIP] = Landmark3D { x: 0.56, y: 0.57, z: 0.0 };
        landmarks[RING_DIP] = Landmark3D { x: 0.58, y: 0.55, z: 0.0 };

        landmarks[PINKY_MCP] = Landmark3D { x: 0.58, y: 0.63, z: 0.0 };
        landmarks[PINKY_PIP] = Landmark3D { x: 0.60, y: 0.61, z: 0.0 };
        landmarks[PINKY_DIP] = Landmark3D { x: 0.62, y: 0.59, z: 0.0 };

        // Default Neutral fingertips
        landmarks[THUMB_TIP] = Landmark3D { x: 0.35, y: 0.60, z: 0.0 };
        landmarks[INDEX_TIP] = Landmark3D { x: 0.48, y: 0.50, z: 0.0 };
        landmarks[MIDDLE_TIP] = Landmark3D { x: 0.55, y: 0.52, z: 0.0 };
        landmarks[RING_TIP] = Landmark3D { x: 0.62, y: 0.55, z: 0.0 };
        landmarks[PINKY_TIP] = Landmark3D { x: 0.68, y: 0.60, z: 0.0 };

        match pose {
            MockPose::Pinch => {
                // Thumb tip and Index tip close (dist <= 0.05)
                landmarks[THUMB_TIP] = Landmark3D { x: 0.51, y: 0.40, z: 0.0 };
                landmarks[INDEX_TIP] = Landmark3D { x: 0.53, y: 0.41, z: 0.0 }; // dist ~ 0.0224
                landmarks[MIDDLE_TIP] = Landmark3D { x: 0.60, y: 0.65, z: 0.0 };
                landmarks[RING_TIP] = Landmark3D { x: 0.65, y: 0.70, z: 0.0 };
                landmarks[PINKY_TIP] = Landmark3D { x: 0.70, y: 0.75, z: 0.0 };
            }
            MockPose::Pointing => {
                // Index tip extended upward (y < 0.35), others curled (y > 0.55)
                landmarks[INDEX_PIP] = Landmark3D { x: 0.50, y: 0.50, z: 0.0 };
                landmarks[INDEX_DIP] = Landmark3D { x: 0.50, y: 0.35, z: 0.0 };
                landmarks[INDEX_TIP] = Landmark3D { x: 0.50, y: 0.20, z: 0.0 };
                landmarks[THUMB_TIP] = Landmark3D { x: 0.40, y: 0.60, z: 0.0 };
                landmarks[MIDDLE_TIP] = Landmark3D { x: 0.55, y: 0.65, z: 0.0 };
                landmarks[RING_TIP] = Landmark3D { x: 0.60, y: 0.70, z: 0.0 };
                landmarks[PINKY_TIP] = Landmark3D { x: 0.65, y: 0.75, z: 0.0 };
            }
            MockPose::OpenPalm => {
                // All 5 fingertips extended upward (y < 0.45)
                landmarks[THUMB_TIP] = Landmark3D { x: 0.30, y: 0.40, z: 0.0 };
                landmarks[INDEX_TIP] = Landmark3D { x: 0.40, y: 0.20, z: 0.0 };
                landmarks[MIDDLE_TIP] = Landmark3D { x: 0.50, y: 0.18, z: 0.0 };
                landmarks[RING_TIP] = Landmark3D { x: 0.60, y: 0.22, z: 0.0 };
                landmarks[PINKY_TIP] = Landmark3D { x: 0.70, y: 0.30, z: 0.0 };
            }
            MockPose::Fist => {
                // All fingertips curled downward (y > 0.55)
                landmarks[THUMB_TIP] = Landmark3D { x: 0.35, y: 0.65, z: 0.0 };
                landmarks[INDEX_TIP] = Landmark3D { x: 0.48, y: 0.68, z: 0.0 };
                landmarks[MIDDLE_TIP] = Landmark3D { x: 0.55, y: 0.70, z: 0.0 };
                landmarks[RING_TIP] = Landmark3D { x: 0.62, y: 0.72, z: 0.0 };
                landmarks[PINKY_TIP] = Landmark3D { x: 0.68, y: 0.75, z: 0.0 };
            }
            MockPose::Neutral | MockPose::Empty => {}
        }

        Some(landmarks)
    }

    /// Synthesizes 21 3D landmarks for a given mock pose and handedness.
    /// Positions left hand around x in [0.15, 0.45] and right hand around x in [0.55, 0.85].
    pub fn synthesize_hand_landmarks(
        pose: MockPose,
        handedness: Handedness,
    ) -> Option<[Landmark3D; 21]> {
        let base = Self::synthesize_landmarks(pose)?;
        let mut scaled = base;
        for lm in scaled.iter_mut() {
            match handedness {
                Handedness::Left => {
                    lm.x = (lm.x * 0.55 + 0.05).clamp(0.15, 0.45);
                }
                Handedness::Right => {
                    lm.x = (lm.x * 0.55 + 0.45).clamp(0.55, 0.85);
                }
            }
        }
        Some(scaled)
    }
}

impl HandLandmarkEngine for MockMediaPipeEngine {
    fn process_frame(&self, frame: &FrameBufferRef<'_>) -> Result<Option<HandLandmarks>> {
        if !self.is_ready() {
            return Err(GestureError::EngineNotReady(
                "MockMediaPipeEngine is not ready".into(),
            ));
        }

        // Validate buffer dimensions
        if frame.width == 0 || frame.height == 0 {
            return Err(GestureError::InvalidFrameDimensions {
                width: frame.width,
                height: frame.height,
            });
        }

        let expected_bytes = frame.stride * (frame.height as usize);
        if frame.data.len() < expected_bytes {
            return Err(GestureError::InvalidBufferLength {
                expected: expected_bytes,
                actual: frame.data.len(),
                width: frame.width,
                height: frame.height,
                stride: frame.stride,
            });
        }

        // Check if top-left pixel encodes a synthetic marker tag:
        // (R=255, G=0, B=0) -> Pointing
        // (R=0, G=255, B=0) -> Pinch
        // (R=0, G=0, B=255) -> OpenPalm
        // (R=128, G=128, B=128) -> Fist
        // (R=200, G=200, B=200) -> Neutral
        // (R=1, G=1, B=1) -> Empty
        let pose = if let Some(px) = frame.get_pixel(0, 0) {
            match px {
                (255, 0, 0) => MockPose::Pointing,
                (0, 255, 0) => MockPose::Pinch,
                (0, 0, 255) => MockPose::OpenPalm,
                (128, 128, 128) => MockPose::Fist,
                (200, 200, 200) => MockPose::Neutral,
                (1, 1, 1) => MockPose::Empty,
                _ => *self.current_pose.read().unwrap(),
            }
        } else {
            *self.current_pose.read().unwrap()
        };

        if let Some(landmarks) = Self::synthesize_landmarks(pose) {
            let handedness = *self.handedness.read().unwrap();
            let confidence = *self.confidence.read().unwrap();
            Ok(Some(HandLandmarks {
                landmarks,
                handedness,
                confidence,
            }))
        } else {
            Ok(None)
        }
    }

    fn process_frame_multi(&self, frame: &FrameBufferRef<'_>) -> Result<MultiHandLandmarks> {
        if !self.is_ready() {
            return Err(GestureError::EngineNotReady(
                "MockMediaPipeEngine is not ready".into(),
            ));
        }

        // Validate buffer dimensions
        if frame.width == 0 || frame.height == 0 {
            return Err(GestureError::InvalidFrameDimensions {
                width: frame.width,
                height: frame.height,
            });
        }

        let expected_bytes = frame.stride * (frame.height as usize);
        if frame.data.len() < expected_bytes {
            return Err(GestureError::InvalidBufferLength {
                expected: expected_bytes,
                actual: frame.data.len(),
                width: frame.width,
                height: frame.height,
                stride: frame.stride,
            });
        }

        // Decode primary (right) pose from pixel (0, 0)
        let right_pose = if let Some(px) = frame.get_pixel(0, 0) {
            match px {
                (255, 0, 0) => MockPose::Pointing,
                (0, 255, 0) => MockPose::Pinch,
                (0, 0, 255) => MockPose::OpenPalm,
                (128, 128, 128) => MockPose::Fist,
                (200, 200, 200) => MockPose::Neutral,
                (1, 1, 1) => MockPose::Empty,
                _ => *self.current_pose.read().unwrap(),
            }
        } else {
            *self.current_pose.read().unwrap()
        };

        // Decode secondary (left) pose from pixel (1, 0) if present, or from left_pose
        let left_pose = if frame.width > 1 {
            if let Some(px) = frame.get_pixel(1, 0) {
                match px {
                    (255, 0, 0) => MockPose::Pointing,
                    (0, 255, 0) => MockPose::Pinch,
                    (0, 0, 255) => MockPose::OpenPalm,
                    (128, 128, 128) => MockPose::Fist,
                    (200, 200, 200) => MockPose::Neutral,
                    (1, 1, 1) => MockPose::Empty,
                    _ => {
                        if self.dual_hand_mode.load(Ordering::SeqCst) {
                            *self.left_pose.read().unwrap()
                        } else {
                            MockPose::Empty
                        }
                    }
                }
            } else if self.dual_hand_mode.load(Ordering::SeqCst) {
                *self.left_pose.read().unwrap()
            } else {
                MockPose::Empty
            }
        } else if self.dual_hand_mode.load(Ordering::SeqCst) {
            *self.left_pose.read().unwrap()
        } else {
            MockPose::Empty
        };

        let confidence = *self.confidence.read().unwrap();
        let left_hand = if left_pose != MockPose::Empty {
            Self::synthesize_hand_landmarks(left_pose, Handedness::Left).map(|landmarks| {
                HandLandmarks {
                    landmarks,
                    handedness: Handedness::Left,
                    confidence,
                }
            })
        } else {
            None
        };

        let right_hand = if right_pose != MockPose::Empty {
            Self::synthesize_hand_landmarks(right_pose, Handedness::Right).map(|landmarks| {
                HandLandmarks {
                    landmarks,
                    handedness: Handedness::Right,
                    confidence,
                }
            })
        } else {
            None
        };

        Ok(MultiHandLandmarks::new(left_hand, right_hand))
    }

    fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::SeqCst)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

