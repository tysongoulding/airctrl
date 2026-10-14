//! Continuous Virtual Camera Splitter fanning out video frames to MediaPipe and meeting video sinks.

use super::buffer::ZeroLockVideoRingBuffer;
use super::slate::PrivacySlateGenerator;
use crate::error::{MeetingError, Result};
use airctrl_core::types::camera::{
    FrameBuffer, FrameFormat, PrivacyMuteState, VirtualCameraConfig, VirtualCameraStatus,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

pub type FanOutFrameResult = SplitterFanOutResult;

/// Result of fanning out a camera frame across Consumer 1 (MediaPipe) and Consumer 2 (meeting apps).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitterFanOutResult {
    pub device: String,
    pub frame_id: u64,
    #[serde(skip)]
    pub consumer_1_frame: Option<Arc<FrameBuffer>>,
    #[serde(skip)]
    pub consumer_2_frame: Option<Arc<FrameBuffer>>,
    pub consumer_1_mediapipe: Option<String>,
    pub consumer_2_meeting: String,
    pub privacy_muted: bool,
    pub dual_muted: bool,
}

/// Continuous Virtual Camera Splitter that runs persistently and fans out frames
/// to both local MediaPipe tracking and external meeting capture sinks.
pub struct VirtualCameraSplitter {
    config: RwLock<VirtualCameraConfig>,
    is_running: AtomicBool,
    privacy_mode: AtomicU8, // 0 = Active, 1 = PrivacySlate, 2 = DualMute
    privacy_mute_active: AtomicBool,
    frames_fanned_out: AtomicU64,
    ring_buffer: Arc<ZeroLockVideoRingBuffer>,
    slate_generator: Arc<PrivacySlateGenerator>,
}

impl Default for VirtualCameraSplitter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl VirtualCameraSplitter {
    /// Creates a new splitter with the given configuration.
    pub fn new(config: VirtualCameraConfig) -> Self {
        let width = config.width();
        let height = config.height();
        Self {
            config: RwLock::new(config),
            is_running: AtomicBool::new(true),
            privacy_mode: AtomicU8::new(PrivacyMuteState::Active as u8),
            privacy_mute_active: AtomicBool::new(false),
            frames_fanned_out: AtomicU64::new(0),
            ring_buffer: Arc::new(ZeroLockVideoRingBuffer::new()),
            slate_generator: Arc::new(PrivacySlateGenerator::new(width, height)),
        }
    }

    /// Creates a new splitter with default configuration ("AirCtrl Virtual Camera", 1280x720, 30 FPS).
    pub fn with_default_config() -> Self {
        Self::new(VirtualCameraConfig::default())
    }

    /// Creates a new splitter with a customized virtual camera device name.
    pub fn with_device_name(name: impl Into<String>) -> Self {
        let cfg = VirtualCameraConfig {
            device_name: name.into(),
            ..VirtualCameraConfig::default()
        };
        Self::new(cfg)
    }

    /// Ingests a real physical camera frame and fans it out concurrently to both consumers.
    pub fn feed_frame(
        &self,
        raw_frame: Arc<FrameBuffer>,
        frame_id: u64,
    ) -> Result<SplitterFanOutResult> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(MeetingError::SplitterNotRunning);
        }

        self.frames_fanned_out.fetch_add(1, Ordering::SeqCst);
        self.ring_buffer.publish_frame(raw_frame.clone(), frame_id);

        let mode_u8 = self.privacy_mode.load(Ordering::SeqCst);
        let mode = PrivacyMuteState::from_u8(mode_u8);
        let device = self.device_name();

        // Consumer 1: MediaPipe hand tracker
        // Receives raw frame in Active and PrivacySlate modes; receives None ONLY in DualMute.
        let (c1_frame, c1_tag) = if mode == PrivacyMuteState::DualMute {
            (None, None)
        } else {
            (Some(raw_frame.clone()), Some(format!("raw_frame_{}", frame_id)))
        };

        // Consumer 2: External meeting video
        // Receives privacy slate in PrivacySlate or DualMute; receives raw frame in Active.
        let (c2_frame, c2_tag) = if mode == PrivacyMuteState::Active {
            (raw_frame, format!("outbound_frame_{}", frame_id))
        } else {
            let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            (
                self.slate_generator.generate_slate_frame(frame_id, ts),
                "privacy_slate_frame".to_string(),
            )
        };

        Ok(SplitterFanOutResult {
            device,
            frame_id,
            consumer_1_frame: c1_frame,
            consumer_2_frame: Some(c2_frame),
            consumer_1_mediapipe: c1_tag,
            consumer_2_meeting: c2_tag,
            privacy_muted: mode.is_privacy_muted(),
            dual_muted: mode.is_dual_muted(),
        })
    }

    /// Feeds a synthetic camera frame by monotonic frame_id (used in tests and simulation).
    pub fn feed_camera_frame(&self, frame_id: u64) -> Result<SplitterFanOutResult> {
        let (w, h) = {
            let cfg = self.config.read().unwrap();
            (cfg.width(), cfg.height())
        };
        let mut frame = FrameBuffer::allocate(w, h, FrameFormat::Rgb8);
        // Stamp frame id into top-left pixel
        frame.set_pixel(0, 0, (frame_id & 0xFF) as u8, ((frame_id >> 8) & 0xFF) as u8, 0);
        self.feed_frame(Arc::new(frame), frame_id)
    }

    /// Directly and atomically sets the privacy mute mode.
    ///
    /// - `Active`: Both local MediaPipe tracking and outbound meeting video receive live camera frames.
    /// - `PrivacySlate`: Outbound meeting video receives synthetic privacy slate frames, while local MediaPipe
    ///   tracking strictly continues receiving raw physical frames.
    /// - `DualMute`: Both local MediaPipe tracking and outbound meeting video are muted.
    pub fn set_privacy_mode(&self, mode: PrivacyMuteState) {
        match mode {
            PrivacyMuteState::Active => {
                self.privacy_mute_active.store(false, Ordering::SeqCst);
            }
            PrivacyMuteState::PrivacySlate => {
                self.privacy_mute_active.store(true, Ordering::SeqCst);
            }
            PrivacyMuteState::DualMute => {}
        }
        self.privacy_mode.store(mode as u8, Ordering::SeqCst);
    }

    /// Sets the privacy mute state for Consumer 2 (meeting outbound).
    /// When privacy_muted is true, Consumer 2 outputs privacy slate frames,
    /// while Consumer 1 (MediaPipe) CONTINUES receiving raw frames.
    ///
    /// Transitions from DualMute to PrivacySlate safely re-enable Consumer 1 (local gestures)
    /// while keeping Consumer 2 obscured by the privacy slate.
    pub fn set_privacy_mute(&self, muted: bool) {
        self.privacy_mute_active.store(muted, Ordering::SeqCst);
        if muted {
            self.privacy_mode.store(PrivacyMuteState::PrivacySlate as u8, Ordering::SeqCst);
        } else {
            let _ = self.privacy_mode.compare_exchange(
                PrivacyMuteState::PrivacySlate as u8,
                PrivacyMuteState::Active as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
        }
    }

    /// Sets explicit dual mute, disabling both local gestures and outbound meeting stream.
    /// When dual_muted is false, restores PrivacySlate if privacy mute was active, or Active otherwise.
    pub fn set_dual_mute(&self, dual_muted: bool) {
        if dual_muted {
            self.privacy_mode.store(PrivacyMuteState::DualMute as u8, Ordering::SeqCst);
        } else if self.privacy_mute_active.load(Ordering::SeqCst) {
            self.privacy_mode.store(PrivacyMuteState::PrivacySlate as u8, Ordering::SeqCst);
        } else {
            self.privacy_mode.store(PrivacyMuteState::Active as u8, Ordering::SeqCst);
        }
    }

    /// Sets the running status of the splitter.
    pub fn set_running(&self, running: bool) {
        self.is_running.store(running, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn is_privacy_muted(&self) -> bool {
        let mode = PrivacyMuteState::from_u8(self.privacy_mode.load(Ordering::SeqCst));
        mode.is_privacy_muted()
    }

    pub fn is_dual_muted(&self) -> bool {
        let mode = PrivacyMuteState::from_u8(self.privacy_mode.load(Ordering::SeqCst));
        mode.is_dual_muted()
    }

    /// Returns whether Consumer 1 (local MediaPipe tracking) should process frames.
    /// Invariant: Remains true in Active AND PrivacySlate modes!
    pub fn is_consumer_1_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
            && self.privacy_mode.load(Ordering::SeqCst) != PrivacyMuteState::DualMute as u8
    }

    pub fn frames_fanned_out(&self) -> u64 {
        self.frames_fanned_out.load(Ordering::SeqCst)
    }

    pub fn device_name(&self) -> String {
        self.config.read().unwrap().device_name.clone()
    }

    pub fn config(&self) -> VirtualCameraConfig {
        self.config.read().unwrap().clone()
    }

    pub fn status(&self) -> VirtualCameraStatus {
        let cfg = self.config();
        let mode = PrivacyMuteState::from_u8(self.privacy_mode.load(Ordering::SeqCst));

        VirtualCameraStatus {
            is_running: self.is_running.load(Ordering::SeqCst),
            device_name: cfg.device_name.clone(),
            resolution: cfg.resolution,
            privacy_mode: mode,
            privacy_muted: mode.is_privacy_muted(),
            dual_muted: mode.is_dual_muted(),
            frames_fanned_out: self.frames_fanned_out.load(Ordering::SeqCst),
            target_fps: cfg.target_fps,
            actual_fps: cfg.target_fps as f32,
            consumer_count: 2,
        }
    }

    pub fn ring_buffer(&self) -> &Arc<ZeroLockVideoRingBuffer> {
        &self.ring_buffer
    }

    pub fn slate_generator(&self) -> &Arc<PrivacySlateGenerator> {
        &self.slate_generator
    }
}
