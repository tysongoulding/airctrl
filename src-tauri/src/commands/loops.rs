use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn get_loop_status(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getLoopStatus", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_worker_loops(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("startWorkerLoops", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_worker_loops(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("stopWorkerLoops", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_dag_status(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getDagStatus", args)
        .map_err(|e| e.to_string())
}
