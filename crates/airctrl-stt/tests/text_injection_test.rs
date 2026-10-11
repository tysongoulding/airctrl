use airctrl_core::platform::mock::create_mock_platform;
use airctrl_core::types::{WindowInfo, WindowRect};
use airctrl_stt::{
    ActiveWindowTextInjector, InjectionMethod, InjectionStatus, TextInjectionConfig,
};

#[tokio::test]
async fn test_direct_unicode_text_injection() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    controllers.window.add_window(WindowInfo {
        id: 1001,
        process_id: 10,
        process_name: "notepad.exe".to_string(),
        title: "Untitled - Notepad".to_string(),
        bounds: WindowRect::default(),
        is_focused: true,
        is_minimized: false,
    });

    let report = injector
        .inject("Build airctrl desktop app", Some(1001))
        .await
        .unwrap();

    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.method, InjectionMethod::SendInputUnicode);
    assert_eq!(report.text_length, 25);
    assert_eq!(
        controllers.input.get_typed_strings(),
        vec!["Build airctrl desktop app"]
    );
}

#[tokio::test]
async fn test_empty_string_ignored() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    let report = injector.inject("", Some(1001)).await.unwrap();

    assert_eq!(report.status, InjectionStatus::Ignored);
    assert_eq!(report.method, InjectionMethod::None);
    assert_eq!(report.text_length, 0);
    assert!(controllers.input.get_recorded_actions().is_empty());
}

#[tokio::test]
async fn test_window_focus_restoration_during_injection() {
    let (provider, controllers) = create_mock_platform();
    let config = TextInjectionConfig {
        refocus_settle_delay_ms: 0, // 0 for fast tests
        ..Default::default()
    };
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, Some(config));

    let win_target = WindowInfo {
        id: 101,
        process_id: 1,
        process_name: "target.exe".into(),
        title: "Target Window".into(),
        bounds: WindowRect::default(),
        is_focused: false,
        is_minimized: false,
    };
    let win_other = WindowInfo {
        id: 102,
        process_id: 2,
        process_name: "other.exe".into(),
        title: "Other Window".into(),
        bounds: WindowRect::default(),
        is_focused: true,
        is_minimized: false,
    };
    controllers.window.add_window(win_target);
    controllers.window.add_window(win_other);

    let report = injector
        .inject("Refocused text", Some(101))
        .await
        .unwrap();

    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(controllers.window.get_focus_history(), vec![101]);
}

#[tokio::test]
async fn test_large_payload_clipboard_fallback_and_boundary() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    // 20,000 character boundary payload
    let massive_text = "A".repeat(20_000);
    let report = injector.inject(&massive_text, None).await.unwrap();

    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.method, InjectionMethod::ClipboardPaste);
    assert_eq!(report.text_length, 20_000);
    assert_eq!(controllers.input.get_pasted_strings(), vec![massive_text]);
}

#[tokio::test]
async fn test_multilingual_unicode_and_emojis() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    let text = "Testing: 日本語, Español (ñ), Emoji 🎙️";
    let report = injector.inject(text, None).await.unwrap();

    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.method, InjectionMethod::SendInputUnicode);
    assert_eq!(controllers.input.get_typed_strings(), vec![text]);
}

#[tokio::test]
async fn test_null_byte_and_whitespace_boundary() {
    let (provider, _controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    // String with embedded null byte
    let null_text = "Hello\x00World";
    let report = injector.inject(null_text, None).await.unwrap();
    assert_eq!(report.status, InjectionStatus::Success);

    // String with whitespace and newlines
    let ws_text = "   \n\r\t   ";
    let report_ws = injector.inject(ws_text, None).await.unwrap();
    assert_eq!(report_ws.status, InjectionStatus::Success);
}

#[tokio::test]
async fn test_maximum_window_handle_id() {
    let (provider, controllers) = create_mock_platform();
    let config = TextInjectionConfig {
        refocus_settle_delay_ms: 0,
        ..Default::default()
    };
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, Some(config));

    let max_hwnd = 4_294_967_295u64; // Max 32-bit HWND
    controllers.window.add_window(WindowInfo {
        id: max_hwnd,
        process_id: 999,
        process_name: "max.exe".into(),
        title: "Max HWND".into(),
        bounds: WindowRect::default(),
        is_focused: true,
        is_minimized: false,
    });

    let report = injector.inject("Valid", Some(max_hwnd)).await.unwrap();
    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.target_window_id, Some(max_hwnd));
}
