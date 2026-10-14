use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn trigger_action(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("triggerAction", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_cataloged_apps(
    bridge: State<'_, IpcBridge>,
) -> Result<Value, String> {
    bridge
        .invoke("getCatalogedApps", None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_cataloged_apps(
    bridge: State<'_, IpcBridge>,
) -> Result<Value, String> {
    bridge
        .invoke("refreshCatalogedApps", None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launch_app(
    bridge: State<'_, IpcBridge>,
    path: String,
) -> Result<Value, String> {
    bridge
        .invoke("launchApp", Some(serde_json::json!({ "path": path })))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_command_palette(
    app: tauri::AppHandle,
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    let result = bridge
        .invoke("toggleCommandPalette", args)
        .map_err(|e| e.to_string())?;

    if let Some(palette_window) = tauri::Manager::get_webview_window(&app, "command-palette") {
        if let Some(visible) = result.get("command_palette_visible").and_then(Value::as_bool) {
            if visible {
                let _ = palette_window.show();
                let _ = palette_window.set_focus();
            } else {
                let _ = palette_window.hide();
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn execute_voice_command(
    bridge: State<'_, IpcBridge>,
    utterance: String,
) -> Result<Value, String> {
    bridge
        .invoke("executeVoiceCommand", Some(serde_json::json!({ "utterance": utterance })))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn snap_window(
    bridge: State<'_, IpcBridge>,
    position: String,
) -> Result<Value, String> {
    bridge
        .invoke("snapWindow", Some(serde_json::json!({ "position": position })))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resize_window(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("resizeWindow", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cycle_window(
    bridge: State<'_, IpcBridge>,
    direction: Option<String>,
) -> Result<Value, String> {
    let args = direction.map(|d| serde_json::json!({ "direction": d }));
    bridge
        .invoke("cycleWindow", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn navigate_ui(
    bridge: State<'_, IpcBridge>,
    element: String,
) -> Result<Value, String> {
    bridge
        .invoke("navigateUi", Some(serde_json::json!({ "element": element })))
        .map_err(|e| e.to_string())
}

