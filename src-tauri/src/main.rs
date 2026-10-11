#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use airctrl_shell::{commands, AppState, IpcBridge};
use std::sync::Arc;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Initializing airctrl Tauri 2.0 shell...");

    let app_state = Arc::new(AppState::new());
    let bridge = IpcBridge::new(app_state);

    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .manage(bridge)
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::toggle_stt_overlay,
            commands::trigger_action,
            commands::toggle_radial_dial,
            commands::get_settings,
            commands::update_settings,
            commands::get_meeting_status,
            commands::trigger_camera_kill_switch,
            commands::get_cataloged_apps,
            commands::refresh_cataloged_apps,
            commands::launch_app,
            commands::toggle_command_palette,
        ])
        .run(tauri::generate_context!())
        .expect("error while running airctrl tauri application");
}
