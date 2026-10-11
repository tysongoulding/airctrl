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
