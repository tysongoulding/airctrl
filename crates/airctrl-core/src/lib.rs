//! # airctrl-core
//!
//! Core domain models, shared events, error types, and platform abstraction
//! traits for the `airctrl` application suite.

pub mod config;
pub mod dag;
pub mod error;
pub mod events;
pub mod platform;
pub mod types;

// Re-export key abstractions at the crate root for ergonomics
pub use config::AirCtrlConfig;
pub use dag::*;
pub use error::CoreError;
pub use events::{DomainEvent, EventBus};
pub use platform::{
    create_mock_platform, create_platform_provider, MockAudioOps, MockCameraOps, MockInputOps,
    MockPlatformControllers, MockWindowOps, PhysicalCameraStream, PlatformAudioOps,
    PlatformCameraOps, PlatformInputOps, PlatformProvider, PlatformWindowOps,
    VirtualCameraDeviceSink,
};
pub use types::*;

