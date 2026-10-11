//! # airctrl-meetings
//!
//! Intelligent meeting detection, audio stream monitoring, camera privacy auto-kill switch,
//! and compliance recording audio injection for `airctrl`.

pub mod audio;
pub mod compliance;
pub mod config;
pub mod detector;
pub mod error;
pub mod privacy;
pub mod types;
pub mod watcher;

// Re-exports
pub use audio::{
    compute_dbfs, compute_peak, compute_rms, AudioProcessCorrelator, AudioStreamMetrics,
    AudioStreamMonitor, CorrelatedState, HangoverStateMachine, SpeechState,
};
pub use compliance::{ComplianceAudioGenerator, ComplianceAudioInjector, MockAudioSink};
pub use config::{
    AudioMonitorConfig, ComplianceConfig, ComplianceMessageKind, MeetingWatcherConfig,
};
pub use detector::{
    browser::scan_browser_item, process::scan_dedicated_item, signatures::DEDICATED_SIGNATURES,
    DefaultMeetingDetector, MeetingDetector,
};
pub use error::{MeetingError, Result};
pub use privacy::CameraKillSwitch;
pub use types::{
    ActiveMeetingInfo, CameraState, ComplianceInjectionEvent, CompliancePlaybackResult,
    KillSwitchStatus, MeetingApp, MeetingInfo, MeetingPlatform, MeetingSignature, MeetingStatus,
    ProcessScanItem,
};
pub use watcher::{MeetingWatcher, WatcherHandle};

use std::sync::OnceLock;
use std::sync::Arc;

static GLOBAL_KILL_SWITCH: OnceLock<Arc<CameraKillSwitch>> = OnceLock::new();
static GLOBAL_INJECTOR: OnceLock<Arc<ComplianceAudioInjector>> = OnceLock::new();

/// Returns the shared global `CameraKillSwitch` instance.
pub fn get_global_camera_kill_switch() -> Arc<CameraKillSwitch> {
    GLOBAL_KILL_SWITCH
        .get_or_init(|| Arc::new(CameraKillSwitch::new()))
        .clone()
}

/// Returns the shared global `ComplianceAudioInjector` instance.
pub fn get_global_compliance_injector() -> Arc<ComplianceAudioInjector> {
    GLOBAL_INJECTOR
        .get_or_init(|| Arc::new(ComplianceAudioInjector::new()))
        .clone()
}

/// Launches the asynchronous meeting watcher background monitoring task.
pub fn start_meeting_watcher(config: MeetingWatcherConfig) -> Result<WatcherHandle> {
    let kill_switch = get_global_camera_kill_switch();
    let injector = get_global_compliance_injector();
    let watcher = MeetingWatcher::new(config)
        .with_kill_switch(kill_switch)
        .with_compliance_injector(injector);
    watcher.start()
}

/// Triggers the camera kill switch, guaranteeing sub-millisecond execution (<1.0ms)
/// and idempotent state handling.
pub fn trigger_camera_kill_switch() -> Result<KillSwitchStatus> {
    Ok(get_global_camera_kill_switch().trigger())
}

/// Plays the legal compliance disclosure recording announcement.
pub fn play_compliance_recording_announcement() -> Result<CompliancePlaybackResult> {
    let config = ComplianceConfig::default();
    get_global_compliance_injector().play_announcement(&config)
}

/// Convenience factory for creating a `DefaultMeetingDetector`.
pub fn create_meeting_detector() -> DefaultMeetingDetector {
    DefaultMeetingDetector::new()
}

/// Convenience factory for creating an `AudioStreamMonitor`.
pub fn create_audio_monitor(config: AudioMonitorConfig) -> AudioStreamMonitor {
    AudioStreamMonitor::new(config)
}

/// Convenience factory for creating a standalone `CameraKillSwitch`.
pub fn create_camera_kill_switch() -> CameraKillSwitch {
    CameraKillSwitch::new()
}

/// Convenience factory for creating a standalone `ComplianceAudioInjector`.
pub fn create_compliance_injector() -> ComplianceAudioInjector {
    ComplianceAudioInjector::new()
}
