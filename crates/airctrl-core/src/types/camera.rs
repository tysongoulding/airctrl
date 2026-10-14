//! Domain models for video frame buffers, camera devices, virtual camera splitting, and privacy governance.

use serde::{Deserialize, Serialize};

/// Supported pixel buffer formats for camera and video capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameFormat {
    Rgb8,
    Rgba8,
    Bgr8,
    Bgra8,
    Yuy2,
    Nv12,
}

impl FrameFormat {
    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgb8 | Self::Bgr8 => 3,
            Self::Rgba8 | Self::Bgra8 => 4,
            Self::Yuy2 => 2,
            Self::Nv12 => 1,
        }
    }

    #[inline]
    pub fn has_alpha(&self) -> bool {
        matches!(self, Self::Rgba8 | Self::Bgra8)
    }
}

/// Standard 2D camera resolution dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraResolution {
    pub width: u32,
    pub height: u32,
}

impl CameraResolution {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn hd720p() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }

    pub const fn fhd1080p() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }

    pub const fn vga() -> Self {
        Self {
            width: 640,
            height: 480,
        }
    }
}

impl Default for CameraResolution {
    fn default() -> Self {
        Self::hd720p()
    }
}

/// Camera target or measured frames per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CameraFps(pub u32);

impl CameraFps {
    pub const FPS_30: Self = Self(30);
    pub const FPS_60: Self = Self(60);

    pub const fn as_u32(&self) -> u32 {
        self.0
    }

    pub const fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

impl Default for CameraFps {
    fn default() -> Self {
        Self::FPS_30
    }
}

/// Owned heap-allocated video frame buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub format: FrameFormat,
}

impl FrameBuffer {
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: usize,
        format: FrameFormat,
    ) -> Result<Self, String> {
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
        format: FrameFormat,
    ) -> Result<Self, String> {
        let stride = (width as usize) * format.bytes_per_pixel();
        Self::new(data, width, height, stride, format)
    }

    pub fn allocate(width: u32, height: u32, format: FrameFormat) -> Self {
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

    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err(format!(
                "Invalid frame dimensions: {}x{}",
                self.width, self.height
            ));
        }
        let bpp = self.format.bytes_per_pixel();
        let min_stride = (self.width as usize) * bpp;
        if self.stride < min_stride {
            return Err(format!(
                "Stride {} less than minimum {}",
                self.stride, min_stride
            ));
        }
        let expected_bytes = self.stride * (self.height as usize);
        if self.data.len() < expected_bytes {
            return Err(format!(
                "Buffer length {} less than expected {}",
                self.data.len(),
                expected_bytes
            ));
        }
        Ok(())
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let bpp = self.format.bytes_per_pixel();
        let offset = (y as usize) * self.stride + (x as usize) * bpp;
        if offset + bpp > self.data.len() {
            return None;
        }
        match self.format {
            FrameFormat::Rgb8 | FrameFormat::Rgba8 => {
                Some((self.data[offset], self.data[offset + 1], self.data[offset + 2]))
            }
            FrameFormat::Bgr8 | FrameFormat::Bgra8 => {
                Some((self.data[offset + 2], self.data[offset + 1], self.data[offset]))
            }
            _ => None,
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let bpp = self.format.bytes_per_pixel();
        let offset = (y as usize) * self.stride + (x as usize) * bpp;
        if offset + bpp > self.data.len() {
            return;
        }
        match self.format {
            FrameFormat::Rgb8 => {
                self.data[offset] = r;
                self.data[offset + 1] = g;
                self.data[offset + 2] = b;
            }
            FrameFormat::Rgba8 => {
                self.data[offset] = r;
                self.data[offset + 1] = g;
                self.data[offset + 2] = b;
                self.data[offset + 3] = 255;
            }
            FrameFormat::Bgr8 => {
                self.data[offset] = b;
                self.data[offset + 1] = g;
                self.data[offset + 2] = r;
            }
            FrameFormat::Bgra8 => {
                self.data[offset] = b;
                self.data[offset + 1] = g;
                self.data[offset + 2] = r;
                self.data[offset + 3] = 255;
            }
            _ => {}
        }
    }
}

/// Tri-state privacy mute model for virtual camera fan-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum PrivacyMuteState {
    /// Outbound meeting video is LIVE, local hand gesture tracking is LIVE.
    #[default]
    Active = 0,
    /// Outbound meeting video is replaced by synthetic privacy slate frame.
    /// Local hand gesture tracking REMAINS 100% ACTIVE and receives raw camera frames!
    PrivacySlate = 1,
    /// Both outbound meeting video and local gesture tracking are MUTED.
    DualMute = 2,
}

impl PrivacyMuteState {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::PrivacySlate,
            2 => Self::DualMute,
            _ => Self::Active,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub fn is_privacy_muted(&self) -> bool {
        matches!(self, Self::PrivacySlate | Self::DualMute)
    }

    pub fn is_dual_muted(&self) -> bool {
        matches!(self, Self::DualMute)
    }
}

impl std::str::FromStr for PrivacyMuteState {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace(['_', '-'], "").as_str() {
            "active" | "live" | "unmuted" => Ok(Self::Active),
            "privacyslate" | "privacy" | "slate" | "muted" => Ok(Self::PrivacySlate),
            "dualmute" | "dual" | "both" => Ok(Self::DualMute),
            other => Err(format!(
                "Unknown privacy mute state '{}'. Expected 'Active', 'PrivacySlate', or 'DualMute'",
                other
            )),
        }
    }
}

impl std::fmt::Display for PrivacyMuteState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::PrivacySlate => write!(f, "PrivacySlate"),
            Self::DualMute => write!(f, "DualMute"),
        }
    }
}

/// Configuration settings for the continuous Virtual Camera Splitter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualCameraConfig {
    pub device_name: String,
    pub resolution: CameraResolution,
    pub target_fps: u32,
    pub auto_start: bool,
}

impl Default for VirtualCameraConfig {
    fn default() -> Self {
        Self {
            device_name: "AirCtrl Virtual Camera".to_string(),
            resolution: CameraResolution::hd720p(),
            target_fps: 30,
            auto_start: true,
        }
    }
}

impl VirtualCameraConfig {
    pub fn new(device_name: impl Into<String>, width: u32, height: u32, target_fps: u32) -> Self {
        Self {
            device_name: device_name.into(),
            resolution: CameraResolution::new(width, height),
            target_fps,
            auto_start: true,
        }
    }

    pub fn width(&self) -> u32 {
        self.resolution.width
    }

    pub fn height(&self) -> u32 {
        self.resolution.height
    }

    pub fn fps(&self) -> u32 {
        self.target_fps
    }
}

/// Status and telemetry snapshot for the continuous Virtual Camera Splitter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualCameraStatus {
    pub is_running: bool,
    pub device_name: String,
    pub resolution: CameraResolution,
    pub privacy_mode: PrivacyMuteState,
    pub privacy_muted: bool,
    pub dual_muted: bool,
    pub frames_fanned_out: u64,
    pub target_fps: u32,
    pub actual_fps: f32,
    pub consumer_count: usize,
}

/// Operational metrics for video fan-out throughput and performance.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SplitterMetrics {
    pub total_frames_ingested: u64,
    pub consumer_1_frames_delivered: u64,
    pub consumer_2_frames_delivered: u64,
    pub frames_dropped: u64,
    pub last_frame_timestamp_ns: u64,
}
