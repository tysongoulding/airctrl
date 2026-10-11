import React, { useEffect } from "react";
import { RADIAL_SECTORS, RADIAL_DIAL_CONFIG } from "./types";
import { RadialSector } from "./RadialSector";
import { RadialCenterHub } from "./RadialCenterHub";
import { SpatialPointerDot } from "./SpatialPointerDot";
import { useRadialState } from "../../hooks/useRadialState";
import { tauriApi } from "../../services/ipc";

interface RadialDialProps {
  onAction?: (command: string) => void;
}

export const RadialDial: React.FC<RadialDialProps> = ({ onAction }) => {
  const {
    isVisible,
    activeSector,
    inDeadzone,
    pointerPos,
    isPinching,
    toggle,
    hide,
    hoverSector,
    selectSector,
    handlePointerMove,
    resetInactivityTimer,
  } = useRadialState((sectorId) => {
    const sector = RADIAL_SECTORS.find((s) => s.id === sectorId);
    if (sector) {
      tauriApi.triggerAction(sector.actionCommand);
      if (onAction) {
        onAction(sector.actionCommand);
      }
    }
  });

  // Global hotkey and keyboard navigation listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+Alt toggles radial dial
      if (e.ctrlKey && e.altKey) {
        e.preventDefault();
        toggle();
        return;
      }

      if (!isVisible) return;

      if (e.key === "Escape") {
        e.preventDefault();
        hide();
        return;
      }

      resetInactivityTimer();

      if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        const next = activeSector === null ? 0 : (activeSector + 1) % 6;
        hoverSector(next);
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        const prev = activeSector === null ? 5 : (activeSector + 5) % 6;
        hoverSector(prev);
      } else if (/^[0-5]$/.test(e.key)) {
        e.preventDefault();
        hoverSector(parseInt(e.key, 10));
      } else if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        if (activeSector !== null) {
          selectSector(activeSector);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isVisible, activeSector, toggle, hide, hoverSector, selectSector, resetInactivityTimer]);

  if (!isVisible) {
    return (
      <div className="fixed inset-0 pointer-events-none flex items-center justify-center">
        <button
          onClick={toggle}
          className="pointer-events-auto px-4 py-2 bg-indigo-600/80 hover:bg-indigo-600 text-white text-xs rounded-full shadow-lg backdrop-blur-md transition-all font-mono"
        >
          Press Ctrl+Alt or Click to Open Radial Dial
        </button>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm z-50">
      <div className="relative w-[600px] h-[600px] flex items-center justify-center select-none">
        <svg
          width={RADIAL_DIAL_CONFIG.width}
          height={RADIAL_DIAL_CONFIG.height}
          viewBox={`0 0 ${RADIAL_DIAL_CONFIG.width} ${RADIAL_DIAL_CONFIG.height}`}
          className="overflow-visible filter drop-shadow-2xl"
          onMouseMove={handlePointerMove}
        >
          {/* Dial Background Disc */}
          <circle
            cx={RADIAL_DIAL_CONFIG.width / 2}
            cy={RADIAL_DIAL_CONFIG.height / 2}
            r={RADIAL_DIAL_CONFIG.radius}
            className="fill-slate-950/75 stroke-indigo-500/30 stroke-1 backdrop-blur-xl"
          />

          {/* 6 Sectors */}
          {RADIAL_SECTORS.map((sector) => (
            <RadialSector
              key={sector.id}
              sector={sector}
              isActive={activeSector === sector.id}
              onHover={hoverSector}
              onSelect={selectSector}
            />
          ))}

          {/* Center Deadband Hub */}
          <RadialCenterHub
            inDeadzone={inDeadzone}
            activeSector={activeSector}
            onDismiss={hide}
          />

          {/* Fingertip Spatial Pointer Reticle */}
          <SpatialPointerDot
            x={pointerPos.x}
            y={pointerPos.y}
            isPinching={isPinching}
            isVisible={isVisible && pointerPos.x !== null}
          />
        </svg>
      </div>
    </div>
  );
};
