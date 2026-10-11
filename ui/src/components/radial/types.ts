export type SectorIconName = "window" | "x" | "layout" | "cpu" | "list" | "gear" | "terminal" | "mic" | "play";

export interface RadialSectorItem {
  id: number;
  label: string;
  icon: SectorIconName;
  startAngleDeg: number;
  endAngleDeg: number;
  actionCommand: string;
  description: string;
}

export const RADIAL_SECTORS: RadialSectorItem[] = [
  {
    id: 0,
    label: "Focus Window",
    icon: "window",
    startAngleDeg: 0,
    endAngleDeg: 60,
    actionCommand: "window_focus",
    description: "Cycle or bring target window to foreground",
  },
  {
    id: 1,
    label: "Close Window",
    icon: "x",
    startAngleDeg: 60,
    endAngleDeg: 120,
    actionCommand: "window_close",
    description: "Gracefully terminate active foreground window",
  },
  {
    id: 2,
    label: "Snap Window",
    icon: "layout",
    startAngleDeg: 120,
    endAngleDeg: 180,
    actionCommand: "window_snap",
    description: "Tile or snap window to desktop grid layout",
  },
  {
    id: 3,
    label: "Agent Dispatch",
    icon: "cpu",
    startAngleDeg: 180,
    endAngleDeg: 240,
    actionCommand: "agent_dispatch",
    description: "Dispatch prompt to Cursor, Claude, Antigravity, or Grokbot",
  },
  {
    id: 4,
    label: "App Crawler",
    icon: "list",
    startAngleDeg: 240,
    endAngleDeg: 300,
    actionCommand: "app_crawler",
    description: "Enumerate running processes and extract UI structure",
  },
  {
    id: 5,
    label: "Settings",
    icon: "gear",
    startAngleDeg: 300,
    endAngleDeg: 360,
    actionCommand: "settings_hud",
    description: "Open airctrl settings HUD and configuration controls",
  },
];

export const RADIAL_DIAL_CONFIG = {
  width: 600,
  height: 600,
  radius: 300,
  deadzoneRadiusPx: 30, // r < 0.10 * 300 = 30px
  innerAnnulusPx: 80,
  outerAnnulusPx: 285,
  inactivityTimeoutSecs: 3.0,
  hysteresisRad: 0.05,
  activeClass: "scale-105 bg-indigo-600 text-white shadow-lg",
} as const;
