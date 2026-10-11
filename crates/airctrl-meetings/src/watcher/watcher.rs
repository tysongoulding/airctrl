use super::handle::WatcherHandle;
use crate::compliance::ComplianceAudioInjector;
use crate::config::MeetingWatcherConfig;
use crate::detector::{DefaultMeetingDetector, MeetingDetector};
use crate::error::MeetingError;
use crate::privacy::CameraKillSwitch;
use crate::types::{
    ActiveMeetingInfo, CameraState, ComplianceInjectionEvent, CompliancePlaybackResult,
    KillSwitchStatus, MeetingInfo, MeetingStatus, ProcessScanItem,
};
use airctrl_core::events::{DomainEvent, EventBus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Real-time meeting watcher monitoring process and window signatures,
/// controlling camera privacy auto-kill switch, and managing compliance audio.
pub struct MeetingWatcher {
    config: MeetingWatcherConfig,
    detector: Arc<dyn MeetingDetector>,
    kill_switch: Arc<CameraKillSwitch>,
    compliance_injector: Arc<ComplianceAudioInjector>,
    event_bus: Option<EventBus>,
    status: Arc<RwLock<MeetingStatus>>,
    active_meeting: Arc<RwLock<Option<ActiveMeetingInfo>>>,
    compliance_audio_played: Arc<AtomicBool>,
}

impl Default for MeetingWatcher {
    fn default() -> Self {
        Self::new(MeetingWatcherConfig::default())
    }
}

impl MeetingWatcher {
    /// Creates a new `MeetingWatcher` with the given configuration.
    pub fn new(config: MeetingWatcherConfig) -> Self {
        Self {
            config,
            detector: Arc::new(DefaultMeetingDetector::new()),
            kill_switch: Arc::new(CameraKillSwitch::new()),
            compliance_injector: Arc::new(ComplianceAudioInjector::new()),
            event_bus: None,
            status: Arc::new(RwLock::new(MeetingStatus::Inactive)),
            active_meeting: Arc::new(RwLock::new(None)),
            compliance_audio_played: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Attaches an `EventBus` for domain event publication.
    pub fn with_event_bus(mut self, bus: EventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Attaches a custom `MeetingDetector`.
    pub fn with_detector(mut self, detector: Arc<dyn MeetingDetector>) -> Self {
        self.detector = detector;
        self
    }

    /// Attaches an existing `CameraKillSwitch`.
    pub fn with_kill_switch(mut self, kill_switch: Arc<CameraKillSwitch>) -> Self {
        self.kill_switch = kill_switch;
        self
    }

    /// Attaches an existing `ComplianceAudioInjector`.
    pub fn with_compliance_injector(mut self, injector: Arc<ComplianceAudioInjector>) -> Self {
        self.compliance_injector = injector;
        self
    }

    /// Evaluates an explicit slice of process scan items.
    ///
    /// If an active meeting is discovered, triggers the camera kill switch (if enabled)
    /// and returns the `MeetingInfo`.
    pub fn scan_processes(&self, procs: &[ProcessScanItem]) -> Option<MeetingInfo> {
        if let Some(active) = self.detector.detect_meeting(procs) {
            let info: MeetingInfo = active.clone().into();

            if self.config.auto_kill_camera {
                self.kill_switch.trigger();
            }

            if let Ok(mut cur) = self.active_meeting.write() {
                *cur = Some(active);
            }
            if let Ok(mut st) = self.status.write() {
                *st = MeetingStatus::Active;
            }

            Some(info)
        } else {
            None
        }
    }

    /// Directly triggers the camera kill switch.
    pub fn trigger_camera_kill_switch(&self) -> KillSwitchStatus {
        self.kill_switch.trigger()
    }

    /// Directly plays the legal compliance disclosure announcement.
    pub fn play_compliance_recording_announcement(
        &self,
    ) -> Result<CompliancePlaybackResult, MeetingError> {
        let result = self
            .compliance_injector
            .play_announcement(&self.config.compliance_config)?;
        self.compliance_audio_played.store(true, Ordering::SeqCst);
        Ok(result)
    }

    /// Returns `true` if camera capture is currently active.
    pub fn is_camera_active(&self) -> bool {
        self.kill_switch.is_camera_active()
    }

    /// Returns `true` if compliance disclosure audio has been played.
    pub fn is_compliance_audio_played(&self) -> bool {
        self.compliance_audio_played.load(Ordering::SeqCst)
    }

    /// Returns the current lifecycle status of the meeting watcher.
    pub fn get_status(&self) -> MeetingStatus {
        self.status
            .read()
            .map(|s| *s)
            .unwrap_or(MeetingStatus::Inactive)
    }

    /// Returns the active meeting details, if any.
    pub fn get_active_meeting(&self) -> Option<ActiveMeetingInfo> {
        self.active_meeting
            .read()
            .ok()
            .and_then(|m| m.clone())
    }

    /// Starts the asynchronous polling loop in a background Tokio task.
    ///
    /// Implements 2-tick debounce hysteresis:
    /// - Meeting discovery transitions `Inactive` -> `Detected` -> `Active`
    /// - Meeting termination transitions `Active` -> `Ended` -> `Inactive`
    pub fn start(&self) -> Result<WatcherHandle, MeetingError> {
        let cancellation_token = CancellationToken::new();
        let is_running_flag = Arc::new(AtomicBool::new(true));

        let token = cancellation_token.clone();
        let running_flag = is_running_flag.clone();
        let status = self.status.clone();
        let active_meeting = self.active_meeting.clone();
        let detector = self.detector.clone();
        let kill_switch = self.kill_switch.clone();
        let compliance_injector = self.compliance_injector.clone();
        let event_bus = self.event_bus.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut appear_ticks: u32 = 0;
            let mut disappear_ticks: u32 = 0;
            let poll_duration = Duration::from_millis(config.poll_interval_ms);

            while !token.is_cancelled() {
                // Poll system processes
                let detected = match detector.scan_system() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("Meeting detector scan error: {e}");
                        None
                    }
                };

                let current_status = status.read().map(|s| *s).unwrap_or(MeetingStatus::Inactive);

                match detected {
                    Some(meeting) => {
                        disappear_ticks = 0;
                        appear_ticks += 1;

                        if appear_ticks >= config.debounce_ticks
                            && (current_status == MeetingStatus::Inactive
                                || current_status == MeetingStatus::Ended)
                        {
                            // Transition: Detected
                            if let Ok(mut st) = status.write() {
                                *st = MeetingStatus::Detected;
                            }
                            if let Some(ref bus) = event_bus {
                                let _ = bus.publish(DomainEvent::MeetingStatusChanged(
                                    MeetingStatus::Detected,
                                ));
                            }

                            let info: MeetingInfo = meeting.clone().into();
                            if let Some(ref bus) = event_bus {
                                let _ = bus.publish(DomainEvent::MeetingDetected(info.clone()));
                            }

                            // Transition: Active
                            if let Ok(mut st) = status.write() {
                                *st = MeetingStatus::Active;
                            }
                            if let Ok(mut cur) = active_meeting.write() {
                                *cur = Some(meeting);
                            }
                            if let Some(ref bus) = event_bus {
                                let _ = bus.publish(DomainEvent::MeetingStatusChanged(
                                    MeetingStatus::Active,
                                ));
                            }

                            // Auto kill camera switch if configured
                            if config.auto_kill_camera {
                                kill_switch.trigger();
                                if let Some(ref bus) = event_bus {
                                    let _ = bus.publish(DomainEvent::CameraKillSwitchTriggered {
                                        meeting: info.clone(),
                                        camera_state: CameraState::KilledByMeeting,
                                    });
                                }
                            }

                            // Auto compliance audio announcement if configured
                            if config.auto_compliance_audio {
                                if let Ok(res) =
                                    compliance_injector.play_announcement(&config.compliance_config)
                                {
                                    if let Some(ref bus) = event_bus {
                                        let _ = bus.publish(DomainEvent::ComplianceAudioInjected(
                                            ComplianceInjectionEvent {
                                                meeting_info: info,
                                                message: res.announcement,
                                                audio_duration_ms: res.duration_ms,
                                                status: res.status,
                                            },
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        appear_ticks = 0;

                        if current_status == MeetingStatus::Active
                            || current_status == MeetingStatus::Detected
                        {
                            disappear_ticks += 1;

                            if disappear_ticks >= config.debounce_ticks {
                                // Transition: Ended
                                if let Ok(mut st) = status.write() {
                                    *st = MeetingStatus::Ended;
                                }
                                if let Some(ref bus) = event_bus {
                                    let _ = bus.publish(DomainEvent::MeetingStatusChanged(
                                        MeetingStatus::Ended,
                                    ));
                                }

                                let last_info: Option<MeetingInfo> = active_meeting
                                    .read()
                                    .ok()
                                    .and_then(|m| m.clone())
                                    .map(|m| m.into());

                                if let Some(info) = last_info {
                                    if let Some(ref bus) = event_bus {
                                        let _ = bus.publish(DomainEvent::MeetingEnded(info));
                                    }
                                }

                                // Transition: Inactive
                                if let Ok(mut st) = status.write() {
                                    *st = MeetingStatus::Inactive;
                                }
                                if let Ok(mut cur) = active_meeting.write() {
                                    *cur = None;
                                }
                                if let Some(ref bus) = event_bus {
                                    let _ = bus.publish(DomainEvent::MeetingStatusChanged(
                                        MeetingStatus::Inactive,
                                    ));
                                }
                            }
                        }
                    }
                }

                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(poll_duration) => {}
                }
            }

            running_flag.store(false, Ordering::SeqCst);
        });

        Ok(WatcherHandle::new(
            cancellation_token,
            is_running_flag,
            self.status.clone(),
            self.active_meeting.clone(),
        ))
    }
}
