use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn get_settings(bridge: State<'_, IpcBridge>) -> Result<Value, String> {
    bridge
        .invoke("getSettings", None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    bridge: State<'_, IpcBridge>,
    args: Value,
) -> Result<Value, String> {
    bridge
        .invoke("updateSettings", Some(args))
        .map_err(|e| e.to_string())
}
