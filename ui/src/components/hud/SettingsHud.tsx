import React, { useState, useEffect } from "react";
import { Mic, Zap, Shield, Hand, Activity, Moon, Sun, Monitor, Radio, ChevronRight } from "lucide-react";
import { SttConfigCard } from "./SttConfigCard";
import { SpeechToActionCard } from "./SpeechToActionCard";
import { GesturesConfigSection } from "./GesturesConfigSection";
import { PrivacyControlsCard } from "./PrivacyControlsCard";
import { StatusMonitorGrid } from "./StatusMonitorGrid";
import { useTheme } from "../../hooks/useTheme";
import { DEFAULT_SETTINGS, SettingsState, ApplicationItem } from "../../types/config";
import { tauriApi } from "../../services/ipc";

type NavSection = "stt" | "sta" | "gestures" | "meetings" | "telemetry";

const NAV_ITEMS = [
  {
    id: "stt" as const,
    label: "Speech-to-Text",
    sublabel: "Voice keybinds & dictation",
    hotkey: "Ctrl+Win",
    icon: Mic,
  },
  {
    id: "sta" as const,
    label: "Speech-to-Action",
    sublabel: "Agents & action keybinds",
    hotkey: "Shift+Space",
    icon: Zap,
  },
  {
    id: "gestures" as const,
    label: "Gestures",
    sublabel: "Radial dial & hand keybinds",
    hotkey: "Ctrl+Space",
    icon: Hand,
  },
  {
    id: "meetings" as const,
    label: "Meetings",
    sublabel: "Privacy & kill-switch",
    hotkey: "Watcher",
    icon: Shield,
  },
];

export const SettingsHud: React.FC = () => {
  const { theme, setTheme } = useTheme("dark");
  const [settings, setSettings] = useState<SettingsState>(DEFAULT_SETTINGS);
  const [activeNav, setActiveNav] = useState<NavSection>("stt");

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

  const currentNav = NAV_ITEMS.find((n) => n.id === activeNav) || {
    label: "System Telemetry",
    sublabel: "Hardware metrics & latency",
  };

  return (
    <div
      className="w-full h-screen flex select-none bg-background text-foreground backdrop-blur-xl transition-colors duration-200 overflow-hidden font-sans"
    >
      {/* Lefthand Menu Navigation Bar */}
      <aside className="w-80 h-full border-r border-border bg-card/50 flex flex-col justify-between p-5 shrink-0 backdrop-blur-2xl">
        <div className="flex flex-col gap-6">
          {/* Top Brand & Version */}
          <div className="flex items-center gap-3 pb-4 border-b border-border/70">
            <div className="w-9 h-9 rounded-xl bg-primary text-primary-foreground flex items-center justify-center shadow-md">
              <Zap className="w-5 h-5" />
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-sm font-bold tracking-tight text-foreground">airctrl</span>
                <span className="text-[10px] px-1.5 py-0.5 rounded bg-muted border border-border text-foreground font-mono font-medium">
                  v0.2.0
                </span>
              </div>
              <div className="flex items-center gap-1.5 mt-0.5">
                <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                <span className="text-[11px] text-muted-foreground font-medium">Offline Engine Ready</span>
              </div>
            </div>
          </div>

          {/* Navigation Section */}
          <div className="flex flex-col gap-1.5">
            <div className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground px-2 mb-1">
              Workflows & Control
            </div>

            {NAV_ITEMS.map((item) => {
              const Icon = item.icon;
              const isActive = activeNav === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => setActiveNav(item.id)}
                  className={`w-full flex items-center justify-between p-3 rounded-xl text-left transition-all group ${
                    isActive
                      ? "bg-primary text-primary-foreground shadow-md"
                      : "text-muted-foreground hover:bg-muted/70 hover:text-foreground"
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 transition-colors ${
                        isActive
                          ? "bg-primary-foreground/20 text-primary-foreground"
                          : "bg-muted border border-border/60 text-foreground group-hover:border-primary/40"
                      }`}
                    >
                      <Icon className="w-4 h-4" />
                    </div>
                    <div className="min-w-0">
                      <div className="text-xs font-semibold leading-tight whitespace-nowrap">
                        {item.label}
                      </div>
                      <div
                        className={`text-[10px] truncate mt-0.5 ${
                          isActive ? "text-primary-foreground/80" : "text-muted-foreground"
                        }`}
                      >
                        {item.sublabel}
                      </div>
                    </div>
                  </div>

                  <span
                    className={`text-[10px] font-mono px-1.5 py-0.5 rounded shrink-0 ml-2 ${
                      isActive
                        ? "bg-primary-foreground/25 text-primary-foreground font-semibold"
                        : "bg-muted/80 border border-border text-muted-foreground"
                    }`}
                  >
                    {item.hotkey}
                  </span>
                </button>
              );
            })}

            {/* Diagnostics Link */}
            <div className="pt-2 mt-2 border-t border-border/50">
              <button
                onClick={() => setActiveNav("telemetry")}
                className={`w-full flex items-center justify-between p-2.5 rounded-xl text-left transition-all ${
                  activeNav === "telemetry"
                    ? "bg-primary text-primary-foreground shadow-md"
                    : "text-muted-foreground hover:bg-muted/70 hover:text-foreground"
                }`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`w-7 h-7 rounded-lg flex items-center justify-center shrink-0 ${
                      activeNav === "telemetry"
                        ? "bg-primary-foreground/20 text-primary-foreground"
                        : "bg-muted border border-border text-foreground"
                    }`}
                  >
                    <Activity className="w-3.5 h-3.5" />
                  </div>
                  <div>
                    <div className="text-xs font-semibold">Hardware Telemetry</div>
                    <div
                      className={`text-[10px] ${
                        activeNav === "telemetry" ? "text-primary-foreground/80" : "text-muted-foreground"
                      }`}
                    >
                      Real-time audio & latency
                    </div>
                  </div>
                </div>
                <ChevronRight className="w-3.5 h-3.5 opacity-50" />
              </button>
            </div>
          </div>
        </div>

        {/* Sidebar Footer Controls */}
        <div className="flex flex-col gap-3 pt-4 border-t border-border/70">
          {/* Theme Switcher & Status */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-1 bg-muted/60 p-0.5 rounded-md border border-border/40">
              <button
                onClick={() => setTheme("light")}
                className={`p-1.5 rounded transition-colors ${
                  theme === "light"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
                title="Light Theme"
              >
                <Sun className="w-3.5 h-3.5" />
              </button>
              <button
                onClick={() => setTheme("dark")}
                className={`p-1.5 rounded transition-colors ${
                  theme === "dark"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
                title="Dark Theme"
              >
                <Moon className="w-3.5 h-3.5" />
              </button>
              <button
                onClick={() => setTheme("system")}
                className={`p-1.5 rounded transition-colors ${
                  theme === "system"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
                title="System Theme"
              >
                <Monitor className="w-3.5 h-3.5" />
              </button>
            </div>

            <span className="text-[10px] font-mono text-muted-foreground">
              Zero-Cloud Local
            </span>
          </div>
        </div>
      </aside>

      {/* Main Content Viewport */}
      <main className="flex-1 h-screen overflow-y-auto p-6 lg:p-8 flex flex-col gap-6">
        {/* Breadcrumb Header Bar */}
        <div className="flex items-center justify-between pb-3 border-b border-border/60">
          <div className="flex items-center gap-2 text-xs font-medium">
            <span className="text-muted-foreground">airctrl</span>
            <span className="text-muted-foreground">/</span>
            <span className="text-foreground font-semibold">{currentNav.label}</span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => tauriApi.toggleRadialDial()}
              className="px-3 py-1.5 bg-muted hover:bg-muted/80 border border-border rounded-md text-xs font-medium text-foreground transition-all flex items-center gap-1.5 shadow-sm"
            >
              <Radio className="w-3.5 h-3.5 text-primary" />
              <span>Preview Radial Dial</span>
            </button>
          </div>
        </div>

        {/* View Component */}
        <div className="flex-1 pb-12">
          {activeNav === "stt" && (
            <SttConfigCard settings={settings} onUpdate={updateSettings} />
          )}

          {activeNav === "sta" && (
            <SpeechToActionCard
              settings={settings}
              onUpdate={updateSettings}
              onAssignToSector={handleAssignToSector}
            />
          )}

          {/* Keep GesturesConfigSection mounted so Dual-Camera Splitter & Hand Tracking stay active across tabs and during meetings */}
          <div className={activeNav === "gestures" ? "block" : "hidden"}>
            <GesturesConfigSection
              settings={settings}
              onUpdate={updateSettings}
              onAssignToSector={handleAssignToSector}
            />
          </div>

          {activeNav === "meetings" && (
            <PrivacyControlsCard settings={settings} onUpdate={updateSettings} />
          )}

          {activeNav === "telemetry" && (
            <StatusMonitorGrid />
          )}
        </div>
      </main>
    </div>
  );
};
