/**
 * Application item discovered by the Windows App Crawler.
 */
export interface ApplicationItem {
  id: string;
  name: string;
  path: string;
  icon?: string;
  category: string;
  running?: boolean;
}

export interface GestureKeybindMapping {
  gestureId: string;
  gesture: string;
  condition: string;
  badge: string;
  desc: string;
  keybind: string;
  action: string;
  enabled: boolean;
}

export const DEFAULT_GESTURE_KEYBINDS: GestureKeybindMapping[] = [
  {
    gestureId: "pinch",
    gesture: "Pinch (Thumb + Index)",
    action: "left click",
    keybind: "Left Click / Enter",
    condition: "Euclidean distance d ≤ 0.05",
    badge: "Primary Click",
    desc: "Brings thumb tip (4) and index tip (8) together to confirm and execute the active radial dial sector or click.",
    enabled: true,
  },
  {
    gestureId: "pointing",
    gesture: "Pointing Ray (Index Extended)",
    action: "radial dial",
    keybind: "Ctrl+Spacebar",
    condition: "Ray vector atan2(dy, dx) from MCP (5)",
    badge: "Spatial Pointer",
    desc: "Projects an index finger pointing vector through polar coordinates to highlight sectors 0 through 5.",
    enabled: true,
  },
  {
    gestureId: "fist",
    gesture: "Fist (All Fingers Curled)",
    action: "press escape",
    keybind: "Escape",
    condition: "Fingertip to wrist distance < 80px",
    badge: "Cancel / Escape",
    desc: "Curl fingers into a closed fist to dismiss overlays or send Escape to the active window.",
    enabled: true,
  },
  {
    gestureId: "open_palm",
    gesture: "Open Palm (Fingers Open)",
    action: "center window",
    keybind: "Neutral / Center",
    condition: "All 5 fingertips extended",
    badge: "Neutral State",
    desc: "Keeps the dial in a neutral resting hover state (or optionally centers the active window).",
    enabled: true,
  },
  {
    gestureId: "swipe_left",
    gesture: "Velocity Swipe Left",
    action: "snap window left",
    keybind: "Win+Left",
    condition: "Palm velocity ≥ 1.2 m/s left",
    badge: "Window Snap Left",
    desc: "Fast leftward hand swipe snaps the focused window to the left half of your screen.",
    enabled: true,
  },
  {
    gestureId: "swipe_right",
    gesture: "Velocity Swipe Right",
    action: "snap window right",
    keybind: "Win+Right",
    condition: "Palm velocity ≥ 1.2 m/s right",
    badge: "Window Snap Right",
    desc: "Fast rightward hand swipe snaps the focused window to the right half of your screen.",
    enabled: true,
  },
  {
    gestureId: "v_sign",
    gesture: "V-Sign (Peace Sign)",
    action: "next window",
    keybind: "Alt+Tab",
    condition: "Index & Middle extended, others curled",
    badge: "Switch Window / Mute",
    desc: "Hold up index and middle fingers to cycle windows or toggle microphone mute hands-free.",
    enabled: true,
  },
];

/**
 * Settings state matching airctrl-core::AirCtrlConfig and tests.
 */
export interface SettingsState {
  // STT Configuration
  hotkey_stt: string; // "Ctrl+Win" or "Ctrl+Spacebar"
  push_to_talk: boolean; // true = push-to-talk, false = toggle
  whisper_model: string; // "models/ggml-tiny.en.bin"
  stt_mode?: "smart_dual" | "push_to_talk" | "toggle" | "continuous";
  stt_overlay_position?: "bottom" | "top" | "center" | "none";
  stt_audio_cues?: boolean;
  stt_auto_paste?: boolean;
  stt_reliable_paste?: boolean;
  stt_paste_delay_before_ms?: number;
  stt_paste_delay_after_ms?: number;
  stt_post_process?: boolean;

  // Speech-to-Action & Gestures
  hotkey_sta?: string; // "Ctrl+Spacebar" or "Ctrl+Shift+Space"
  sta_agent_target?: string; // "cursor" | "claude" | "antigravity" | "grok"
  sta_fuzzy_threshold?: number; // 0.75
  sta_acronym_boost?: boolean; // true
  sta_continuous_wake?: boolean; // true
  hotkey_dial: string; // "Ctrl+Spacebar" or "Ctrl+Alt"
  radial_dial_sectors: number; // 6 (bounds: 4..8)
  gesture_fps_target: number; // 30
  deadzone_radius: number; // 0.10
  angular_hysteresis: number; // 0.05
  dial_timeout_secs: number; // 3.0
  swap_hand_labels?: boolean; // false (camera perspective corrected by default)
  live_gesture_execution?: boolean; // true
  gesture_keybinds?: GestureKeybindMapping[];

  // Meeting Privacy Controls, Meetily & Virtual Camera Splitter
  camera_auto_kill: boolean; // false when virtual camera splitter active
  compliance_audio_enabled: boolean; // true
  virtual_camera_enabled?: boolean; // true ("AirCtrl Virtual Camera")
  virtual_camera_privacy_mute?: boolean; // false
  meetily_provider?: "ollama" | "claude" | "groq" | "openrouter" | "custom_openai";
  meetily_model?: string;
  meetily_endpoint?: string;

  // Configured Sectors (0 to 5)
  dial_sectors?: DialSectorDefinition[];

  // Legacy/Optional for compatibility
  lan_port?: number;

  // HUD UI Customization
  theme: "dark" | "light" | "system";
  hud_opacity: number; // 1.0
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
  hotkey_sta: "Ctrl+Shift+Space",
  hotkey_dial: "Ctrl+Spacebar",
  sta_agent_target: "cursor",
  sta_fuzzy_threshold: 0.75,
  sta_acronym_boost: true,
  push_to_talk: true,
  whisper_model: "models/ggml-tiny.en.bin",
  radial_dial_sectors: 6,
  gesture_fps_target: 30,
  deadzone_radius: 0.1,
  angular_hysteresis: 0.05,
  dial_timeout_secs: 3.0,
  swap_hand_labels: false,
  live_gesture_execution: true,
  gesture_keybinds: DEFAULT_GESTURE_KEYBINDS,
  camera_auto_kill: true,
  compliance_audio_enabled: true,
  dial_sectors: DEFAULT_DIAL_SECTORS,
  lan_port: 8080,
  theme: "dark",
  hud_opacity: 1.0,
};
