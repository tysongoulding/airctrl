//! Pluggable Hand Landmark Inference Engine Abstraction.

pub mod mock;
pub use mock::{MockMediaPipeEngine, MockPose};

use crate::error::Result;
use crate::frame::FrameBufferRef;
use airctrl_core::types::gesture::{HandLandmarks, MultiHandLandmarks};

/// Pluggable CPU Hand Landmark inference engine contract.
pub trait HandLandmarkEngine: Send + Sync {
    /// Detects hand landmarks from an input raw frame buffer.
    /// Returns `Ok(Some(HandLandmarks))` if a hand is detected,
    /// `Ok(None)` if no hand is detected in the frame,
    /// or `Err(GestureError)` if decoding/inference fails.
    fn process_frame(&self, frame: &FrameBufferRef<'_>) -> Result<Option<HandLandmarks>>;

    /// Detects landmarks for up to 2 hands concurrently from an input frame buffer.
    fn process_frame_multi(&self, frame: &FrameBufferRef<'_>) -> Result<MultiHandLandmarks> {
        let single = self.process_frame(frame)?;
        Ok(match single {
            Some(hand) => MultiHandLandmarks::from_hands(vec![hand]),
            None => MultiHandLandmarks::default(),
        })
    }

    /// Returns whether the engine model weights and runtime are initialized.
    fn is_ready(&self) -> bool;

    /// Returns human-readable engine name and version.
    fn name(&self) -> &str;
}

