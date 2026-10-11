import React, { useState } from "react";
import { Hand, Compass, Camera, Sliders, Sparkles } from "lucide-react";
import { SettingsState, ApplicationItem } from "../../types/config";
import { GesturesLiveView } from "./GesturesLiveView";
import { RadialConfigCard } from "./RadialConfigCard";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
  onAssignToSector?: (sectorId: number, app: ApplicationItem) => void;
}

const GESTURE_KEYBINDS = [
  {
    gesture: "Pinch (Thumb + Index)",
    action: "Click / Select Dial Sector",
    condition: "Euclidean distance d ≤ 0.05",
    badge: "Primary Click",
    desc: "Brings thumb tip (4) and index tip (8) together to confirm and execute the active radial dial sector.",
  },
  {
    gesture: "Pointing Ray (Index Extended)",
    action: "Aim & Navigate Radial Sectors",
    condition: "Ray vector atan2(dy, dx) from MCP (5)",
    badge: "Spatial Pointer",
    desc: "Projects an index finger pointing vector through polar coordinates to highlight sectors 0 through 5.",
  },
  {
    gesture: "Fist (All Fingers Curled)",
    action: "Dismiss / Close Radial Dial",
    condition: "Fingertip to wrist distance < 80px",
    badge: "Cancel / Close",
    desc: "Curl fingers into a closed fist to instantly abort radial dial selection and hide the transparent overlay.",
  },
  {
    gesture: "Open Palm (Fingers Open)",
    action: "Neutral / Idle Hover",
    condition: "All 5 fingertips extended",
    badge: "Neutral State",
    desc: "Keeps the dial active in a neutral resting state without triggering accidental clicks or dismissals.",
  },
  {
    gesture: "Velocity Swipe Left / Right",
    action: "Snap Window Left / Right",
    condition: "Palm velocity ≥ 1.2 m/s with 300ms cooldown",
    badge: "Window Automation",
    desc: "Fast horizontal hand swipe triggers instant OS-level snapping of the currently focused window.",
  },
  {
    gesture: "V-Sign (Peace Sign)",
    action: "Toggle Microphone Mute",
    condition: "Index & Middle extended, others curled",
    badge: "Audio Mute",
    desc: "Quickly toggle audio input mute without moving hands away from your natural desk posture.",
  },
];

export const GesturesConfigSection: React.FC<Props> = ({ settings, onUpdate }) => {
  const [activeTab, setActiveTab] = useState<"camera" | "dial" | "keybinds">("camera");

  return (
    <div className="flex flex-col gap-6">
      {/* Navigation Sub-Bar for Gestures */}
      <div className="flex items-center justify-between p-4 rounded-xl border border-border bg-card/60 backdrop-blur-xl">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-primary text-primary-foreground flex items-center justify-center shadow-sm">
            <Hand className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-xs font-semibold text-foreground flex items-center gap-2">
              Gestures & Spatial Radial Dial
            </h2>
            <p className="text-[11px] text-muted-foreground">
              Configure real-time MediaPipe hand tracking, 6-sector dial layout, and hand keybinds
            </p>
          </div>
        </div>

        <div className="flex items-center gap-1 bg-muted p-1 rounded-lg border border-border">
          <button
            onClick={() => setActiveTab("camera")}
            className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium transition-all ${
              activeTab === "camera"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Camera className="w-3.5 h-3.5" /> Live Tracking & Camera
          </button>
          <button
            onClick={() => setActiveTab("dial")}
            className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium transition-all ${
              activeTab === "dial"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Compass className="w-3.5 h-3.5" /> Radial Dial Setup
          </button>
          <button
            onClick={() => setActiveTab("keybinds")}
            className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium transition-all ${
              activeTab === "keybinds"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Sliders className="w-3.5 h-3.5" /> Hand Keybinds Reference
          </button>
        </div>
      </div>

      {/* Tab 1: Live Hand Tracking & 21-Landmark Canvas */}
      {activeTab === "camera" && <GesturesLiveView />}

      {/* Tab 2: 6-Sector Radial Dial Configuration */}
      {activeTab === "dial" && <RadialConfigCard settings={settings} onUpdate={onUpdate} />}

      {/* Tab 3: Hand Gesture Keybinds & Calibration */}
      {activeTab === "keybinds" && (
        <div className="flex flex-col gap-6">
          {/* Keybind Calibration Sliders */}
          <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-4 shadow-sm">
            <div className="flex items-center justify-between border-b border-border pb-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Sliders className="w-4 h-4 text-foreground" /> Spatial Sensitivity & Gesture Thresholds
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Fine-tune Euclidean distance gates and angle hysteresis for hand keybinds
                </p>
              </div>
              <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-[10px] font-mono">
                CPU Pipeline 30 FPS
              </span>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {/* Pinch Distance Threshold */}
              <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs font-medium text-foreground">Pinch Click Threshold</label>
                    <span className="text-xs font-mono text-primary font-bold">≤ 0.05</span>
                  </div>
                  <p className="text-[11px] text-muted-foreground mb-3">
                    Distance between thumb tip & index tip triggering a dial sector click
                  </p>
                </div>
                <input
                  type="range"
                  min="0.02"
                  max="0.10"
                  step="0.01"
                  defaultValue="0.05"
                  className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
                />
              </div>

              {/* Deadzone Radius */}
              <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs font-medium text-foreground">Center Deadzone</label>
                    <span className="text-xs font-mono text-primary font-bold">{settings.deadzone_radius.toFixed(2)}</span>
                  </div>
                  <p className="text-[11px] text-muted-foreground mb-3">
                    Normalized radius at center where no sector selection occurs
                  </p>
                </div>
                <input
                  type="range"
                  min="0.05"
                  max="0.25"
                  step="0.01"
                  value={settings.deadzone_radius}
                  onChange={(e) => onUpdate({ deadzone_radius: parseFloat(e.target.value) })}
                  className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
                />
              </div>

              {/* Angular Hysteresis */}
              <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs font-medium text-foreground">Angular Hysteresis</label>
                    <span className="text-xs font-mono text-primary font-bold">{settings.angular_hysteresis.toFixed(2)}</span>
                  </div>
                  <p className="text-[11px] text-muted-foreground mb-3">
                    Angle buffer preventing jitter when crossing sector boundaries
                  </p>
                </div>
                <input
                  type="range"
                  min="0.01"
                  max="0.15"
                  step="0.01"
                  value={settings.angular_hysteresis}
                  onChange={(e) => onUpdate({ angular_hysteresis: parseFloat(e.target.value) })}
                  className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
                />
              </div>
            </div>
          </div>

          {/* Full Keybinds Catalog */}
          <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-4 shadow-sm">
            <div className="flex items-center justify-between border-b border-border pb-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Sparkles className="w-4 h-4 text-foreground" /> Hand Keybinding Directory
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Complete gesture-to-action bindings for hands-free desktop orchestration
                </p>
              </div>
              <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-[10px] font-mono">
                {GESTURE_KEYBINDS.length} Keybinds Active
              </span>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {GESTURE_KEYBINDS.map((kb) => (
                <div
                  key={kb.gesture}
                  className="p-4 rounded-xl border border-border bg-card flex flex-col justify-between gap-3 shadow-sm hover:border-primary/40 transition-all"
                >
                  <div className="flex flex-col gap-1.5">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-semibold text-foreground">{kb.gesture}</span>
                      <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-primary/10 border border-primary/20 text-primary font-medium">
                        {kb.badge}
                      </span>
                    </div>
                    <div className="text-xs font-medium text-foreground flex items-center gap-1.5">
                      <span className="text-muted-foreground">Action:</span> {kb.action}
                    </div>
                    <p className="text-[11px] text-muted-foreground leading-relaxed">
                      {kb.desc}
                    </p>
                  </div>
                  <div className="pt-2 border-t border-border/50 text-[10px] font-mono text-muted-foreground">
                    Trigger Condition: {kb.condition}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
