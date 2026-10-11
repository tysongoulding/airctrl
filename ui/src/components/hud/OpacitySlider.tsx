import React from "react";
import { Sliders } from "lucide-react";

interface OpacitySliderProps {
  opacity: number;
  onChange: (val: number) => void;
}

export const OpacitySlider: React.FC<OpacitySliderProps> = ({ opacity, onChange }) => {
  return (
    <div className="flex items-center gap-2">
      <Sliders className="w-4 h-4 text-slate-400" />
      <span className="text-xs text-slate-400 font-mono">Opacity</span>
      <input
        type="range"
        min="0.0"
        max="1.0"
        step="0.05"
        value={opacity}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-24 h-1.5 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-indigo-500"
      />
      <span className="text-xs font-mono text-indigo-400 w-8 text-right">
        {Math.round(opacity * 100)}%
      </span>
    </div>
  );
};
