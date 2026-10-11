import { useState, useEffect, useCallback, useRef } from "react";
import { RADIAL_DIAL_CONFIG } from "../components/radial/types";

export interface RadialPointerUpdate {
  inDeadzone: boolean;
  angleRad: number;
  normalizedRadius: number;
  activeSector: number | null;
  screenX: number | null;
  screenY: number | null;
  isSelected: boolean;
}

export function useRadialState(onActionExecute?: (sectorId: number) => void) {
  const [isVisible, setIsVisible] = useState<boolean>(false);
  const [activeSector, setActiveSector] = useState<number | null>(null);
  const [inDeadzone, setInDeadzone] = useState<boolean>(true);
  const [pointerPos, setPointerPos] = useState<{ x: number | null; y: number | null }>({
    x: null,
    y: null,
  });
  const [isPinching, setIsPinching] = useState<boolean>(false);

  // Inactivity timeout timer ref (3.0s boundary)
  const timeoutTimerRef = useRef<NodeJS.Timeout | null>(null);

  // Reset 3.0s inactivity timer
  const resetInactivityTimer = useCallback(() => {
    if (timeoutTimerRef.current) {
      clearTimeout(timeoutTimerRef.current);
    }
    timeoutTimerRef.current = setTimeout(() => {
      // Auto-dismiss after 3.0s of inactivity
      setIsVisible(false);
      setActiveSector(null);
      setInDeadzone(true);
    }, RADIAL_DIAL_CONFIG.inactivityTimeoutSecs * 1000);
  }, []);

  // Show / Hide / Toggle
  const show = useCallback(() => {
    setIsVisible(true);
    resetInactivityTimer();
  }, [resetInactivityTimer]);

  const hide = useCallback(() => {
    setIsVisible(false);
    setActiveSector(null);
    setInDeadzone(true);
    if (timeoutTimerRef.current) {
      clearTimeout(timeoutTimerRef.current);
    }
  }, []);

  const toggle = useCallback(() => {
    setIsVisible((prev) => {
      if (!prev) {
        resetInactivityTimer();
        return true;
      } else {
        setActiveSector(null);
        if (timeoutTimerRef.current) clearTimeout(timeoutTimerRef.current);
        return false;
      }
    });
  }, [resetInactivityTimer]);

  // Sector hover update
  const hoverSector = useCallback(
    (sectorIdx: number | null) => {
      if (!isVisible) return;
      if (sectorIdx === null) {
        setActiveSector(null);
        setInDeadzone(true);
      } else if (sectorIdx >= 0 && sectorIdx < 6) {
        setActiveSector(sectorIdx);
        setInDeadzone(false);
      }
      resetInactivityTimer();
    },
    [isVisible, resetInactivityTimer]
  );

  // Action selection trigger
  const selectSector = useCallback(
    (sectorIdx: number) => {
      if (!isVisible || sectorIdx < 0 || sectorIdx >= 6) return;
      setActiveSector(sectorIdx);
      if (onActionExecute) {
        onActionExecute(sectorIdx);
      }
      // Hide on action completion
      hide();
    },
    [isVisible, onActionExecute, hide]
  );

  // Mouse coordinate calculation with deadzone and hysteresis
  const handlePointerMove = useCallback(
    (e: React.MouseEvent<SVGSVGElement>) => {
      if (!isVisible) return;
      const rect = e.currentTarget.getBoundingClientRect();
      const px = e.clientX - rect.left;
      const py = e.clientY - rect.top;
      setPointerPos({ x: px, y: py });

      const cx = RADIAL_DIAL_CONFIG.width / 2;
      const cy = RADIAL_DIAL_CONFIG.height / 2;
      const dx = px - cx;
      const dy = py - cy;
      const radius = Math.sqrt(dx * dx + dy * dy);

      // Deadzone check: r < 30px (0.10 normalized radius)
      if (radius < RADIAL_DIAL_CONFIG.deadzoneRadiusPx) {
        setInDeadzone(true);
        setActiveSector(null);
        resetInactivityTimer();
        return;
      }

      setInDeadzone(false);

      // Angle calculation in radians [0, 2pi)
      let angle = Math.atan2(dy, dx);
      if (angle < 0) {
        angle += 2 * Math.PI;
      }

      const sectorWidth = (2 * Math.PI) / 6;
      let nominalSector = Math.floor(angle / sectorWidth) % 6;

      // Angular hysteresis check
      if (activeSector !== null) {
        const centerAngle = (activeSector + 0.5) * sectorWidth;
        const halfWidth = sectorWidth / 2;
        let delta = Math.abs(Math.atan2(Math.sin(angle - centerAngle), Math.cos(angle - centerAngle)));
        if (delta <= halfWidth + RADIAL_DIAL_CONFIG.hysteresisRad) {
          nominalSector = activeSector;
        }
      }

      setActiveSector(nominalSector);
      resetInactivityTimer();
    },
    [isVisible, activeSector, resetInactivityTimer]
  );

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (timeoutTimerRef.current) clearTimeout(timeoutTimerRef.current);
    };
  }, []);

  return {
    isVisible,
    activeSector,
    inDeadzone,
    pointerPos,
    isPinching,
    show,
    hide,
    toggle,
    hoverSector,
    selectSector,
    setPointerPos,
    setIsPinching,
    resetInactivityTimer,
    handlePointerMove,
  };
}
