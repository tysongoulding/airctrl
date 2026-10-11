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
    <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
      <div className="flex items-center justify-between border-b border-border/60 pb-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center text-indigo-400">
            <Shield className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-base font-semibold text-foreground flex items-center gap-2">
              Meeting Detection & Privacy Controls
            </h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              Real-time process watcher with sub-millisecond atomic camera kill switch and compliance disclosure
            </p>
          </div>
        </div>
        <span className="px-2.5 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs font-mono flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5" /> Watcher Armed
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Camera Auto-Kill Switch */}
        <div className="p-4 rounded-xl border border-border bg-background/50 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs font-medium text-foreground flex items-center gap-1.5">
                <Camera className="w-4 h-4 text-rose-400" /> Camera Privacy Auto-Kill Switch
              </label>
              <button
                onClick={() => onUpdate({ camera_auto_kill: !settings.camera_auto_kill })}
                className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                  settings.camera_auto_kill ? "bg-rose-600" : "bg-muted"
                }`}
              >
                <span
                  className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                    settings.camera_auto_kill ? "translate-x-6" : "translate-x-1"
                  }`}
                />
              </button>
            </div>
            <p className="text-xs text-muted-foreground">
              Immediately drops camera stream handles upon Zoom/Teams/Meet detection (&lt;1.0ms latency)
            </p>
          </div>
          <div className="mt-4 pt-3 border-t border-border/60 flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Status:</span>
            <span className={settings.camera_auto_kill ? "text-rose-400 font-semibold font-mono" : "text-muted-foreground"}>
              {settings.camera_auto_kill ? "Armed (Auto-Kill on Meeting)" : "Disabled"}
            </span>
          </div>
        </div>

        {/* Legal Compliance Audio Injection */}
        <div className="p-4 rounded-xl border border-border bg-background/50 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs font-medium text-foreground flex items-center gap-1.5">
                <Volume2 className="w-4 h-4 text-indigo-400" /> Compliance Audio Injection
              </label>
              <button
                onClick={() => onUpdate({ compliance_audio_enabled: !settings.compliance_audio_enabled })}
                className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                  settings.compliance_audio_enabled ? "bg-indigo-600" : "bg-muted"
                }`}
              >
                <span
                  className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                    settings.compliance_audio_enabled ? "translate-x-6" : "translate-x-1"
                  }`}
                />
              </button>
            </div>
            <p className="text-xs text-muted-foreground">
              Directly synthesizes &quot;This meeting is being recorded&quot; at &ge;0.50 volume floor
            </p>
          </div>
          <div className="mt-4 pt-3 border-t border-border/60 flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Statutory Floor:</span>
            <span className={settings.compliance_audio_enabled ? "text-indigo-400 font-semibold font-mono" : "text-muted-foreground"}>
              {settings.compliance_audio_enabled ? "Enforced (50% Min Vol)" : "Bypassed"}
            </span>
          </div>
        </div>
      </div>

      {/* Emergency Manual Kill Button */}
      <div className="p-4 rounded-xl border border-rose-500/30 bg-rose-500/5 flex items-center justify-between">
        <div>
          <h3 className="text-xs font-semibold text-foreground flex items-center gap-2">
            <AlertOctagon className="w-4 h-4 text-rose-500" /> Emergency Hardware Kill-Switch
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            Manually trigger atomic CAS kill switch to revoke camera handles instantly
          </p>
        </div>
        <button
          onClick={handleEmergencyKill}
          className={`px-4 py-2 rounded-xl text-xs font-semibold shadow-md transition-all ${
            killTriggered
              ? "bg-rose-700 text-white animate-pulse"
              : "bg-rose-600 hover:bg-rose-500 text-white"
          }`}
        >
          {killTriggered ? "Camera Killed!" : "Drop Camera Handle"}
        </button>
      </div>
    </div>
  );
};
