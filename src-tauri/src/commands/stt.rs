use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::State;

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
