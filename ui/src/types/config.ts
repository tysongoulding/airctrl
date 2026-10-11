/**
 * Application item discovered by the Windows App Crawler.
 */
export interface ApplicationItem {
  id: string;
  name: string;
  path: string;
  icon?: string;
  category: string;
}

/**
 * Settings state matching airctrl-core::AirCtrlConfig and tests.
 */
export interface SettingsState {
  // STT Configuration
  hotkey_stt: string; // "Ctrl+Win" or "Ctrl+Spacebar"
  push_to_talk: boolean; // true = push-to-talk, false = toggle
  whisper_model: string; // "models/ggml-tiny.en.bin"

  // Speech-to-Action & Gestures
  hotkey_dial: string; // "Ctrl+Spacebar" or "Ctrl+Alt"
  radial_dial_sectors: number; // 6 (bounds: 4..8)
  gesture_fps_target: number; // 30
  deadzone_radius: number; // 0.10
  angular_hysteresis: number; // 0.05
  dial_timeout_secs: number; // 3.0

  // Meeting Privacy Controls
  camera_auto_kill: boolean; // true
  compliance_audio_enabled: boolean; // true

  // Configured Sectors (0 to 5)
  dial_sectors?: DialSectorDefinition[];

  // Legacy/Optional for compatibility
  lan_port?: number;

  // HUD UI Customization
  theme: "dark" | "light" | "system";
  hud_opacity: number; // 0.0 to 1.0, default 0.95
}

export type SectorIconType = "window" | "x" | "layout" | "cpu" | "list" | "gear" | "terminal" | "mic" | "play";

export interface DialSectorDefinition {
  id: number;
  label: string; // length: 1..30 characters
  icon: SectorIconType;
  action_type?: "app" | "window" | "agent" | "system";
  target?: string;
  description?: string;
}

export const DEFAULT_DIAL_SECTORS: DialSectorDefinition[] = [
  { id: 0, label: "Cursor IDE", icon: "cpu", action_type: "app", target: "cursor.exe", description: "Open or focus Cursor AI editor" },
  { id: 1, label: "VS Code", icon: "layout", action_type: "app", target: "code.exe", description: "Open or focus Visual Studio Code" },
  { id: 2, label: "Snap Left", icon: "window", action_type: "window", target: "left", description: "Snap active window to screen left" },
  { id: 3, label: "Snap Right", icon: "window", action_type: "window", target: "right", description: "Snap active window to screen right" },
  { id: 4, label: "Terminal", icon: "terminal", action_type: "app", target: "wt.exe", description: "Launch Windows Terminal" },
  { id: 5, label: "Agent Dispatch", icon: "play", action_type: "agent", target: "claude", description: "Dispatch prompt to autonomous agent" },
];

export const DEFAULT_SETTINGS: SettingsState = {
  hotkey_stt: "Ctrl+Win",
  hotkey_dial: "Ctrl+Spacebar",
  push_to_talk: true,
  whisper_model: "models/ggml-tiny.en.bin",
  radial_dial_sectors: 6,
  gesture_fps_target: 30,
  deadzone_radius: 0.1,
  angular_hysteresis: 0.05,
  dial_timeout_secs: 3.0,
  camera_auto_kill: true,
  compliance_audio_enabled: true,
  dial_sectors: DEFAULT_DIAL_SECTORS,
  lan_port: 8080,
  theme: "dark",
  hud_opacity: 0.95,
};
