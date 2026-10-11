import React from "react";
import { RADIAL_DIAL_CONFIG } from "./types";

interface RadialCenterHubProps {
  inDeadzone: boolean;
  activeSector: number | null;
  onDismiss: () => void;
}

export const RadialCenterHub: React.FC<RadialCenterHubProps> = ({
  inDeadzone,
  activeSector,
  onDismiss,
}) => {
  const { width, height, deadzoneRadiusPx, innerAnnulusPx } = RADIAL_DIAL_CONFIG;
  const cx = width / 2;
  const cy = height / 2;

  return (
    <g className="cursor-pointer" onClick={onDismiss}>
      {/* Outer Inner-Boundary Ring */}
      <circle
        cx={cx}
        cy={cy}
        r={innerAnnulusPx - 2}
        className="fill-gray-950/90 stroke-gray-800 stroke-2 backdrop-blur-md"
      />

      {/* Deadzone Boundary Ring (30px radius = r < 0.10) */}
      <circle
        cx={cx}
        cy={cy}
        r={deadzoneRadiusPx}
        className={`transition-colors duration-200 stroke-1 ${
          inDeadzone
            ? "fill-indigo-950/80 stroke-indigo-400 stroke-dashed animate-pulse"
            : "fill-gray-900/90 stroke-gray-700"
        }`}
        strokeDasharray={inDeadzone ? "3 3" : undefined}
      />

      {/* Center Status Reticle */}
      <circle
        cx={cx}
        cy={cy}
        r={6}
        className={`transition-colors duration-150 ${
          activeSector !== null
            ? "fill-indigo-400 shadow-sm"
            : inDeadzone
            ? "fill-indigo-500 animate-ping"
            : "fill-gray-500"
        }`}
      />

      {/* Text Indicator */}
      <text
        x={cx}
        y={cy + 48}
        textAnchor="middle"
        className="text-[10px] font-mono fill-gray-400 select-none uppercase tracking-wider"
      >
        {activeSector !== null ? `Sector ${activeSector}` : inDeadzone ? "Deadband" : "Idle"}
      </text>
    </g>
  );
};
