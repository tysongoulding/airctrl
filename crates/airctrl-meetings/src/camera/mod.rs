//! Continuous Virtual Camera Splitter, zero-lock ring buffer, and synthetic privacy slate.

pub mod buffer;
pub mod slate;
pub mod splitter;

pub use buffer::{RingSlot, ZeroLockVideoRingBuffer, RING_CAPACITY, RING_MASK};
pub use slate::{PrivacySlateGenerator, PrivacySlateRenderer};
pub use splitter::{FanOutFrameResult, SplitterFanOutResult, VirtualCameraSplitter};
