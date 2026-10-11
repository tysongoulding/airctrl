use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn toggle_radial_dial(
    app: AppHandle,
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    let result = bridge
        .invoke("toggleRadialDial", args)
        .map_err(|e| e.to_string())?;

    // Synchronize window visibility if running in live Tauri desktop runtime
    if let Some(dial_window) = app.get_webview_window("radial-overlay") {
        if let Some(visible) = result.get("dial_visible").and_then(Value::as_bool) {
            if visible {
                let _ = dial_window.show();
                let _ = dial_window.set_focus();
            } else {
                let _ = dial_window.hide();
            }
        }
    }

    Ok(result)
}
