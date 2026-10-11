import React, { useState } from "react";
import { Mic, Volume2, HardDrive, CheckCircle2, Play, Square } from "lucide-react";
import { SettingsState } from "../../types/config";
import { tauriApi } from "../../services/ipc";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

export const SttConfigCard: React.FC<Props> = ({ settings, onUpdate }) => {
  const [recording, setRecording] = useState(false);
  const [transcript, setTranscript] = useState<string | null>(null);

  const toggleRecording = async () => {
    if (!recording) {
      await tauriApi.startRecording();
      setRecording(true);
      setTranscript(null);
    } else {
      const res = await tauriApi.stopRecording();
      setRecording(false);
      setTranscript(res.text);
    }
  };

  return (
    <div className="hud-glass-card p-6 rounded-2xl flex flex-col gap-6">
      <div className="flex items-center justify-between border-b border-slate-700/40 pb-4">
        <div>
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Mic className="w-5 h-5 text-indigo-400" /> Offline Speech-to-Text Configuration
          </h2>
          <p className="text-xs text-slate-400 mt-1">
            Zero-cloud Whisper STT with active window injection
          </p>
        </div>
        <span className="px-2.5 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs font-mono flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5" /> Engine Ready
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Global STT Hotkey */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <label className="text-xs font-medium text-slate-300 block mb-1">
              Global Transcription Hotkey
            </label>
            <p className="text-xs text-slate-500 mb-3">
              Press to activate push-to-talk or toggle capture
            </p>
          </div>
          <div className="flex items-center justify-between">
            <kbd className="px-3 py-1.5 rounded-lg bg-slate-800 border border-slate-700 text-indigo-400 font-mono text-sm font-semibold shadow-inner">
              {settings.hotkey_stt}
            </kbd>
            <span className="text-xs text-slate-400">Default: Ctrl+Spacebar</span>
          </div>
        </div>

        {/* Push-to-Talk Toggle */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <label className="text-xs font-medium text-slate-300 block mb-1">
              Capture Mode
            </label>
            <p className="text-xs text-slate-500 mb-3">
              Push-to-talk holds key down; toggle presses once to start/stop
            </p>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-white font-medium">
              {settings.push_to_talk ? "Push-to-Talk Active" : "Toggle Transcription"}
            </span>
            <button
              onClick={() => onUpdate({ push_to_talk: !settings.push_to_talk })}
              className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                settings.push_to_talk ? "bg-indigo-600" : "bg-slate-700"
              }`}
            >
              <span
                className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                  settings.push_to_talk ? "translate-x-6" : "translate-x-1"
                }`}
              />
            </button>
          </div>
        </div>
      </div>

      {/* Whisper Model Selector */}
      <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-slate-300 flex items-center gap-2">
            <HardDrive className="w-4 h-4 text-indigo-400" /> Whisper Model Selection
          </label>
          <span className="text-xs text-indigo-400 font-mono">
            {settings.whisper_model}
          </span>
        </div>
        <select
          value={settings.whisper_model}
          onChange={(e) => onUpdate({ whisper_model: e.target.value })}
          className="bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-indigo-500 font-mono"
        >
          <option value="models/ggml-tiny.en.bin">
            ggml-tiny.en.bin (39 MB — Sub-second latency, default)
          </option>
          <option value="models/ggml-base.en.bin">
            ggml-base.en.bin (142 MB — Balanced accuracy)
          </option>
          <option value="models/ggml-small.en.bin">
            ggml-small.en.bin (466 MB — High precision)
          </option>
        </select>
      </div>

      {/* Test Recording Button & Live Result */}
      <div className="bg-slate-900/40 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <button
            onClick={toggleRecording}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg font-medium text-sm transition-all ${
              recording
                ? "bg-rose-600 text-white animate-pulse"
                : "bg-indigo-600 hover:bg-indigo-500 text-white shadow-md shadow-indigo-600/30"
            }`}
          >
            {recording ? (
              <>
                <Square className="w-4 h-4" /> Stop Recording
              </>
            ) : (
              <>
                <Play className="w-4 h-4" /> Test STT Capture
              </>
            )}
          </button>
          {transcript && (
            <span className="text-xs font-mono text-emerald-400 bg-emerald-500/10 px-3 py-1.5 rounded-lg border border-emerald-500/20">
              Transcription: "{transcript}"
            </span>
          )}
        </div>
        <span className="text-xs text-slate-500 flex items-center gap-1">
          <Volume2 className="w-3.5 h-3.5" /> 16kHz Mono VAD Protected
        </span>
      </div>
    </div>
  );
};
