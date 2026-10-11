//! Video Frame Buffer Abstraction and Zero-Copy Views.

use crate::error::{GestureError, Result};
use serde::{Deserialize, Serialize};

/// Supported pixel buffer formats for camera and video capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Rgb8,
    Rgba8,
    Bgr8,
    Bgra8,
}

impl PixelFormat {
    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgb8 | PixelFormat::Bgr8 => 3,
            PixelFormat::Rgba8 | PixelFormat::Bgra8 => 4,
        }
    }

    #[inline]
    pub fn has_alpha(&self) -> bool {
        matches!(self, PixelFormat::Rgba8 | PixelFormat::Bgra8)
    }
}

/// Owned heap-allocated video frame buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub format: PixelFormat,
}

impl FrameBuffer {
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: usize,
        format: PixelFormat,
    ) -> Result<Self> {
        let frame = Self {
            data,
            width,
            height,
            stride,
            format,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn with_default_stride(
        data: Vec<u8>,
        width: u32,
        height: u32,
        format: PixelFormat,
    ) -> Result<Self> {
        let stride = (width as usize) * format.bytes_per_pixel();
        Self::new(data, width, height, stride, format)
    }

    pub fn allocate(width: u32, height: u32, format: PixelFormat) -> Self {
        let stride = (width as usize) * format.bytes_per_pixel();
        let total_bytes = stride * (height as usize);
        Self {
            data: vec![0u8; total_bytes],
            width,
            height,
            stride,
            format,
        }
    }

    pub fn as_ref(&self) -> FrameBufferRef<'_> {
        FrameBufferRef {
            data: &self.data,
            width: self.width,
            height: self.height,
            stride: self.stride,
            format: self.format,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(GestureError::InvalidFrameDimensions {
                width: self.width,
                height: self.height,
            });
        }
        let bpp = self.format.bytes_per_pixel();
        let min_stride = (self.width as usize) * bpp;
        if self.stride < min_stride {
            return Err(GestureError::InvalidBufferLength {
                expected: min_stride * (self.height as usize),
                actual: self.data.len(),
                width: self.width,
                height: self.height,
                stride: self.stride,
            });
        }
        let expected_bytes = self.stride * (self.height as usize);
        if self.data.len() < expected_bytes {
            return Err(GestureError::InvalidBufferLength {
                expected: expected_bytes,
                actual: self.data.len(),
                width: self.width,
                height: self.height,
                stride: self.stride,
            });
        }
        Ok(())
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        self.as_ref().get_pixel(x, y)
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = (y as usize) * self.stride + (x as usize) * self.format.bytes_per_pixel();
        if offset + self.format.bytes_per_pixel() > self.data.len() {
            return;
        }
        match self.format {
            PixelFormat::Rgb8 => {
                self.data[offset] = r;
                self.data[offset + 1] = g;
                self.data[offset + 2] = b;
            }
            PixelFormat::Rgba8 => {
                self.data[offset] = r;
                self.data[offset + 1] = g;
                self.data[offset + 2] = b;
                self.data[offset + 3] = 255;
            }
            PixelFormat::Bgr8 => {
                self.data[offset] = b;
                self.data[offset + 1] = g;
                self.data[offset + 2] = r;
            }
            PixelFormat::Bgra8 => {
                self.data[offset] = b;
                self.data[offset + 1] = g;
                self.data[offset + 2] = r;
                self.data[offset + 3] = 255;
            }
        }
    }
}

/// Borrowed, zero-copy view over raw video frame data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameBufferRef<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub format: PixelFormat,
}

impl<'a> FrameBufferRef<'a> {
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = (y as usize) * self.stride + (x as usize) * self.format.bytes_per_pixel();
        if offset + self.format.bytes_per_pixel() > self.data.len() {
            return None;
        }
        match self.format {
            PixelFormat::Rgb8 | PixelFormat::Rgba8 => {
                Some((self.data[offset], self.data[offset + 1], self.data[offset + 2]))
            }
            PixelFormat::Bgr8 | PixelFormat::Bgra8 => {
                Some((self.data[offset + 2], self.data[offset + 1], self.data[offset]))
            }
        }
    }

    /// Converts the borrowed buffer to contiguous RGB8 bytes.
    pub fn to_rgb8(&self) -> Vec<u8> {
        let mut rgb = Vec::with_capacity((self.width * self.height * 3) as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some((r, g, b)) = self.get_pixel(x, y) {
                    rgb.push(r);
                    rgb.push(g);
                    rgb.push(b);
                }
            }
        }
        rgb
    }
}
