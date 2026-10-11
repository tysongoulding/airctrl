use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

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

    if let Some(overlay_window) = app.get_webview_window("stt-overlay") {
        if let Some(visible) = result.get("stt_overlay_visible").and_then(Value::as_bool) {
            if visible {
                // Position at Bottom Center of the active monitor (clearing Windows taskbar)
                if let Ok(Some(monitor)) = overlay_window.current_monitor() {
                    let screen = monitor.size();
                    let scale = monitor.scale_factor();
                    let win_w = 460.0 * scale;
                    let win_h = 90.0 * scale;
                    let x = monitor.position().x as f64 + (screen.width as f64 - win_w) / 2.0;
                    let y = monitor.position().y as f64 + screen.height as f64 - win_h - (40.0 * scale);
                    let _ = overlay_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x as i32, y as i32)));
                }
                let _ = overlay_window.show();
                // Handy Parity: Do NOT call set_focus()! Keep target application focused.
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

