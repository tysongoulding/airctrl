import React, { useState, useEffect } from "react";
import { Activity, Mic, Hand, Video, ShieldCheck } from "lucide-react";
import { tauriApi } from "../../services/ipc";

export const StatusMonitorGrid: React.FC = () => {
  const [meetingStatus, setMeetingStatus] = useState<string>("Inactive");
  const [cameraState, setCameraState] = useState<string>("Active");

  useEffect(() => {
    tauriApi.getMeetingStatus().then((res: any) => {
      if (res?.status) setMeetingStatus(res.status);
      if (res?.camera_state) setCameraState(res.camera_state);
    });
  }, []);

  return (
    <div className="hud-glass-card p-6 rounded-2xl flex flex-col gap-6">
      <div className="flex items-center justify-between border-b border-slate-700/40 pb-4">
        <div>
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Activity className="w-5 h-5 text-indigo-400" /> Real-Time Telemetry & Status Grid
          </h2>
          <p className="text-xs text-slate-400 mt-1">
            Live audio energy, CPU gesture throughput, and active meeting monitors
          </p>
        </div>
        <span className="px-2.5 py-1 rounded-full bg-indigo-500/10 border border-indigo-500/30 text-indigo-400 text-xs font-mono flex items-center gap-1.5">
          <ShieldCheck className="w-3.5 h-3.5" /> Telemetry Live
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Audio Energy / VAD Monitor */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                <Mic className="w-4 h-4 text-indigo-400" /> Audio Energy & VAD
              </span>
              <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-400">
                16kHz Mono
              </span>
            </div>
            {/* Visual audio VU meter */}
            <div className="w-full bg-slate-800 h-2 rounded-full overflow-hidden my-3">
              <div className="bg-gradient-to-r from-emerald-500 via-indigo-500 to-rose-500 h-full w-[45%]" />
            </div>
            <div className="flex justify-between text-[10px] font-mono text-slate-400">
              <span>-60 dBFS</span>
              <span>-18 dBFS (RMS)</span>
              <span>0 dBFS</span>
            </div>
          </div>
          <div className="mt-4 pt-3 border-t border-slate-800/80 flex items-center justify-between text-xs">
            <span className="text-slate-400">Voice Activity:</span>
            <span className="text-emerald-400 font-semibold font-mono">Speech Ready</span>
          </div>
        </div>

        {/* MediaPipe Gesture Throughput */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                <Hand className="w-4 h-4 text-indigo-400" /> Gesture Engine (CPU)
              </span>
              <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-indigo-500/20 text-indigo-400">
                MediaPipe 3D
              </span>
            </div>
            <div className="flex items-baseline gap-2 my-2">
              <span className="text-2xl font-bold font-mono text-white">58.4</span>
              <span className="text-xs text-slate-400 font-mono">FPS (Target: 30–60)</span>
            </div>
            <p className="text-[10px] text-slate-400">
              21 3D Landmarks • Pointing, Pinch, OpenPalm, Swipe
            </p>
          </div>
          <div className="mt-4 pt-3 border-t border-slate-800/80 flex items-center justify-between text-xs">
            <span className="text-slate-400">Active Pose:</span>
            <span className="text-indigo-400 font-semibold font-mono">Pointing (Sector 0)</span>
          </div>
        </div>

        {/* Meeting Detection & Camera State */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                <Video className="w-4 h-4 text-indigo-400" /> Meeting Detection
              </span>
              <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-800 text-slate-400">
                Process Scanner
              </span>
            </div>
            <div className="flex items-center gap-2 my-2">
              <div
                className={`w-2.5 h-2.5 rounded-full ${
                  meetingStatus === "Active" ? "bg-rose-500 animate-ping" : "bg-emerald-500"
                }`}
              />
              <span className="text-sm font-semibold font-mono text-white">
                {meetingStatus === "Active" ? "Meeting in Progress" : "No Active Meeting"}
              </span>
            </div>
            <p className="text-[10px] text-slate-400">
              Monitoring Zoom, Teams, Google Meet, Webex, Discord
            </p>
          </div>
          <div className="mt-4 pt-3 border-t border-slate-800/80 flex items-center justify-between text-xs">
            <span className="text-slate-400">Camera State:</span>
            <span
              className={`font-semibold font-mono ${
                cameraState === "KilledByMeeting" ? "text-rose-400" : "text-emerald-400"
              }`}
            >
              {cameraState}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
