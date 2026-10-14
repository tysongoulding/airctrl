import React, { useState } from "react";
import { Hand, Compass, Camera, Sliders, Sparkles, Keyboard, Play, RotateCcw, Check } from "lucide-react";
import {
  SettingsState,
  ApplicationItem,
  DEFAULT_GESTURE_KEYBINDS,
  GestureKeybindMapping,
} from "../../types/config";
import { GesturesLiveView } from "./GesturesLiveView";
import { RadialConfigCard } from "./RadialConfigCard";
import { tauriApi } from "../../services/ipc";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
  onAssignToSector?: (sectorId: number, app: ApplicationItem) => void;
}

const PRESET_ACTIONS = [
  { label: "Left Click / Confirm", action: "left click", keybind: "Left Click / Enter" },
  { label: "Double Click", action: "double click", keybind: "Double Click" },
  { label: "Right Click", action: "right click", keybind: "Right Click" },
  { label: "Open Radial Dial", action: "radial dial", keybind: "Ctrl+Spacebar" },
  { label: "Press Escape / Dismiss", action: "press escape", keybind: "Escape" },
  { label: "Press Enter", action: "press enter", keybind: "Enter" },
  { label: "Press Tab (Next Field)", action: "press tab", keybind: "Tab" },
  { label: "Snap Window Left", action: "snap window left", keybind: "Win+Left" },
  { label: "Snap Window Right", action: "snap window right", keybind: "Win+Right" },
  { label: "Maximize Window", action: "maximize window", keybind: "Win+Up" },
  { label: "Minimize Window", action: "minimize window", keybind: "Win+Down" },
  { label: "Center Window", action: "center window", keybind: "Win+Center" },
  { label: "Switch Next Window (Alt+Tab)", action: "next window", keybind: "Alt+Tab" },
  { label: "Switch Previous Window", action: "previous window", keybind: "Alt+Shift+Tab" },
  { label: "Scroll Down", action: "scroll down", keybind: "PageDown" },
  { label: "Scroll Up", action: "scroll up", keybind: "PageUp" },
  { label: "Make Window Larger", action: "make window larger", keybind: "Ctrl+Plus" },
  { label: "Make Window Smaller", action: "make window smaller", keybind: "Ctrl+Minus" },
];

export const GesturesConfigSection: React.FC<Props> = ({ settings, onUpdate }) => {
  const [activeTab, setActiveTab] = useState<"camera" | "dial" | "keybinds">("camera");
  const [recordingGestureId, setRecordingGestureId] = useState<string | null>(null);
  const [testedGestureId, setTestedGestureId] = useState<string | null>(null);

  const gestureKeybinds: GestureKeybindMapping[] =
    settings.gesture_keybinds || DEFAULT_GESTURE_KEYBINDS;

  const updateGestureMapping = (
    gestureId: string,
    patch: Partial<GestureKeybindMapping>
  ) => {
    const updated = gestureKeybinds.map((item) =>
      item.gestureId === gestureId ? { ...item, ...patch } : item
    );
    onUpdate({ gesture_keybinds: updated });
  };

  const handleRecordKeyDown = (
    e: React.KeyboardEvent<HTMLButtonElement>,
    gestureId: string
  ) => {
    if (recordingGestureId !== gestureId) return;
    e.preventDefault();
    e.stopPropagation();

    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) {
      return;
    }

    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Win");

    const keyName = e.key === " " ? "Spacebar" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
    parts.push(keyName);
    const combo = parts.join("+");

    // Map common keys to voice actions automatically if applicable
    let inferredAction = gestureKeybinds.find((g) => g.gestureId === gestureId)?.action || "press enter";
    if (keyName === "Escape") inferredAction = "press escape";
    else if (keyName === "Enter") inferredAction = "press enter";
    else if (keyName === "Tab" && e.altKey) inferredAction = "next window";
    else if (keyName === "Tab") inferredAction = "press tab";
    else if (keyName === "ArrowLeft" && (e.metaKey || e.ctrlKey)) inferredAction = "snap window left";
    else if (keyName === "ArrowRight" && (e.metaKey || e.ctrlKey)) inferredAction = "snap window right";
    else if (keyName === "ArrowUp" && (e.metaKey || e.ctrlKey)) inferredAction = "maximize window";
    else if (keyName === "ArrowDown" && (e.metaKey || e.ctrlKey)) inferredAction = "minimize window";
    else if (keyName === "PageDown") inferredAction = "scroll down";
    else if (keyName === "PageUp") inferredAction = "scroll up";

    updateGestureMapping(gestureId, { keybind: combo, action: inferredAction });
    setRecordingGestureId(null);
  };

  const handleTestGesture = async (kb: GestureKeybindMapping) => {
    setTestedGestureId(kb.gestureId);
    try {
      if (kb.action.toLowerCase() === "radial dial") {
        await tauriApi.toggleRadialDial();
      } else {
        await tauriApi.triggerAction(kb.action, { keybind: kb.keybind });
      }
    } catch (err) {
      console.warn("Gesture test error:", err);
    } finally {
      setTimeout(() => setTestedGestureId(null), 1500);
    }
  };

  return (
    <div className="flex flex-col gap-6">
      {/* Navigation Sub-Bar for Gestures */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4 rounded-xl border border-border bg-card/60 backdrop-blur-xl">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-primary text-primary-foreground flex items-center justify-center shadow-sm">
            <Hand className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-xs font-semibold text-foreground flex items-center gap-2">
              Gestures &amp; Spatial Radial Dial
            </h2>
            <p className="text-[11px] text-muted-foreground">
              Test live MediaPipe hand motions, customize 6-sector dial, and bind custom keybinds to any hand gesture
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
            <Camera className="w-3.5 h-3.5" /> Live Tracking &amp; Tester
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
            <Sliders className="w-3.5 h-3.5" /> Set Gesture Keybinds
          </button>
        </div>
      </div>

      {/* Tab 1: Live Hand Tracking & 21-Landmark Canvas (kept mounted so camera + hand gestures stay active in background) */}
      <div className={activeTab === "camera" ? "block" : "hidden"}>
        <GesturesLiveView
          settings={settings}
          onUpdate={onUpdate}
          onOpenKeybindsTab={() => setActiveTab("keybinds")}
        />
      </div>

      {/* Tab 2: 6-Sector Radial Dial Configuration */}
      {activeTab === "dial" && <RadialConfigCard settings={settings} onUpdate={onUpdate} />}

      {/* Tab 3: Interactive Hand Gesture Keybind Editor & Calibration */}
      {activeTab === "keybinds" && (
        <div className="flex flex-col gap-6">
          {/* Interactive Keybind Mapper */}
          <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-4 shadow-sm">
            <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border pb-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Sparkles className="w-4 h-4 text-[#d87943]" /> Interactive Gesture-to-Keybind Mapper
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Click &quot;Record Key&quot; and press any keyboard shortcut, or choose a preset action/command for each hand motion
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => onUpdate({ gesture_keybinds: DEFAULT_GESTURE_KEYBINDS })}
                  className="px-2.5 py-1 rounded-md bg-muted hover:bg-muted/80 border border-border text-xs font-mono text-foreground flex items-center gap-1.5"
                >
                  <RotateCcw className="w-3 h-3" /> Reset Defaults
                </button>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {gestureKeybinds.map((kb) => {
                const isRecordingKey = recordingGestureId === kb.gestureId;
                const isTested = testedGestureId === kb.gestureId;
                return (
                  <div
                    key={kb.gestureId}
                    className={`p-4 rounded-xl border bg-card flex flex-col justify-between gap-3 shadow-sm transition-all ${
                      kb.enabled ? "border-border hover:border-primary/40" : "border-border/40 opacity-60"
                    }`}
                  >
                    <div className="flex flex-col gap-2">
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex items-center gap-2">
                          <span className="text-xs font-semibold text-foreground">{kb.gesture}</span>
                          <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-primary/10 border border-primary/20 text-primary font-medium">
                            {kb.badge}
                          </span>
                        </div>
                        <button
                          onClick={() => updateGestureMapping(kb.gestureId, { enabled: !kb.enabled })}
                          className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${
                            kb.enabled ? "bg-emerald-600" : "bg-muted border border-border"
                          }`}
                          title={kb.enabled ? "Gesture Enabled" : "Gesture Disabled"}
                        >
                          <span
                            className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
                              kb.enabled ? "translate-x-4" : "translate-x-1"
                            }`}
                          />
                        </button>
                      </div>

                      <p className="text-[11px] text-muted-foreground leading-relaxed">{kb.desc}</p>

                      {/* Keybind Recorder + Preset Dropdown */}
                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1">
                        <div>
                          <label className="text-[10px] font-mono text-muted-foreground block mb-1">
                            Bound Keybind (Click to Record)
                          </label>
                          <button
                            onClick={() =>
                              setRecordingGestureId(isRecordingKey ? null : kb.gestureId)
                            }
                            onKeyDown={(e) => handleRecordKeyDown(e, kb.gestureId)}
                            className={`w-full px-2.5 py-1.5 rounded-lg border text-xs font-mono flex items-center justify-between transition-all ${
                              isRecordingKey
                                ? "bg-[#d87943] text-white border-[#d87943] animate-pulse font-semibold"
                                : "bg-background border-border text-foreground hover:border-[#d87943]"
                            }`}
                          >
                            <span className="truncate">
                              {isRecordingKey ? "Press any key combo..." : kb.keybind}
                            </span>
                            <Keyboard className="w-3.5 h-3.5 shrink-0 ml-1 opacity-75" />
                          </button>
                        </div>

                        <div>
                          <label className="text-[10px] font-mono text-muted-foreground block mb-1">
                            Preset Action Target
                          </label>
                          <select
                            value={
                              PRESET_ACTIONS.some((p) => p.action === kb.action)
                                ? kb.action
                                : "custom"
                            }
                            onChange={(e) => {
                              const found = PRESET_ACTIONS.find((p) => p.action === e.target.value);
                              if (found) {
                                updateGestureMapping(kb.gestureId, {
                                  action: found.action,
                                  keybind: found.keybind,
                                });
                              }
                            }}
                            className="w-full px-2.5 py-1.5 rounded-lg bg-background border border-border text-xs font-mono text-foreground"
                          >
                            {PRESET_ACTIONS.map((p) => (
                              <option key={p.action} value={p.action}>
                                {p.label}
                              </option>
                            ))}
                            <option value="custom">Custom Action Command...</option>
                          </select>
                        </div>
                      </div>

                      {/* Custom Command Input */}
                      <div>
                        <label className="text-[10px] font-mono text-muted-foreground block mb-1">
                          Action / Command Dispatched on Gesture
                        </label>
                        <div className="flex items-center gap-2">
                          <input
                            type="text"
                            value={kb.action}
                            onChange={(e) =>
                              updateGestureMapping(kb.gestureId, { action: e.target.value })
                            }
                            placeholder="e.g. snap window left, open slack, next window..."
                            className="flex-1 px-2.5 py-1.5 rounded-lg bg-background border border-border text-xs font-mono text-foreground"
                          />
                          <button
                            onClick={() => handleTestGesture(kb)}
                            className={`px-3 py-1.5 rounded-lg text-xs font-mono font-semibold flex items-center gap-1 transition-all shrink-0 ${
                              isTested
                                ? "bg-emerald-600 text-white"
                                : "bg-[#d87943] hover:bg-[#c46835] text-white"
                            }`}
                          >
                            {isTested ? (
                              <>
                                <Check className="w-3.5 h-3.5" /> Fired!
                              </>
                            ) : (
                              <>
                                <Play className="w-3 h-3" /> Test Fire
                              </>
                            )}
                          </button>
                        </div>
                      </div>
                    </div>

                    <div className="pt-2 border-t border-border/50 text-[10px] font-mono text-muted-foreground">
                      Trigger Condition: {kb.condition}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Keybind Calibration Sliders */}
          <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-4 shadow-sm">
            <div className="flex items-center justify-between border-b border-border pb-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Sliders className="w-4 h-4 text-foreground" /> Spatial Sensitivity &amp; Gesture Thresholds
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Fine-tune Euclidean distance gates and angle hysteresis for hand keybinds
                </p>
              </div>
              <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-[10px] font-mono">
                CPU Pipeline 30–60 FPS
              </span>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {/* Pinch Distance Threshold */}
              <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs font-medium text-foreground">Pinch Click Threshold</label>
                    <span className="text-xs font-mono text-primary font-bold">&le; 0.05</span>
                  </div>
                  <p className="text-[11px] text-muted-foreground mb-3">
                    Distance between thumb tip &amp; index tip triggering a dial sector click
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
        </div>
      )}
    </div>
  );
};
