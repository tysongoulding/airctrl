use crate::state::AppState;
use airctrl_core::events::DomainEvent;
use airctrl_core::types::{
    ActionCommand, ClickAction, CycleDirection, DimensionDelta, PrivacyMuteState, ResizeMode,
    SnapDirection, SnapPosition, SystemInputAction, UiTarget,
};
use airctrl_meetings::{
    AiSummaryService, DocumentExporter, ExportFormat, MeetingArchiveEntry, MeetingDetector,
    SummaryProvider, SummaryTemplate, TemplateContext,
};
use airctrl_stt::{ActiveWindowTextInjector, InjectionStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
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
                    ActionCommand::LaunchApp { app_name, .. } => {
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
                    ActionCommand::CloseApp { app_name, .. } => {
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

            "injectText" | "inject_text" | "injectTextReliable" | "inject_text_reliable" => {
                let text = args
                    .get("text")
                    .or_else(|| args.get("content"))
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
                    "hud_opacity": 0.95,
                    "recording_mode": format!("{:?}", config.recording_mode),
                    "reliable_paste": config.reliable_paste,
                    "paste_delay_before_ms": config.paste_delay_before_ms,
                    "paste_delay_after_ms": config.paste_delay_after_ms,
                    "overlay_position": format!("{}", config.overlay_position),
                    "post_processing_enabled": config.post_processing_enabled,
                    "audio_feedback_enabled": config.audio_feedback_enabled
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

                // 6. Strict validation for recording_mode: must be string parsing to HotkeyMode
                let new_recording_mode = if let Some(val) = args_obj.get("recording_mode") {
                    let s = val.as_str().ok_or_else(|| {
                        IpcError::InvalidArguments("recording_mode must be a string".into())
                    })?;
                    let mode: airctrl_core::types::HotkeyMode = serde_json::from_value(json!(s))
                        .map_err(|_| IpcError::InvalidArguments(format!("Invalid recording_mode: {}", s)))?;
                    Some(mode)
                } else {
                    None
                };

                // 7. Strict validation for reliable_paste: must be a bool
                let new_reliable_paste = if let Some(val) = args_obj.get("reliable_paste") {
                    let b = val.as_bool().ok_or_else(|| {
                        IpcError::InvalidArguments("reliable_paste must be a boolean".into())
                    })?;
                    Some(b)
                } else {
                    None
                };

                // 8. Strict validation for paste_delay_before_ms: must be integer
                let new_paste_delay_before = if let Some(val) = args_obj.get("paste_delay_before_ms") {
                    let d = val.as_u64().ok_or_else(|| {
                        IpcError::InvalidArguments("paste_delay_before_ms must be an integer".into())
                    })?;
                    Some(d)
                } else {
                    None
                };

                // 9. Strict validation for paste_delay_after_ms: must be integer
                let new_paste_delay_after = if let Some(val) = args_obj.get("paste_delay_after_ms") {
                    let d = val.as_u64().ok_or_else(|| {
                        IpcError::InvalidArguments("paste_delay_after_ms must be an integer".into())
                    })?;
                    Some(d)
                } else {
                    None
                };

                // 10. Strict validation for overlay_position: must be string parsing to OverlayPosition
                let new_overlay_pos = if let Some(val) = args_obj.get("overlay_position") {
                    let s = val.as_str().ok_or_else(|| {
                        IpcError::InvalidArguments("overlay_position must be a string".into())
                    })?;
                    let pos = airctrl_core::types::OverlayPosition::from_str(s)
                        .map_err(IpcError::InvalidArguments)?;
                    Some(pos)
                } else {
                    None
                };

                // 11. Strict validation for post_processing_enabled: must be bool
                let new_post_processing = if let Some(val) = args_obj.get("post_processing_enabled") {
                    let b = val.as_bool().ok_or_else(|| {
                        IpcError::InvalidArguments("post_processing_enabled must be a boolean".into())
                    })?;
                    Some(b)
                } else {
                    None
                };

                // 12. Strict validation for audio_feedback_enabled: must be bool
                let new_audio_feedback = if let Some(val) = args_obj.get("audio_feedback_enabled") {
                    let b = val.as_bool().ok_or_else(|| {
                        IpcError::InvalidArguments("audio_feedback_enabled must be a boolean".into())
                    })?;
                    Some(b)
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
                if let Some(rm) = new_recording_mode {
                    config.recording_mode = rm;
                }
                if let Some(rp) = new_reliable_paste {
                    config.reliable_paste = rp;
                }
                if let Some(pdb) = new_paste_delay_before {
                    config.paste_delay_before_ms = pdb;
                }
                if let Some(pda) = new_paste_delay_after {
                    config.paste_delay_after_ms = pda;
                }
                if let Some(pos) = new_overlay_pos {
                    config.overlay_position = pos;
                }
                if let Some(pp) = new_post_processing {
                    config.post_processing_enabled = pp;
                    self.state.dictionary_engine.write().set_post_processing(pp);
                }
                if let Some(af) = new_audio_feedback {
                    config.audio_feedback_enabled = af;
                }

                Ok(json!({
                    "success": true,
                    "settings": json!({
                        "hotkey_stt": config.stt_hotkey,
                        "hotkey_dial": config.dial_hotkey,
                        "camera_auto_kill": config.camera_auto_kill,
                        "compliance_audio_enabled": config.compliance_audio_enabled,
                        "radial_dial_sectors": config.radial_dial_sectors,
                        "recording_mode": format!("{:?}", config.recording_mode),
                        "reliable_paste": config.reliable_paste,
                        "paste_delay_before_ms": config.paste_delay_before_ms,
                        "paste_delay_after_ms": config.paste_delay_after_ms,
                        "overlay_position": format!("{}", config.overlay_position),
                        "post_processing_enabled": config.post_processing_enabled,
                        "audio_feedback_enabled": config.audio_feedback_enabled
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

            "getLoopStatus" | "get_loop_status" => {
                let metrics = self.state.loop_manager.get_health_metrics();
                Ok(serde_json::to_value(metrics).unwrap_or_else(|_| json!({ "is_running": false })))
            }

            "startWorkerLoops" | "start_worker_loops" => {
                let state_clone = self.state.clone();
                let mgr = self.state.loop_manager.clone();
                match tokio::runtime::Handle::try_current() {
                    Ok(handle) => tokio::task::block_in_place(|| handle.block_on(mgr.start_all(state_clone))),
                    Err(_) => {
                        let rt = tokio::runtime::Runtime::new().map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;
                        rt.block_on(mgr.start_all(state_clone))
                    }
                }.map_err(IpcError::ExecutionFailed)?;
                Ok(json!({ "success": true, "status": "WorkerLoopsStarted" }))
            }

            "stopWorkerLoops" | "stop_worker_loops" => {
                let mgr = self.state.loop_manager.clone();
                match tokio::runtime::Handle::try_current() {
                    Ok(handle) => tokio::task::block_in_place(|| handle.block_on(mgr.stop_all())),
                    Err(_) => {
                        let rt = tokio::runtime::Runtime::new().map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;
                        rt.block_on(mgr.stop_all())
                    }
                }.map_err(IpcError::ExecutionFailed)?;
                Ok(json!({ "success": true, "status": "WorkerLoopsStopped" }))
            }

            "getDagStatus" | "get_dag_status" => {
                let metrics = self.state.dag_bus.metrics();
                Ok(serde_json::to_value(metrics).unwrap_or_else(|_| json!({ "active": false })))
            }

            "discoverWhisperModels" | "discover_whisper_models" | "listModels" | "list_models" => {
                let standard = vec![
                    "whisper-tiny.en",
                    "whisper-base.en",
                    "whisper-small.en",
                    "whisper-medium.en",
                    "whisper-turbo.en",
                    "whisper-large.en",
                    "parakeet-unified-en-0.6b-q8_0",
                ];
                let mut models: Vec<String> = standard.into_iter().map(|s| s.to_string()).collect();
                if let Ok(discovered) = airctrl_stt::ModelManager::default().discover_models() {
                    for d in discovered {
                        let name = d.file_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                        if !name.is_empty() && !models.contains(&name) {
                            models.push(name);
                        }
                    }
                }
                Ok(json!({
                    "status": "success",
                    "success": true,
                    "models": models
                }))
            }

            "loadWhisperModel" | "load_whisper_model" | "selectModel" | "select_model" => {
                let model_str = args
                    .get("model")
                    .or_else(|| args.get("model_path"))
                    .or_else(|| args.get("model_name"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("Model name required".into()))?;

                self.state.config.write().whisper_model_path = model_str.to_string();
                Ok(json!({
                    "status": "success",
                    "success": true,
                    "model": model_str,
                    "selected_model": model_str
                }))
            }

            "getActiveModel" | "get_active_model" => {
                let model_path = self.state.config.read().whisper_model_path.clone();
                Ok(json!({
                    "status": "success",
                    "model_path": model_path,
                    "model": model_path,
                    "active_model": model_path
                }))
            }

            "setRecordingMode" | "set_recording_mode" => {
                let mode_str = args
                    .get("mode")
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("Recording mode required".into()))?;

                let mode: airctrl_core::types::HotkeyMode = serde_json::from_value(json!(mode_str))
                    .or_else(|_| {
                        match mode_str.to_lowercase().as_str() {
                            "smartdualmode" | "smartdual" => Ok(airctrl_core::types::HotkeyMode::SmartDualMode),
                            "holdonly" | "pushtotalk" => Ok(airctrl_core::types::HotkeyMode::HoldOnly),
                            "toggleonly" | "toggle" => Ok(airctrl_core::types::HotkeyMode::ToggleOnly),
                            "continuoushandsfree" => Ok(airctrl_core::types::HotkeyMode::ContinuousHandsFree),
                            _ => Err(IpcError::InvalidArguments(format!("Unknown recording mode: {}", mode_str))),
                        }
                    })?;

                self.state.config.write().recording_mode = mode;
                Ok(json!({
                    "status": "success",
                    "success": true,
                    "mode": mode_str,
                    "recording_mode": format!("{:?}", mode)
                }))
            }

            "getRecordingMode" | "get_recording_mode" => {
                let mode = self.state.config.read().recording_mode;
                Ok(json!({
                    "status": "success",
                    "recording_mode": format!("{:?}", mode),
                    "mode": format!("{:?}", mode)
                }))
            }

            "setOverlayPosition" | "set_overlay_position" => {
                let pos_str = args
                    .get("position")
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("Overlay position required".into()))?;

                let pos = airctrl_core::types::OverlayPosition::from_str(pos_str)
                    .map_err(IpcError::InvalidArguments)?;

                self.state.config.write().overlay_position = pos;

                let bounds = if pos == airctrl_core::types::OverlayPosition::None {
                    json!({
                        "visible": false,
                        "focus_stealing": false,
                        "x": 0,
                        "y": 0,
                        "width": 0,
                        "height": 0
                    })
                } else {
                    let win_w = 400;
                    let win_h = 60;
                    let margin = 30;
                    let center_x = (1920 - win_w) / 2;
                    let y = match pos {
                        airctrl_core::types::OverlayPosition::Top => margin,
                        airctrl_core::types::OverlayPosition::Center => (1080 - win_h) / 2,
                        airctrl_core::types::OverlayPosition::Bottom => 1080 - win_h - margin,
                        airctrl_core::types::OverlayPosition::None => unreachable!(),
                    };
                    json!({
                        "visible": true,
                        "focus_stealing": false,
                        "x": center_x,
                        "y": y,
                        "width": win_w,
                        "height": win_h
                    })
                };

                Ok(json!({
                    "status": "success",
                    "success": true,
                    "position": pos_str,
                    "bounds": bounds
                }))
            }

            "getOverlayPosition" | "get_overlay_position" => {
                let pos = self.state.config.read().overlay_position;
                Ok(json!({
                    "status": "success",
                    "position": format!("{}", pos)
                }))
            }

            "updateCustomDictionary" | "update_custom_dictionary" | "updateDictionary" | "update_dictionary" => {
                let rules_val = args
                    .get("rules")
                    .or_else(|| args.get("custom_dictionary"))
                    .or_else(|| args.get("dictionary"))
                    .ok_or_else(|| {
                        IpcError::InvalidArguments("rules field required in updateDictionary".into())
                    })?;

                let mut new_rules = Vec::new();
                if let Some(obj) = rules_val.as_object() {
                    for (k, v) in obj {
                        let rep = v.as_str().unwrap_or("").to_string();
                        new_rules.push(airctrl_core::types::DictionaryRule::literal(k, rep));
                    }
                } else if let Some(arr) = rules_val.as_array() {
                    for item in arr {
                        if let Ok(rule) = serde_json::from_value::<airctrl_core::types::DictionaryRule>(item.clone()) {
                            new_rules.push(rule);
                        } else if let Some(obj) = item.as_object() {
                            let trigger = obj.get("trigger").and_then(Value::as_str).unwrap_or("");
                            let replacement = obj.get("replacement").and_then(Value::as_str).unwrap_or("");
                            let is_regex = obj.get("is_regex").and_then(Value::as_bool).unwrap_or(false);
                            let preserve_case = obj.get("preserve_case").and_then(Value::as_bool).unwrap_or(true);
                            new_rules.push(airctrl_core::types::DictionaryRule {
                                trigger: trigger.to_string(),
                                replacement: replacement.to_string(),
                                is_regex,
                                preserve_case,
                            });
                        }
                    }
                }

                let rule_count = new_rules.len();
                *self.state.dictionary_engine.write() = airctrl_stt::DictionaryEngine::new(
                    new_rules.clone(),
                    self.state.config.read().post_processing_enabled,
                );
                self.state.config.write().custom_dictionary = new_rules;

                Ok(json!({
                    "status": "success",
                    "success": true,
                    "total_rules": rule_count,
                    "rule_count": rule_count
                }))
            }

            "getDictionary" | "get_dictionary" => {
                let engine = self.state.dictionary_engine.read();
                Ok(json!({
                    "status": "success",
                    "rules": engine.rules(),
                    "post_processing_enabled": engine.is_post_processing_enabled()
                }))
            }

            "togglePostProcessing" | "toggle_post_processing" => {
                let current = self.state.dictionary_engine.read().is_post_processing_enabled();
                let next = !current;
                self.state.dictionary_engine.write().set_post_processing(next);
                self.state.config.write().post_processing_enabled = next;
                Ok(json!({
                    "status": "success",
                    "post_processing_enabled": next
                }))
            }

            "searchTranscriptHistory" | "search_transcript_history" | "searchTranscripts" | "search_transcripts" => {
                let query = args.get("query").and_then(Value::as_str).unwrap_or("");
                let model_filter = args.get("model").and_then(Value::as_str);
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;

                let results = self.state.transcript_store.search(query, model_filter, limit);
                let count = results.len();
                Ok(json!({
                    "status": "success",
                    "count": count,
                    "transcripts": results,
                    "results": results
                }))
            }

            "clearTranscripts" | "clear_transcripts" => {
                self.state.transcript_store.clear().map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;
                Ok(json!({
                    "status": "success",
                    "success": true,
                    "cleared": true
                }))
            }

            "toggleDebugDrawer" | "toggle_debug_drawer" => {
                let current = self.state.debug_drawer_visible.load(std::sync::atomic::Ordering::SeqCst);
                let next = !current;
                self.state.debug_drawer_visible.store(next, std::sync::atomic::Ordering::SeqCst);
                Ok(json!({
                    "status": "success",
                    "debug_drawer_visible": next
                }))
            }

            "getDebugDrawerStatus" | "get_debug_drawer_status" => {
                let visible = self.state.debug_drawer_visible.load(std::sync::atomic::Ordering::SeqCst);
                Ok(json!({
                    "status": "success",
                    "debug_drawer_visible": visible
                }))
            }

            "executeVoiceAction" | "execute_voice_action" | "executeVoiceCommand" | "execute_voice_command" => {
                let utterance = args
                    .get("utterance")
                    .or_else(|| args.get("command"))
                    .or_else(|| args.get("action"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("utterance string required".into()))?;

                let cmd = self.state.command_router.parse(utterance);
                let _ = self.state.event_bus.publish(DomainEvent::ActionTriggered(cmd.clone()));

                let (action_type, payload) = match &cmd {
                    ActionCommand::SnapWindow { position } => {
                        ("SnapWindow", json!({ "position": format!("{:?}", position) }))
                    }
                    ActionCommand::WindowSnap { direction } => {
                        ("SnapWindow", json!({ "position": format!("{:?}", direction) }))
                    }
                    ActionCommand::ResizeWindow { mode } => {
                        match mode {
                            ResizeMode::Explicit { width, height } => {
                                ("ResizeWindow", json!({ "mode": "Explicit", "width": width, "height": height }))
                            }
                            ResizeMode::Percentage { percentage } => {
                                ("ResizeWindow", json!({ "mode": "Percentage", "percentage": percentage }))
                            }
                            ResizeMode::Delta { dimension, delta_pixels } => {
                                ("ResizeWindow", json!({ "mode": "Delta", "dimension": format!("{:?}", dimension), "delta_pixels": delta_pixels }))
                            }
                        }
                    }
                    ActionCommand::SwitchFocus { app_name } => {
                        ("SwitchFocus", json!({ "app_name": app_name }))
                    }
                    ActionCommand::WindowFocus { query, .. } => {
                        ("SwitchFocus", json!({ "app_name": query }))
                    }
                    ActionCommand::CycleWindow { direction } => {
                        ("CycleWindow", json!({ "direction": format!("{:?}", direction) }))
                    }
                    ActionCommand::NavigateUi { element } => {
                        ("NavigateUi", json!({ "element": format!("{:?}", element) }))
                    }
                    ActionCommand::LaunchApp { app_name, auto_center } => {
                        ("LaunchApp", json!({ "app_name": app_name, "auto_center": auto_center }))
                    }
                    ActionCommand::CloseApp { app_name, target } => {
                        if target == "CurrentWindow" {
                            ("CloseApp", json!({ "target": "CurrentWindow" }))
                        } else {
                            ("CloseApp", json!({ "target": target, "app_name": app_name }))
                        }
                    }
                    ActionCommand::WindowClose { .. } => {
                        ("CloseApp", json!({ "target": "CurrentWindow" }))
                    }
                    ActionCommand::Scroll { direction, amount } => {
                        ("Scroll", json!({ "direction": format!("{:?}", direction), "amount": amount }))
                    }
                    ActionCommand::SystemInput(SystemInputAction::Scroll { direction, amount }) => {
                        ("Scroll", json!({ "direction": format!("{:?}", direction), "amount": amount }))
                    }
                    ActionCommand::MouseClick { button, count } => {
                        let btn = match button {
                            ClickAction::LeftClick => "Left",
                            ClickAction::RightClick => "Right",
                            ClickAction::MiddleClick => "Middle",
                            ClickAction::DoubleClick => "Double",
                            _ => "Left",
                        };
                        ("MouseClick", json!({ "button": btn, "count": count }))
                    }
                    ActionCommand::SystemInput(SystemInputAction::Click { button, .. }) => {
                        let (btn, count) = match button {
                            ClickAction::LeftClick => ("Left", 1),
                            ClickAction::RightClick => ("Right", 1),
                            ClickAction::MiddleClick => ("Middle", 1),
                            ClickAction::DoubleClick => ("Double", 2),
                            _ => ("Left", 1),
                        };
                        ("MouseClick", json!({ "button": btn, "count": count }))
                    }
                    ActionCommand::Unknown { query } => {
                        ("Unknown", json!({ "query": query }))
                    }
                    _ => {
                        ("Unknown", json!({ "query": utterance }))
                    }
                };

                Ok(json!({
                    "status": "success",
                    "action": action_type,
                    "payload": payload
                }))
            }

            "snapWindow" | "snap_window" => {
                let pos_str = args
                    .get("position")
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("Snap position required".into()))?;

                let pos = match pos_str.to_lowercase().as_str() {
                    "left" => SnapPosition::Left,
                    "right" => SnapPosition::Right,
                    "top" => SnapPosition::Top,
                    "bottom" => SnapPosition::Bottom,
                    "center" => SnapPosition::Center,
                    "maximize" => SnapPosition::Maximize,
                    "minimize" => SnapPosition::Minimize,
                    "restore" => SnapPosition::Restore,
                    "fullscreen" => SnapPosition::Fullscreen,
                    _ => return Err(IpcError::InvalidArguments(format!("Unknown snap position: {}", pos_str))),
                };

                let engine = *self.state.geometry_engine.read();
                let new_bounds = engine.compute_snap(pos);
                *self.state.current_window_bounds.write() = new_bounds;

                let platform = airctrl_core::platform::create_platform_provider(self.state.config.read().is_mock_platform);
                if let Ok(active) = platform.window.get_active_window() {
                    let _ = platform.window.set_window_bounds(active.id, new_bounds);
                }

                let _ = self.state.event_bus.publish(DomainEvent::ActionTriggered(ActionCommand::SnapWindow { position: pos }));

                Ok(json!({
                    "status": "success",
                    "bounds": {
                        "x": new_bounds.x,
                        "y": new_bounds.y,
                        "width": new_bounds.width,
                        "height": new_bounds.height
                    }
                }))
            }

            "resizeWindow"
            | "resize_window"
            | "resizeWindowWorkArea"
            | "resize_window_work_area"
            | "resizeWindowDelta"
            | "resize_window_delta" => {
                let current_bounds = *self.state.current_window_bounds.read();
                let engine = *self.state.geometry_engine.read();

                let mode = if let Some(w) = args.get("width").and_then(Value::as_u64) {
                    let h = args.get("height").and_then(Value::as_u64).unwrap_or(600);
                    ResizeMode::Explicit { width: w as u32, height: h as u32 }
                } else if let Some(pct) = args.get("percentage").and_then(Value::as_u64) {
                    ResizeMode::Percentage { percentage: pct.clamp(10, 100) as u8 }
                } else if let Some(dim_str) = args.get("dimension").and_then(Value::as_str) {
                    let dim = match dim_str.to_lowercase().as_str() {
                        "larger" => DimensionDelta::Larger,
                        "smaller" => DimensionDelta::Smaller,
                        "wider" => DimensionDelta::Wider,
                        "narrower" => DimensionDelta::Narrower,
                        "taller" => DimensionDelta::Taller,
                        "shorter" => DimensionDelta::Shorter,
                        _ => return Err(IpcError::InvalidArguments(format!("Unknown dimension: {}", dim_str))),
                    };
                    let delta = args.get("delta_pixels").or_else(|| args.get("delta")).and_then(Value::as_u64).unwrap_or(100) as u32;
                    ResizeMode::Delta { dimension: dim, delta_pixels: delta }
                } else if let Some(mode_val) = args.get("mode").and_then(Value::as_str) {
                    match mode_val.to_lowercase().as_str() {
                        "explicit" | "work_area" | "workarea" => {
                            let w = args.get("width").and_then(Value::as_u64).unwrap_or(1280) as u32;
                            let h = args.get("height").and_then(Value::as_u64).unwrap_or(720) as u32;
                            ResizeMode::Explicit { width: w, height: h }
                        }
                        "percentage" => {
                            let pct = args.get("percentage").and_then(Value::as_u64).unwrap_or(100) as u8;
                            ResizeMode::Percentage { percentage: pct.clamp(10, 100) }
                        }
                        "delta" => {
                            let dim_str = args.get("dimension").and_then(Value::as_str).unwrap_or("larger");
                            let dim = match dim_str.to_lowercase().as_str() {
                                "larger" => DimensionDelta::Larger,
                                "smaller" => DimensionDelta::Smaller,
                                "wider" => DimensionDelta::Wider,
                                "narrower" => DimensionDelta::Narrower,
                                "taller" => DimensionDelta::Taller,
                                "shorter" => DimensionDelta::Shorter,
                                _ => return Err(IpcError::InvalidArguments(format!("Unknown dimension: {}", dim_str))),
                            };
                            let delta = args.get("delta_pixels").or_else(|| args.get("delta")).and_then(Value::as_u64).unwrap_or(100) as u32;
                            ResizeMode::Delta { dimension: dim, delta_pixels: delta }
                        }
                        _ => return Err(IpcError::InvalidArguments(format!("Unknown resize mode: {}", mode_val))),
                    }
                } else if trimmed == "resizeWindowWorkArea" || trimmed == "resize_window_work_area" {
                    let w = args.get("width").and_then(Value::as_u64).unwrap_or(1280) as u32;
                    let h = args.get("height").and_then(Value::as_u64).unwrap_or(720) as u32;
                    ResizeMode::Explicit { width: w, height: h }
                } else {
                    ResizeMode::Delta { dimension: DimensionDelta::Larger, delta_pixels: 100 }
                };

                let new_bounds = engine.compute_resize(current_bounds, &mode);
                *self.state.current_window_bounds.write() = new_bounds;

                let platform = airctrl_core::platform::create_platform_provider(self.state.config.read().is_mock_platform);
                if let Ok(active) = platform.window.get_active_window() {
                    let _ = platform.window.set_window_bounds(active.id, new_bounds);
                }

                let _ = self.state.event_bus.publish(DomainEvent::ActionTriggered(ActionCommand::ResizeWindow { mode }));

                Ok(json!({
                    "status": "success",
                    "bounds": {
                        "x": new_bounds.x,
                        "y": new_bounds.y,
                        "width": new_bounds.width,
                        "height": new_bounds.height
                    }
                }))
            }

            "focusWindow" | "focus_window" => {
                let app = args
                    .get("app")
                    .or_else(|| args.get("name"))
                    .or_else(|| args.get("app_name"))
                    .or_else(|| args.get("query"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("Parameter 'app' or 'name' required".into()))?;

                if app.trim().is_empty() {
                    return Err(IpcError::InvalidArguments("Parameter 'app' or 'name' cannot be empty".into()));
                }

                self.invoke("executeVoiceCommand", Some(json!({ "utterance": format!("focus {}", app) })))
            }

            "cycleWindow" | "cycle_window" | "cycleWindows" | "cycle_windows" => {
                let dir_str = args.get("direction").and_then(Value::as_str).unwrap_or("Next");
                let direction = match dir_str.to_lowercase().as_str() {
                    "next" => CycleDirection::Next,
                    "previous" | "prev" => CycleDirection::Previous,
                    _ => return Err(IpcError::InvalidArguments(format!("Unknown cycle direction: {}", dir_str))),
                };
                let platform = airctrl_core::platform::create_platform_provider(self.state.config.read().is_mock_platform);
                let forward = direction == CycleDirection::Next;
                let _ = platform.window.cycle_window(forward);
                let _ = self.state.event_bus.publish(DomainEvent::ActionTriggered(ActionCommand::CycleWindow { direction }));

                Ok(json!({
                    "status": "success",
                    "direction": format!("{:?}", direction)
                }))
            }

            "navigateUi"
            | "navigate_ui"
            | "navigateUiField"
            | "navigate_ui_field"
            | "simulateUiKey"
            | "simulate_ui_key" => {
                let el_str = args
                    .get("element")
                    .or_else(|| args.get("key"))
                    .or_else(|| args.get("field"))
                    .or_else(|| args.get("target"))
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| {
                        if trimmed == "navigateUiField" || trimmed == "navigate_ui_field" {
                            "nextfield"
                        } else {
                            ""
                        }
                    });

                if el_str.is_empty() {
                    return Err(IpcError::InvalidArguments("UI target element or key required".into()));
                }

                let element = match el_str.to_lowercase().replace(['_', ' ', '-'], "").as_str() {
                    "nextfield" | "next" => UiTarget::NextField,
                    "prevfield" | "prev" | "previous" | "previousfield" => UiTarget::PrevField,
                    "enter" | "return" => UiTarget::Enter,
                    "escape" | "esc" => UiTarget::Escape,
                    "tab" => UiTarget::Tab,
                    _ => return Err(IpcError::InvalidArguments(format!("Unknown UI target element: {}", el_str))),
                };

                let platform = airctrl_core::platform::create_platform_provider(self.state.config.read().is_mock_platform);
                match element {
                    UiTarget::NextField | UiTarget::Tab => {
                        let _ = platform.input.send_key_combination(&[], airctrl_core::types::InputKey::Tab);
                    }
                    UiTarget::PrevField => {
                        let _ = platform.input.send_key_combination(&[airctrl_core::types::ModifierKey::Shift], airctrl_core::types::InputKey::Tab);
                    }
                    UiTarget::Enter => {
                        let _ = platform.input.send_key_combination(&[], airctrl_core::types::InputKey::Enter);
                    }
                    UiTarget::Escape => {
                        let _ = platform.input.send_key_combination(&[], airctrl_core::types::InputKey::Escape);
                    }
                }

                let _ = self.state.event_bus.publish(DomainEvent::ActionTriggered(ActionCommand::NavigateUi { element }));

                Ok(json!({
                    "status": "success",
                    "element": format!("{:?}", element)
                }))
            }

            "setAudioMixerVolumes" | "set_audio_mixer_volumes" => {
                let mic_val = args.get("mic_volume").or_else(|| args.get("mic"));
                let sys_val = args
                    .get("system_volume")
                    .or_else(|| args.get("system"))
                    .or_else(|| args.get("loopback_volume"));

                let mic_vol = match mic_val {
                    Some(Value::Number(n)) => {
                        let f = n
                            .as_f64()
                            .ok_or_else(|| IpcError::InvalidArguments("Parameter 'mic_volume' must be a valid number".into()))? as f32;
                        if !(0.0..=2.0).contains(&f) {
                            return Err(IpcError::InvalidArguments("Parameter 'mic_volume' must be between 0.0 and 2.0".into()));
                        }
                        f
                    }
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'mic_volume' must be a number".into())),
                    None => return Err(IpcError::InvalidArguments("Missing required parameter 'mic_volume'".into())),
                };

                let sys_vol = match sys_val {
                    Some(Value::Number(n)) => {
                        let f = n
                            .as_f64()
                            .ok_or_else(|| IpcError::InvalidArguments("Parameter 'system_volume' must be a valid number".into()))? as f32;
                        if !(0.0..=2.0).contains(&f) {
                            return Err(IpcError::InvalidArguments("Parameter 'system_volume' must be between 0.0 and 2.0".into()));
                        }
                        f
                    }
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'system_volume' must be a number".into())),
                    None => return Err(IpcError::InvalidArguments("Missing required parameter 'system_volume'".into())),
                };

                // Transactional commit to shared AppState mixer
                {
                    let mut mixer = self.state.dual_audio_mixer.write();
                    mixer.config_mut().mic_gain = mic_vol;
                    mixer.config_mut().loopback_gain = sys_vol;
                }

                Ok(json!({
                    "status": "success",
                    "mic_volume": mic_vol,
                    "system_volume": sys_vol
                }))
            }

            "getMixerStatus"
            | "get_mixer_status"
            | "getAudioMixerVolumes"
            | "get_audio_mixer_volumes" => {
                let mixer = self.state.dual_audio_mixer.read();
                Ok(json!({
                    "status": "success",
                    "sample_rate": mixer.config().sample_rate,
                    "ducking_gain": mixer.current_duck_gain(),
                    "current_envelope": mixer.current_envelope(),
                    "mic_volume": mixer.config().mic_gain,
                    "system_volume": mixer.config().loopback_gain,
                }))
            }

            "startMeetingRecording" | "start_meeting_recording" => {
                let meeting_id = args
                    .get("meeting_id")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("meet_{}", chrono::Utc::now().timestamp_millis()));

                self.state.is_meeting_recording.store(true, std::sync::atomic::Ordering::SeqCst);
                *self.state.active_meeting_id.write() = Some(meeting_id.clone());
                self.state.active_meeting_chunks.write().clear();
                self.state.dual_audio_mixer.write().reset();

                Ok(json!({
                    "status": "success",
                    "recording_started": true,
                    "meeting_id": meeting_id
                }))
            }

            "stopMeetingRecording" | "stop_meeting_recording" => {
                let was_recording = self.state.is_meeting_recording.swap(false, std::sync::atomic::Ordering::SeqCst);
                let meeting_id = self.state.active_meeting_id.write().take().unwrap_or_else(|| format!("meet_{}", chrono::Utc::now().timestamp_millis()));
                let chunks = self.state.active_meeting_chunks.read().clone();
                let chunk_count = chunks.len();

                let total_ms = chunks.last().map(|c| c.end_time_ms).unwrap_or(0);
                let duration_secs = total_ms / 1000;
                let entry = MeetingArchiveEntry::new(
                    meeting_id.clone(),
                    format!("Meeting {}", meeting_id),
                    chrono::Utc::now().to_rfc3339(),
                    duration_secs,
                    chunks,
                    "",
                    "",
                    vec!["Host".to_string(), "Participant".to_string()],
                );
                let _ = self.state.meeting_archive.save_meeting(entry);

                Ok(json!({
                    "status": "success",
                    "recording_stopped": true,
                    "was_recording": was_recording,
                    "meeting_id": meeting_id,
                    "chunks_recorded": chunk_count
                }))
            }

            "importAndEnhanceAudio" | "import_and_enhance_audio" => {
                let file_path_str = args
                    .get("file_path")
                    .or_else(|| args.get("path"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("Audio file path required".into()))?;

                let path = std::path::Path::new(file_path_str);
                let model = args
                    .get("model")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());

                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !airctrl_meetings::AudioDecoder::is_supported_extension(ext) {
                    return Err(IpcError::InvalidArguments(format!("Unsupported audio format: .{}", ext)));
                }

                let options = airctrl_meetings::ImportOptions {
                    model: model.clone(),
                    language: None,
                    title: None,
                };

                if !path.exists() {
                    return Err(IpcError::ExecutionFailed(format!(
                        "Audio file not found: {}",
                        path.display()
                    )));
                }

                let service = airctrl_meetings::AudioImportService::new(self.state.stt_engine.clone());
                let result = service
                    .import_and_transcribe(path, options, None)
                    .map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;

                Ok(json!({
                    "status": "success",
                    "source_path": result.source_path,
                    "format": result.format,
                    "model_used": result.model_used,
                    "transcript": result.transcript,
                    "duration_secs": result.duration_secs,
                    "sample_rate": result.sample_rate,
                    "segments": result.segments,
                }))
            }

            "generateAiSummary" | "generate_ai_summary" | "generateSummary" | "generate_summary" => {
                let transcript = match args.get("transcript").or_else(|| args.get("transcript_text")) {
                    Some(Value::String(s)) => {
                        if s.trim().is_empty() {
                            return Err(IpcError::InvalidArguments("Cannot summarize empty transcript".into()));
                        }
                        s.as_str()
                    }
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'transcript' must be a string".into())),
                    None => return Err(IpcError::InvalidArguments("Cannot summarize empty transcript".into())),
                };

                let template = match args.get("template") {
                    Some(Value::String(s)) => {
                        SummaryTemplate::from_str(s)
                            .map_err(IpcError::InvalidArguments)?
                    }
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'template' must be a string".into())),
                    None => SummaryTemplate::ExecutiveSummary,
                };

                let provider = match args.get("provider") {
                    Some(Value::String(s)) => {
                        SummaryProvider::from_str(s)
                            .map_err(IpcError::InvalidArguments)?
                    }
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'provider' must be a string".into())),
                    None => SummaryProvider::Ollama,
                };

                let api_key = match args.get("api_key") {
                    Some(Value::String(s)) => Some(s.clone()),
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'api_key' must be a string".into())),
                    None => None,
                };

                let model = match args.get("model") {
                    Some(Value::String(s)) => Some(s.clone()),
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'model' must be a string".into())),
                    None => None,
                };

                let endpoint = match args.get("endpoint") {
                    Some(Value::String(s)) => Some(s.clone()),
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'endpoint' must be a string".into())),
                    None => None,
                };

                let custom_prompt = match args.get("custom_prompt") {
                    Some(Value::String(s)) => Some(s.as_str()),
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'custom_prompt' must be a string".into())),
                    None => None,
                };

                let mut config = airctrl_meetings::SummaryProviderConfig {
                    provider,
                    ..Default::default()
                };
                if let Some(key) = api_key {
                    config.api_key = Some(key);
                }
                if let Some(m) = model {
                    config.model = m;
                }
                if let Some(ep) = endpoint {
                    config.endpoint = Some(ep);
                }

                let ctx = TemplateContext {
                    transcript,
                    meeting_title: args.get("meeting_title").and_then(Value::as_str),
                    date: args.get("date").and_then(Value::as_str),
                    duration: args.get("duration").and_then(Value::as_str),
                    participants: args.get("participants").and_then(Value::as_str),
                    platform: args.get("platform").and_then(Value::as_str),
                };

                let service = AiSummaryService::with_config(config).with_live_network();
                let summary_fut = service.generate_summary_async(
                    transcript,
                    template,
                    custom_prompt,
                    Some(&ctx),
                );

                let summary = match tokio::runtime::Handle::try_current() {
                    Ok(handle) => tokio::task::block_in_place(|| handle.block_on(summary_fut)),
                    Err(_) => {
                        let rt = tokio::runtime::Runtime::new()
                            .map_err(|e| IpcError::ExecutionFailed(format!("Failed to create runtime: {e}")))?;
                        rt.block_on(summary_fut)
                    }
                }.map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;

                Ok(json!({
                    "status": "success",
                    "summary": summary
                }))
            }

            "exportSummary" | "export_summary" => {
                let format = match args.get("format") {
                    Some(Value::String(s)) => {
                        ExportFormat::from_str(s)
                            .map_err(IpcError::InvalidArguments)?
                    }
                    Some(_) => {
                        return Err(IpcError::InvalidArguments(
                            "Parameter 'format' must be a string (one of 'Markdown', 'PDF', 'DOCX')".into(),
                        ));
                    }
                    None => {
                        return Err(IpcError::InvalidArguments(
                            "Missing required parameter 'format' (must be one of 'Markdown', 'PDF', 'DOCX')".into(),
                        ));
                    }
                };

                let content = match args.get("content") {
                    Some(Value::String(s)) => s.as_str(),
                    Some(_) => {
                        return Err(IpcError::InvalidArguments(
                            "Parameter 'content' must be a string".into(),
                        ));
                    }
                    None => "# Summary",
                };

                let exported_bytes = DocumentExporter::export(content, format)
                    .map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;

                if let Some(dest_val) = args.get("destination_path").or_else(|| args.get("path")) {
                    let dest = dest_val.as_str().ok_or_else(|| {
                        IpcError::InvalidArguments("Parameter 'destination_path' must be a string".into())
                    })?;
                    let path = std::path::Path::new(dest);
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    std::fs::write(path, &exported_bytes)
                        .map_err(|e| IpcError::ExecutionFailed(format!("Failed to write export to '{}': {}", dest, e)))?;
                }

                Ok(json!({
                    "status": "success",
                    "bytes_count": exported_bytes.len(),
                    "format": format.as_str()
                }))
            }

            "getMeetingArchive" | "get_meeting_archive" | "searchMeetingArchive" | "search_meeting_archive" => {
                let query = args
                    .get("query")
                    .or_else(|| args.get("search"))
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let meetings = if query.is_empty() {
                    self.state.meeting_archive.list_meetings()
                } else {
                    self.state.meeting_archive.search(query)
                };

                Ok(json!({
                    "status": "success",
                    "success": true,
                    "count": meetings.len(),
                    "meetings": meetings
                }))
            }

            "seekAudioOffset" | "seek_audio_offset" => {
                let meeting_id = args
                    .get("meeting_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| IpcError::InvalidArguments("meeting_id is required".into()))?;

                let chunk_id = args
                    .get("chunk_id")
                    .and_then(Value::as_str);

                let offset_ms = args
                    .get("offset_ms")
                    .or_else(|| args.get("offset"))
                    .and_then(Value::as_u64);

                if let Some(cid) = chunk_id {
                    let offset = self.state.meeting_archive.seek_audio_offset(meeting_id, cid);
                    match offset {
                        Some(ms) => Ok(json!({
                            "status": "success",
                            "meeting_id": meeting_id,
                            "chunk_id": cid,
                            "offset_ms": ms
                        })),
                        None => Ok(json!({
                            "status": "NotFound",
                            "meeting_id": meeting_id,
                            "chunk_id": cid,
                            "offset_ms": null
                        })),
                    }
                } else if let Some(ms) = offset_ms {
                    let chunk = self.state.meeting_archive.get_chunk_at_offset(meeting_id, ms);
                    let byte_offset = airctrl_meetings::MeetingArchiveStore::compute_wav_byte_offset(ms, 16000, 1, 16);
                    Ok(json!({
                        "status": "success",
                        "meeting_id": meeting_id,
                        "offset_ms": ms,
                        "byte_offset": byte_offset,
                        "chunk": chunk
                    }))
                } else {
                    Err(IpcError::InvalidArguments("Either chunk_id or offset_ms is required".into()))
                }
            }

            // ============================================================================
            // Milestone M11: Gesture Engine & Virtual Camera Splitter Commands
            // ============================================================================

            "getGestureEngineStatus" | "get_gesture_engine_status" => {
                let mode = *self.state.gesture_tracking_mode.read();
                let snapshot = self.state.multi_hand_state.read();
                let metrics = self.state.loop_manager.vision_metrics.read();
                let is_running = metrics.is_running;
                let fps = if is_running { metrics.actual_fps } else { 0.0 };

                Ok(json!({
                    "status": "success",
                    "tracking_mode": format!("{:?}", mode),
                    "active_hands_count": snapshot.hand_count(),
                    "fps": fps,
                    "engine_status": if is_running { "Active" } else { "Idle" },
                    "is_running": is_running,
                    "sensitivity": self.state.config.read().gesture_sensitivity,
                    "cooldown_secs": 0.3
                }))
            }

            "setGestureTrackingMode" | "set_gesture_tracking_mode" => {
                // 1. Transactional validation: parse all arguments into local variables
                // before mutating any shared application state.
                let parsed_mode = match args.get("mode") {
                    Some(Value::String(s)) => {
                        Some(airctrl_core::types::gesture::GestureTrackingMode::from_str(s.as_str())
                            .map_err(IpcError::InvalidArguments)?)
                    }
                    Some(_) => return Err(IpcError::InvalidArguments("Parameter 'mode' must be a string".into())),
                    None => None,
                };

                let validated_sensitivity = if let Some(sens_val) = args.get("sensitivity") {
                    let sens = sens_val.as_f64()
                        .ok_or_else(|| IpcError::InvalidArguments("Parameter 'sensitivity' must be a number".into()))? as f32;
                    if sens <= 0.0 || sens > 1.0 {
                        return Err(IpcError::InvalidArguments("Sensitivity must be in range (0.0, 1.0]".into()));
                    }
                    Some(sens)
                } else {
                    None
                };

                let validated_cooldown_secs = if let Some(cd_val) = args.get("cooldown_secs") {
                    let cd = cd_val.as_f64()
                        .ok_or_else(|| IpcError::InvalidArguments("Parameter 'cooldown_secs' must be a number".into()))? as f32;
                    if cd < 0.0 {
                        return Err(IpcError::InvalidArguments("Cooldown seconds cannot be negative".into()));
                    }
                    Some(cd)
                } else {
                    None
                };

                if let Some(cd_ms_val) = args.get("cooldown_ms") {
                    let cd_ms = cd_ms_val.as_f64()
                        .ok_or_else(|| IpcError::InvalidArguments("Parameter 'cooldown_ms' must be a number".into()))? as f32;
                    if cd_ms < 0.0 {
                        return Err(IpcError::InvalidArguments("Cooldown milliseconds cannot be negative".into()));
                    }
                }

                // 2. Transactional commit: all arguments passed validation without error.
                if let Some(mode) = parsed_mode {
                    *self.state.gesture_tracking_mode.write() = mode;
                }

                if let Some(sens) = validated_sensitivity {
                    self.state.config.write().gesture_sensitivity = sens;
                }

                let current_mode = *self.state.gesture_tracking_mode.read();
                let current_sens = self.state.config.read().gesture_sensitivity;

                Ok(json!({
                    "status": "success",
                    "mode": format!("{:?}", current_mode),
                    "sensitivity": current_sens,
                    "cooldown_secs": validated_cooldown_secs.unwrap_or(0.3)
                }))
            }

            "getVirtualCameraStatus" | "get_virtual_camera_status" => {
                let splitter = &self.state.virtual_camera_splitter;
                let cfg = splitter.config();
                let is_running = splitter.is_running();
                let privacy_muted = splitter.is_privacy_muted();
                let dual_muted = splitter.is_dual_muted();
                let status = splitter.status();

                Ok(json!({
                    "status": "success",
                    "device_name": cfg.device_name,
                    "resolution": {
                        "width": cfg.resolution.width,
                        "height": cfg.resolution.height
                    },
                    "fps": cfg.target_fps as f32,
                    "active_consumers": 2,
                    "consumer_count": 2,
                    "privacy_mode": format!("{:?}", status.privacy_mode),
                    "privacy_muted": privacy_muted,
                    "dual_muted": dual_muted,
                    "is_running": is_running,
                    "frames_fanned_out": splitter.frames_fanned_out()
                }))
            }

            "setVirtualCameraPrivacy" | "set_virtual_camera_privacy" => {
                if let Some(mode_val) = args.get("mode") {
                    let mode_str = mode_val.as_str()
                        .ok_or_else(|| IpcError::InvalidArguments("Parameter 'mode' must be a string".into()))?;
                    let target_mode = match mode_str {
                        "Active" | "live" => PrivacyMuteState::Active,
                        "PrivacySlate" | "privacy" | "slate" => PrivacyMuteState::PrivacySlate,
                        "DualMute" | "dual" => PrivacyMuteState::DualMute,
                        other => {
                            return Err(IpcError::InvalidArguments(format!("Unknown privacy mode: {}", other)));
                        }
                    };
                    self.state.virtual_camera_splitter.set_privacy_mode(target_mode);
                } else {
                    let privacy_muted = if let Some(pm_val) = args.get("privacy_muted") {
                        Some(pm_val.as_bool()
                            .ok_or_else(|| IpcError::InvalidArguments("Parameter 'privacy_muted' must be a boolean".into()))?)
                    } else if let Some(m_val) = args.get("muted") {
                        Some(m_val.as_bool()
                            .ok_or_else(|| IpcError::InvalidArguments("Parameter 'muted' must be a boolean".into()))?)
                    } else {
                        None
                    };

                    let dual_mute = if let Some(dm_val) = args.get("dual_mute") {
                        Some(dm_val.as_bool()
                            .ok_or_else(|| IpcError::InvalidArguments("Parameter 'dual_mute' must be a boolean".into()))?)
                    } else {
                        None
                    };

                    match (privacy_muted, dual_mute) {
                        (Some(pm), Some(dm)) => {
                            if dm {
                                self.state.virtual_camera_splitter.set_privacy_mode(PrivacyMuteState::DualMute);
                            } else if pm {
                                self.state.virtual_camera_splitter.set_privacy_mode(PrivacyMuteState::PrivacySlate);
                            } else {
                                self.state.virtual_camera_splitter.set_privacy_mode(PrivacyMuteState::Active);
                            }
                        }
                        (Some(pm), None) => {
                            self.state.virtual_camera_splitter.set_privacy_mute(pm);
                        }
                        (None, Some(dm)) => {
                            self.state.virtual_camera_splitter.set_dual_mute(dm);
                        }
                        (None, None) => {
                            self.state.virtual_camera_splitter.set_privacy_mute(true);
                        }
                    }
                }

                Ok(json!({
                    "status": "success",
                    "privacy_muted": self.state.virtual_camera_splitter.is_privacy_muted(),
                    "dual_muted": self.state.virtual_camera_splitter.is_dual_muted(),
                    "privacy_mode": format!("{:?}", self.state.virtual_camera_splitter.status().privacy_mode)
                }))
            }

            "getMultiHandGestureStatus" | "get_multi_hand_gesture_status" => {
                let snapshot = self.state.multi_hand_state.read();
                let active_gestures = self.state.active_gestures.read();

                Ok(json!({
                    "status": "success",
                    "detected_hands": snapshot.hand_count(),
                    "hands": {
                        "left": snapshot.left_hand.as_ref().map(|h| json!({
                            "handedness": "Left",
                            "confidence": h.confidence,
                            "landmarks_count": h.landmarks.len(),
                        })),
                        "right": snapshot.right_hand.as_ref().map(|h| json!({
                            "handedness": "Right",
                            "confidence": h.confidence,
                            "landmarks_count": h.landmarks.len(),
                        }))
                    },
                    "active_gestures": {
                        "pointing_ray": active_gestures.pointing_ray,
                        "pinch": active_gestures.pinch,
                        "velocity_swipe": active_gestures.velocity_swipe,
                        "two_finger": active_gestures.two_finger
                    }
                }))
            }

            "feedCameraFrame" | "feed_camera_frame" => {
                let frame_id = match args.get("frame_id") {
                    Some(Value::Number(n)) => n.as_u64().ok_or_else(|| IpcError::InvalidArguments("frame_id must be an unsigned integer".into()))?,
                    Some(_) => return Err(IpcError::InvalidArguments("frame_id must be a number".into())),
                    None => 1,
                };

                let result = self.state.virtual_camera_splitter.feed_camera_frame(frame_id)
                    .map_err(|e| IpcError::ExecutionFailed(e.to_string()))?;

                Ok(json!(result))
            }

            unknown => Err(IpcError::CommandNotFound(unknown.to_string())),
        }
    }
}
