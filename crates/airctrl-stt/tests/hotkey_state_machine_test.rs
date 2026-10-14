use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::{HotkeyMode, SpeechState, TranscriptionPayload};
use airctrl_stt::{
    HotkeyStateMachine, StateMachineAction, StateTransitionResult,
};

#[test]
fn test_ptt_happy_path_with_event_bus() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();

    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, Some(bus));
    assert_eq!(*sm.state(), SpeechState::Idle);

    // 1. Key down at 1000ms
    let res = sm.on_key_down(1000, Some(555));
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Listening,
            action: StateMachineAction::StartRecording { target_window_id: Some(555) },
            ..
        }
    ));

    // Verify event bus received SpeechStateChanged(Listening)
    let ev = rx.try_recv().expect("event received");
    assert_eq!(ev, DomainEvent::SpeechStateChanged(SpeechState::Listening));

    // 2. Key up at 1600ms (600ms press)
    let res = sm.on_key_up(1600);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));

    // 3. Transcription completed
    let payload = TranscriptionPayload {
        text: "Testing speech".to_string(),
        duration_ms: 600,
        confidence: 0.92,
        target_window_title: Some("Terminal".into()),
        target_process_name: Some("alacritty".into()),
    };
    let res = sm.on_transcription_completed(payload.clone());
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Injecting,
            action: StateMachineAction::StartInjection,
            ..
        }
    ));

    // 4. Injection completed
    let res = sm.on_injection_completed();
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::ReturnToIdle,
            ..
        }
    ));
}

#[test]
fn test_ptt_boundary_press_durations() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // 1ms tap -> Cancelled
    sm.on_key_down(1000, None);
    let res = sm.on_key_up(1001);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
            ..
        }
    ));

    // 99ms press -> Cancelled
    sm.on_key_down(2000, None);
    let res = sm.on_key_up(2099);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
            ..
        }
    ));

    // Exactly 100ms press -> Accepted
    sm.on_key_down(3000, None);
    let res = sm.on_key_up(3100);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));

    // Reset
    sm.reset();

    // 101ms press -> Accepted
    sm.on_key_down(4000, None);
    let res = sm.on_key_up(4101);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));

    // Reset
    sm.reset();

    // 30,000ms long press -> Accepted
    sm.on_key_down(10000, None);
    let res = sm.on_key_up(40000);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));
}

#[test]
fn test_toggle_mode_complete_flow() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::Toggle, None);

    // Initial state
    assert_eq!(*sm.state(), SpeechState::Idle);

    // First press -> Listening
    sm.on_key_down(1000, Some(42));
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Key up in toggle is ignored
    assert_eq!(sm.on_key_up(1200), StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Second press -> Transcribing
    sm.on_key_down(3000, Some(42));
    assert_eq!(*sm.state(), SpeechState::Transcribing);
}

#[test]
fn test_smart_dual_mode_hold_and_tap_flow() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::SmartDual, None);

    // Initial state
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Scenario A: Push-to-Talk Hold (held >= 250ms)
    sm.on_key_down(1000, Some(101));
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Key up after 300ms (held >= 250ms) -> transitions to Transcribing
    let res = sm.on_key_up(1300);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));
    assert_eq!(*sm.state(), SpeechState::Transcribing);

    // Reset back to Idle
    sm.reset();
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Scenario B: Quick Tap (< 250ms, >= 30ms) -> latches into Toggle Listening
    sm.on_key_down(2000, Some(102));
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Released after 80ms -> remains Listening!
    let res = sm.on_key_up(2080);
    assert_eq!(res, StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Listening);

    // User taps again at 3500ms -> stops and transcribes!
    let res = sm.on_key_down(3500, Some(102));
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
            ..
        }
    ));
    assert_eq!(*sm.state(), SpeechState::Transcribing);

    // Reset back to Idle
    sm.reset();

    // Scenario C: Cancel via Esc
    sm.on_key_down(4000, Some(103));
    assert_eq!(*sm.state(), SpeechState::Listening);
    let res = sm.cancel();
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
            ..
        }
    ));
    assert_eq!(*sm.state(), SpeechState::Idle);
}

#[test]
fn test_m8_smart_dual_mode_flutter_rejection() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::SmartDualMode, None);

    // Press at 1000ms
    sm.on_key_down(1000, None);
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Release at 1025ms (25ms < 30ms jitter threshold) -> Cancelled!
    let res = sm.on_key_up(1025);
    assert!(matches!(
        res,
        StateTransitionResult::Transitioned {
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
            ..
        }
    ));
    assert_eq!(*sm.state(), SpeechState::Idle);
}

#[test]
fn test_m8_hold_only_mode() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::HoldOnly, None);

    // Micro-press under 100ms
    sm.on_key_down(1000, None);
    let res = sm.on_key_up(1050);
    assert_eq!(*sm.state(), SpeechState::Idle);
    assert!(matches!(res, StateTransitionResult::Transitioned { action: StateMachineAction::CancelRecordingDiscardBuffer, .. }));

    // Hold over 100ms
    sm.on_key_down(2000, None);
    let res = sm.on_key_up(2200);
    assert_eq!(*sm.state(), SpeechState::Transcribing);
    assert!(matches!(res, StateTransitionResult::Transitioned { action: StateMachineAction::StopRecordingAndTranscribe, .. }));
}

#[test]
fn test_m8_toggle_only_mode() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::ToggleOnly, None);

    sm.on_key_down(1000, None);
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Key up does nothing
    assert_eq!(sm.on_key_up(1500), StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Second press stops and transcribes
    sm.on_key_down(2000, None);
    assert_eq!(*sm.state(), SpeechState::Transcribing);
}

#[test]
fn test_m8_continuous_hands_free_vad_lifecycle() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::ContinuousHandsFree, None);

    // Speech onset detected
    let res = sm.on_vad_speech_start(1000, Some(77));
    assert_eq!(*sm.state(), SpeechState::Listening);
    assert!(matches!(res, StateTransitionResult::Transitioned { action: StateMachineAction::StartRecording { target_window_id: Some(77) }, .. }));

    // Speech offset detected
    let res = sm.on_vad_speech_end(2500);
    assert_eq!(*sm.state(), SpeechState::Transcribing);
    assert!(matches!(res, StateTransitionResult::Transitioned { action: StateMachineAction::StopRecordingAndTranscribe, .. }));

    // Discard handling
    sm.reset();
    sm.on_vad_speech_start(3000, None);
    let res = sm.on_vad_silence_discard();
    assert_eq!(*sm.state(), SpeechState::Idle);
    assert!(matches!(res, StateTransitionResult::Transitioned { action: StateMachineAction::CancelRecordingDiscardBuffer, .. }));
}

