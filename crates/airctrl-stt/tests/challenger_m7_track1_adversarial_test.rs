use airctrl_core::platform::create_mock_platform;
use airctrl_core::types::*;
use airctrl_stt::audio::AudioRingBuffer;
use airctrl_stt::config::{TextInjectionConfig, VadConfig};
use airctrl_stt::hotkey::state_machine::{
    HotkeyStateMachine, StateMachineAction, StateTransitionResult,
};
use airctrl_stt::injection::injector::{
    ActiveWindowTextInjector, InjectionMethod, InjectionStatus,
};
use airctrl_stt::vad::detector::{
    DiscardReason, StreamVadState, VadDecision, VoiceActivityDetector,
};

// =========================================================================
// 1. VAD Mathematical & State Machine Adversarial Tests
// =========================================================================

#[test]
fn test_adversarial_vad_rms_dbfs_mathematical_precision() {
    let vad = VoiceActivityDetector::default();

    // 1. Zero amplitude / silence
    let zeros = vec![0.0f32; 1600];
    assert_eq!(vad.calculate_rms_energy(&zeros), 0.0);
    assert_eq!(vad.calculate_dbfs(&zeros), -120.0);

    // 2. DC constant: all 0.5
    let dc_half = vec![0.5f32; 1600];
    let rms_dc = vad.calculate_rms_energy(&dc_half);
    assert!((rms_dc - 0.5).abs() < 1e-5);
    let dbfs_dc = vad.calculate_dbfs(&dc_half);
    // 20 * log10(0.5) ≈ -6.0205999
    assert!((dbfs_dc - (-6.0206)).abs() < 1e-2);

    // 3. Sine wave with peak amplitude A = 0.2
    // Expected RMS = A / sqrt(2) ≈ 0.2 * 0.70710678 = 0.141421356
    let mut sine_samples = Vec::with_capacity(1600);
    for i in 0..1600 {
        let t = i as f32 / 16000.0;
        sine_samples.push(0.2 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
    }
    let rms_sine = vad.calculate_rms_energy(&sine_samples);
    assert!(
        (rms_sine - 0.14142).abs() < 1e-3,
        "Sine RMS expected ~0.14142, got {}",
        rms_sine
    );
    let dbfs_sine = vad.calculate_dbfs(&sine_samples);
    // 20 * log10(0.14142) ≈ -16.9897
    assert!(
        (dbfs_sine - (-16.99)).abs() < 0.1,
        "Sine dBFS expected ~ -16.99, got {}",
        dbfs_sine
    );

    // 4. Nyquist square wave alternating +0.8 and -0.8
    let mut square_samples = Vec::with_capacity(1600);
    for i in 0..1600 {
        square_samples.push(if i % 2 == 0 { 0.8 } else { -0.8 });
    }
    let rms_square = vad.calculate_rms_energy(&square_samples);
    assert!((rms_square - 0.8).abs() < 1e-5);
}

#[test]
fn test_adversarial_vad_trim_silence_pre_post_roll_bounds() {
    let config = VadConfig {
        pre_roll_samples: 400,
        post_roll_samples: 400,
        frame_size: 320,
        hop_size: 160,
        energy_threshold: 0.05,
        ..Default::default()
    };
    let vad = VoiceActivityDetector::new(config);

    // Scenario A: Speech at very beginning (index 0) - pre-roll cannot underflow
    let mut speech_start = vec![0.0f32; 3200];
    for s in &mut speech_start[0..1000] {
        *s = 0.5;
    }
    let (trimmed_a, meta_a) = vad.trim_silence(&speech_start);
    assert!(!trimmed_a.is_empty());
    assert_eq!(meta_a.leading_trimmed_samples, 0); // 0 saturating_sub 400 is 0
    assert!(meta_a.trimmed_samples > 1000);

    // Scenario B: Speech extends to the very end of buffer - post-roll cannot exceed length
    let mut speech_end = vec![0.0f32; 3200];
    for s in &mut speech_end[2000..3200] {
        *s = 0.5;
    }
    let (trimmed_b, meta_b) = vad.trim_silence(&speech_end);
    assert!(!trimmed_b.is_empty());
    assert_eq!(meta_b.trailing_trimmed_samples, 0); // post-roll capped at 3200
    assert_eq!(meta_b.original_samples, 3200);

    // Scenario C: Buffer too short (< min_samples 1600) evaluated returns Discarded TooShort
    let short_buf = vec![0.5f32; 500];
    let decision = vad.evaluate(&short_buf);
    assert_eq!(
        decision,
        VadDecision::Discarded {
            reason: DiscardReason::TooShort,
            sample_count: 500,
        }
    );
}

#[test]
fn test_adversarial_vad_streaming_hangover_smoothing_and_reset() {
    let config = VadConfig {
        energy_threshold: 0.05,
        hangover_duration_ms: 40, // 40ms * 16 samples/ms = 640 samples = 2 frames of 320
        ..Default::default()
    };
    let mut vad = VoiceActivityDetector::new(config);

    let loud_frame = vec![0.3f32; 320];
    let silent_frame = vec![0.0f32; 320];

    // Frame 1: Loud -> Speech
    assert_eq!(vad.process_stream_frame(&loud_frame), StreamVadState::Speech);

    // Frame 2: Silent -> Hangover frame 1
    assert_eq!(vad.process_stream_frame(&silent_frame), StreamVadState::Hangover);

    // Frame 3: Silent -> Hangover frame 2
    assert_eq!(vad.process_stream_frame(&silent_frame), StreamVadState::Hangover);

    // Frame 4: Silent -> Exhausted hangover, decays to Silence
    assert_eq!(vad.process_stream_frame(&silent_frame), StreamVadState::Silence);

    // Re-trigger speech then test reset_stream
    assert_eq!(vad.process_stream_frame(&loud_frame), StreamVadState::Speech);
    vad.reset_stream();
    assert_eq!(vad.current_state(), StreamVadState::Silence);
}

// =========================================================================
// 2. Audio Ring Buffer Circular FIFO Wrap-Around Invariants
// =========================================================================

#[test]
fn test_adversarial_ring_buffer_exact_capacity_and_batch_wraparound() {
    // 1. Minimum capacity = 1
    let rb_one = AudioRingBuffer::new(1);
    assert_eq!(rb_one.capacity(), 1);
    rb_one.push(&[1.0, 2.0, 3.0]);
    assert_eq!(rb_one.len(), 1);
    assert_eq!(rb_one.pop_all(), vec![3.0]);

    // 2. Wrap-around with multiple pushes
    let rb = AudioRingBuffer::new(5);
    rb.push(&[1.0, 2.0, 3.0]);
    assert_eq!(rb.len(), 3);
    assert_eq!(rb.peek(2), vec![1.0, 2.0]);
    assert_eq!(rb.len(), 3); // peek did not consume

    // Push 4 more (total 7 > 5): 1.0 and 2.0 must be dropped
    rb.push(&[4.0, 5.0, 6.0, 7.0]);
    assert_eq!(rb.len(), 5);
    assert_eq!(rb.pop_all(), vec![3.0, 4.0, 5.0, 6.0, 7.0]);
    assert!(rb.is_empty());

    // 3. Batch exceeding capacity directly
    let huge_batch = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
    rb.push(&huge_batch);
    assert_eq!(rb.len(), 5);
    assert_eq!(rb.pop_all(), vec![40.0, 50.0, 60.0, 70.0, 80.0]);

    // 4. Repeated push/pop churn (1,000 cycles) maintaining FIFO ordering
    let rb_stress = AudioRingBuffer::new(10);
    for i in 0..1000 {
        let val = i as f32;
        rb_stress.push(&[val]);
        if i >= 10 {
            assert_eq!(rb_stress.len(), 10);
            let popped = rb_stress.pop(1);
            assert_eq!(popped[0], (i - 9) as f32);
        }
    }
}

// =========================================================================
// 3. Hotkey State Machine Timing Invariants
// =========================================================================

#[test]
fn test_adversarial_hotkey_ptt_micro_press_and_jitter() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);
    assert_eq!(sm.min_press_duration_ms(), 100);

    // 1. Accidental micro-press: 50ms duration (<100ms)
    let down_res = sm.on_key_down(1000, Some(101));
    assert!(matches!(
        down_res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Listening,
            action: StateMachineAction::StartRecording { target_window_id: Some(101) },
            ..
        }
    ));

    let up_short = sm.on_key_up(1050);
    assert!(matches!(
        up_short,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
            ..
        }
    ));
    assert_eq!(sm.state(), &SpeechState::Idle);

    // 2. Boundary test: 99ms duration (<100ms) -> cancelled
    sm.on_key_down(2000, Some(102));
    let up_99 = sm.on_key_up(2099);
    assert!(matches!(
        up_99,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
            ..
        }
    ));

    // 3. Exact threshold: 100ms duration (>=100ms) -> transitions to Transcribing
    sm.on_key_down(3000, Some(103));
    let up_100 = sm.on_key_up(3100);
    assert!(matches!(
        up_100,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));
    assert_eq!(sm.state(), &SpeechState::Transcribing);

    // 4. Reverse clock jitter: timestamp_up < timestamp_down
    sm.reset();
    sm.on_key_down(5000, Some(104));
    let up_jitter = sm.on_key_up(4500); // 4500 < 5000 -> saturating_sub is 0 < 100ms
    assert!(matches!(
        up_jitter,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
            ..
        }
    ));
}

#[test]
fn test_adversarial_hotkey_toggle_mode_complete_flow_and_spurious_events() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::Toggle, None);

    // Press 1 (Key Down): Idle -> Listening
    let down_1 = sm.on_key_down(100, Some(201));
    assert!(matches!(
        down_1,
        StateTransitionResult::Transitioned {
            to: SpeechState::Listening,
            action: StateMachineAction::StartRecording { target_window_id: Some(201) },
            ..
        }
    ));

    // Key Up 1 is ignored in toggle mode
    let up_1 = sm.on_key_up(150);
    assert_eq!(up_1, StateTransitionResult::NoChange);
    assert_eq!(sm.state(), &SpeechState::Listening);

    // Press 2 (Key Down): Listening -> Transcribing
    let down_2 = sm.on_key_down(800, Some(201));
    assert!(matches!(
        down_2,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));

    // Spurious Key Down during Transcribing is ignored
    let down_spurious = sm.on_key_down(900, None);
    assert_eq!(down_spurious, StateTransitionResult::NoChange);

    // Engine finishes transcription
    let payload = TranscriptionPayload {
        text: "Voice command injected".to_string(),
        duration_ms: 700,
        confidence: 0.95,
        target_window_title: None,
        target_process_name: None,
    };
    let trans_done = sm.on_transcription_completed(payload);
    assert!(matches!(
        trans_done,
        StateTransitionResult::Transitioned {
            to: SpeechState::Injecting,
            action: StateMachineAction::StartInjection,
            ..
        }
    ));

    // Injection completes -> Idle
    let inj_done = sm.on_injection_completed();
    assert!(matches!(
        inj_done,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::ReturnToIdle,
            ..
        }
    ));
    assert_eq!(sm.state(), &SpeechState::Idle);
}

// =========================================================================
// 4. Active Window Text Injection & Clipboard Fallback Invariants
// =========================================================================

#[tokio::test]
async fn test_adversarial_text_injection_refocus_and_clipboard_fallback() {
    let (provider, controllers) = create_mock_platform();

    // Set up window 101 (focused) and window 102 (unfocused)
    controllers.window.add_window(WindowInfo {
        id: 101,
        process_id: 100,
        process_name: "app1.exe".to_string(),
        title: "App 1".to_string(),
        bounds: WindowRect::default(),
        is_focused: true,
        is_minimized: false,
    });
    controllers.window.add_window(WindowInfo {
        id: 102,
        process_id: 200,
        process_name: "app2.exe".to_string(),
        title: "App 2".to_string(),
        bounds: WindowRect::default(),
        is_focused: false,
        is_minimized: false,
    });

    let config = TextInjectionConfig {
        clipboard_fallback_threshold: 500, // Small threshold for testing
        refocus_settle_delay_ms: 0,
        ..Default::default()
    };

    let injector = ActiveWindowTextInjector::new(
        provider.window.clone(),
        provider.input.clone(),
        Some(config),
    );

    // 1. Empty string injection ignored
    let empty_report = injector.inject("", Some(101)).await.unwrap();
    assert_eq!(empty_report.status, InjectionStatus::Ignored);
    assert_eq!(empty_report.method, InjectionMethod::None);

    // 2. Injection to unfocused window 102 triggers refocus
    let normal_text = "Short text direct typing";
    let report_102 = injector.inject(normal_text, Some(102)).await.unwrap();
    assert_eq!(report_102.status, InjectionStatus::Success);
    assert_eq!(report_102.method, InjectionMethod::SendInputUnicode);

    // Verify window 102 was refocused
    let active = provider.window.get_active_window().unwrap();
    assert_eq!(active.id, 102);

    // 3. Huge text exceeding threshold (600 chars > 500) triggers clipboard paste fallback
    let huge_text = "A".repeat(600);
    let report_huge = injector.inject(&huge_text, Some(102)).await.unwrap();
    assert_eq!(report_huge.status, InjectionStatus::Success);
    assert_eq!(report_huge.method, InjectionMethod::ClipboardPaste);

    let pasted = controllers.input.get_pasted_strings();
    assert_eq!(pasted.len(), 1);
    assert_eq!(pasted[0].len(), 600);

    // 4. Injection to non-existent target window fails with WindowFocus error
    let missing_win_res = injector.inject("Text", Some(999999)).await;
    assert!(
        missing_win_res.is_err(),
        "Injecting to missing target window must return Err"
    );
}
