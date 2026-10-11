//! Challenger Hotkey State Machine Fuzzing & Adversarial Stress Tests
//!
//! Objectives:
//! 1. Rapid key-down/key-up flutter & bounce stress.
//! 2. Micro-press timing jitter (0ms, 1ms, 50ms, 99ms -> cancel; 100ms, 101ms -> transcribe; clock skew; timestamp overflow).
//! 3. Concurrent toggle presses & multi-threaded race conditions (12 threads x 10,000 ops).
//! 4. Out-of-order event dispatch fuzzing (premature events across all states).
//! 5. Error state isolation and reset recovery.

use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::{HotkeyMode, SpeechState, TranscriptionPayload};
use airctrl_stt::{HotkeyStateMachine, StateMachineAction, StateTransitionResult};
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_fuzz_micro_press_timing_jitter_boundaries() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // 1. 0ms tap (instantaneous key down and up at same millisecond)
    sm.on_key_down(1000, Some(10));
    assert_eq!(*sm.state(), SpeechState::Listening);
    let res = sm.on_key_up(1000);
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // 2. 1ms tap
    sm.on_key_down(2000, Some(10));
    let res = sm.on_key_up(2001);
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // 3. 50ms tap
    sm.on_key_down(3000, Some(10));
    let res = sm.on_key_up(3050);
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // 4. 99ms press (strictly below 100ms threshold)
    sm.on_key_down(4000, Some(10));
    let res = sm.on_key_up(4099);
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // 5. Exactly 100ms press (threshold boundary) -> must transcribe
    sm.on_key_down(5000, Some(10));
    let res = sm.on_key_up(5100);
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

    // 6. 101ms press (threshold + 1) -> must transcribe
    sm.on_key_down(6000, Some(10));
    let res = sm.on_key_up(6101);
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

    // 7. Clock skew (key_up timestamp is earlier than key_down timestamp)
    sm.on_key_down(7000, Some(10));
    let res = sm.on_key_up(6950);
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // 8. Timestamp overflow boundary (near u64::MAX)
    let base = u64::MAX - 500;
    sm.on_key_down(base, Some(10));
    let res = sm.on_key_up(base + 100);
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

    // 9. Configurable custom threshold (250ms)
    sm.set_min_press_duration_ms(250);
    sm.on_key_down(8000, Some(10));
    let res = sm.on_key_up(8249); // 249ms < 250ms
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Idle,
            action: StateMachineAction::CancelRecordingDiscardBuffer,
        }
    );
    sm.on_key_down(9000, Some(10));
    let res = sm.on_key_up(9250); // 250ms >= 250ms
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
fn test_fuzz_rapid_keydown_keyup_flutter() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // 5,000 rapid flutter cycles alternating micro-press (<100ms) and valid press (>=100ms)
    let mut current_time = 1_000_000u64;
    for i in 0..5_000 {
        let is_valid = i % 2 == 1;
        let duration = if is_valid { 120 } else { 45 };

        let res_down = sm.on_key_down(current_time, Some(42));
        assert_eq!(*sm.state(), SpeechState::Listening);
        assert!(matches!(
            res_down,
            StateTransitionResult::Transitioned {
                to: SpeechState::Listening,
                ..
            }
        ));

        current_time += duration;
        let res_up = sm.on_key_up(current_time);

        if is_valid {
            assert_eq!(*sm.state(), SpeechState::Transcribing);
            assert_eq!(
                res_up,
                StateTransitionResult::Transitioned {
                    from: SpeechState::Listening,
                    to: SpeechState::Transcribing,
                    action: StateMachineAction::StopRecordingAndTranscribe,
                }
            );

            // Complete transcription and injection cycle
            sm.on_transcription_completed(TranscriptionPayload {
                text: format!("Flutter speech {}", i),
                duration_ms: duration,
                confidence: 0.95,
                target_window_title: None,
                target_process_name: None,
            });
            assert_eq!(*sm.state(), SpeechState::Injecting);
            sm.on_injection_completed();
            assert_eq!(*sm.state(), SpeechState::Idle);
        } else {
            assert_eq!(*sm.state(), SpeechState::Idle);
            assert_eq!(
                res_up,
                StateTransitionResult::Transitioned {
                    from: SpeechState::Listening,
                    to: SpeechState::Idle,
                    action: StateMachineAction::CancelRecordingDiscardBuffer,
                }
            );
        }

        current_time += 10;
    }
}

#[test]
fn test_fuzz_keyboard_auto_repeat_and_unsolicited_chatter() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // 1. Initial key down at t=1000
    let res = sm.on_key_down(1000, Some(101));
    assert!(matches!(res, StateTransitionResult::Transitioned { .. }));
    assert_eq!(*sm.state(), SpeechState::Listening);

    // 2. 500 repeated key_down events (OS auto-repeat chatter while holding key)
    for i in 1..=500 {
        let res_repeat = sm.on_key_down(1000 + i * 20, Some(101));
        assert_eq!(res_repeat, StateTransitionResult::NoChange);
        assert_eq!(*sm.state(), SpeechState::Listening);
    }

    // 3. Key up at t=11_020 (total duration 10,020ms) -> must transcribe
    let res_up = sm.on_key_up(11_020);
    assert_eq!(
        res_up,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );

    // 4. 500 unsolicited key_up events while already Transcribing -> must be NoChange
    for i in 1..=500 {
        let res_repeat_up = sm.on_key_up(11_020 + i * 10);
        assert_eq!(res_repeat_up, StateTransitionResult::NoChange);
        assert_eq!(*sm.state(), SpeechState::Transcribing);
    }
}

#[test]
fn test_fuzz_out_of_order_event_dispatch() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);
    let dummy_payload = TranscriptionPayload {
        text: "Out of order".to_string(),
        duration_ms: 100,
        confidence: 0.90,
        target_window_title: None,
        target_process_name: None,
    };

    // State: Idle
    assert_eq!(*sm.state(), SpeechState::Idle);
    assert_eq!(sm.on_key_up(1000), StateTransitionResult::NoChange);
    assert_eq!(
        sm.on_transcription_completed(dummy_payload.clone()),
        StateTransitionResult::NoChange
    );
    assert_eq!(sm.on_injection_completed(), StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Idle);

    // State: Listening
    sm.on_key_down(2000, Some(1));
    assert_eq!(*sm.state(), SpeechState::Listening);
    assert_eq!(
        sm.on_transcription_completed(dummy_payload.clone()),
        StateTransitionResult::NoChange
    );
    assert_eq!(sm.on_injection_completed(), StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Move to Transcribing
    sm.on_key_up(2200);
    assert_eq!(*sm.state(), SpeechState::Transcribing);
    assert_eq!(sm.on_key_down(2300, Some(1)), StateTransitionResult::NoChange);
    assert_eq!(sm.on_key_up(2400), StateTransitionResult::NoChange);
    assert_eq!(sm.on_injection_completed(), StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Transcribing);

    // Move to Injecting
    sm.on_transcription_completed(dummy_payload.clone());
    assert_eq!(*sm.state(), SpeechState::Injecting);
    assert_eq!(sm.on_key_down(2500, Some(1)), StateTransitionResult::NoChange);
    assert_eq!(sm.on_key_up(2600), StateTransitionResult::NoChange);
    assert_eq!(
        sm.on_transcription_completed(dummy_payload),
        StateTransitionResult::NoChange
    );
    assert_eq!(*sm.state(), SpeechState::Injecting);

    // Complete injection back to Idle
    sm.on_injection_completed();
    assert_eq!(*sm.state(), SpeechState::Idle);
}

#[test]
fn test_fuzz_error_state_isolation_and_reset_recovery() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // Inject device error
    let res = sm.on_error("Microphone device disconnected");
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Idle,
            to: SpeechState::Error("Microphone device disconnected".to_string()),
            action: StateMachineAction::ReportError("Microphone device disconnected".to_string()),
        }
    );
    assert_eq!(
        *sm.state(),
        SpeechState::Error("Microphone device disconnected".to_string())
    );

    // In Error state, normal key events must be ignored
    assert_eq!(sm.on_key_down(1000, Some(1)), StateTransitionResult::NoChange);
    assert_eq!(sm.on_key_up(1200), StateTransitionResult::NoChange);
    assert_eq!(
        *sm.state(),
        SpeechState::Error("Microphone device disconnected".to_string())
    );

    // Reset restores to Idle
    let res_reset = sm.reset();
    assert_eq!(
        res_reset,
        StateTransitionResult::Transitioned {
            from: SpeechState::Error("Microphone device disconnected".to_string()),
            to: SpeechState::Idle,
            action: StateMachineAction::ReturnToIdle,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Subsequent normal usage functions seamlessly
    sm.on_key_down(2000, Some(1));
    assert_eq!(*sm.state(), SpeechState::Listening);
    sm.on_key_up(2150);
    assert_eq!(*sm.state(), SpeechState::Transcribing);
}

#[test]
fn test_fuzz_concurrent_multi_threaded_hammer() {
    let sm = Arc::new(Mutex::new(HotkeyStateMachine::new(
        HotkeyMode::PushToTalk,
        None,
    )));

    let mut handles = Vec::new();
    let num_threads = 12;
    let ops_per_thread = 5_000;

    for thread_idx in 0..num_threads {
        let sm_clone = Arc::clone(&sm);
        handles.push(thread::spawn(move || {
            let mut lcg = (thread_idx as u64 + 1).wrapping_mul(6364136223846793005);
            for step in 0..ops_per_thread {
                lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let op = (lcg >> 32) % 6;
                let ts = (step as u64 * 10).wrapping_add(thread_idx as u64);

                let mut guard = sm_clone.lock().unwrap();
                match op {
                    0 => {
                        let _ = guard.on_key_down(ts, Some(thread_idx as u64));
                    }
                    1 => {
                        let _ = guard.on_key_up(ts + 120);
                    }
                    2 => {
                        let _ = guard.on_transcription_completed(TranscriptionPayload {
                            text: "Concurrent test".into(),
                            duration_ms: 120,
                            confidence: 0.9,
                            target_window_title: None,
                            target_process_name: None,
                        });
                    }
                    3 => {
                        let _ = guard.on_injection_completed();
                    }
                    4 => {
                        let mode = if (lcg & 1) == 0 {
                            HotkeyMode::PushToTalk
                        } else {
                            HotkeyMode::Toggle
                        };
                        guard.set_mode(mode);
                    }
                    _ => {
                        let _ = guard.reset();
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("Thread panicked during concurrent stress test");
    }

    // Verify final state machine is intact and can execute cleanly
    let mut final_sm = sm.lock().unwrap();
    final_sm.reset();
    assert_eq!(*final_sm.state(), SpeechState::Idle);

    final_sm.set_mode(HotkeyMode::PushToTalk);
    final_sm.on_key_down(100, Some(999));
    assert_eq!(*final_sm.state(), SpeechState::Listening);
    final_sm.on_key_up(250);
    assert_eq!(*final_sm.state(), SpeechState::Transcribing);
}

#[test]
fn test_fuzz_toggle_mode_rapid_concurrency_and_out_of_order() {
    let mut sm = HotkeyStateMachine::new(HotkeyMode::Toggle, None);

    // Initial state
    assert_eq!(*sm.state(), SpeechState::Idle);

    // Toggle 1st press: Idle -> Listening
    let res = sm.on_key_down(1000, Some(42));
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Idle,
            to: SpeechState::Listening,
            action: StateMachineAction::StartRecording {
                target_window_id: Some(42)
            },
        }
    );
    assert_eq!(*sm.state(), SpeechState::Listening);

    // Key up in Toggle mode must ALWAYS be NoChange
    for t in [1010, 1050, 1100, 1500] {
        assert_eq!(sm.on_key_up(t), StateTransitionResult::NoChange);
        assert_eq!(*sm.state(), SpeechState::Listening);
    }

    // Toggle 2nd press: Listening -> Transcribing
    let res = sm.on_key_down(2000, Some(42));
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Listening,
            to: SpeechState::Transcribing,
            action: StateMachineAction::StopRecordingAndTranscribe,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Transcribing);

    // Spurious 3rd press while already Transcribing must be NoChange
    assert_eq!(sm.on_key_down(2100, Some(42)), StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Transcribing);

    // Transcription finishes
    let res = sm.on_transcription_completed(TranscriptionPayload {
        text: "Toggle speech".into(),
        duration_ms: 1000,
        confidence: 0.98,
        target_window_title: None,
        target_process_name: None,
    });
    assert_eq!(
        res,
        StateTransitionResult::Transitioned {
            from: SpeechState::Transcribing,
            to: SpeechState::Injecting,
            action: StateMachineAction::StartInjection,
        }
    );
    assert_eq!(*sm.state(), SpeechState::Injecting);

    // Spurious press while Injecting must be NoChange
    assert_eq!(sm.on_key_down(2200, Some(42)), StateTransitionResult::NoChange);
    assert_eq!(*sm.state(), SpeechState::Injecting);

    // Injection completes
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
fn test_fuzz_event_bus_delivery_under_rapid_churn() {
    let bus = EventBus::new(2048);
    let mut rx = bus.subscribe();

    let mut sm = HotkeyStateMachine::new(HotkeyMode::PushToTalk, Some(bus));

    for i in 0..50 {
        sm.on_key_down(1000 + i * 1000, Some(i));
        sm.on_key_up(1000 + i * 1000 + 200);
        sm.on_transcription_completed(TranscriptionPayload {
            text: format!("Churn {}", i),
            duration_ms: 200,
            confidence: 0.9,
            target_window_title: None,
            target_process_name: None,
        });
        sm.on_injection_completed();
    }

    // Verify events were dispatched into bus
    let mut received_state_changes = 0;
    while let Ok(event) = rx.try_recv() {
        if let DomainEvent::SpeechStateChanged(_) = event {
            received_state_changes += 1;
        }
    }
    // Each iteration emits: Listening, Transcribing, Injecting, Idle = 4 states * 50 = 200
    assert_eq!(received_state_changes, 200);
}
