use super::metrics::MeetingWatcherHealthMetrics;
use airctrl_core::dag::{InferenceEvent, KillCameraPayload, PlayDisclosurePayload, ProcessScanPayload, SensorEvent, SinkAction};
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::audio::ComplianceAudioSpec;
use airctrl_core::types::meeting::{
    ActiveMeetingInfo, CameraState, ComplianceInjectionEvent, MeetingInfo, MeetingStatus,
};
use airctrl_core::types::window::ProcessInfo;
use airctrl_meetings::{
    CameraKillSwitch, ComplianceAudioInjector, DefaultMeetingDetector, MeetingDetector,
    MeetingWatcherConfig, ProcessScanItem,
};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct MeetingWatcherLoop {
    pub config: MeetingWatcherConfig,
    pub detector: Arc<dyn MeetingDetector>,
    pub kill_switch: Arc<CameraKillSwitch>,
    pub compliance_injector: Arc<ComplianceAudioInjector>,
    pub event_bus: EventBus,
    pub status: Arc<RwLock<MeetingStatus>>,
    pub active_meeting: Arc<RwLock<Option<ActiveMeetingInfo>>>,
    pub metrics: Arc<RwLock<MeetingWatcherHealthMetrics>>,
    pub scan_override: Option<Arc<RwLock<Option<Vec<ProcessScanItem>>>>>,
}

impl MeetingWatcherLoop {
    pub fn new(
        config: MeetingWatcherConfig,
        kill_switch: Arc<CameraKillSwitch>,
        compliance_injector: Arc<ComplianceAudioInjector>,
        event_bus: EventBus,
        metrics: Arc<RwLock<MeetingWatcherHealthMetrics>>,
    ) -> Self {
        Self {
            config,
            detector: Arc::new(DefaultMeetingDetector::new()),
            kill_switch,
            compliance_injector,
            event_bus,
            status: Arc::new(RwLock::new(MeetingStatus::Inactive)),
            active_meeting: Arc::new(RwLock::new(None)),
            metrics,
            scan_override: None,
        }
    }

    pub fn with_scan_override(mut self, scan_override: Arc<RwLock<Option<Vec<ProcessScanItem>>>>) -> Self {
        self.scan_override = Some(scan_override);
        self
    }

    pub async fn run(self, token: CancellationToken) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(self.config.poll_interval_ms));
        let mut appear_ticks: u32 = 0;
        let mut disappear_ticks: u32 = 0;

        self.metrics.write().is_running = true;

        while !token.is_cancelled() {
            tokio::select! {
                _ = token.cancelled() => break,
                _ = interval.tick() => {
                    let start_t = tokio::time::Instant::now();
                    self.metrics.write().poll_count += 1;

                    // Scan processes
                    let (detected, raw_items) = if let Some(ref ov) = self.scan_override {
                        let guard = ov.read();
                        if let Some(ref procs) = *guard {
                            (self.detector.detect_meeting(procs), Some(procs.clone()))
                        } else {
                            (self.detector.scan_system().unwrap_or(None), None)
                        }
                    } else {
                        (self.detector.scan_system().unwrap_or(None), None)
                    };

                    let now_ms = chrono::Utc::now().timestamp_millis() as u64;

                    // Publish ProcessScan sensor event if raw items available
                    if let Some(items) = raw_items {
                        let process_infos: Vec<ProcessInfo> = items.into_iter().map(|item| ProcessInfo {
                            pid: item.process_id,
                            name: item.process_name,
                            exe_path: None,
                            cpu_usage: 0.0,
                            memory_bytes: 0,
                            window_ids: vec![],
                        }).collect();
                        let _ = self.event_bus.publish_sensor(SensorEvent::ProcessScan(ProcessScanPayload {
                            processes: process_infos,
                            timestamp_ns: now_ms * 1_000_000,
                        }));
                    }

                    let current_status = *self.status.read();

                    match detected {
                        Some(meeting) => {
                            disappear_ticks = 0;
                            appear_ticks += 1;

                            if appear_ticks >= self.config.debounce_ticks
                                && (current_status == MeetingStatus::Inactive || current_status == MeetingStatus::Ended)
                            {
                                *self.status.write() = MeetingStatus::Detected;
                                let _ = self.event_bus.publish(DomainEvent::MeetingStatusChanged(MeetingStatus::Detected));

                                let info: MeetingInfo = meeting.clone().into();
                                let _ = self.event_bus.publish_inference(InferenceEvent::MeetingDetected(info.clone()));
                                let _ = self.event_bus.publish(DomainEvent::MeetingDetected(info.clone()));

                                *self.status.write() = MeetingStatus::Active;
                                *self.active_meeting.write() = Some(meeting);
                                let _ = self.event_bus.publish(DomainEvent::MeetingStatusChanged(MeetingStatus::Active));

                                // Camera Kill-Switch
                                if self.config.auto_kill_camera {
                                    self.kill_switch.trigger();
                                    let _ = self.event_bus.publish_sink(SinkAction::KillCamera(KillCameraPayload {
                                        meeting: info.clone(),
                                        reason: "Automatic meeting discovery privacy kill-switch".into(),
                                        force_hardware_release: true,
                                    }));
                                    let _ = self.event_bus.publish(DomainEvent::CameraKillSwitchTriggered {
                                        meeting: info.clone(),
                                        camera_state: CameraState::KilledByMeeting,
                                    });
                                }

                                // Compliance Audio Injection
                                if self.config.auto_compliance_audio {
                                    if let Ok(res) = self.compliance_injector.play_announcement(&self.config.compliance_config) {
                                        let _ = self.event_bus.publish_sink(SinkAction::PlayDisclosure(PlayDisclosurePayload {
                                            spec: ComplianceAudioSpec {
                                                announcement_text: res.announcement.clone(),
                                                sample_rate: self.config.compliance_config.sample_rate,
                                                volume: self.config.compliance_config.volume,
                                            },
                                            meeting: Some(info.clone()),
                                        }));
                                        let _ = self.event_bus.publish(DomainEvent::ComplianceAudioInjected(
                                            ComplianceInjectionEvent {
                                                meeting_info: info,
                                                message: res.announcement,
                                                audio_duration_ms: res.duration_ms,
                                                status: res.status,
                                            }
                                        ));
                                        self.metrics.write().compliance_audio_played = true;
                                    }
                                }
                            }
                        }
                        None => {
                            appear_ticks = 0;
                            if current_status == MeetingStatus::Active || current_status == MeetingStatus::Detected {
                                disappear_ticks += 1;
                                if disappear_ticks >= self.config.debounce_ticks {
                                    *self.status.write() = MeetingStatus::Ended;
                                    let _ = self.event_bus.publish(DomainEvent::MeetingStatusChanged(MeetingStatus::Ended));

                                    let last_info = self.active_meeting.read().clone().map(Into::into);
                                    if let Some(info) = last_info {
                                        let _ = self.event_bus.publish(DomainEvent::MeetingEnded(info));
                                    }

                                    *self.status.write() = MeetingStatus::Inactive;
                                    *self.active_meeting.write() = None;
                                    let _ = self.event_bus.publish(DomainEvent::MeetingStatusChanged(MeetingStatus::Inactive));
                                }
                            }
                        }
                    }

                    // Update metrics
                    {
                        let mut m = self.metrics.write();
                        m.status = *self.status.read();
                        m.active_meeting = self.active_meeting.read().clone().map(Into::into);
                        m.camera_state = self.kill_switch.get_camera_state();
                        m.last_poll_duration_ms = start_t.elapsed().as_millis() as u64;
                        m.last_tick_timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
                    }
                }
            }
        }

        self.metrics.write().is_running = false;
    }
}
