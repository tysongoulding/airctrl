import React from "react";
import { Mic, HardDrive, CheckCircle2, Info, Sparkles, Radio, Volume2, Check } from "lucide-react";
import { SettingsState } from "../../types/config";
import { tauriApi } from "../../services/ipc";
import { SttOverlay } from "../stt/SttOverlay";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

export const SttConfigCard: React.FC<Props> = ({ settings, onUpdate }) => {
  const currentMode = settings.stt_mode ?? (settings.push_to_talk ? "smart_dual" : "toggle");
  const overlayPos = settings.stt_overlay_position ?? "bottom";
  const audioCues = settings.stt_audio_cues ?? true;
  const autoPaste = settings.stt_auto_paste ?? true;

  const handleModeChange = (mode: "smart_dual" | "push_to_talk" | "toggle") => {
    onUpdate({
      stt_mode: mode,
      push_to_talk: mode !== "toggle",
    });
  };

  return (
    <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border pb-4">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-muted border border-border flex items-center justify-center text-foreground">
            <Mic className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
              Speech-to-Text Configuration
              <span className="px-2 py-0.5 text-[10px] rounded bg-[#d87943]/10 border border-[#d87943]/20 text-[#d87943] font-mono font-medium">
                Handy Parity
              </span>
            </h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              Offline Whisper STT with Smart Dual-Mode, non-focus-stealing floating pill, and auto-paste
            </p>
          </div>
        </div>
        <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-xs font-mono flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500" /> Engine Ready
        </span>
      </div>

      {/* Grid: Hotkey & Capture Mode */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Global STT Hotkey */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Global Transcription Hotkey
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Press to activate push-to-talk or toggle transcription into focused application
            </p>
          </div>
          <div className="flex items-center justify-between">
            <select
              value={settings.hotkey_stt}
              onChange={(e) => onUpdate({ hotkey_stt: e.target.value })}
              className="px-3 py-1.5 rounded-md bg-background border border-border text-foreground font-mono text-xs font-medium focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <option value="Ctrl+Spacebar">Ctrl+Spacebar (Handy Default)</option>
              <option value="Ctrl+Win">Ctrl+Win (Safe Intercept)</option>
              <option value="Ctrl+Win+Space">Ctrl+Win+Space</option>
              <option value="Ctrl+Shift+Space">Ctrl+Shift+Space</option>
            </select>
            <span className="text-[11px] text-muted-foreground font-mono">Default: Ctrl+Spacebar</span>
          </div>
        </div>

        {/* Handy Smart Dual-Mode / Capture Mode */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Trigger Behavior
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Smart Dual-Mode automatically detects hold vs quick-tap
            </p>
          </div>
          <div className="grid grid-cols-3 gap-1.5">
            <button
              onClick={() => handleModeChange("smart_dual")}
              className={`px-2 py-1.5 rounded-md text-[11px] font-medium border text-center transition-all ${
                currentMode === "smart_dual"
                  ? "bg-[#d87943] text-white border-[#d87943] shadow-sm font-semibold"
                  : "bg-background border-border text-muted-foreground hover:text-foreground"
              }`}
            >
              Smart Dual
            </button>
            <button
              onClick={() => handleModeChange("push_to_talk")}
              className={`px-2 py-1.5 rounded-md text-[11px] font-medium border text-center transition-all ${
                currentMode === "push_to_talk"
                  ? "bg-[#d87943] text-white border-[#d87943] shadow-sm font-semibold"
                  : "bg-background border-border text-muted-foreground hover:text-foreground"
              }`}
            >
              Push-to-Talk
            </button>
            <button
              onClick={() => handleModeChange("toggle")}
              className={`px-2 py-1.5 rounded-md text-[11px] font-medium border text-center transition-all ${
                currentMode === "toggle"
                  ? "bg-[#d87943] text-white border-[#d87943] shadow-sm font-semibold"
                  : "bg-background border-border text-muted-foreground hover:text-foreground"
              }`}
            >
              Toggle Mode
            </button>
          </div>
        </div>
      </div>

      {/* Grid: Overlay Position & Audio/Auto-Paste Options */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Overlay Position */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Overlay Position
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Where the floating pill appears on your screen
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => onUpdate({ stt_overlay_position: "bottom" })}
              className={`flex-1 py-1.5 px-2 rounded-md text-xs font-medium border text-center transition-all ${
                overlayPos === "bottom"
                  ? "bg-[#527575] text-white border-[#527575] font-semibold"
                  : "bg-background border-border text-muted-foreground hover:text-foreground"
              }`}
            >
              Bottom Center (Handy)
            </button>
            <button
              onClick={() => onUpdate({ stt_overlay_position: "top" })}
              className={`flex-1 py-1.5 px-2 rounded-md text-xs font-medium border text-center transition-all ${
                overlayPos === "top"
                  ? "bg-[#527575] text-white border-[#527575] font-semibold"
                  : "bg-background border-border text-muted-foreground hover:text-foreground"
              }`}
            >
              Top Center
            </button>
          </div>
        </div>

        {/* Audio Chimes / Feedback */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground flex items-center gap-1.5 mb-1">
              <Volume2 className="w-3.5 h-3.5 text-[#d87943]" /> Acoustic Feedback
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Play subtle synthesized chimes on start and paste
            </p>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-foreground">
              {audioCues ? "Audio Cues Enabled" : "Silent Dictation"}
            </span>
            <button
              onClick={() => onUpdate({ stt_audio_cues: !audioCues })}
              className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                audioCues ? "bg-[#d87943]" : "bg-muted border border-border"
              }`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
                  audioCues ? "translate-x-5" : "translate-x-1"
                }`}
              />
            </button>
          </div>
        </div>

        {/* Auto-Paste into Active Window */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground flex items-center gap-1.5 mb-1">
              <Check className="w-3.5 h-3.5 text-emerald-400" /> Active Window Paste
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Automatically paste text into active cursor position
            </p>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-foreground">
              {autoPaste ? "Simulated Ctrl+V" : "Clipboard Only"}
            </span>
            <button
              onClick={() => onUpdate({ stt_auto_paste: !autoPaste })}
              className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                autoPaste ? "bg-emerald-600" : "bg-muted border border-border"
              }`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
                  autoPaste ? "translate-x-5" : "translate-x-1"
                }`}
              />
            </button>
          </div>
        </div>
      </div>

      {/* Safe Hotkey Explanation */}
      <div className="p-3.5 rounded-lg border border-border bg-muted/40 flex items-start gap-3">
        <Info className="w-4 h-4 text-foreground shrink-0 mt-0.5" />
        <div className="text-xs text-muted-foreground leading-relaxed">
          <span className="font-semibold text-foreground">Zero Focus-Stealing: </span>
          The floating pill overlay never takes keyboard focus away from your active editor, browser, or chat window.
          Press and hold <code className="text-foreground font-mono px-1 py-0.5 bg-background border border-border rounded">Ctrl+Spacebar</code> to speak; release to instantly paste into your cursor. Or tap quickly to toggle recording.
        </div>
      </div>

      {/* Whisper Model Selector */}
      <div className="p-4 rounded-lg border border-border bg-card flex flex-col gap-3 shadow-sm">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-foreground flex items-center gap-2">
            <HardDrive className="w-3.5 h-3.5 text-muted-foreground" /> Whisper Model Configuration
          </label>
          <span className="text-xs text-muted-foreground font-mono">
            {settings.whisper_model}
          </span>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
          {["models/ggml-tiny.en.bin", "models/ggml-base.en.bin", "models/ggml-small.en.bin"].map((m) => (
            <button
              key={m}
              onClick={() => onUpdate({ whisper_model: m })}
              className={`p-2.5 rounded-lg border text-xs font-mono text-left transition-all ${
                settings.whisper_model === m
                  ? "bg-primary text-primary-foreground border-primary shadow-sm"
                  : "bg-background border-border text-muted-foreground hover:text-foreground hover:bg-muted/50"
              }`}
            >
              <div className="font-bold text-[11px] mb-0.5">{m.replace("models/ggml-", "").replace(".bin", "").toUpperCase()}</div>
              <div className={`text-[10px] ${settings.whisper_model === m ? "text-primary-foreground/80" : "text-muted-foreground"}`}>Local CPU quantized model</div>
            </button>
          ))}
        </div>
      </div>

      {/* Handy-Style Voice Dictation Overlay Section */}
      <div className="p-6 rounded-2xl border border-[#d87943]/30 bg-gradient-to-b from-card/80 to-card/40 backdrop-blur-xl flex flex-col gap-4 shadow-lg shadow-[#d87943]/5">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-border/70 pb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-[#d87943] text-white flex items-center justify-center shadow-md shadow-[#d87943]/30">
              <Sparkles className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                Handy Floating Pill Dictation Overlay
                <span className="px-2 py-0.5 text-[10px] rounded bg-[#d87943]/10 border border-[#d87943]/20 text-[#d87943] font-mono font-medium">
                  {overlayPos === "bottom" ? "Bottom Center" : "Top Center"}
                </span>
              </h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                Minimalist floating pill with live real-time equalizer soundwaves, audio metering, and auto-injection
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => tauriApi.toggleSttOverlay()}
              className="flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-[#d87943] text-white hover:bg-[#d87943]/90 text-xs font-semibold shadow-md shadow-[#d87943]/20 transition-all whitespace-nowrap"
            >
              <Radio className="w-3.5 h-3.5" />
              <span>Launch Desktop Overlay</span>
            </button>
          </div>
        </div>

        {/* Live Interactive In-HUD Overlay Tester */}
        <div className="pt-1">
          <div className="text-[11px] font-mono text-muted-foreground mb-2 flex items-center justify-between">
            <span>LIVE INTERACTIVE OVERLAY PREVIEW:</span>
            <span className="text-[#d87943] font-medium">Click mic or hold/tap Space to test</span>
          </div>
          <SttOverlay standalone={false} position={overlayPos} soundEnabled={audioCues} />
        </div>
      </div>
    </div>
  );
};
