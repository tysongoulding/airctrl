import React, { useState, useEffect } from "react";
import { Mic, Zap, Shield, Network, Activity, Sliders, Moon, Sun, Monitor } from "lucide-react";
import { SttConfigCard } from "./SttConfigCard";
import { SpeechActionCard } from "./SpeechActionCard";
import { PrivacyControlsCard } from "./PrivacyControlsCard";
import { LanDashboardCard } from "./LanDashboardCard";
import { StatusMonitorGrid } from "./StatusMonitorGrid";
import { OpacitySlider } from "./OpacitySlider";
import { useTheme } from "../../hooks/useTheme";
import { useHudOpacity } from "../../hooks/useHudOpacity";
import { DEFAULT_SETTINGS, SettingsState } from "../../types/config";
import { tauriApi } from "../../services/ipc";

export const SettingsHud: React.FC = () => {
  const { theme, setTheme } = useTheme("dark");
  const { opacity, setOpacity } = useHudOpacity(0.95);
  const [settings, setSettings] = useState<SettingsState>(DEFAULT_SETTINGS);
  const [activeTab, setActiveTab] = useState<"stt" | "actions" | "privacy" | "lan" | "status">("stt");

  useEffect(() => {
    tauriApi.getSettings().then((remoteSettings: any) => {
      if (remoteSettings && typeof remoteSettings === "object") {
        setSettings((prev) => ({ ...prev, ...remoteSettings }));
      }
    });
  }, []);

  const updateSettings = (partial: Partial<SettingsState>) => {
    setSettings((prev) => {
      const updated = { ...prev, ...partial };
      tauriApi.updateSettings(partial);
      return updated;
    });
  };

  return (
    <div className="w-full h-screen p-6 flex flex-col gap-6 select-none bg-slate-950/70 text-slate-100 backdrop-blur-xl">
      {/* Top Header & HUD Controls */}
      <header className="flex items-center justify-between pb-4 border-b border-slate-700/60">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-indigo-600 flex items-center justify-center shadow-lg shadow-indigo-600/30">
            <Zap className="w-6 h-6 text-white" />
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight text-white flex items-center gap-2">
              airctrl <span className="text-xs px-2 py-0.5 rounded-full bg-indigo-500/20 text-indigo-400 font-mono">v0.1.0</span>
            </h1>
            <p className="text-xs text-slate-400">Modern Settings HUD & Spatial Automation Engine</p>
          </div>
        </div>

        {/* Global HUD Controls: Theme & Opacity */}
        <div className="flex items-center gap-4 bg-slate-800/80 px-4 py-2 rounded-xl border border-slate-700/50">
          <div className="flex items-center gap-1 bg-slate-900/60 p-1 rounded-lg">
            <button
              onClick={() => setTheme("light")}
              className={`p-1.5 rounded-md transition-colors ${theme === "light" ? "bg-indigo-600 text-white" : "text-slate-400 hover:text-white"}`}
              title="Light Mode"
            >
              <Sun className="w-4 h-4" />
            </button>
            <button
              onClick={() => setTheme("dark")}
              className={`p-1.5 rounded-md transition-colors ${theme === "dark" ? "bg-indigo-600 text-white" : "text-slate-400 hover:text-white"}`}
              title="Dark Mode"
            >
              <Moon className="w-4 h-4" />
            </button>
            <button
              onClick={() => setTheme("system")}
              className={`p-1.5 rounded-md transition-colors ${theme === "system" ? "bg-indigo-600 text-white" : "text-slate-400 hover:text-white"}`}
              title="System Mode"
            >
              <Monitor className="w-4 h-4" />
            </button>
          </div>

          <div className="h-5 w-[1px] bg-slate-700" />
          <OpacitySlider opacity={opacity} onChange={setOpacity} />
        </div>
      </header>

      {/* Navigation Tabs */}
      <nav className="flex items-center gap-2 border-b border-slate-800 pb-2">
        <button
          onClick={() => setActiveTab("stt")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === "stt" ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30" : "text-slate-400 hover:bg-slate-800/60 hover:text-white"
          }`}
        >
          <Mic className="w-4 h-4" /> Speech-to-Text
        </button>
        <button
          onClick={() => setActiveTab("actions")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === "actions" ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30" : "text-slate-400 hover:bg-slate-800/60 hover:text-white"
          }`}
        >
          <Sliders className="w-4 h-4" /> Gestures & Radial Dial
        </button>
        <button
          onClick={() => setActiveTab("privacy")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === "privacy" ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30" : "text-slate-400 hover:bg-slate-800/60 hover:text-white"
          }`}
        >
          <Shield className="w-4 h-4" /> Meeting Privacy
        </button>
        <button
          onClick={() => setActiveTab("lan")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === "lan" ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30" : "text-slate-400 hover:bg-slate-800/60 hover:text-white"
          }`}
        >
          <Network className="w-4 h-4" /> LAN Deployment
        </button>
        <button
          onClick={() => setActiveTab("status")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === "status" ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30" : "text-slate-400 hover:bg-slate-800/60 hover:text-white"
          }`}
        >
          <Activity className="w-4 h-4" /> Real-Time Telemetry
        </button>
      </nav>

      {/* Main Content Area */}
      <main className="flex-1 overflow-y-auto pr-1">
        {activeTab === "stt" && <SttConfigCard settings={settings} onUpdate={updateSettings} />}
        {activeTab === "actions" && <SpeechActionCard settings={settings} onUpdate={updateSettings} />}
        {activeTab === "privacy" && <PrivacyControlsCard settings={settings} onUpdate={updateSettings} />}
        {activeTab === "lan" && <LanDashboardCard settings={settings} />}
        {activeTab === "status" && <StatusMonitorGrid />}
      </main>
    </div>
  );
};
