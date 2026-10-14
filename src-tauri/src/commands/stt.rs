use crate::ipc::IpcBridge;
use airctrl_core::types::OverlayPosition;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
    pub focus_stealing: bool,
}

pub fn calculate_overlay_geometry(
    position: OverlayPosition,
    monitor_origin_x: f64,
    monitor_origin_y: f64,
    screen_width: f64,
    screen_height: f64,
    scale_factor: f64,
) -> OverlayGeometry {
    if position == OverlayPosition::None {
        return OverlayGeometry {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            visible: false,
            focus_stealing: false,
        };
    }

    let win_w = 460.0 * scale_factor;
    let win_h = 90.0 * scale_factor;
    let center_x = monitor_origin_x + (screen_width - win_w) / 2.0;

    let y = match position {
        OverlayPosition::Top => monitor_origin_y + (20.0 * scale_factor),
        OverlayPosition::Center => monitor_origin_y + (screen_height - win_h) / 2.0,
        OverlayPosition::Bottom => {
            monitor_origin_y + screen_height - win_h - (40.0 * scale_factor)
        }
        OverlayPosition::None => unreachable!(),
    };

    OverlayGeometry {
        x: center_x as i32,
        y: y as i32,
        width: win_w as u32,
        height: win_h as u32,
        visible: true,
        focus_stealing: false,
    }
}

#[tauri::command]
pub async fn start_recording(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("startRecording", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_recording(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("stopRecording", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_stt_overlay(
    app: AppHandle,
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    let result = bridge
        .invoke("toggleSttOverlay", args)
        .map_err(|e| e.to_string())?;

    let position = bridge.state.config.read().overlay_position;
    if let Some(overlay_window) = app.get_webview_window("stt-overlay") {
        if let Some(visible) = result.get("stt_overlay_visible").and_then(Value::as_bool) {
            if visible && position != OverlayPosition::None {
                if let Ok(Some(monitor)) = overlay_window.current_monitor() {
                    let screen = monitor.size();
                    let scale = monitor.scale_factor();
                    let geom = calculate_overlay_geometry(
                        position,
                        monitor.position().x as f64,
                        monitor.position().y as f64,
                        screen.width as f64,
                        screen.height as f64,
                        scale,
                    );
                    let _ = overlay_window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
                        geom.width,
                        geom.height,
                    )));
                    let _ = overlay_window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition::new(geom.x, geom.y),
                    ));
                }
                // Zero Focus Stealing:
                let _ = overlay_window.set_focusable(false);
                let _ = overlay_window.show();
                // Never call set_focus()! Keep target application focused.
            } else {
                let _ = overlay_window.hide();
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn inject_text(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("injectText", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_models(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("listModels", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn select_model(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("selectModel", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_recording_mode(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("setRecordingMode", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_recording_mode(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getRecordingMode", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_overlay_position(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("setOverlayPosition", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_overlay_position(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getOverlayPosition", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_dictionary(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("updateDictionary", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_dictionary(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getDictionary", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_post_processing(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("togglePostProcessing", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_transcripts(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("searchTranscripts", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_transcripts(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("clearTranscripts", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_debug_drawer(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("toggleDebugDrawer", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_debug_drawer_status(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getDebugDrawerStatus", args)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_geometry_calculations() {
        let geom_bottom = calculate_overlay_geometry(
            OverlayPosition::Bottom,
            0.0,
            0.0,
            1920.0,
            1080.0,
            1.0,
        );
        assert!(geom_bottom.visible);
        assert!(!geom_bottom.focus_stealing);
        assert_eq!(geom_bottom.width, 460);
        assert_eq!(geom_bottom.height, 90);
        assert_eq!(geom_bottom.x, (1920 - 460) / 2);
        assert_eq!(geom_bottom.y, 1080 - 90 - 40);

        let geom_top = calculate_overlay_geometry(
            OverlayPosition::Top,
            0.0,
            0.0,
            1920.0,
            1080.0,
            1.0,
        );
        assert!(geom_top.visible);
        assert_eq!(geom_top.y, 20);

        let geom_center = calculate_overlay_geometry(
            OverlayPosition::Center,
            0.0,
            0.0,
            1920.0,
            1080.0,
            1.0,
        );
        assert!(geom_center.visible);
        assert_eq!(geom_center.y, (1080 - 90) / 2);

        let geom_none = calculate_overlay_geometry(
            OverlayPosition::None,
            0.0,
            0.0,
            1920.0,
            1080.0,
            1.0,
        );
        assert!(!geom_none.visible);
        assert!(!geom_none.focus_stealing);
        assert_eq!(geom_none.width, 0);
        assert_eq!(geom_none.height, 0);
    }
}
