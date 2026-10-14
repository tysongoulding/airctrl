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

#[tauri::command]
pub async fn get_mixer_status(bridge: State<'_, IpcBridge>) -> Result<Value, String> {
    bridge
        .invoke("getMixerStatus", None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_meeting_recording(
    bridge: State<'_, IpcBridge>,
    payload: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("startMeetingRecording", payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_meeting_recording(bridge: State<'_, IpcBridge>) -> Result<Value, String> {
    bridge
        .invoke("stopMeetingRecording", None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_and_enhance_audio(
    bridge: State<'_, IpcBridge>,
    payload: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("importAndEnhanceAudio", payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_summary(
    bridge: State<'_, IpcBridge>,
    payload: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("generateSummary", payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_summary(
    bridge: State<'_, IpcBridge>,
    payload: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("exportSummary", payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_meeting_archive(
    bridge: State<'_, IpcBridge>,
    payload: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("getMeetingArchive", payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_audio_offset(
    bridge: State<'_, IpcBridge>,
    payload: Option<Value>,
) -> Result<Value, String> {
    bridge
        .invoke("seekAudioOffset", payload)
        .map_err(|e| e.to_string())
}

