import React, { useState } from "react";
import { Layout, Hand, Compass } from "lucide-react";
import { SettingsState, DEFAULT_DIAL_SECTORS } from "../../types/config";
import { RadialIcon } from "../radial/RadialIcons";
import { RADIAL_DIAL_CONFIG } from "../radial/types";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

export const SpeechActionCard: React.FC<Props> = ({ settings, onUpdate }) => {
  const [hoveredSector, setHoveredSector] = useState<number | null>(null);

  return (
    <div className="hud-glass-card p-6 rounded-2xl flex flex-col gap-6">
      <div className="flex items-center justify-between border-b border-slate-700/40 pb-4">
        <div>
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Layout className="w-5 h-5 text-indigo-400" /> Speech-to-Action & Gestures
          </h2>
          <p className="text-xs text-slate-400 mt-1">
            Radial dial overlay and CPU MediaPipe 3D gesture mapper
          </p>
        </div>
        <span className="px-2.5 py-1 rounded-full bg-indigo-500/10 border border-indigo-500/30 text-indigo-400 text-xs font-mono flex items-center gap-1.5">
          <Hand className="w-3.5 h-3.5" /> 30–60 FPS CPU Gestures
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Global Dial Hotkey */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <label className="text-xs font-medium text-slate-300 block mb-1">
              Radial Dial Overlay Hotkey
            </label>
            <p className="text-xs text-slate-500 mb-3">
              Press to activate center-focus radial dial overlay
            </p>
          </div>
          <div className="flex items-center justify-between">
            <kbd className="px-3 py-1.5 rounded-lg bg-slate-800 border border-slate-700 text-indigo-400 font-mono text-sm font-semibold shadow-inner">
              {settings.hotkey_dial}
            </kbd>
            <span className="text-xs text-slate-400">Default: Ctrl+Alt</span>
          </div>
        </div>

        {/* Sector Count Config */}
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col justify-between">
          <div>
            <label className="text-xs font-medium text-slate-300 block mb-1">
              Radial Dial Sectors ({settings.radial_dial_sectors})
            </label>
            <p className="text-xs text-slate-500 mb-3">
              Configurable sector inventory (bounded between 4 and 8)
            </p>
          </div>
          <div className="flex items-center gap-3">
            <input
              type="range"
              min="4"
              max="8"
              step="1"
              value={settings.radial_dial_sectors}
              onChange={(e) => onUpdate({ radial_dial_sectors: parseInt(e.target.value, 10) })}
              className="w-full h-1.5 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-indigo-500"
            />
            <span className="text-xs font-mono text-indigo-400 font-bold w-6 text-right">
              {settings.radial_dial_sectors}
            </span>
          </div>
        </div>
      </div>

      {/* 6-Sector Interactive Preview Grid */}
      <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-slate-300 flex items-center gap-2">
            <Compass className="w-4 h-4 text-indigo-400" /> 6-Sector Dial Action Layout
          </label>
          <span className="text-xs text-slate-500 font-mono">
            Hover to preview active sector styling
          </span>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-2">
          {DEFAULT_DIAL_SECTORS.map((sector) => {
            const isActive = hoveredSector === sector.id;
            return (
              <div
                key={sector.id}
                onMouseEnter={() => setHoveredSector(sector.id)}
                onMouseLeave={() => setHoveredSector(null)}
                className={`p-3 rounded-xl border flex flex-col items-center justify-center gap-2 transition-all cursor-pointer select-none text-center ${
                  isActive
                    ? RADIAL_DIAL_CONFIG.activeClass
                    : "bg-slate-800/80 border-slate-700/60 text-slate-300 hover:border-slate-600"
                }`}
              >
                <RadialIcon
                  name={sector.icon}
                  size={20}
                  className={isActive ? "text-white" : "text-indigo-400"}
                />
                <span className="text-xs font-medium leading-tight">
                  {sector.label}
                </span>
                <span className="text-[10px] font-mono text-slate-400 opacity-80">
                  Sector {sector.id}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Spatial Sensitivity Controls */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col gap-2">
          <div className="flex justify-between items-center text-xs">
            <span className="text-slate-300 font-medium">Deadzone Radius</span>
            <span className="text-indigo-400 font-mono">
              {(settings.deadzone_radius * 100).toFixed(0)}% (30px)
            </span>
          </div>
          <input
            type="range"
            min="0.05"
            max="0.25"
            step="0.01"
            value={settings.deadzone_radius}
            onChange={(e) => onUpdate({ deadzone_radius: parseFloat(e.target.value) })}
            className="w-full h-1.5 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          />
        </div>

        <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 flex flex-col gap-2">
          <div className="flex justify-between items-center text-xs">
            <span className="text-slate-300 font-medium">Angular Hysteresis</span>
            <span className="text-indigo-400 font-mono">
              {settings.angular_hysteresis.toFixed(2)} rad (~2.9°)
            </span>
          </div>
          <input
            type="range"
            min="0.00"
            max="0.15"
            step="0.01"
            value={settings.angular_hysteresis}
            onChange={(e) => onUpdate({ angular_hysteresis: parseFloat(e.target.value) })}
            className="w-full h-1.5 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          />
        </div>
      </div>
    </div>
  );
};
