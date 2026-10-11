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
      <div className="flex items-center justify-between border-b border-border pb-4">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-muted border border-border flex items-center justify-center text-foreground">
            <Shield className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
              Meeting Detection & Privacy Controls
            </h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              Real-time process watcher with sub-millisecond atomic camera kill switch and compliance disclosure
            </p>
          </div>
        </div>
        <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-xs font-mono flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500" /> Watcher Armed
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Camera Auto-Kill Switch */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs font-medium text-foreground flex items-center gap-1.5">
                <Camera className="w-4 h-4 text-rose-500" /> Camera Privacy Auto-Kill Switch
              </label>
              <button
                onClick={() => onUpdate({ camera_auto_kill: !settings.camera_auto_kill })}
                className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                  settings.camera_auto_kill ? "bg-rose-600" : "bg-muted border border-border"
                }`}
              >
                <span
                  className={`inline-block h-3.5 w-3.5 transform rounded-full bg-background shadow transition-transform ${
                    settings.camera_auto_kill ? "translate-x-5" : "translate-x-1"
                  }`}
                />
              </button>
            </div>
            <p className="text-xs text-muted-foreground">
              Immediately drops camera stream handles upon Zoom/Teams/Meet detection (&lt;1.0ms latency)
            </p>
          </div>
          <div className="mt-4 pt-3 border-t border-border flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Status:</span>
            <span className={settings.camera_auto_kill ? "text-rose-500 font-semibold font-mono" : "text-muted-foreground"}>
              {settings.camera_auto_kill ? "Armed (Auto-Kill on Meeting)" : "Disabled"}
            </span>
          </div>
        </div>

        {/* Legal Compliance Audio Injection */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs font-medium text-foreground flex items-center gap-1.5">
                <Volume2 className="w-4 h-4 text-foreground" /> Compliance Audio Injection
              </label>
              <button
                onClick={() => onUpdate({ compliance_audio_enabled: !settings.compliance_audio_enabled })}
                className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                  settings.compliance_audio_enabled ? "bg-primary" : "bg-muted border border-border"
                }`}
              >
                <span
                  className={`inline-block h-3.5 w-3.5 transform rounded-full bg-background shadow transition-transform ${
                    settings.compliance_audio_enabled ? "translate-x-5" : "translate-x-1"
                  }`}
                />
              </button>
            </div>
            <p className="text-xs text-muted-foreground">
              Directly synthesizes &quot;This meeting is being recorded&quot; at &ge;0.50 volume floor
            </p>
          </div>
          <div className="mt-4 pt-3 border-t border-border flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Statutory Floor:</span>
            <span className={settings.compliance_audio_enabled ? "text-foreground font-semibold font-mono" : "text-muted-foreground"}>
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
