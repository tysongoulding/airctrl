//! # airctrl-core
//!
//! Core domain models, shared events, error types, and platform abstraction
//! traits for the `airctrl` application suite.

pub mod config;
pub mod error;
pub mod events;
pub mod platform;
pub mod types;

// Re-export key abstractions at the crate root for ergonomics
pub use config::AirCtrlConfig;
pub use error::CoreError;
pub use events::{DomainEvent, EventBus};
pub use platform::{
    create_mock_platform, create_platform_provider, MockAudioOps, MockInputOps,
    MockPlatformControllers, MockWindowOps, PlatformAudioOps, PlatformInputOps, PlatformProvider,
    PlatformWindowOps,
};
pub use types::*;
