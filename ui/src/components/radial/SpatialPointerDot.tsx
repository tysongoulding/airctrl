import React from "react";

interface SpatialPointerDotProps {
  x: number | null;
  y: number | null;
  isPinching: boolean;
  isVisible: boolean;
}

export const SpatialPointerDot: React.FC<SpatialPointerDotProps> = ({
  x,
  y,
  isPinching,
  isVisible,
}) => {
  if (!isVisible || x === null || y === null) return null;

  return (
    <g className="pointer-events-none transition-transform duration-75">
      {/* Pointer Halo Ring */}
      <circle
        cx={x}
        cy={y}
        r={isPinching ? 12 : 18}
        className={`transition-all duration-100 ${
          isPinching
            ? "fill-indigo-500/40 stroke-indigo-300 stroke-2"
            : "fill-indigo-400/20 stroke-indigo-400/70 stroke-1 animate-pulse"
        }`}
      />
      {/* Central Fingertip Reticle Dot */}
      <circle
        cx={x}
        cy={y}
        r={isPinching ? 4 : 5}
        className="fill-indigo-300 shadow-md"
      />
      {/* Crosshair lines */}
      <line
        x1={x - 8}
        y1={y}
        x2={x + 8}
        y2={y}
        className="stroke-indigo-200/60 stroke-1"
      />
      <line
        x1={x}
        y1={y - 8}
        x2={x}
        y2={y + 8}
        className="stroke-indigo-200/60 stroke-1"
      />
    </g>
  );
};
