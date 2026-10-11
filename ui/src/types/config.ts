/**
 * Settings state matching test_t1_f25_radial_settings_ui and airctrl-core::AirCtrlConfig.
 */
export interface SettingsState {
  // STT Configuration
  hotkey_stt: string; // "Ctrl+Spacebar"
  push_to_talk: boolean; // true = push-to-talk, false = toggle
  whisper_model: string; // "models/ggml-tiny.en.bin"

  // Speech-to-Action & Gestures
  hotkey_dial: string; // "Ctrl+Alt"
  radial_dial_sectors: number; // 6 (bounds: 4..8)
  gesture_fps_target: number; // 30
  deadzone_radius: number; // 0.10
  angular_hysteresis: number; // 0.05
  dial_timeout_secs: number; // 3.0

  // Meeting Privacy Controls
  camera_auto_kill: boolean; // true
  compliance_audio_enabled: boolean; // true

  // LAN Deployment
  lan_port: number; // 8080

  // HUD UI Customization
  theme: "dark" | "light" | "system";
  hud_opacity: number; // 0.0 to 1.0, default 0.95
}

export interface DialSectorDefinition {
  id: number;
  label: string; // length: 1..30 characters
  icon: "window" | "x" | "layout" | "cpu" | "list" | "gear";
  description?: string;
}

export const DEFAULT_DIAL_SECTORS: DialSectorDefinition[] = [
  { id: 0, label: "Focus Window", icon: "window", description: "Bring active window to foreground" },
  { id: 1, label: "Close Window", icon: "x", description: "Close the targeted window" },
  { id: 2, label: "Snap Window", icon: "layout", description: "Snap window to screen quadrants" },
  { id: 3, label: "Agent Dispatch", icon: "cpu", description: "Route instructions to AI agents" },
  { id: 4, label: "App Crawler", icon: "list", description: "Index active windows and processes" },
  { id: 5, label: "Settings", icon: "gear", description: "Open HUD settings dashboard" },
];

export const DEFAULT_SETTINGS: SettingsState = {
  hotkey_stt: "Ctrl+Spacebar",
  hotkey_dial: "Ctrl+Alt",
  push_to_talk: true,
  whisper_model: "models/ggml-tiny.en.bin",
  radial_dial_sectors: 6,
  gesture_fps_target: 30,
  deadzone_radius: 0.1,
  angular_hysteresis: 0.05,
  dial_timeout_secs: 3.0,
  camera_auto_kill: true,
  compliance_audio_enabled: true,
  lan_port: 8080,
  theme: "dark",
  hud_opacity: 0.95,
};
