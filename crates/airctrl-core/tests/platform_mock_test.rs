use airctrl_core::platform::create_mock_platform;
use airctrl_core::types::*;

#[test]
fn test_mock_window_operations() {
    let (provider, controllers) = create_mock_platform();

    let win1 = WindowInfo {
        id: 101,
        process_id: 1234,
        process_name: "cursor.exe".into(),
        title: "airctrl - Cursor".into(),
        bounds: WindowRect { x: 0, y: 0, width: 1280, height: 720 },
        is_focused: true,
        is_minimized: false,
    };
    let win2 = WindowInfo {
        id: 102,
        process_id: 5678,
        process_name: "teams.exe".into(),
        title: "Microsoft Teams".into(),
        bounds: WindowRect { x: 100, y: 100, width: 800, height: 600 },
        is_focused: false,
        is_minimized: false,
    };

    controllers.window.add_window(win1.clone());
    controllers.window.add_window(win2.clone());

    // Active window inspection
    let active = provider.window.get_active_window().expect("active window exists");
    assert_eq!(active.id, 101);

    // Enumerate windows
    let list = provider.window.enumerate_windows().expect("enumerate windows");
    assert_eq!(list.len(), 2);

    // Focus switching
    provider.window.focus_window(102).expect("focus win2");
    assert_eq!(provider.window.get_active_window().unwrap().id, 102);
    assert_eq!(controllers.window.get_focus_history(), vec![102]);

    // Minimize window
    provider.window.minimize_window(102).expect("minimize win2");

    // Close window
    provider.window.close_window(101).expect("close win1");
    assert_eq!(provider.window.enumerate_windows().unwrap().len(), 1);
}

#[test]
fn test_mock_input_operations() {
    let (provider, controllers) = create_mock_platform();

    provider.input.type_text("Hello world").expect("type text");
    provider.input.paste_text("Pasted clipboard text").expect("paste text");
    provider
        .input
        .send_key_combination(&[ModifierKey::Control], InputKey::Char('s'))
        .expect("send Ctrl+S");
    provider.input.move_pointer(450, 300).expect("move pointer");
    provider.input.click_pointer(ClickAction::LeftClick).expect("click pointer");

    assert_eq!(controllers.input.get_typed_strings(), vec!["Hello world"]);
    assert_eq!(controllers.input.get_pasted_strings(), vec!["Pasted clipboard text"]);

    let recorded = controllers.input.get_recorded_actions();
    assert_eq!(recorded.len(), 5);
}

#[test]
fn test_mock_audio_operations() {
    let (provider, controllers) = create_mock_platform();

    // Verify synthetic playback
    let synthetic_tone = vec![0.5f32; 1600];
    provider.audio.play_audio_stream(&synthetic_tone, 16000).expect("play audio");
    let played = controllers.audio.get_played_buffers();
    assert_eq!(played.len(), 1);
    assert_eq!(played[0].0.len(), 1600);
    assert_eq!(played[0].1, 16000);

    // Verify synthetic capture queue FIFO
    let test_chunk = vec![0.1f32, 0.2f32, 0.3f32];
    controllers.audio.enqueue_capture_samples(test_chunk.clone());
    let captured = provider.audio.capture_audio_chunk().expect("capture audio");
    assert_eq!(captured, test_chunk);

    // Verify capture queue underflow error
    assert!(provider.audio.capture_audio_chunk().is_err());

    // Verify audio output detection toggle
    assert!(!provider.audio.is_audio_output_active().unwrap());
    controllers.audio.set_output_active(true);
    assert!(provider.audio.is_audio_output_active().unwrap());
}

#[test]
fn test_mock_focus_window_single_focus_and_history_integrity() {
    let (provider, controllers) = create_mock_platform();

    let win1 = WindowInfo {
        id: 1,
        process_id: 10,
        process_name: "app1.exe".into(),
        title: "App 1".into(),
        bounds: WindowRect::default(),
        is_focused: true,
        is_minimized: false,
    };
    let win2 = WindowInfo {
        id: 2,
        process_id: 20,
        process_name: "app2.exe".into(),
        title: "App 2".into(),
        bounds: WindowRect::default(),
        is_focused: false,
        is_minimized: false,
    };

    controllers.window.add_window(win1);
    controllers.window.add_window(win2);

    // 1. Non-existent window failure must not touch history
    let fail_res = provider.window.focus_window(999);
    assert!(fail_res.is_err());
    assert_eq!(controllers.window.get_focus_history(), Vec::<u64>::new());

    // 2. Successful focus switches focus and records in history
    provider.window.focus_window(2).expect("focus win2");
    assert_eq!(controllers.window.get_focus_history(), vec![2]);

    let windows = provider.window.enumerate_windows().unwrap();
    let w1 = windows.iter().find(|w| w.id == 1).unwrap();
    let w2 = windows.iter().find(|w| w.id == 2).unwrap();
    assert!(!w1.is_focused, "win1 focus must be cleared");
    assert!(w2.is_focused, "win2 must be focused");
    assert_eq!(provider.window.get_active_window().unwrap().id, 2);
}

