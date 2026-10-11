use airctrl_actions::*;
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::{ActionCommand, HotkeyMode, SnapDirection};

#[test]
fn test_hotkey_state_machine_ptt_happy_path() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, Some(bus));

    assert_eq!(*sm.state(), ActionState::Idle);

    // Key down at t=1000ms
    let res1 = sm.on_key_down(1000, Some(101));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    assert!(sm.dial().is_visible());
    assert!(matches!(res1, ActionStateTransitionResult::Transitioned { to: ActionState::ActionListening, .. }));

    // Key up at t=1500ms (500ms press duration >= 100ms)
    let res2 = sm.on_key_up(1500);
    assert_eq!(*sm.state(), ActionState::ActionParsing);
    assert!(matches!(res2, ActionStateTransitionResult::Transitioned { to: ActionState::ActionParsing, .. }));

    // Command parsed
    let cmd = ActionCommand::WindowSnap { direction: SnapDirection::Maximize };
    let res3 = sm.on_command_parsed(cmd.clone());
    assert_eq!(*sm.state(), ActionState::ActionExecuting);
    assert!(matches!(res3, ActionStateTransitionResult::Transitioned { to: ActionState::ActionExecuting, .. }));

    // Verify EventBus received ActionTriggered
    let event = rx.try_recv().expect("Expected ActionTriggered domain event");
    assert_eq!(event, DomainEvent::ActionTriggered(cmd));

    // Execution completed -> returns to Idle
    let res4 = sm.on_execution_completed();
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert!(matches!(res4, ActionStateTransitionResult::Transitioned { to: ActionState::Idle, .. }));
}

#[test]
fn test_hotkey_state_machine_ptt_micro_press_cancellation() {
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // Key down at t=1000ms
    sm.on_key_down(1000, Some(202));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    assert!(sm.dial().is_visible());

    // Key up at t=1050ms (50ms press duration < 100ms threshold)
    let res = sm.on_key_up(1050);
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert!(matches!(
        res,
        ActionStateTransitionResult::Transitioned {
            to: ActionState::Idle,
            instruction: ActionMachineInstruction::CancelListeningDiscardBuffer,
            ..
        }
    ));
}

#[test]
fn test_hotkey_state_machine_toggle_mode_flow() {
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::Toggle, None);

    // First key down triggers listening
    sm.on_key_down(1000, Some(303));
    assert_eq!(*sm.state(), ActionState::ActionListening);
    assert!(sm.dial().is_visible());

    // Key up in toggle mode does not stop listening
    sm.on_key_up(1200);
    assert_eq!(*sm.state(), ActionState::ActionListening);

    // Second key down triggers parsing
    let res = sm.on_key_down(2000, Some(303));
    assert_eq!(*sm.state(), ActionState::ActionParsing);
    assert!(matches!(res, ActionStateTransitionResult::Transitioned { to: ActionState::ActionParsing, .. }));
}

#[test]
fn test_hotkey_state_machine_inactivity_timeout_and_error() {
    let mut sm = ActionHotkeyStateMachine::new(HotkeyMode::PushToTalk, None);

    // Enter listening
    sm.on_key_down(1000, Some(404));
    assert_eq!(*sm.state(), ActionState::ActionListening);

    // Timeout triggers return to Idle
    let res_timeout = sm.on_inactivity_timeout();
    assert_eq!(*sm.state(), ActionState::Idle);
    assert!(!sm.dial().is_visible());
    assert!(matches!(res_timeout, ActionStateTransitionResult::Transitioned { to: ActionState::Idle, .. }));

    // Trigger error state and reset
    sm.on_error("Audio recording failed");
    assert!(matches!(sm.state(), ActionState::ActionError(_)));

    sm.reset();
    assert_eq!(*sm.state(), ActionState::Idle);
}
