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
      <header className="flex items-center justify-between pb-4 border-b border-border">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-primary text-primary-foreground flex items-center justify-center shadow-sm">
            <Zap className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-lg font-bold tracking-tight text-foreground flex items-center gap-2">
              airctrl <span className="text-[11px] px-2 py-0.5 rounded-md bg-muted border border-border text-foreground font-mono font-medium">v0.2.0</span>
            </h1>
            <p className="text-xs text-muted-foreground">Spatial Automation, Offline Whisper STT & CPU Gesture Engine</p>
          </div>
        </div>

        {/* Global Controls: Theme Switcher & Opacity Slider */}
        <div className="flex items-center gap-4 bg-card px-3 py-1.5 rounded-lg border border-border shadow-sm">
          <div className="flex items-center gap-1 bg-muted/60 p-0.5 rounded-md border border-border/40">
            <button
              onClick={() => setTheme("light")}
              className={`p-1 rounded transition-colors ${theme === "light" ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
              title="Light Mode"
            >
              <Sun className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setTheme("dark")}
              className={`p-1 rounded transition-colors ${theme === "dark" ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
              title="Dark Mode"
            >
              <Moon className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setTheme("system")}
              className={`p-1 rounded transition-colors ${theme === "system" ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
              title="System Mode"
            >
              <Monitor className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="h-4 w-[1px] bg-border" />
          <OpacitySlider opacity={opacity} onChange={setOpacity} />
        </div>
      </header>

      {/* Navigation Tabs (Zero LAN Clutter) */}
      <nav className="flex items-center gap-1.5 border-b border-border pb-2 overflow-x-auto">
        <button
          onClick={() => setActiveTab("stt")}
          className={`flex items-center gap-2 px-3.5 py-1.5 rounded-md text-xs font-medium whitespace-nowrap transition-all ${
            activeTab === "stt"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Mic className="w-3.5 h-3.5" /> Speech-to-Text
        </button>

        <button
          onClick={() => setActiveTab("gestures")}
          className={`flex items-center gap-2 px-3.5 py-1.5 rounded-md text-xs font-medium whitespace-nowrap transition-all ${
            activeTab === "gestures"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Hand className="w-3.5 h-3.5" /> Gestures & Camera
        </button>

        <button
          onClick={() => setActiveTab("dial")}
          className={`flex items-center gap-2 px-3.5 py-1.5 rounded-md text-xs font-medium whitespace-nowrap transition-all ${
            activeTab === "dial"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Compass className="w-3.5 h-3.5" /> 6-Sector Radial Dial
        </button>

        <button
          onClick={() => setActiveTab("apps")}
          className={`flex items-center gap-2 px-3.5 py-1.5 rounded-md text-xs font-medium whitespace-nowrap transition-all ${
            activeTab === "apps"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <List className="w-3.5 h-3.5" /> Windows App Catalog
        </button>

        <button
          onClick={() => setActiveTab("privacy")}
          className={`flex items-center gap-2 px-3.5 py-1.5 rounded-md text-xs font-medium whitespace-nowrap transition-all ${
            activeTab === "privacy"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Shield className="w-3.5 h-3.5" /> Meeting Privacy
        </button>

        <button
          onClick={() => setActiveTab("status")}
          className={`flex items-center gap-2 px-3.5 py-1.5 rounded-md text-xs font-medium whitespace-nowrap transition-all ${
            activeTab === "status"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
        >
          <Activity className="w-3.5 h-3.5" /> Hardware Telemetry
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
