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
