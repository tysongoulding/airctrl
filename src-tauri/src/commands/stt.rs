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
                let _ = overlay_window.show();
                let _ = overlay_window.set_focus();
            } else {
                let _ = overlay_window.hide();
            }
        }
    }

    Ok(result)
}

