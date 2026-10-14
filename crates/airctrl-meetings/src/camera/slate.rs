//! Privacy slate generator producing synthetic video frames for muted meeting outbound feeds.

use airctrl_core::types::camera::{FrameBuffer, FrameFormat};
use std::sync::Arc;

/// Generates synthetic privacy slate video frames with a dynamic pulsing watermark.
pub struct PrivacySlateGenerator {
    base_frame: FrameBuffer,
    width: u32,
    height: u32,
}

pub type PrivacySlateRenderer = PrivacySlateGenerator;

impl PrivacySlateGenerator {
    pub fn new(width: u32, height: u32) -> Self {
        let mut base = FrameBuffer::allocate(width, height, FrameFormat::Rgb8);

        // 1. Fill background with sleek dark slate (#0f172a / Navy Slate)
        let bg_r = 15u8;
        let bg_g = 23u8;
        let bg_b = 42u8;
        for y in 0..height {
            for x in 0..width {
                base.set_pixel(x, y, bg_r, bg_g, bg_b);
            }
        }

        // 2. Render centered shield / camera-off box outline
        let cx = (width / 2) as i32;
        let cy = (height / 2) as i32;
        let box_w = 60i32;
        let box_h = 45i32;

        let border_r = 56u8;
        let border_g = 189u8;
        let border_b = 248u8;

        for x in (cx - box_w)..=(cx + box_w) {
            for y in (cy - box_h)..=(cy + box_h) {
                if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                    let is_border = x == cx - box_w || x == cx + box_w || y == cy - box_h || y == cy + box_h;
                    // slash through camera icon
                    let is_slash = (x - cx) == (y - cy);
                    if is_border || is_slash {
                        base.set_pixel(x as u32, y as u32, border_r, border_g, border_b);
                    }
                }
            }
        }

        Self {
            base_frame: base,
            width,
            height,
        }
    }

    /// Renders an outbound privacy frame with a pulsing keep-alive watermark.
    pub fn generate_slate_frame(&self, frame_id: u64, _timestamp_ns: u64) -> Arc<FrameBuffer> {
        let mut frame = self.base_frame.clone();

        // Render subtle pulsing keep-alive dot in corner (x: 40, y: 40)
        let pulse_intensity = (((frame_id % 60) as f32 / 60.0) * std::f32::consts::PI).sin().abs();
        let dot_color = (
            (40.0 + 56.0 * pulse_intensity) as u8,
            (60.0 + 120.0 * pulse_intensity) as u8,
            (100.0 + 140.0 * pulse_intensity) as u8,
        );

        for dy in 0..6 {
            for dx in 0..6 {
                let px = 40 + dx;
                let py = 40 + dy;
                if px < self.width && py < self.height {
                    frame.set_pixel(px, py, dot_color.0, dot_color.1, dot_color.2);
                }
            }
        }

        Arc::new(frame)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
