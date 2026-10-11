use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn get_meeting_status(bridge: State<'_, IpcBridge>) -> Result<Value, String> {
    bridge
        .invoke("getMeetingStatus", None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn trigger_camera_kill_switch(bridge: State<'_, IpcBridge>) -> Result<Value, String> {
    bridge
        .invoke("triggerCameraKillSwitch", None)
        .map_err(|e| e.to_string())
}
