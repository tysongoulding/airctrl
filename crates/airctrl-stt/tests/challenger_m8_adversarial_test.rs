use airctrl_core::error::CoreError;
use airctrl_core::platform::traits::{PlatformInputOps, PlatformWindowOps};
use airctrl_core::types::*;
use airctrl_stt::config::TextInjectionConfig;
use airctrl_stt::error::SttError;
use airctrl_stt::hotkey::state_machine::{
    HotkeyStateMachine, StateMachineAction, StateTransitionResult,
};
use airctrl_stt::injection::injector::{
    ActiveWindowTextInjector, InjectionMethod, InjectionStatus,
};
use airctrl_stt::model::discovery::ModelManager;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// =========================================================================
// PART 1: WhisperModelVariant Parsing & ModelManager Multi-Directory Discovery
// =========================================================================

#[test]
fn test_adversarial_whisper_model_variant_filename_heuristics_and_edge_names() {
    // 1. Standard filenames for all variants
    assert_eq!(
        WhisperModelVariant::from_filename("whisper-tiny.en.bin"),
        WhisperModelVariant::TinyEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("whisper-base.en.bin"),
        WhisperModelVariant::BaseEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("whisper-small.en.bin"),
        WhisperModelVariant::SmallEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("whisper-medium.en.bin"),
        WhisperModelVariant::MediumEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("whisper-turbo.en.bin"),
        WhisperModelVariant::TurboEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("whisper-large.en.bin"),
        WhisperModelVariant::LargeEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("parakeet-unified-en-0.6b-Q8_0.gguf"),
        WhisperModelVariant::ParakeetV3
    );

    // 2. Case insensitivity and alternate prefix conventions
    assert_eq!(
        WhisperModelVariant::from_filename("WHISPER-BASE.EN.BIN"),
        WhisperModelVariant::BaseEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("GGML-LARGE-V3-TURBO.BIN"),
        WhisperModelVariant::TurboEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("ggml-small.bin"),
        WhisperModelVariant::SmallEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("ggml-medium.bin"),
        WhisperModelVariant::MediumEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("ggml-tiny.bin"),
        WhisperModelVariant::TinyEn
    );
    assert_eq!(
        WhisperModelVariant::from_filename("large-v3.bin"),
        WhisperModelVariant::LargeEn
    );

    // 3. Custom models and strange/empty filenames
    assert_eq!(
        WhisperModelVariant::from_filename(""),
        WhisperModelVariant::Custom("".to_string())
    );
    assert_eq!(
        WhisperModelVariant::from_filename("my-custom-model.bin"),
        WhisperModelVariant::Custom("my-custom-model.bin".to_string())
    );
    assert_eq!(
        WhisperModelVariant::from_filename("unknown_weights.gguf"),
        WhisperModelVariant::Custom("unknown_weights.gguf".to_string())
    );

    // 4. Format & Architecture classifications
    assert!(WhisperModelVariant::ParakeetV3.is_gguf());
    assert!(!WhisperModelVariant::ParakeetV3.is_whisper());
    assert_eq!(WhisperModelVariant::ParakeetV3.format(), ModelFormat::Gguf);

    assert!(WhisperModelVariant::TurboEn.is_whisper());
    assert!(!WhisperModelVariant::TurboEn.is_gguf());
    assert_eq!(WhisperModelVariant::TurboEn.format(), ModelFormat::GgmlBin);

    let custom_gguf = WhisperModelVariant::Custom("finetuned.gguf".to_string());
    assert!(custom_gguf.is_gguf());
    assert_eq!(custom_gguf.format(), ModelFormat::Gguf);

    let custom_bin = WhisperModelVariant::Custom("finetuned.bin".to_string());
    assert!(!custom_bin.is_gguf());
    assert_eq!(custom_bin.format(), ModelFormat::GgmlBin);

    // 5. ModelFormat extension parsing
    assert_eq!(ModelFormat::from_extension("bin"), Some(ModelFormat::GgmlBin));
    assert_eq!(ModelFormat::from_extension("BIN"), Some(ModelFormat::GgmlBin));
    assert_eq!(ModelFormat::from_extension("gguf"), Some(ModelFormat::Gguf));
    assert_eq!(ModelFormat::from_extension("GGUF"), Some(ModelFormat::Gguf));
    assert_eq!(ModelFormat::from_extension("onnx"), None);
    assert_eq!(ModelFormat::from_extension("pt"), None);
    assert_eq!(ModelFormat::from_extension("safetensors"), None);
    assert_eq!(ModelFormat::from_extension(""), None);

    // 6. Sizes and SHA-256 properties
    for variant in WhisperModelVariant::standard_variants() {
        assert!(variant.approximate_size_bytes() > 50_000_000);
        let sha = variant.official_sha256().expect("Standard variant must have sha256");
        assert_eq!(sha.len(), 64);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }
    assert_eq!(custom_bin.approximate_size_bytes(), 0);
    assert_eq!(custom_bin.official_sha256(), None);
}

#[test]
fn test_adversarial_model_manager_corrupted_directory_traversal() {
    let temp_root = tempfile::tempdir().unwrap();
    let root = temp_root.path();

    let valid_dir = root.join("valid_models");
    let corrupt_dir = root.join("corrupt_models");
    let non_existent_dir = root.join("ghost_dir_404");

    fs::create_dir_all(&valid_dir).unwrap();
    fs::create_dir_all(&corrupt_dir).unwrap();

    // 1. Create a valid file matching TinyEn official SHA-256
    let tiny_path = valid_dir.join("whisper-tiny.en.bin");
    // We write dummy bytes, so its hash won't match official SHA, which verifies is_verified=false
    fs::write(&tiny_path, b"test-tiny-weights-data").unwrap();

    // 2. Create a 0-byte file (corrupt/incomplete download)
    let zero_byte_path = corrupt_dir.join("whisper-base.en.bin");
    File::create(&zero_byte_path).unwrap(); // 0 bytes

    // 3. Create a subdirectory masquerading as a model file
    let fake_dir_model = corrupt_dir.join("whisper-small.en.bin");
    fs::create_dir_all(&fake_dir_model).unwrap();

    // 4. Create files with invalid extensions
    fs::write(corrupt_dir.join("whisper-turbo.en.bin.tmp"), b"partial download").unwrap();
    fs::write(corrupt_dir.join("whisper-large.en.bin.part"), b"partial").unwrap();
    fs::write(corrupt_dir.join("model_info.json"), b"{}").unwrap();
    fs::write(corrupt_dir.join("notes.txt"), b"documentation").unwrap();

    // 5. Create a hidden directory with a model (should be ignored)
    let hidden_dir = corrupt_dir.join(".internal_models");
    fs::create_dir_all(&hidden_dir).unwrap();
    fs::write(hidden_dir.join("whisper-medium.en.bin"), b"hidden weights").unwrap();

    // 6. Overly deep nesting (depth 3 in a non-huggingface dir, max_depth is 2)
    let deep_dir = corrupt_dir.join("sub1").join("sub2").join("sub3");
    fs::create_dir_all(&deep_dir).unwrap();
    fs::write(deep_dir.join("whisper-large.en.bin"), b"too deep weights").unwrap();

    // 7. Valid custom GGUF in sub1
    let custom_gguf = corrupt_dir.join("sub1").join("custom_expert.gguf");
    fs::write(&custom_gguf, b"custom-gguf-weights").unwrap();

    // Initialize manager with valid, corrupt, and non-existent directories
    let search_dirs = vec![valid_dir.clone(), non_existent_dir, corrupt_dir];
    let manager = ModelManager::with_search_dirs(search_dirs);

    // Discovery should not panic or fail
    let discovered = manager.discover_models().expect("Discovery should handle corrupt dirs cleanly");

    // Assertions:
    // - tiny_path should be found
    // - zero_byte_path MUST NOT be found (0-byte filter)
    // - fake_dir_model MUST NOT be found (is_dir filter)
    // - .tmp / .part / .json / .txt MUST NOT be found (extension filter)
    // - hidden dir model MUST NOT be found (hidden filter)
    // - deep nesting model MUST NOT be found (depth filter)
    // - custom_gguf SHOULD be found (depth 1, .gguf)

    let discovered_paths: Vec<PathBuf> = discovered.iter().map(|m| m.file_path.clone()).collect();

    assert!(
        discovered_paths.contains(&tiny_path),
        "Valid tiny model must be discovered"
    );
    assert!(
        discovered_paths.contains(&custom_gguf),
        "Custom GGUF in sub1 must be discovered"
    );
    assert!(
        !discovered_paths.contains(&zero_byte_path),
        "0-byte file must be excluded"
    );
    assert!(
        !discovered_paths.iter().any(|p| p.ends_with("whisper-small.en.bin")),
        "Subdirectory masquerading as model must be excluded"
    );
    assert!(
        !discovered_paths.iter().any(|p| p.ends_with("whisper-medium.en.bin")),
        "Hidden directory model must be excluded"
    );
    assert!(
        !discovered_paths.iter().any(|p| p.ends_with("whisper-large.en.bin")),
        "Overly deep nested model must be excluded"
    );

    // Verify hash integrity flag
    let tiny_info = discovered.iter().find(|m| m.file_path == tiny_path).unwrap();
    assert_eq!(tiny_info.variant, WhisperModelVariant::TinyEn);
    assert_eq!(tiny_info.format, ModelFormat::GgmlBin);
    assert!(!tiny_info.is_verified, "Dummy bytes should not match official sha256");

    let custom_info = discovered.iter().find(|m| m.file_path == custom_gguf).unwrap();
    assert_eq!(
        custom_info.variant,
        WhisperModelVariant::Custom("custom_expert.gguf".to_string())
    );
    assert_eq!(custom_info.format, ModelFormat::Gguf);
    assert!(custom_info.is_verified, "Custom model without official hash defaults to verified");
}

#[test]
fn test_adversarial_model_manager_duplicate_and_symlink_deduplication() {
    let temp_root = tempfile::tempdir().unwrap();
    let dir_a = temp_root.path().join("dir_a");
    let dir_b = temp_root.path().join("dir_b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    let model_a = dir_a.join("whisper-tiny.en.bin");
    fs::write(&model_a, b"tiny-model-content").unwrap();

    // Create a symlink in dir_b pointing to model_a
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let symlink_b = dir_b.join("whisper-tiny.en.bin");
        symlink(&model_a, &symlink_b).unwrap();

        // Search dirs include dir_a twice and dir_b
        let manager = ModelManager::with_search_dirs(vec![dir_a.clone(), dir_a, dir_b]);
        let discovered = manager.discover_models().unwrap();

        // Should deduplicate canonical paths so it appears exactly once
        let count = discovered
            .iter()
            .filter(|m| m.variant == WhisperModelVariant::TinyEn)
            .count();
        assert_eq!(count, 1, "Canonical path deduplication must prevent duplicates");
    }
}

#[test]
fn test_adversarial_model_manager_resolve_and_verify_stress() {
    let temp_root = tempfile::tempdir().unwrap();
    let model_dir = temp_root.path().join("models");
    fs::create_dir_all(&model_dir).unwrap();

    let tiny_file = model_dir.join("whisper-tiny.en.bin");
    fs::write(&tiny_file, b"content").unwrap();

    let manager = ModelManager::new(model_dir);

    // Resolve existing variant
    let resolved = manager.resolve_model(WhisperModelVariant::TinyEn).unwrap();
    assert_eq!(resolved, tiny_file);

    // Resolve missing variant
    let err = manager.resolve_model(WhisperModelVariant::TurboEn).unwrap_err();
    match err {
        SttError::ModelNotFound(p) => {
            assert!(p.to_string_lossy().contains("turbo"));
        }
        _ => panic!("Expected ModelNotFound error, got {:?}", err),
    }

    // Verify model with dummy content should return SttError::ModelCorrupted on checksum mismatch
    let err = manager.verify_model(WhisperModelVariant::TinyEn).unwrap_err();
    match err {
        SttError::ModelCorrupted { expected, actual, .. } => {
            assert_eq!(expected, WhisperModelVariant::TinyEn.official_sha256().unwrap());
            assert!(!actual.is_empty());
        }
        _ => panic!("Expected SttError::ModelCorrupted, got {:?}", err),
    }
}

// =========================================================================
// PART 2: HotkeyStateMachine Edge Conditions & Concurrent VAD Lifecycle
// =========================================================================

#[test]
fn test_adversarial_hotkey_smart_dual_mode_micro_press_and_jitter_boundaries() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::SmartDualMode, None);
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Boundary 1: Micro-press flutter jitter (0ms) -> Discard
    sm.on_key_down(1000, Some(10));
    assert_eq!(*sm.state(), SpeechState::Listening);
    let res = sm.on_key_up(1000); // delta = 0ms (< 30ms)
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Boundary 2: Micro-press flutter jitter (29ms) -> Discard
    sm.on_key_down(2000, Some(10));
    assert_eq!(*sm.state(), SpeechState::Listening);
    let res = sm.on_key_up(2029); // delta = 29ms (< 30ms)
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Boundary 3: Exact lower bound for Toggle Tap latch (30ms) -> Latched, stays Listening!
    sm.on_key_down(3000, Some(10));
    assert_eq!(*sm.state(), SpeechState::Listening);
    let res = sm.on_key_up(3030); // delta = 30ms
    assert_eq!(res, StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Listening, "30ms tap must latch toggle mode");

    // Second tap (KeyDown) stops toggle and transcribes
    let res = sm.on_key_down(3500, Some(10));
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Transcribing);
    sm.reset();

    // Boundary 4: Upper bound for Toggle Tap latch (249ms) -> Stays Listening!
    sm.on_key_down(4000, Some(10));
    let res = sm.on_key_up(4249); // delta = 249ms
    assert_eq!(res, StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Listening);
    sm.reset();

    // Boundary 5: Exact lower bound for Push-to-Talk hold (250ms) -> StopRecordingAndTranscribe
    sm.on_key_down(5000, Some(10));
    let res = sm.on_key_up(5250); // delta = 250ms (>= 250ms)
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Transcribing);
    sm.reset();

    // Boundary 6: Long push-to-talk hold (5000ms)
    sm.on_key_down(6000, Some(10));
    let res = sm.on_key_up(11000); // delta = 5000ms
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );
    sm.reset();

    // Boundary 7: Out-of-order clock timestamp / monotonic glitch (t_up < t_down)
    sm.on_key_down(7000, Some(10));
    let res = sm.on_key_up(6900); // saturating_sub yields 0ms (<30ms)
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);
}

#[test]
fn test_adversarial_hotkey_hold_only_and_toggle_only_strict_isolation() {
    // 1. HoldOnly Mode
    let mut sm = HotkeyStateMachine::new(HotkeyMode::HoldOnly, None);
    assert_eq!(sm.min_press_duration_ms(), 100);

    // Micro-press <100ms cancels
    sm.on_key_down(100, Some(1));
    let res = sm.on_key_up(150); // 50ms < 100ms
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );

    // Hold >= 100ms transcribes
    sm.on_key_down(200, Some(1));
    let res = sm.on_key_up(300); // 100ms >= 100ms
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );
    sm.reset();

    // 2. ToggleOnly Mode
    let mut sm_toggle = HotkeyStateMachine::new(HotkeyMode::ToggleOnly, None);
    sm_toggle.on_key_down(1000, Some(1));
    assert_eq!(*sm_toggle.state(), SpeechState::Listening);

    // In ToggleOnly, KeyUp at ANY duration MUST NEVER cancel or stop!
    for duration in [1, 29, 30, 99, 100, 250, 1000, 5000] {
        let res = sm_toggle.on_key_up(1000 + duration);
        assert_eq!(res, StateTransitionResult::NoChange);
        assert_eq!(*sm_toggle.state(), SpeechState::Listening);
    }

    // Only second KeyDown triggers StopRecordingAndTranscribe
    let res = sm_toggle.on_key_down(8000, Some(1));
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );
}

#[test]
fn test_adversarial_continuous_hands_free_vad_lifecycle_stress() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::ContinuousHandsFree, None);
    assert_eq!(*sm.state(), SpeechState::Idle);

    // KeyUp in ContinuousHandsFree is always ignored
    assert_eq!(sm.on_key_up(500), StateTransitionResult::NoChange);

    // VAD speech onset: Idle -> Listening
    let res = sm.on_vad_speech_start(1000, Some(42));
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Idle,
            to: SpeechState::Listening,
            action: StateMachineAction::StartRecording {
                target_window_id: Some(42),
            },
        }
    );
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Duplicate VAD speech onset while already listening -> NoChange
    assert_eq!(sm.on_vad_speech_start(1100, Some(42)), StateTransitionResult::NoChange);

    // VAD silence discard -> Cancel and return to Idle
    let res = sm.on_vad_silence_discard();
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Duplicate silence discard when already in Idle -> NoChange
    assert_eq!(sm.on_vad_silence_discard(), StateTransitionResult::NoChange);

    // Speech onset -> speech end -> transcribing
    sm.on_vad_speech_start(2000, Some(42));
    let res = sm.on_vad_speech_end(2500);
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Transcribing);

    // Spurious VAD speech onset while Transcribing -> NoChange
    assert_eq!(sm.on_vad_speech_start(2600, Some(42)), StateTransitionResult::NoChange);
    // Spurious VAD speech end while Transcribing -> NoChange
    assert_eq!(sm.on_vad_speech_end(2700), StateTransitionResult::NoChange);

    // Transcription completed -> Injecting
    let payload = TranscriptionPayload {
        text: "hands free speech".to_string(),
        duration_ms: 500,
        confidence: 0.95,
        target_window_title: None,
        target_process_name: None,
    };
    let res = sm.on_transcription_completed(payload);
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Transcribing,
            to: SpeechState::Injecting,
            action: StateMachineAction::StartInjection,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Injecting);

    // Spurious events during injection
    assert_eq!(sm.on_key_down(2800, None), StateTransitionResult::NoChange);
    assert_eq!(sm.on_key_up(2900), StateTransitionResult::NoChange);
    assert_eq!(sm.on_vad_speech_start(3000, None), StateTransitionResult::NoChange);

    // Injection completed -> Return to Idle
    let res = sm.on_injection_completed();
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Injecting,
            to: SpeechState::Idle,
            action: StateMachineAction::ReturnToIdle,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);
}

#[test]
fn test_adversarial_hotkey_error_recovery_and_cancel_matrix() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::SmartDualMode, None);

    // Error from Idle
    sm.on_error("audio subsystem crashed");
    assert_eq!(
        *sm.state(),
        SpeechState::Error("audio subsystem crashed".to_string())
    );

    // KeyDown does not transition when in Error
    assert_eq!(sm.on_key_down(100, None), StateTransitionResult::NoChange);

    // Reset recovers from Error
    let res = sm.reset();
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Error("audio subsystem crashed".to_string()),
            to: SpeechState::Idle,
            action: StateMachineAction::ReturnToIdle,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Cancel recovers from Listening
    sm.on_key_down(200, None);
    assert_eq!(*sm.state(), SpeechState::Listening);
    let res = sm.cancel();
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Cancel when already Idle -> NoChange
    assert_eq!(sm.cancel(), StateTransitionResult::NoChange);
}

#[test]
fn test_adversarial_hotkey_concurrent_fuzz_simulation() {
    // Stress test 4 hotkey state machines across modes with rapid interleaved event stream
    let modes = [
        HotkeyMode::SmartDualMode,
        HotkeyMode::HoldOnly,
        HotkeyMode::ToggleOnly,
        HotkeyMode::ContinuousHandsFree,
    ];

    for mode in modes {
        let sm = Arc::new(Mutex::new(HotkeyStateMachine::new(mode, None)));
        let mut handles = Vec::new();

        for worker_id in 0..4 {
            let sm_clone = Arc::clone(&sm);
            handles.push(thread::spawn(move || {
                for i in 0..500 {
                    let ts = (worker_id * 10_000 + i * 20) as u64;
                    let mut lock = sm_clone.lock().unwrap();
                    match i % 7 {
                        0 => {
                            lock.on_key_down(ts, Some(worker_id as u64));
                        }
                        1 => {
                            lock.on_key_up(ts + (i % 300) as u64);
                        }
                        2 => {
                            lock.on_vad_speech_start(ts, Some(worker_id as u64));
                        }
                        3 => {
                            lock.on_vad_speech_end(ts + 100);
                        }
                        4 => {
                            lock.on_vad_silence_discard();
                        }
                        5 => {
                            let _ = lock.on_transcription_completed(TranscriptionPayload {
                                text: "fuzz".into(),
                                duration_ms: 100,
                                confidence: 0.9,
                                target_window_title: None,
                                target_process_name: None,
                            });
                            let _ = lock.on_injection_completed();
                        }
                        _ => {
                            if i % 20 == 0 {
                                lock.reset();
                            }
                        }
                    }
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Must be in a valid consistent state at the end
        let final_sm = sm.lock().unwrap();
        match final_sm.state() {
            SpeechState::Idle
            | SpeechState::Listening
            | SpeechState::Transcribing
            | SpeechState::Injecting
            | SpeechState::Error(_) => {}
        }
    }
}

// =========================================================================
// PART 3: Reliable Paste Clipboard Preservation Adversarial Suite
// =========================================================================

/// Mock window provider that fails focus operations.
struct FailingFocusWindowOps {
    active_window_id: u64,
}

impl PlatformWindowOps for FailingFocusWindowOps {
    fn get_active_window(&self) -> Result<WindowInfo, CoreError> {
        Ok(WindowInfo {
            id: self.active_window_id,
            title: "Background Window".to_string(),
            process_name: "other_app".to_string(),
            process_id: 1234,
            bounds: WindowRect::default(),
            is_focused: true,
            is_minimized: false,
        })
    }

    fn focus_window(&self, id: u64) -> Result<(), CoreError> {
        // Deliberately fail window focus
        Err(CoreError::WindowNotFound(id))
    }

    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>, CoreError> {
        Ok(vec![])
    }

    fn minimize_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }

    fn close_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }
}

/// Mock input provider tracking all clipboard and typing events.
#[derive(Clone, Default)]
struct MockClipboardInputOps {
    clipboard: Arc<Mutex<Option<String>>>,
    pasted: Arc<Mutex<Vec<String>>>,
    fail_paste: Arc<AtomicBool>,
    fail_get_clipboard: Arc<AtomicBool>,
}

impl MockClipboardInputOps {
    fn new(initial: Option<String>) -> Self {
        Self {
            clipboard: Arc::new(Mutex::new(initial)),
            pasted: Arc::new(Mutex::new(Vec::new())),
            fail_paste: Arc::new(AtomicBool::new(false)),
            fail_get_clipboard: Arc::new(AtomicBool::new(false)),
        }
    }

    fn get_clipboard(&self) -> Option<String> {
        self.clipboard.lock().unwrap().clone()
    }

    fn get_pasted(&self) -> Vec<String> {
        self.pasted.lock().unwrap().clone()
    }
}

impl PlatformInputOps for MockClipboardInputOps {
    fn type_text(&self, _text: &str) -> Result<(), CoreError> {
        // Fail direct typing to force reliable paste fallback
        Err(CoreError::Input("Direct typing unavailable".to_string()))
    }

    fn paste_text(&self, text: &str) -> Result<(), CoreError> {
        if self.fail_paste.load(Ordering::SeqCst) {
            return Err(CoreError::Internal("Simulated OS paste failure".to_string()));
        }
        *self.clipboard.lock().unwrap() = Some(text.to_string());
        self.pasted.lock().unwrap().push(text.to_string());
        Ok(())
    }

    fn get_clipboard_text(&self) -> Result<Option<String>, CoreError> {
        if self.fail_get_clipboard.load(Ordering::SeqCst) {
            return Err(CoreError::Internal("Clipboard access locked".to_string()));
        }
        Ok(self.clipboard.lock().unwrap().clone())
    }

    fn set_clipboard_text(&self, text: &str) -> Result<(), CoreError> {
        *self.clipboard.lock().unwrap() = Some(text.to_string());
        Ok(())
    }

    fn clear_clipboard(&self) -> Result<(), CoreError> {
        *self.clipboard.lock().unwrap() = None;
        Ok(())
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

#[tokio::test]
async fn test_adversarial_reliable_paste_target_window_focus_failure_preserves_clipboard() {
    let window_ops = Arc::new(FailingFocusWindowOps {
        active_window_id: 100,
    });
    let input_ops = Arc::new(MockClipboardInputOps::new(Some(
        "CRITICAL_USER_SECRET_DO_NOT_LOSE".to_string(),
    )));

    let config = TextInjectionConfig {
        clipboard_fallback_threshold: 5,
        refocus_settle_delay_ms: 0,
        reliable_paste: true,
        paste_delay_before_ms: 0,
        paste_delay_after_ms: 0,
    };

    let injector = ActiveWindowTextInjector::new(window_ops, input_ops.clone(), Some(config));

    // Target window 999 is different from active 100, triggering focus_window(999) which FAILS
    let result = injector.inject("Transcribed speech", Some(999)).await;

    // Injector must return WindowFocus error
    assert!(result.is_err());
    match result.unwrap_err() {
        SttError::WindowFocus(id, _) => assert_eq!(id, 999),
        other => panic!("Expected WindowFocus error, got {:?}", other),
    }

    // Critical assertion: Clipboard MUST still contain the original secret!
    assert_eq!(
        input_ops.get_clipboard(),
        Some("CRITICAL_USER_SECRET_DO_NOT_LOSE".to_string()),
        "Clipboard must be preserved when target window fails to focus"
    );

    // Verify nothing was pasted
    assert!(input_ops.get_pasted().is_empty());
}

#[tokio::test]
async fn test_adversarial_reliable_paste_when_clipboard_is_empty() {
    let (provider, _controllers) = airctrl_core::platform::create_mock_platform();
    let input_ops = Arc::new(MockClipboardInputOps::new(None)); // Clipboard is initially None

    let config = TextInjectionConfig {
        clipboard_fallback_threshold: 5,
        refocus_settle_delay_ms: 0,
        reliable_paste: true,
        paste_delay_before_ms: 0,
        paste_delay_after_ms: 0,
    };

    let injector = ActiveWindowTextInjector::new(provider.window, input_ops.clone(), Some(config));

    let report = injector
        .inject("Transcribed speech longer than 5 chars", None)
        .await
        .expect("Injection should succeed");

    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.method, InjectionMethod::ClipboardPaste);

    // Verify the text was actually pasted
    assert_eq!(
        input_ops.get_pasted(),
        vec!["Transcribed speech longer than 5 chars"]
    );

    // Critical assertion: Clipboard MUST be cleanly cleared back to None!
    assert_eq!(
        input_ops.get_clipboard(),
        None,
        "Clipboard must be restored to empty None when initial clipboard was empty"
    );
}

#[tokio::test]
async fn test_adversarial_reliable_paste_when_paste_fails_preserves_clipboard() {
    let (provider, _controllers) = airctrl_core::platform::create_mock_platform();
    let input_ops = Arc::new(MockClipboardInputOps::new(Some(
        "CONFIDENTIAL_SPREADSHEET_DATA".to_string(),
    )));

    // Configure paste operation to fail
    input_ops.fail_paste.store(true, Ordering::SeqCst);

    let config = TextInjectionConfig {
        clipboard_fallback_threshold: 5,
        refocus_settle_delay_ms: 0,
        reliable_paste: true,
        paste_delay_before_ms: 0,
        paste_delay_after_ms: 0,
    };

    let injector = ActiveWindowTextInjector::new(provider.window, input_ops.clone(), Some(config));

    let result = injector
        .inject("Transcribed text to paste", None)
        .await;

    // Injector must return Clipboard error
    assert!(result.is_err());
    match result.unwrap_err() {
        SttError::Clipboard(msg) => {
            assert!(msg.contains("Simulated OS paste failure"));
        }
        other => panic!("Expected Clipboard error, got {:?}", other),
    }

    // Critical assertion: Clipboard MUST still be restored to original text even though paste failed!
    assert_eq!(
        input_ops.get_clipboard(),
        Some("CONFIDENTIAL_SPREADSHEET_DATA".to_string()),
        "Clipboard must be restored even when paste operation returns an error"
    );
}

#[tokio::test]
async fn test_adversarial_reliable_paste_clipboard_read_error_graceful_degradation() {
    let (provider, _controllers) = airctrl_core::platform::create_mock_platform();
    let input_ops = Arc::new(MockClipboardInputOps::new(None));

    // Simulate clipboard read failure
    input_ops.fail_get_clipboard.store(true, Ordering::SeqCst);

    let config = TextInjectionConfig {
        clipboard_fallback_threshold: 5,
        refocus_settle_delay_ms: 0,
        reliable_paste: true,
        paste_delay_before_ms: 0,
        paste_delay_after_ms: 0,
    };

    let injector = ActiveWindowTextInjector::new(provider.window, input_ops.clone(), Some(config));

    // Even if reading clipboard text fails, paste should execute without crashing
    let report = injector.inject("Transcribed text", None).await.unwrap();
    assert_eq!(report.status, InjectionStatus::Success);
    assert_eq!(report.method, InjectionMethod::ClipboardPaste);
}

// =========================================================================
// PART 4: DictionaryEngine & TranscriptHistoryStore Adversarial Suite
// =========================================================================

#[test]
fn test_adversarial_dictionary_engine_topological_and_case_preservation() {
    use airctrl_stt::dictionary::DictionaryEngine;

    // Rules with overlapping triggers: "air", "air control", and regex
    let rules = vec![
        DictionaryRule::literal("air", "sky"),
        DictionaryRule::literal("air control", "AirCtrl"),
        DictionaryRule::regex(r"ticket-(\d+)", "JIRA-$1"),
    ];

    let engine = DictionaryEngine::new(rules, true);

    // 1. Longest rule must match first ("air control" -> "AirCtrl", NOT "sky control")
    let out = engine.replace_words("I am testing air control software");
    assert_eq!(out, "I am testing AirCtrl software");

    // 2. Case preservation heuristics
    let upper = engine.replace_words("START AIR CONTROL NOW");
    assert_eq!(upper, "START AIRCTRL NOW");

    let lower = engine.replace_words("start air control now");
    assert_eq!(lower, "Start AirCtrl now"); // Leading char capitalized, AirCtrl branding preserved

    // 3. Regex replacement with capture group
    let reg = engine.replace_words("Please review ticket-4299 immediately");
    assert_eq!(reg, "Please review JIRA-4299 immediately");

    // 4. Standalone "air" should still be replaced when not part of "air control"
    let single = engine.replace_words("breathe the clean air today");
    assert_eq!(single, "Breathe the clean sky today");

    // 5. Empty / whitespace string
    assert_eq!(engine.replace_words(""), "");
    assert_eq!(engine.replace_words("   "), "");
}

#[test]
fn test_adversarial_transcript_history_store_concurrent_atomic_persistence() {
    use airctrl_stt::history::TranscriptHistoryStore;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("transcripts.json");

    let store = Arc::new(TranscriptHistoryStore::new(Some(file_path.clone())));
    let mut handles = Vec::new();

    // Spawn 6 writers concurrently pushing 50 items each = 300 total
    for thread_idx in 0..6 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                let item = TranscriptItem {
                    id: format!("t{}_{}", thread_idx, i),
                    timestamp: 1700000000.0 + (thread_idx * 100 + i) as f64,
                    text: format!("Transcribed phrase {} from thread {}", i, thread_idx),
                    raw_text: format!("raw phrase {} thread {}", i, thread_idx),
                    duration_ms: 1000 + i as u64,
                    model_used: if thread_idx % 2 == 0 {
                        "whisper-turbo".to_string()
                    } else {
                        "parakeet-v3".to_string()
                    },
                    confidence: 0.95,
                    session_id: "stress_session".to_string(),
                    target_app: Some("code.exe".to_string()),
                    target_title: Some("VS Code - Project".to_string()),
                    injected: true,
                    starred: i % 10 == 0,
                };
                store_clone.store(item).unwrap();
            }
        }));
    }

    // Spawn 2 readers concurrently querying the store
    for _ in 0..2 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                let _ = store_clone.search("thread", None, 10);
                let _ = store_clone.search("", Some("parakeet"), 5);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Verify all 300 items were recorded
    assert_eq!(store.count(), 300);

    // Verify search with query and model filter
    let results = store.search("thread 1", Some("parakeet"), 100);
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.model_used.contains("parakeet")));

    // Verify persistence reload: create new store from the same file path
    let reloaded = TranscriptHistoryStore::new(Some(file_path));
    assert_eq!(reloaded.count(), 300, "Crash-safe atomic persistence must retain all 300 items");
}

