#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use airctrl_shell::{commands, AppState, CliOptions, IpcBridge};
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opts = if args.len() > 1 {
        CliOptions::parse(&args[1..])
    } else {
        CliOptions::default()
    };

    if opts.help {
        CliOptions::print_help();
        return;
    }

    if opts.is_remote_command() {
        tracing::info!("Remote command received via CLI: {:?}", opts);
        println!("airctrl: remote command dispatched: {:?}", opts);
        return;
    }

    let filter = if opts.debug {
        "airctrl=debug,airctrl_shell=debug,info"
    } else {
        "airctrl=info,warn"
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init();

    tracing::info!("Initializing airctrl Tauri 2.0 shell with opts: {:?}", opts);

    let app_state = Arc::new(AppState::new());
    if opts.debug {
        app_state.debug_drawer_visible.store(true, Ordering::SeqCst);
    }

    if let Some(ref model_name) = opts.model {
        app_state.config.write().whisper_model_path = model_name.clone();
    }

    let bridge = IpcBridge::new(app_state);

    let start_hidden = opts.start_hidden;
    let is_debug = opts.debug;

    tauri::Builder::default()
        .setup(move |app| {
            use tauri::Manager;
            if !start_hidden {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }
            if is_debug {
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
            commands::inject_text,
            commands::get_loop_status,
            commands::start_worker_loops,
            commands::stop_worker_loops,
            commands::get_dag_status,
            commands::list_models,
            commands::select_model,
            commands::set_recording_mode,
            commands::get_recording_mode,
            commands::set_overlay_position,
            commands::get_overlay_position,
            commands::update_dictionary,
            commands::get_dictionary,
            commands::toggle_post_processing,
            commands::search_transcripts,
            commands::clear_transcripts,
            commands::toggle_debug_drawer,
            commands::get_debug_drawer_status,
            commands::execute_voice_command,
            commands::snap_window,
            commands::resize_window,
            commands::cycle_window,
            commands::navigate_ui,
            commands::get_gesture_engine_status,
            commands::set_gesture_tracking_mode,
            commands::get_virtual_camera_status,
            commands::set_virtual_camera_privacy,
            commands::get_multi_hand_gesture_status,
            commands::feed_camera_frame,
            commands::get_mixer_status,
            commands::start_meeting_recording,
            commands::stop_meeting_recording,
            commands::import_and_enhance_audio,
            commands::generate_summary,
            commands::export_summary,
            commands::get_meeting_archive,
            commands::seek_audio_offset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running airctrl tauri application");
}
