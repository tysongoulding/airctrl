import React from "react";
import { Mic, HardDrive, CheckCircle2, Info, Sparkles, Radio } from "lucide-react";
import { SettingsState } from "../../types/config";
import { tauriApi } from "../../services/ipc";
import { SttOverlay } from "../stt/SttOverlay";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

export const SttConfigCard: React.FC<Props> = ({ settings, onUpdate }) => {
  return (
    <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
      <div className="flex items-center justify-between border-b border-border pb-4">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-muted border border-border flex items-center justify-center text-foreground">
            <Mic className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
              Offline Speech-to-Text Configuration
            </h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              Zero-cloud Whisper STT with real-time audio capture and active window injection
            </p>
          </div>
        </div>
        <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-xs font-mono flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500" /> Engine Ready
        </span>
      </div>

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
              <option value="Ctrl+Win">Ctrl+Win (Recommended)</option>
              <option value="Ctrl+Spacebar">Ctrl+Spacebar</option>
              <option value="Ctrl+Win+Space">Ctrl+Win+Space</option>
              <option value="Ctrl+Shift+Space">Ctrl+Shift+Space</option>
            </select>
            <span className="text-[11px] text-muted-foreground">Default: Ctrl+Win</span>
          </div>
        </div>

        {/* Push-to-Talk Toggle */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Capture Mode
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Push-to-talk holds key down; toggle presses once to start/stop
            </p>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-foreground font-medium">
              {settings.push_to_talk ? "Push-to-Talk Active" : "Toggle Transcription"}
            </span>
            <button
              onClick={() => onUpdate({ push_to_talk: !settings.push_to_talk })}
              className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                settings.push_to_talk ? "bg-primary" : "bg-muted border border-border"
              }`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-background shadow transition-transform ${
                  settings.push_to_talk ? "translate-x-5" : "translate-x-1"
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
          <span className="font-semibold text-foreground">Safe Windows Hotkey: </span>
          <code className="text-foreground font-mono px-1 py-0.5 bg-background border border-border rounded">Ctrl+Win</code> is intercepted via a low-level OS hook that suppresses the standalone Windows key release, preventing the Start Menu from appearing during transcription.
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
      <div className="p-6 rounded-2xl border border-primary/30 bg-gradient-to-b from-card/80 to-card/40 backdrop-blur-xl flex flex-col gap-4 shadow-lg shadow-primary/5">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-border/70 pb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-primary text-primary-foreground flex items-center justify-center shadow-md">
              <Sparkles className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                Handy-Style Dictation Overlay
                <span className="px-2 py-0.5 text-[10px] rounded bg-primary/10 border border-primary/20 text-primary font-mono font-medium">
                  Qrafthive Colors
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
              className="flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 text-xs font-semibold shadow-md transition-all whitespace-nowrap"
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
            <span className="text-primary font-medium">Click mic or press Space to record</span>
          </div>
          <SttOverlay standalone={false} />
        </div>
      </div>
    </div>
  );
};
