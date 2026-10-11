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
