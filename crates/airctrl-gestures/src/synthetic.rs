//! Synthetic Frame Generator for Clean, Occluded, and Empty Video Frames.

use crate::engine::mock::MockPose;
use crate::frame::{FrameBuffer, PixelFormat};
use airctrl_core::types::gesture::Handedness;

/// Generates synthetic video frame buffers for test harnesses and benchmarks.
pub struct SyntheticFrameGenerator;

impl SyntheticFrameGenerator {
    /// Generates an empty frame with marker (1, 1, 1) indicating no hand detected.
    pub fn create_empty_frame(width: u32, height: u32, format: PixelFormat) -> FrameBuffer {
        let mut frame = FrameBuffer::allocate(width, height, format);
        frame.set_pixel(0, 0, 1, 1, 1);
        frame
    }

    /// Generates a synthetic frame depicting a hand with the requested pose.
    /// Sets a marker pixel at (0, 0) for deterministic mock classification.
    pub fn create_hand_frame(
        width: u32,
        height: u32,
        pose: MockPose,
        handedness: Handedness,
        format: PixelFormat,
    ) -> FrameBuffer {
        let mut frame = FrameBuffer::allocate(width, height, format);

        // Encode pose marker at (0, 0)
        let marker = match pose {
            MockPose::Pointing => (255, 0, 0),
            MockPose::Pinch => (0, 255, 0),
            MockPose::OpenPalm => (0, 0, 255),
            MockPose::Fist => (128, 128, 128),
            MockPose::Neutral => (200, 200, 200),
            MockPose::Empty => (1, 1, 1),
        };
        frame.set_pixel(0, 0, marker.0, marker.1, marker.2);

        // Draw synthetic palm circle at center
        let cx = match handedness {
            Handedness::Right => ((width as f32) * 0.55) as i32,
            Handedness::Left => ((width as f32) * 0.45) as i32,
        };
        let cy = ((height as f32) * 0.7) as i32;
        let radius = ((width.min(height) as f32) * 0.12) as i32;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px > 0 && px < width as i32 && py > 0 && py < height as i32 {
                        frame.set_pixel(px as u32, py as u32, 220, 180, 140);
                    }
                }
            }
        }

        frame
    }

    /// Generates a frame with a synthetic hand partially covered by an occlusion rectangle.
    pub fn create_occluded_hand_frame(
        width: u32,
        height: u32,
        pose: MockPose,
        handedness: Handedness,
        occlusion_rect: (u32, u32, u32, u32), // (x, y, w, h)
        format: PixelFormat,
    ) -> FrameBuffer {
        let mut frame = Self::create_hand_frame(width, height, pose, handedness, format);
        let (ox, oy, ow, oh) = occlusion_rect;

        for y in oy..(oy + oh).min(height) {
            for x in ox..(ox + ow).min(width) {
                // Ensure we don't overwrite marker at (0, 0)
                if x == 0 && y == 0 {
                    continue;
                }
                // Overlay solid gray noise block
                frame.set_pixel(x, y, 40, 40, 40);
            }
        }
        frame
    }
}
