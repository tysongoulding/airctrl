use crate::ipc::IpcBridge;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn get_gesture_engine_status(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getGestureEngineStatus", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_gesture_tracking_mode(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("setGestureTrackingMode", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_virtual_camera_status(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getVirtualCameraStatus", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_virtual_camera_privacy(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("setVirtualCameraPrivacy", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_multi_hand_gesture_status(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getMultiHandGestureStatus", args)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn feed_camera_frame(
    bridge: State<'_, IpcBridge>,
    args: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("feedCameraFrame", args)
        .map_err(|e| e.to_string())
}
