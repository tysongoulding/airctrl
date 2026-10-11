import React, { useState } from "react";
import { Compass, Eye, Sliders, Check } from "lucide-react";
import { SettingsState, DialSectorDefinition, SectorIconType, DEFAULT_DIAL_SECTORS } from "../../types/config";
import { RadialIcon } from "../radial/RadialIcons";
import { RADIAL_DIAL_CONFIG } from "../radial/types";
import { tauriApi } from "../../services/ipc";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

const AVAILABLE_ICONS: SectorIconType[] = [
  "cpu", "layout", "window", "terminal", "mic", "play", "gear", "list", "x"
];

export const RadialConfigCard: React.FC<Props> = ({ settings, onUpdate }) => {
  const [hoveredSector, setHoveredSector] = useState<number | null>(null);
  const [selectedSectorId, setSelectedSectorId] = useState<number>(0);
  const [dialOverlayState, setDialOverlayState] = useState<boolean>(false);
  const [saveToast, setSaveToast] = useState(false);

  const sectors: DialSectorDefinition[] = settings.dial_sectors || DEFAULT_DIAL_SECTORS;

  const currentSector = sectors.find((s) => s.id === selectedSectorId) || sectors[0];

  const updateSector = (updated: Partial<DialSectorDefinition>) => {
    const newSectors = sectors.map((s) => (s.id === selectedSectorId ? { ...s, ...updated } : s));
    onUpdate({ dial_sectors: newSectors });
    setSaveToast(true);
    setTimeout(() => setSaveToast(false), 2000);
  };

  const handleToggleOverlay = async () => {
    try {
      const res = await tauriApi.toggleRadialDial();
      if (res && typeof res.dial_visible === "boolean") {
        setDialOverlayState(res.dial_visible);
      }
    } catch (e) {
      console.error("Failed to toggle radial overlay window:", e);
    }
  };

  return (
    <div className="flex flex-col gap-6">
      {/* 6-Sector Dial Overview & Controls */}
      <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-border pb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-muted border border-border flex items-center justify-center text-foreground">
              <Compass className="w-4 h-4" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
                6-Sector Radial Dial Configuration
              </h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Customize applications, system actions, and AI agent shortcuts on each dial sector
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleToggleOverlay}
              className="flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm transition-all"
            >
              <Eye className="w-3.5 h-3.5" />
              {dialOverlayState ? "Hide Overlay Window" : "Test Overlay Window"}
            </button>
          </div>
        </div>

        {/* Global Dial Hotkey & Sensitivity Settings */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
            <div>
              <label className="text-xs font-medium text-foreground block mb-1">
                Global Dial Hotkey
              </label>
              <p className="text-xs text-muted-foreground mb-3">
                Center-focus trigger overlay
              </p>
            </div>
            <div className="flex items-center justify-between">
              <kbd className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground font-mono text-xs font-semibold">
                {settings.hotkey_dial}
              </kbd>
              <span className="text-[11px] text-muted-foreground">Default: Ctrl+Spacebar</span>
            </div>
          </div>

          <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
            <div>
              <label className="text-xs font-medium text-foreground block mb-1">
                Deadzone Radius ({settings.deadzone_radius.toFixed(2)})
              </label>
              <p className="text-xs text-muted-foreground mb-3">
                Center deadband before sector activation
              </p>
            </div>
            <input
              type="range"
              min="0.05"
              max="0.25"
              step="0.01"
              value={settings.deadzone_radius}
              onChange={(e) => onUpdate({ deadzone_radius: parseFloat(e.target.value) })}
              className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>

          <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
            <div>
              <label className="text-xs font-medium text-foreground block mb-1">
                Dial Inactivity Timeout ({settings.dial_timeout_secs}s)
              </label>
              <p className="text-xs text-muted-foreground mb-3">
                Auto-dismiss when no gesture detected
              </p>
            </div>
            <input
              type="range"
              min="1.0"
              max="10.0"
              step="0.5"
              value={settings.dial_timeout_secs}
              onChange={(e) => onUpdate({ dial_timeout_secs: parseFloat(e.target.value) })}
              className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>
        </div>

        {/* 6-Sector Interactive Preview & Sector Selector */}
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-foreground flex items-center gap-1.5">
              <Sliders className="w-3.5 h-3.5 text-muted-foreground" /> Dial Sectors (Click to Edit)
            </span>
            <span className="text-[11px] text-muted-foreground font-mono">
              Hover to preview styling
            </span>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-2">
            {sectors.map((sector) => {
              const isSelected = selectedSectorId === sector.id;
              const isHovered = hoveredSector === sector.id;
              return (
                <div
                  key={sector.id}
                  onClick={() => setSelectedSectorId(sector.id)}
                  onMouseEnter={() => setHoveredSector(sector.id)}
                  onMouseLeave={() => setHoveredSector(null)}
                  className={`p-3 rounded-lg border flex flex-col items-center justify-center gap-2 transition-all cursor-pointer select-none text-center ${
                    isSelected
                      ? "border-primary bg-primary text-primary-foreground shadow-sm"
                      : isHovered
                      ? RADIAL_DIAL_CONFIG.activeClass
                      : "border-border bg-card text-foreground hover:border-border/80"
                  }`}
                >
                  <div className={`w-8 h-8 rounded-md flex items-center justify-center ${isSelected ? "bg-primary-foreground/10 text-primary-foreground" : "bg-muted text-foreground"}`}>
                    <RadialIcon name={sector.icon} className="w-4 h-4" />
                  </div>
                  <span className="text-xs font-medium truncate max-w-full">
                    {sector.label}
                  </span>
                  <span className={`text-[10px] font-mono ${isSelected ? "text-primary-foreground/70" : "text-muted-foreground"}`}>
                    Sector {sector.id}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Sector Editor Form */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col gap-4 shadow-sm">
          <div className="flex items-center justify-between border-b border-border pb-3">
            <span className="text-xs font-semibold text-foreground flex items-center gap-2">
              Editing Sector {currentSector.id}: <span className="text-foreground font-mono bg-muted px-2 py-0.5 rounded">{currentSector.label}</span>
            </span>
            {saveToast && (
              <span className="text-[10px] text-emerald-500 flex items-center gap-1 font-mono">
                <Check className="w-3 h-3" /> Saved
              </span>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {/* Label */}
            <div>
              <label className="text-xs text-muted-foreground block mb-1">Display Label</label>
              <input
                type="text"
                value={currentSector.label}
                maxLength={30}
                onChange={(e) => updateSector({ label: e.target.value })}
                className="w-full px-3 py-1.5 bg-background border border-border rounded-md text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>

            {/* Action Type */}
            <div>
              <label className="text-xs text-muted-foreground block mb-1">Action Type</label>
              <select
                value={currentSector.action_type || "app"}
                onChange={(e) => updateSector({ action_type: e.target.value as any })}
                className="w-full px-3 py-1.5 bg-background border border-border rounded-md text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              >
                <option value="app">Launch Application (.exe)</option>
                <option value="window">Window Action (Snap / Focus)</option>
                <option value="agent">Agent Dispatch</option>
                <option value="system">System Shortcut</option>
              </select>
            </div>

            {/* Target */}
            <div>
              <label className="text-xs text-muted-foreground block mb-1">Target / Binary / Prompt</label>
              <input
                type="text"
                value={currentSector.target || ""}
                onChange={(e) => updateSector({ target: e.target.value })}
                placeholder="e.g. cursor.exe, left, claude"
                className="w-full px-3 py-1.5 bg-background border border-border rounded-md text-xs text-foreground font-mono focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>
          </div>

          {/* Icon Selector */}
          <div>
            <label className="text-xs text-muted-foreground block mb-2">Sector Icon</label>
            <div className="flex items-center gap-2 flex-wrap">
              {AVAILABLE_ICONS.map((iconName) => (
                <button
                  key={iconName}
                  onClick={() => updateSector({ icon: iconName })}
                  className={`p-2 rounded-md border transition-all flex items-center gap-1.5 text-xs ${
                    currentSector.icon === iconName
                      ? "bg-primary text-primary-foreground border-primary shadow-sm"
                      : "bg-background border-border text-muted-foreground hover:text-foreground hover:bg-muted/50"
                  }`}
                >
                  <RadialIcon name={iconName} className="w-3.5 h-3.5" />
                  <span className="capitalize text-[11px]">{iconName}</span>
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
