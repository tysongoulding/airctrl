import React, { useState } from "react";
import { Network, Copy, Check, Server, Download, ShieldCheck } from "lucide-react";
import { SettingsState } from "../../types/config";

interface Props {
  settings: SettingsState;
}

export const LanDashboardCard: React.FC<Props> = ({ settings }) => {
  const [copied, setCopied] = useState(false);
  const hostIp = "192.168.1.150";
  const port = settings.lan_port;
  const deployCmd = `irm http://${hostIp}:${port}/run.ps1 | iex`;

  const handleCopy = () => {
    navigator.clipboard.writeText(deployCmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="hud-glass-card p-6 rounded-2xl flex flex-col gap-6">
      <div className="flex items-center justify-between border-b border-slate-700/40 pb-4">
        <div>
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Network className="w-5 h-5 text-indigo-400" /> Local LAN Deployment Daemon
          </h2>
          <p className="text-xs text-slate-400 mt-1">
            Zero-friction Windows development deployment from Linux VM via local HTTP daemon
          </p>
        </div>
        <span className="px-2.5 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs font-mono flex items-center gap-1.5">
          <Server className="w-3.5 h-3.5" /> Listening on 0.0.0.0:{port}
        </span>
      </div>

      {/* One-Liner PowerShell Command Card */}
      <div className="bg-slate-900/80 p-4 rounded-xl border border-indigo-500/30 flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-indigo-300">
            Windows Client One-Liner Deployment (PowerShell)
          </span>
          <span className="text-[10px] font-mono text-slate-400">
            Run on target Windows PC
          </span>
        </div>
        <div className="flex items-center justify-between bg-slate-950 p-3 rounded-lg border border-slate-800 font-mono text-xs text-emerald-400">
          <code className="truncate pr-2">{deployCmd}</code>
          <button
            onClick={handleCopy}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md font-sans text-xs font-semibold transition-all flex-shrink-0 ${
              copied
                ? "bg-emerald-600 text-white"
                : "bg-indigo-600 hover:bg-indigo-500 text-white shadow-md shadow-indigo-600/30"
            }`}
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5" /> Copied!
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" /> Copy
              </>
            )}
          </button>
        </div>
      </div>

      {/* Network & Artifact Status Details */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div className="bg-slate-900/60 p-3.5 rounded-xl border border-slate-800">
          <span className="text-slate-400 text-xs block mb-1">Server Host IP</span>
          <span className="text-sm font-mono text-white font-semibold">{hostIp}</span>
        </div>
        <div className="bg-slate-900/60 p-3.5 rounded-xl border border-slate-800">
          <span className="text-slate-400 text-xs block mb-1">HTTP Port</span>
          <span className="text-sm font-mono text-white font-semibold">{port}</span>
        </div>
        <div className="bg-slate-900/60 p-3.5 rounded-xl border border-slate-800 flex items-center justify-between">
          <div>
            <span className="text-slate-400 text-xs block mb-1">Target Binary</span>
            <span className="text-sm font-mono text-emerald-400 font-semibold flex items-center gap-1">
              <Download className="w-3.5 h-3.5" /> airctrl.exe (x86_64)
            </span>
          </div>
          <ShieldCheck className="w-5 h-5 text-emerald-400" />
        </div>
      </div>
    </div>
  );
};
