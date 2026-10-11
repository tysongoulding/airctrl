import React from "react";
import { RadialSectorItem, RADIAL_DIAL_CONFIG } from "./types";
import { RadialIcon } from "./RadialIcons";

interface RadialSectorProps {
  sector: RadialSectorItem;
  isActive: boolean;
  onHover: (id: number) => void;
  onSelect: (id: number) => void;
}

export const RadialSector: React.FC<RadialSectorProps> = ({
  sector,
  isActive,
  onHover,
  onSelect,
}) => {
  const { width, height, innerAnnulusPx, outerAnnulusPx } = RADIAL_DIAL_CONFIG;
  const cx = width / 2;
  const cy = height / 2;

  // Angular boundaries converted to radians
  const startRad = (sector.startAngleDeg * Math.PI) / 180;
  const endRad = (sector.endAngleDeg * Math.PI) / 180;
  const midRad = ((sector.startAngleDeg + sector.endAngleDeg) / 2 * Math.PI) / 180;

  // Arc path coordinates
  const rOuter = isActive ? outerAnnulusPx + 8 : outerAnnulusPx;
  const rInner = isActive ? innerAnnulusPx - 4 : innerAnnulusPx;

  const x1 = cx + rOuter * Math.cos(startRad);
  const y1 = cy + rOuter * Math.sin(startRad);
  const x2 = cx + rOuter * Math.cos(endRad);
  const y2 = cy + rOuter * Math.sin(endRad);

  const x3 = cx + rInner * Math.cos(endRad);
  const y3 = cy + rInner * Math.sin(endRad);
  const x4 = cx + rInner * Math.cos(startRad);
  const y4 = cy + rInner * Math.sin(startRad);

  // SVG Arc command: A rx ry x-axis-rotation large-arc-flag sweep-flag x y
  const pathData = `
    M ${x1} ${y1}
    A ${rOuter} ${rOuter} 0 0 1 ${x2} ${y2}
    L ${x3} ${y3}
    A ${rInner} ${rInner} 0 0 0 ${x4} ${y4}
    Z
  `;

  // Center coordinate for label and icon placement
  const rContent = (innerAnnulusPx + outerAnnulusPx) / 2;
  const contentX = cx + rContent * Math.cos(midRad);
  const contentY = cy + rContent * Math.sin(midRad);

  return (
    <g
      className={`cursor-pointer transition-all duration-150 ${
        isActive ? RADIAL_DIAL_CONFIG.activeClass : "text-gray-300"
      }`}
      onMouseEnter={() => onHover(sector.id)}
      onClick={() => onSelect(sector.id)}
    >
      {/* Sector Arc Path */}
      <path
        d={pathData}
        className={`transition-all duration-150 ${
          isActive
            ? "fill-indigo-600 stroke-indigo-400 stroke-2 drop-shadow-lg"
            : "fill-gray-900/85 stroke-gray-700/60 stroke-1 hover:fill-gray-800/90"
        }`}
      />

      {/* ForeignObject container for modern HTML / Flexbox layout */}
      <foreignObject
        x={contentX - 60}
        y={contentY - 32}
        width={120}
        height={64}
        className="pointer-events-none"
      >
        <div
          className={`w-full h-full flex flex-col items-center justify-center transition-transform duration-150 select-none ${
            isActive ? "scale-105 font-semibold text-white" : "text-gray-300 font-normal"
          }`}
        >
          <RadialIcon
            name={sector.icon}
            size={22}
            className={isActive ? "text-white" : "text-indigo-400"}
          />
          <span className="text-xs mt-1 text-center leading-tight truncate px-1 max-w-[110px]">
            {sector.label}
          </span>
        </div>
      </foreignObject>
    </g>
  );
};
