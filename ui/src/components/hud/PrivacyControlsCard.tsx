import React, { useState } from "react";
import { Shield, Camera, Volume2, AlertOctagon, CheckCircle2 } from "lucide-react";
import { SettingsState } from "../../types/config";
import { tauriApi } from "../../services/ipc";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

export const PrivacyControlsCard: React.FC<Props> = ({ settings, onUpdate }) => {
  const [killTriggered, setKillTriggered] = useState(false);

  const handleEmergencyKill = async () => {
    await tauriApi.triggerCameraKillSwitch();
    setKillTriggered(true);
    setTimeout(() => setKillTriggered(false), 4000);
  };

  return (
    <div className="hud-glass-card p-6 rounded-2xl flex flex-col gap-6">
      <div className="flex items-center justify-between border-b border-slate-700/40 pb-4">
        <div>
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Shield className="w-5 h-5 text-indigo-400" /> Meeting Detection & Privacy Controls
          </h2>
          <p className="text-xs text-slate-400 mt-1">
            Real-time meeting watcher with sub-millisecond camera kill switch and compliance audio
          </p>
        </div>
        <span className="px-2.5 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs font-mono flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5" /> Watcher Armed
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Camera Auto-Kill Switch */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                <Camera className="w-4 h-4 text-rose-400" /> Camera Privacy Auto-Kill Switch
              </label>
              <button
                onClick={() => onUpdate({ camera_auto_kill: !settings.camera_auto_kill })}
                className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                  settings.camera_auto_kill ? "bg-rose-600" : "bg-slate-700"
                }`}
              >
                <span
                  className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                    settings.camera_auto_kill ? "translate-x-6" : "translate-x-1"
                  }`}
                />
              </button>
            </div>
            <p className="text-xs text-slate-500">
              Immediately drops camera stream handles upon Zoom/Teams/Meet detection (&lt;1.0ms latency)
            </p>
          </div>
          <span className="text-xs font-mono text-slate-400 mt-3">
            Status: {settings.camera_auto_kill ? "Active (Auto-Release Enabled)" : "Disabled"}
          </span>
        </div>

        {/* Legal Compliance Audio Injection */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                <Volume2 className="w-4 h-4 text-amber-400" /> Compliance Audio Injection
              </label>
              <button
                onClick={() => onUpdate({ compliance_audio_enabled: !settings.compliance_audio_enabled })}
                className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                  settings.compliance_audio_enabled ? "bg-indigo-600" : "bg-slate-700"
                }`}
              >
                <span
                  className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                    settings.compliance_audio_enabled ? "translate-x-6" : "translate-x-1"
                  }`}
                />
              </button>
            </div>
            <p className="text-xs text-slate-500">
              Injects mandatory legal announcement ("This meeting is being recorded") into audio stream
            </p>
          </div>
          <span className="text-xs font-mono text-slate-400 mt-3">
            Status: {settings.compliance_audio_enabled ? "Active (Disclosure Injected)" : "Disabled"}
          </span>
        </div>
      </div>

      {/* Emergency Manual Kill Button */}
      <div className="bg-rose-950/30 border border-rose-900/50 p-4 rounded-xl flex items-center justify-between">
        <div className="flex items-center gap-3">
          <AlertOctagon className="w-6 h-6 text-rose-500 flex-shrink-0" />
          <div>
            <h3 className="text-sm font-semibold text-rose-300">
              Manual Emergency Camera Kill Switch
            </h3>
            <p className="text-xs text-rose-400/80">
              Force terminates camera hardware handles immediately across all threads
            </p>
          </div>
        </div>
        <button
          onClick={handleEmergencyKill}
          className={`px-4 py-2 rounded-lg font-medium text-xs font-mono uppercase tracking-wider transition-all ${
            killTriggered
              ? "bg-rose-500 text-white shadow-lg shadow-rose-500/50"
              : "bg-rose-700 hover:bg-rose-600 text-white shadow-md shadow-rose-700/40"
          }`}
        >
          {killTriggered ? "Camera Killed!" : "Trigger Kill Switch"}
        </button>
      </div>
    </div>
  );
};
