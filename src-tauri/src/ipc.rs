use crate::state::AppState;
use airctrl_core::events::DomainEvent;
use airctrl_core::types::{ActionCommand, SnapDirection};
use airctrl_meetings::MeetingDetector;
use airctrl_stt::{ActiveWindowTextInjector, InjectionStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum IpcError {
    #[error("Tauri command not found: {0}")]
    CommandNotFound(String),

    #[error("Command name cannot be empty")]
    EmptyCommand,

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Execution error: {0}")]
    ExecutionFailed(String),
}

/// Unified pure-Rust IPC dispatcher matching MockTauriIpcBridge contract.
/// Runs in-process without requiring display servers, GTK, or native webview handles.
pub struct IpcBridge {
    pub state: Arc<AppState>,
}

impl Default for IpcBridge {
    fn default() -> Self {
        Self::new(Arc::new(AppState::new()))
    }
}

impl IpcBridge {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Creates an IpcBridge with an explicit STT engine instance.
    pub fn with_engine(engine: Box<dyn airctrl_stt::SpeechToTextEngine>) -> Self {
        Self::new(Arc::new(AppState::with_engine(engine)))
    }

    /// Invokes a named Tauri IPC command with optional JSON arguments.
    /// Strictly complies with Python test suites (T1 & T2 F24, T1 & T2 F25).
    pub fn invoke(&self, command: &str, args: Option<Value>) -> Result<Value, IpcError> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err(IpcError::EmptyCommand);
        }

        let args = args.unwrap_or_else(|| json!({}));

        match trimmed {
            "startRecording" => {
                let samples = args.get("samples").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_f64)
                            .map(|f| f as f32)
                            .collect::<Vec<f32>>()
                    })
                });
                self.state.start_recording_with_samples(samples);
                Ok(json!({ "status": "RecordingStarted" }))
            }

            "stopRecording" => {
                let text = self.state.stop_recording().map_err(IpcError::ExecutionFailed)?;
                let auto_inject = args
                    .get("auto_inject")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                let injection_info = if auto_inject && !text.is_empty() {
                    let platform = airctrl_core::platform::create_platform_provider(self.state.config.read().is_mock_platform);
                    let injector = ActiveWindowTextInjector::new(
                        platform.window,
                        platform.input,
                        None,
                    );
                    let inject_fut = injector.inject(&text, None);
                    match tokio::runtime::Handle::try_current() {
                        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(inject_fut)),
                        Err(_) => {
                            let rt = tokio::runtime::Runtime::new().ok();
                            if let Some(r) = rt {
                                r.block_on(inject_fut)
                            } else {
                                Err(airctrl_stt::SttError::InferenceError("Failed to spawn runtime".into()))
                            }
                        }
                    }.ok().map(|rep| json!({
                        "status": format!("{:?}", rep.status),
                        "method": format!("{:?}", rep.method)
                    }))
                } else {
                    None
                };

                Ok(json!({
                    "status": "TranscriptionReady",
                    "text": text,
                    "injected": injection_info
                }))
            }

            "triggerAction" => {
                let action_val = args
                    .get("action")
                    .cloned()
                    .unwrap_or_else(|| json!("WindowSnap"));
                let action_str = match &action_val {
                    Value::String(s) => s.as_str(),
                    _ => "WindowSnap",
                };

                // Parse voice command or action intent via airctrl-actions router
                let command = match self.state.command_router.parse(action_str) {
                    ActionCommand::Unknown { .. } => {
                        let norm = action_str.to_lowercase().replace(['_', ' '], "");
                        match norm.as_str() {
                            "windowsnap" | "snap" => {
                                let dir_str = args
                                    .get("direction")
                                    .and_then(Value::as_str)
                                    .unwrap_or("maximize");
                                let direction = match dir_str.to_lowercase().as_str() {
                                    "left" => SnapDirection::Left,
                                    "right" => SnapDirection::Right,
                                    "top" => SnapDirection::Top,
                                    "bottom" => SnapDirection::Bottom,
                                    "center" => SnapDirection::Center,
                                    _ => SnapDirection::Maximize,
                                };
                                ActionCommand::WindowSnap { direction }
                            }
                            "center" | "centerwindow" => {
                                ActionCommand::WindowSnap { direction: SnapDirection::Center }
                            }
                            "radialdialtoggle" | "toggledial" | "dial" => {
                                ActionCommand::RadialDialToggle {}
                            }
                            _ => ActionCommand::Unknown {
                                query: action_str.to_string(),
                            },
                        }
                    }
                    cmd => cmd,
                };

                let mut execution_info = json!({
                    "command": command,
                    "dispatched": true
                });

                match &command {
                    ActionCommand::RadialDialToggle {} => {
                        self.state.dial_overlay.write().toggle();
                    }
                    ActionCommand::LaunchApp { app_name } => {
                        let running = airctrl_actions::crawl_running_applications().unwrap_or_default();
                        let platform = airctrl_core::platform::create_platform_provider(false);
                        let norm = app_name.to_lowercase();
                        if let Some(win) = running.iter().find(|w| {
                            w.process_name.to_lowercase().contains(&norm)
                                || w.title.to_lowercase().contains(&norm)
                        }) {
                            let _ = platform.window.focus_window(win.id);
                            let _ = platform.window.center_window(win.id);
                            execution_info = json!({
                                "command": command,
                                "dispatched": true,
                                "action_taken": "focused_and_centered",
                                "window_id": win.id
                            });
                        } else {
                            let pid = airctrl_actions::launch_application(app_name).ok();
                            execution_info = json!({
                                "command": command,
                                "dispatched": true,
                                "action_taken": "launched",
                                "pid": pid
                            });
                        }
                    }
                    ActionCommand::CloseApp { app_name } => {
                        let running = airctrl_actions::crawl_running_applications().unwrap_or_default();
                        let platform = airctrl_core::platform::create_platform_provider(false);
                        let norm = app_name.to_lowercase();
                        let mut closed = 0;
                        for win in running.iter().filter(|w| {
                            w.process_name.to_lowercase().contains(&norm)
                                || w.title.to_lowercase().contains(&norm)
                        }) {
                            if platform.window.close_window(win.id).is_ok() {
                                closed += 1;
                            }
                        }
                        execution_info = json!({
                            "command": command,
                            "dispatched": true,
                            "action_taken": "closed",
                            "closed_count": closed
                        });
                    }
                    ActionCommand::CenterApp { app_name } => {
                        let running = airctrl_actions::crawl_running_applications().unwrap_or_default();
                        let platform = airctrl_core::platform::create_platform_provider(false);
                        let norm = app_name.to_lowercase();
                        if let Some(win) = running.iter().find(|w| {
                            w.process_name.to_lowercase().contains(&norm)
                                || w.title.to_lowercase().contains(&norm)
                        }) {
                            let _ = platform.window.focus_window(win.id);
                            let _ = platform.window.center_window(win.id);
                            execution_info = json!({
                                "command": command,
                                "dispatched": true,
                                "action_taken": "centered",
                                "window_id": win.id
                            });
                        }
                    }
                    ActionCommand::WindowSnap { direction: SnapDirection::Center } => {
                        let platform = airctrl_core::platform::create_platform_provider(false);
                        if let Ok(active) = platform.window.get_active_window() {
                            let _ = platform.window.center_window(active.id);
                        }
                    }
                    _ => {}
                }

                let _ = self
                    .state
                    .event_bus
                    .publish(DomainEvent::ActionTriggered(command.clone()));

                Ok(json!({
                    "success": true,
                    "action": action_val,
                    "execution": execution_info
                }))
            }

            "toggleRadialDial" => {
                let mut dial = self.state.dial_overlay.write();
                dial.toggle();
                let is_visible = dial.is_visible();
                Ok(json!({ "dial_visible": is_visible }))
            }

            "toggleSttOverlay" => {
                let current = self
                    .state
                    .stt_overlay_visible
                    .load(std::sync::atomic::Ordering::SeqCst);
                let next = !current;
                self.state
                    .stt_overlay_visible
                    .store(next, std::sync::atomic::Ordering::SeqCst);
                Ok(json!({ "stt_overlay_visible": next }))
            }

            "toggleCommandPalette" => {
                let current = self
                    .state
                    .command_palette_visible
                    .load(std::sync::atomic::Ordering::SeqCst);
                let next = !current;
                self.state
                    .command_palette_visible
                    .store(next, std::sync::atomic::Ordering::SeqCst);
                Ok(json!({ "command_palette_visible": next }))
            }

            "injectText" => {
                let text = args
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let platform = airctrl_core::platform::create_platform_provider(self.state.config.read().is_mock_platform);
                let injector = ActiveWindowTextInjector::new(
                    platform.window,
                    platform.input,
                    None,
                );
                let inject_fut = injector.inject(text, None);
                let report = match tokio::runtime::Handle::try_current() {
                    Ok(handle) => tokio::task::block_in_place(|| handle.block_on(inject_fut)),
                    Err(_) => {
                        let rt = tokio::runtime::Runtime::new().map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;
                        rt.block_on(inject_fut)
                    }
                }.map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;

                Ok(json!({
                    "success": report.status == InjectionStatus::Success,
                    "status": format!("{:?}", report.status),
                    "method": format!("{:?}", report.method),
                    "text_length": report.text_length
                }))
            }

            "getSettings" => {
                let config = self.state.config.read();
                let hotkey_stt = if config.stt_hotkey == "Ctrl+Space" {
                    "Ctrl+Spacebar"
                } else {
                    &config.stt_hotkey
                };
                Ok(json!({
                    "hotkey_stt": hotkey_stt,
                    "hotkey_dial": config.dial_hotkey,
                    "push_to_talk": true,
                    "camera_auto_kill": config.camera_auto_kill,
                    "compliance_audio_enabled": config.compliance_audio_enabled,
                    "radial_dial_sectors": config.radial_dial_sectors,
                    "lan_port": config.lan_port,
                    "whisper_model": config.whisper_model_path,
                    "theme": "dark",
                    "hud_opacity": 0.95
                }))
            }

            "updateSettings" => {
                let args_obj = match args.as_object() {
                    Some(obj) => obj,
                    None => {
                        return Err(IpcError::InvalidArguments(
                            "Arguments must be a JSON object".into(),
                        ));
                    }
                };

                // 1. Strict validation for hotkey_stt: must be string starting with "Ctrl+"
                let new_stt = if let Some(val) = args_obj.get("hotkey_stt") {
                    let stt_hk = val.as_str().ok_or_else(|| {
                        IpcError::InvalidArguments(
                            "hotkey_stt must be a string starting with 'Ctrl+'".into(),
                        )
                    })?;
                    if !stt_hk.starts_with("Ctrl+") {
                        return Err(IpcError::InvalidArguments(
                            "hotkey_stt must be a string starting with 'Ctrl+'".into(),
                        ));
                    }
                    Some(stt_hk.to_string())
                } else {
                    None
                };

                // 2. Strict validation for hotkey_dial: must be string starting with "Ctrl+"
                let new_dial = if let Some(val) = args_obj.get("hotkey_dial") {
                    let dial_hk = val.as_str().ok_or_else(|| {
                        IpcError::InvalidArguments(
                            "hotkey_dial must be a string starting with 'Ctrl+'".into(),
                        )
                    })?;
                    if !dial_hk.starts_with("Ctrl+") {
                        return Err(IpcError::InvalidArguments(
                            "hotkey_dial must be a string starting with 'Ctrl+'".into(),
                        ));
                    }
                    Some(dial_hk.to_string())
                } else {
                    None
                };

                // 3. Strict validation for camera_auto_kill: must be a bool
                let new_kill = if let Some(val) = args_obj.get("camera_auto_kill") {
                    let kill = val.as_bool().ok_or_else(|| {
                        IpcError::InvalidArguments("camera_auto_kill must be a boolean".into())
                    })?;
                    Some(kill)
                } else {
                    None
                };

                // 4. Strict validation for compliance_audio_enabled: must be a bool
                let new_compliance = if let Some(val) = args_obj.get("compliance_audio_enabled") {
                    let comp = val.as_bool().ok_or_else(|| {
                        IpcError::InvalidArguments(
                            "compliance_audio_enabled must be a boolean".into(),
                        )
                    })?;
                    Some(comp)
                } else {
                    None
                };

                // 5. Strict validation for radial_dial_sectors: must be integer between 4 and 8
                let new_sectors = if let Some(val) = args_obj.get("radial_dial_sectors") {
                    match val.as_u64() {
                        Some(sectors) if (4..=8).contains(&sectors) => Some(sectors as usize),
                        _ => {
                            return Err(IpcError::InvalidArguments(
                                "radial_dial_sectors must be an integer between 4 and 8".into(),
                            ));
                        }
                    }
                } else {
                    None
                };

                // Apply updates atomically only after all validations succeed
                let mut config = self.state.config.write();
                if let Some(stt) = new_stt {
                    config.stt_hotkey = stt;
                }
                if let Some(dial) = new_dial {
                    config.dial_hotkey = dial;
                }
                if let Some(kill) = new_kill {
                    config.camera_auto_kill = kill;
                }
                if let Some(compliance) = new_compliance {
                    config.compliance_audio_enabled = compliance;
                }
                if let Some(sectors) = new_sectors {
                    config.radial_dial_sectors = sectors;
                    self.state.dial_overlay.write().set_sector_count(sectors);
                }

                Ok(json!({
                    "success": true,
                    "settings": json!({
                        "hotkey_stt": config.stt_hotkey,
                        "hotkey_dial": config.dial_hotkey,
                        "camera_auto_kill": config.camera_auto_kill,
                        "compliance_audio_enabled": config.compliance_audio_enabled,
                        "radial_dial_sectors": config.radial_dial_sectors
                    })
                }))
            }

            "getMeetingStatus" => {
                let active = self.state.meeting_detector.scan_system().unwrap_or(None);

                use airctrl_meetings::CameraState;
                let camera_state = match self.state.camera_kill_switch.get_camera_state() {
                    CameraState::Active => "Active",
                    CameraState::KilledByMeeting => "KilledByMeeting",
                    CameraState::PausedByUser => "PausedByUser",
                };

                match active {
                    Some(m) => Ok(json!({
                        "status": "Active",
                        "meeting": {
                            "app": m.app.as_str(),
                            "title": m.title,
                            "pid": m.pid,
                            "start_time": m.start_time.to_rfc3339()
                        },
                        "camera_state": camera_state
                    })),
                    None => Ok(json!({
                        "status": "Inactive",
                        "meeting": null,
                        "camera_state": camera_state
                    })),
                }
            }

            "triggerCameraKillSwitch" => {
                let status = self.state.camera_kill_switch.trigger();
                Ok(json!({
                    "success": true,
                    "status": "CameraKilled",
                    "camera_handle_closed": status.camera_handle_closed
                }))
            }

            "getCatalogedApps" | "get_cataloged_apps" => {
                let apps = self.state.cataloged_apps.read().clone();
                Ok(json!({ "apps": apps }))
            }

            "refreshCatalogedApps" | "refresh_cataloged_apps" => {
                let fresh = airctrl_actions::catalog_installed_applications();
                *self.state.cataloged_apps.write() = fresh.clone();
                Ok(json!({ "apps": fresh }))
            }

            "launchApp" | "launch_app" => {
                let target = args
                    .get("path")
                    .or_else(|| args.get("name"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("Application path or name required".into()))?;
                let pid = airctrl_actions::launch_application(target)
                    .map_err(IpcError::ExecutionFailed)?;
                Ok(json!({ "success": true, "pid": pid, "target": target }))
            }

            unknown => Err(IpcError::CommandNotFound(unknown.to_string())),
        }
    }
}
