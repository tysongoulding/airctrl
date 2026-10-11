//! Challenger Active Window Text Injection Fuzzing & Adversarial Tests
//!
//! Objectives:
//! 1. Empty text injection fuzzing.
//! 2. 20,000-character massive payload (verifying clipboard fallback) & threshold boundaries.
//! 3. Multi-byte UTF-8 emojis & surrogate pairs (🎙️, ZWJ sequences, astral symbols).
//! 4. Non-ASCII international scripts (Japanese, Cyrillic, Arabic RTL, Greek, Hebrew).
//! 5. Lost focus simulation during injection & focus recovery.
//! 6. Focus failure handling when target window is closed.
//! 7. Automatic clipboard fallback on direct typing failure.
//! 8. Complete error handling when both typing and clipboard fail.

use airctrl_core::error::CoreError;
use airctrl_core::platform::mock::{create_mock_platform, MockWindowOps};
use airctrl_core::platform::traits::{PlatformInputOps, PlatformWindowOps};
use airctrl_core::types::{ClickAction, InputKey, ModifierKey, WindowInfo, WindowRect};
use airctrl_stt::{
    ActiveWindowTextInjector, InjectionMethod, InjectionStatus, SttError, TextInjectionConfig,
};
use std::sync::{Arc, Mutex};

/// Custom mock input ops to simulate transient or permanent input failures.
#[derive(Clone, Default)]
struct FaultyInputOps {
    fail_type: bool,
    fail_paste: bool,
    typed: Arc<Mutex<Vec<String>>>,
    pasted: Arc<Mutex<Vec<String>>>,
}

impl PlatformInputOps for FaultyInputOps {
    fn type_text(&self, text: &str) -> Result<(), CoreError> {
        if self.fail_type {
            Err(CoreError::Input("Simulated SendInput failure".into()))
        } else {
            self.typed.lock().unwrap().push(text.to_string());
            Ok(())
        }
    }

    fn paste_text(&self, text: &str) -> Result<(), CoreError> {
        if self.fail_paste {
            Err(CoreError::Input("Simulated Clipboard failure".into()))
        } else {
            self.pasted.lock().unwrap().push(text.to_string());
            Ok(())
        }
    }

    fn send_key_combination(
        &self,
        _modifiers: &[ModifierKey],
        _key: InputKey,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    fn move_pointer(&self, _x: i32, _y: i32) -> Result<(), CoreError> {
        Ok(())
    }

    fn click_pointer(&self, _action: ClickAction) -> Result<(), CoreError> {
        Ok(())
    }
}

fn create_window(id: u64, is_focused: bool) -> WindowInfo {
    WindowInfo {
        id,
        process_id: (id % 1000) as u32,
        process_name: format!("app_{}.exe", id),
        title: format!("Window {}", id),
        bounds: WindowRect::default(),
        is_focused,
        is_minimized: false,
    }
}

#[tokio::test]
async fn test_fuzz_empty_string_ignored() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    // Empty text with Some window
    let report = injector.inject("", Some(101)).await.unwrap();
    assert_eq!(report.status, InjectionStatus::Ignored);
    assert_eq!(report.method, InjectionMethod::None);
    assert_eq!(report.text_length, 0);
    assert_eq!(report.target_window_id, Some(101));

    // Empty text with None window
    let report_none = injector.inject("", None).await.unwrap();
    assert_eq!(report_none.status, InjectionStatus::Ignored);
    assert_eq!(report_none.method, InjectionMethod::None);
    assert_eq!(report_none.text_length, 0);

    // Verify zero input operations executed
    assert!(controllers.input.get_recorded_actions().is_empty());
}

#[tokio::test]
async fn test_fuzz_massive_payload_20000_chars_clipboard_fallback() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    // Exactly 20,000 characters
    controllers.window.add_window(create_window(505, true));
    let payload = "AdversarialPayload_".repeat(1000) + "1234567890"; // 19 * 1000 + 10 = 19,010 + 990
    let exact_20k: String = payload.chars().cycle().take(20_000).collect();
    assert_eq!(exact_20k.chars().count(), 20_000);
    assert_eq!(exact_20k.len(), 20_000);

    let report = injector.inject(&exact_20k, Some(505)).await.unwrap();
    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.method, InjectionMethod::ClipboardPaste);
    assert_eq!(report.text_length, 20_000);
    assert_eq!(report.target_window_id, Some(505));

    // Must be in pasted strings, NOT typed strings
    assert_eq!(controllers.input.get_pasted_strings(), vec![exact_20k]);
    assert!(controllers.input.get_typed_strings().is_empty());

    // Extreme 100,000 characters payload
    let extreme_100k: String = "X".repeat(100_000);
    let report_100k = injector.inject(&extreme_100k, None).await.unwrap();
    assert_eq!(report_100k.status, InjectionStatus::Success);
    assert_eq!(report_100k.method, InjectionMethod::ClipboardPaste);
    assert_eq!(report_100k.text_length, 100_000);
}

#[tokio::test]
async fn test_fuzz_clipboard_fallback_threshold_boundary() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);
    // Default threshold: 200 bytes

    // 199 characters -> SendInputUnicode
    let str_199 = "A".repeat(199);
    let report_199 = injector.inject(&str_199, None).await.unwrap();
    assert_eq!(report_199.method, InjectionMethod::SendInputUnicode);

    // 200 characters -> SendInputUnicode
    let str_200 = "B".repeat(200);
    let report_200 = injector.inject(&str_200, None).await.unwrap();
    assert_eq!(report_200.method, InjectionMethod::SendInputUnicode);

    // 201 characters -> ClipboardPaste
    let str_201 = "C".repeat(201);
    let report_201 = injector.inject(&str_201, None).await.unwrap();
    assert_eq!(report_201.method, InjectionMethod::ClipboardPaste);

    let typed = controllers.input.get_typed_strings();
    let pasted = controllers.input.get_pasted_strings();
    assert_eq!(typed.len(), 2);
    assert_eq!(typed[0], str_199);
    assert_eq!(typed[1], str_200);
    assert_eq!(pasted.len(), 1);
    assert_eq!(pasted[0], str_201);
}

#[tokio::test]
async fn test_fuzz_multibyte_utf8_emojis_and_surrogate_pairs() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    // 1. Single microphone emoji with variation selector (🎙️ = U+1F399 U+FE0F)
    let mic_emoji = "🎙️";
    let report_mic = injector.inject(mic_emoji, None).await.unwrap();
    assert_eq!(report_mic.status, InjectionStatus::Success);
    assert_eq!(report_mic.method, InjectionMethod::SendInputUnicode);

    // 2. Complex ZWJ sequence emoji (Family: man + ZWJ + woman + ZWJ + girl + ZWJ + boy)
    let family_emoji = "👨‍👩‍👧‍👦";
    let report_family = injector.inject(family_emoji, None).await.unwrap();
    assert_eq!(report_family.status, InjectionStatus::Success);

    // 3. Flag emoji (Regional Indicator Symbol Letter U + Letter S = 🇺🇸)
    let flag_emoji = "🇺🇸 🇯🇵 🇬🇧";
    let report_flag = injector.inject(flag_emoji, None).await.unwrap();
    assert_eq!(report_flag.status, InjectionStatus::Success);

    // 4. Astral plane symbols & math runes
    let astral = "🦀 🚀 💻 🤖 ⚡ ✨ 𝄞 𝕬𝖎𝖗𝖈𝖙𝖗𝖑";
    let report_astral = injector.inject(astral, None).await.unwrap();
    assert_eq!(report_astral.status, InjectionStatus::Success);

    // Verify all strings typed intact with zero character corruption
    let typed = controllers.input.get_typed_strings();
    assert_eq!(typed, vec![mic_emoji, family_emoji, flag_emoji, astral]);
}

#[tokio::test]
async fn test_fuzz_non_ascii_international_scripts() {
    let (provider, controllers) = create_mock_platform();
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    // 1. Japanese (Hiragana, Katakana, Kanji, full-width brackets and punctuation)
    let japanese = "「こんにちは世界！音声入力のテストです。東京都渋谷区道玄坂」";
    let rep_ja = injector.inject(japanese, None).await.unwrap();
    assert_eq!(rep_ja.status, InjectionStatus::Success);

    // 2. Cyrillic (Russian pangram with ё, ъ, ь)
    let cyrillic = "«Съешь же ещё этих мягких французских булок, да выпей чаю.»";
    let rep_cy = injector.inject(cyrillic, None).await.unwrap();
    assert_eq!(rep_cy.status, InjectionStatus::Success);

    // 3. Arabic (Right-to-Left bidirectional script with harakat diacritics)
    let arabic = "مَرْحَبًا بِكَ فِي نِظَامِ التَّحَكُّمِ الصَّوْتِيِّ الْمُتَقَدِّمِ.";
    let rep_ar = injector.inject(arabic, None).await.unwrap();
    assert_eq!(rep_ar.status, InjectionStatus::Success);

    // 4. Greek and Hebrew
    let greek = "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία.";
    let hebrew = "דג סקרן שט לו בים זך אך לפתע פגש חבר.";
    let rep_gr = injector.inject(greek, None).await.unwrap();
    let rep_he = injector.inject(hebrew, None).await.unwrap();
    assert_eq!(rep_gr.status, InjectionStatus::Success);
    assert_eq!(rep_he.status, InjectionStatus::Success);

    // 5. Embedded null byte containment
    let null_contained = "Start\x00Middle\x00End";
    let rep_null = injector.inject(null_contained, None).await.unwrap();
    assert_eq!(rep_null.status, InjectionStatus::Success);

    // 6. Whitespace and multi-line formatting
    let ws = "   \t\r\nLine 1\nLine 2\r\n\t   ";
    let rep_ws = injector.inject(ws, None).await.unwrap();
    assert_eq!(rep_ws.status, InjectionStatus::Success);

    let typed = controllers.input.get_typed_strings();
    assert_eq!(
        typed,
        vec![
            japanese,
            cyrillic,
            arabic,
            greek,
            hebrew,
            null_contained,
            ws
        ]
    );
}

#[tokio::test]
async fn test_fuzz_lost_focus_simulation_and_recovery() {
    let (provider, controllers) = create_mock_platform();
    let config = TextInjectionConfig {
        refocus_settle_delay_ms: 0,
        ..Default::default()
    };
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, Some(config));

    let win_editor = create_window(101, true);
    let win_browser = create_window(202, false);
    controllers.window.add_window(win_editor);
    controllers.window.add_window(win_browser);

    // Simulate user switching away from editor to browser while speaking
    controllers.window.focus_window(202).unwrap();
    assert_eq!(
        controllers.window.get_active_window().unwrap().id,
        202
    );

    // Inject with target_window_id = 101 (editor)
    let report = injector
        .inject("Text meant for editor", Some(101))
        .await
        .unwrap();

    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.target_window_id, Some(101));

    // Verify window 101 was refocused
    assert_eq!(
        controllers.window.get_active_window().unwrap().id,
        101
    );
    assert_eq!(controllers.window.get_focus_history(), vec![202, 101]);
}

#[tokio::test]
async fn test_fuzz_lost_focus_target_window_closed_error() {
    let (provider, controllers) = create_mock_platform();
    let config = TextInjectionConfig {
        refocus_settle_delay_ms: 0,
        ..Default::default()
    };
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, Some(config));

    let win_active = create_window(101, true);
    controllers.window.add_window(win_active);

    // Target window 999 does NOT exist in the window list (closed during recording)
    let res = injector.inject("Target closed", Some(999)).await;

    assert!(res.is_err());
    match res.unwrap_err() {
        SttError::WindowFocus(target_id, err_msg) => {
            assert_eq!(target_id, 999);
            assert!(err_msg.contains("999"));
        }
        other => panic!("Expected SttError::WindowFocus, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_fuzz_direct_typing_failure_falls_back_to_clipboard() {
    let faulty_input = Arc::new(FaultyInputOps {
        fail_type: true,  // Simulate SendInput OS failure
        fail_paste: false, // Clipboard succeeds
        typed: Arc::new(Mutex::new(Vec::new())),
        pasted: Arc::new(Mutex::new(Vec::new())),
    });

    let mock_window = Arc::new(MockWindowOps::new());
    mock_window.add_window(create_window(101, true));

    let injector = ActiveWindowTextInjector::new(mock_window, faulty_input.clone(), None);

    // Short text normally routed via SendInputUnicode (<200 bytes)
    let short_text = "Short text direct typing failed";
    let report = injector.inject(short_text, Some(101)).await.unwrap();

    // Verify it automatically fell back to ClipboardPaste and succeeded!
    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.method, InjectionMethod::ClipboardPaste);
    assert_eq!(report.text_length, short_text.len());

    let pasted = faulty_input.pasted.lock().unwrap();
    assert_eq!(*pasted, vec![short_text]);
}

#[tokio::test]
async fn test_fuzz_both_typing_and_clipboard_failure_reports_error() {
    let faulty_input = Arc::new(FaultyInputOps {
        fail_type: true,  // Both fail
        fail_paste: true,
        typed: Arc::new(Mutex::new(Vec::new())),
        pasted: Arc::new(Mutex::new(Vec::new())),
    });

    let mock_window = Arc::new(MockWindowOps::new());
    mock_window.add_window(create_window(101, true));

    let injector = ActiveWindowTextInjector::new(mock_window, faulty_input, None);

    let res = injector.inject("Total failure scenario", Some(101)).await;
    assert!(res.is_err());
    match res.unwrap_err() {
        SttError::Clipboard(msg) => {
            assert!(msg.contains("Simulated Clipboard failure"));
        }
        other => panic!("Expected SttError::Clipboard, got: {:?}", other),
    }
}
