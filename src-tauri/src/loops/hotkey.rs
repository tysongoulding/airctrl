use super::metrics::GlobalHotkeyHealthMetrics;
use airctrl_actions::{ActionHotkeyStateMachine, RadialDialOverlay};
use airctrl_core::dag::{HotkeyEventPayload, HotkeyTrigger, SensorEvent, ToggleOverlayPayload, OverlayType, SinkAction};
use airctrl_core::events::EventBus;
use airctrl_core::types::HotkeyMode;
use airctrl_stt::HotkeyStateMachine;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotkeyActionTarget {
    Stt,        // Ctrl+Spacebar
    RadialDial, // Ctrl+Alt / Ctrl+Win
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawKeyEvent {
    pub target: HotkeyActionTarget,
    pub direction: KeyDirection,
    pub timestamp_ms: u64,
    pub window_id: Option<u64>,
}

pub struct GlobalHotkeyLoop {
    pub stt_machine: HotkeyStateMachine,
    pub action_machine: ActionHotkeyStateMachine,
    pub event_bus: EventBus,
    pub metrics: Arc<RwLock<GlobalHotkeyHealthMetrics>>,
    pub event_rx: mpsc::Receiver<RawKeyEvent>,
}

impl GlobalHotkeyLoop {
    pub fn new(
        event_bus: EventBus,
        metrics: Arc<RwLock<GlobalHotkeyHealthMetrics>>,
        event_rx: mpsc::Receiver<RawKeyEvent>,
    ) -> Self {
        let stt_machine = HotkeyStateMachine::new(HotkeyMode::SmartDual, Some(event_bus.clone()));
        let action_machine = ActionHotkeyStateMachine::new(HotkeyMode::SmartDual, Some(event_bus.clone()));

        Self {
            stt_machine,
            action_machine,
            event_bus,
            metrics,
            event_rx,
        }
    }

    pub async fn run(
        mut self,
        token: CancellationToken,
        dial_overlay: Arc<RwLock<RadialDialOverlay>>,
    ) {
        self.metrics.write().is_running = true;
        let mut stt_start_ms: u64 = 0;
        let mut dial_start_ms: u64 = 0;

        while !token.is_cancelled() {
            tokio::select! {
                _ = token.cancelled() => break,
                Some(key_event) = self.event_rx.recv() => {
                    self.metrics.write().events_received += 1;

                    match key_event.target {
                        HotkeyActionTarget::Stt => {
                            match key_event.direction {
                                KeyDirection::Down => {
                                    stt_start_ms = key_event.timestamp_ms;
                                    self.stt_machine.on_key_down(key_event.timestamp_ms, key_event.window_id);
                                    self.metrics.write().stt_presses_count += 1;

                                    let _ = self.event_bus.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
                                        trigger: HotkeyTrigger::SttPushToTalkPress,
                                        raw_combination: "Ctrl+Spacebar".to_string(),
                                        duration_ms: 0,
                                        timestamp_ns: key_event.timestamp_ms * 1_000_000,
                                    }));
                                }
                                KeyDirection::Up => {
                                    let duration = key_event.timestamp_ms.saturating_sub(stt_start_ms);
                                    let _transition = self.stt_machine.on_key_up(key_event.timestamp_ms);

                                    let (mode_str, trigger) = if duration < 30 {
                                        ("Debounced", HotkeyTrigger::SttToggle)
                                    } else if duration < 250 {
                                        ("Toggle", HotkeyTrigger::SttToggle)
                                    } else {
                                        ("PushToTalk", HotkeyTrigger::SttPushToTalkRelease)
                                    };

                                    let _ = self.event_bus.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
                                        trigger,
                                        raw_combination: "Ctrl+Spacebar".to_string(),
                                        duration_ms: duration,
                                        timestamp_ns: key_event.timestamp_ms * 1_000_000,
                                    }));

                                    let mut m = self.metrics.write();
                                    m.last_press_duration_ms = duration;
                                    m.last_mode_detected = mode_str.to_string();
                                    m.stt_speech_state = format!("{:?}", self.stt_machine.state());
                                }
                            }
                        }
                        HotkeyActionTarget::RadialDial => {
                            match key_event.direction {
                                KeyDirection::Down => {
                                    dial_start_ms = key_event.timestamp_ms;
                                    self.action_machine.on_key_down(key_event.timestamp_ms, key_event.window_id);
                                    self.metrics.write().dial_presses_count += 1;
                                    dial_overlay.write().show();

                                    let _ = self.event_bus.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
                                        trigger: HotkeyTrigger::ActionRadialDial,
                                        raw_combination: "Ctrl+Alt".to_string(),
                                        duration_ms: 0,
                                        timestamp_ns: key_event.timestamp_ms * 1_000_000,
                                    }));
                                }
                                KeyDirection::Up => {
                                    let duration = key_event.timestamp_ms.saturating_sub(dial_start_ms);
                                    self.action_machine.on_key_up(key_event.timestamp_ms);

                                    let mode_str = if duration < 30 {
                                        dial_overlay.write().hide();
                                        "Debounced"
                                    } else if duration < 250 {
                                        // Toggle mode: dial remains latched open!
                                        let _ = self.event_bus.publish_sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                                            overlay: OverlayType::RadialDial,
                                            visible: Some(true),
                                            active_sector: None,
                                        }));
                                        "Toggle"
                                    } else {
                                        // Push-to-Talk mode: dial closes on release
                                        dial_overlay.write().hide();
                                        let _ = self.event_bus.publish_sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                                            overlay: OverlayType::RadialDial,
                                            visible: Some(false),
                                            active_sector: None,
                                        }));
                                        "PushToTalk"
                                    };

                                    let _ = self.event_bus.publish_sensor(SensorEvent::HotkeyEvent(HotkeyEventPayload {
                                        trigger: HotkeyTrigger::ActionRadialDial,
                                        raw_combination: "Ctrl+Alt".to_string(),
                                        duration_ms: duration,
                                        timestamp_ns: key_event.timestamp_ms * 1_000_000,
                                    }));

                                    let mut m = self.metrics.write();
                                    m.last_press_duration_ms = duration;
                                    m.last_mode_detected = mode_str.to_string();
                                    m.action_state = format!("{:?}", self.action_machine.state());
                                }
                            }
                        }
                    }

                    self.metrics.write().last_tick_timestamp_ms = key_event.timestamp_ms;
                }
            }
        }

        self.metrics.write().is_running = false;
    }
}
