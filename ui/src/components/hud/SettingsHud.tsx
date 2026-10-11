import React, { useState, useEffect } from "react";
import { Mic, Zap, Shield, Activity, Compass, Hand, List, Moon, Sun, Monitor } from "lucide-react";
import { SttConfigCard } from "./SttConfigCard";
import { GesturesLiveView } from "./GesturesLiveView";
import { RadialConfigCard } from "./RadialConfigCard";
import { AppCatalogCard } from "./AppCatalogCard";
import { PrivacyControlsCard } from "./PrivacyControlsCard";
import { StatusMonitorGrid } from "./StatusMonitorGrid";
import { OpacitySlider } from "./OpacitySlider";
import { useTheme } from "../../hooks/useTheme";
import { useHudOpacity } from "../../hooks/useHudOpacity";
import { DEFAULT_SETTINGS, SettingsState, ApplicationItem } from "../../types/config";
import { tauriApi } from "../../services/ipc";

export const SettingsHud: React.FC = () => {
  const { theme, setTheme } = useTheme("dark");
  const { opacity, setOpacity } = useHudOpacity(0.95);
  const [settings, setSettings] = useState<SettingsState>(DEFAULT_SETTINGS);
  const [activeTab, setActiveTab] = useState<"stt" | "gestures" | "dial" | "apps" | "privacy" | "status">("stt");

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

  const handleAssignToSector = (sectorId: number, app: ApplicationItem) => {
    const currentSectors = settings.dial_sectors || DEFAULT_SETTINGS.dial_sectors || [];
    const newSectors = currentSectors.map((s) =>
      s.id === sectorId
        ? { ...s, label: app.name.slice(0, 30), target: app.path, action_type: "app" as const }
        : s
    );
    updateSettings({ dial_sectors: newSectors });
  };

  return (
    <div
      className="w-full h-screen p-6 flex flex-col gap-6 select-none bg-background/90 text-foreground backdrop-blur-xl transition-colors duration-200"
      style={{ opacity }}
    >
      {/* Top Header & Global Controls */}
      <header className="flex items-center justify-between pb-4 border-b border-border/60">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-indigo-600 flex items-center justify-center shadow-lg shadow-indigo-600/30 text-white">
            <Zap className="w-6 h-6" />
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
              airctrl <span className="text-xs px-2 py-0.5 rounded-full bg-indigo-500/20 text-indigo-400 font-mono font-medium">v0.2.0</span>
            </h1>
            <p className="text-xs text-muted-foreground">Spatial Automation, Offline Whisper STT & CPU Gesture Engine</p>
          </div>
        </div>

        {/* Global Controls: Theme Switcher & Opacity Slider */}
        <div className="flex items-center gap-4 bg-card/80 px-4 py-2 rounded-xl border border-border shadow-sm">
          <div className="flex items-center gap-1 bg-background/60 p-1 rounded-lg border border-border/50">
            <button
              onClick={() => setTheme("light")}
              className={`p-1.5 rounded-md transition-colors ${theme === "light" ? "bg-indigo-600 text-white" : "text-muted-foreground hover:text-foreground"}`}
              title="Light Mode"
            >
              <Sun className="w-4 h-4" />
            </button>
            <button
              onClick={() => setTheme("dark")}
              className={`p-1.5 rounded-md transition-colors ${theme === "dark" ? "bg-indigo-600 text-white" : "text-muted-foreground hover:text-foreground"}`}
              title="Dark Mode"
            >
              <Moon className="w-4 h-4" />
            </button>
            <button
              onClick={() => setTheme("system")}
              className={`p-1.5 rounded-md transition-colors ${theme === "system" ? "bg-indigo-600 text-white" : "text-muted-foreground hover:text-foreground"}`}
              title="System Mode"
            >
              <Monitor className="w-4 h-4" />
            </button>
          </div>

          <div className="h-5 w-[1px] bg-border" />
          <OpacitySlider opacity={opacity} onChange={setOpacity} />
        </div>
      </header>

      {/* Navigation Tabs (Zero LAN Clutter) */}
      <nav className="flex items-center gap-2 border-b border-border/60 pb-2 overflow-x-auto">
        <button
          onClick={() => setActiveTab("stt")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold whitespace-nowrap transition-all ${
            activeTab === "stt"
              ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Mic className="w-4 h-4" /> Speech-to-Text
        </button>

        <button
          onClick={() => setActiveTab("gestures")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold whitespace-nowrap transition-all ${
            activeTab === "gestures"
              ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Hand className="w-4 h-4" /> Gestures & Camera
        </button>

        <button
          onClick={() => setActiveTab("dial")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold whitespace-nowrap transition-all ${
            activeTab === "dial"
              ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Compass className="w-4 h-4" /> 6-Sector Radial Dial
        </button>

        <button
          onClick={() => setActiveTab("apps")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold whitespace-nowrap transition-all ${
            activeTab === "apps"
              ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <List className="w-4 h-4" /> Windows App Catalog
        </button>

        <button
          onClick={() => setActiveTab("privacy")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold whitespace-nowrap transition-all ${
            activeTab === "privacy"
              ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Shield className="w-4 h-4" /> Meeting Privacy
        </button>

        <button
          onClick={() => setActiveTab("status")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold whitespace-nowrap transition-all ${
            activeTab === "status"
              ? "bg-indigo-600 text-white shadow-md shadow-indigo-600/30"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Activity className="w-4 h-4" /> Hardware Telemetry
        </button>
      </nav>

      {/* Main Content Viewport */}
      <main className="flex-1 overflow-y-auto pr-1">
        {activeTab === "stt" && <SttConfigCard settings={settings} onUpdate={updateSettings} />}
        {activeTab === "gestures" && <GesturesLiveView />}
        {activeTab === "dial" && <RadialConfigCard settings={settings} onUpdate={updateSettings} />}
        {activeTab === "apps" && (
          <AppCatalogCard
            onAssignToSector={handleAssignToSector}
            sectors={settings.dial_sectors}
          />
        )}
        {activeTab === "privacy" && <PrivacyControlsCard settings={settings} onUpdate={updateSettings} />}
        {activeTab === "status" && <StatusMonitorGrid />}
      </main>
    </div>
  );
};
