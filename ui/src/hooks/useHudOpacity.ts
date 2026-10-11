import { useState, useEffect } from "react";

export function useHudOpacity(initialOpacity: number = 0.95) {
  const [opacity, setOpacityState] = useState<number>(() => {
    if (typeof window === "undefined") return initialOpacity;
    const saved = localStorage.getItem("airctrl_hud_opacity");
    if (saved) {
      const parsed = parseFloat(saved);
      if (!isNaN(parsed) && parsed >= 0.0 && parsed <= 1.0) {
        return parsed;
      }
    }
    return Math.min(Math.max(initialOpacity, 0.0), 1.0);
  });

  const setOpacity = (val: number) => {
    // Strictly clamp between 0.0 and 1.0 (boundary check)
    const clamped = Math.min(Math.max(val, 0.0), 1.0);
    setOpacityState(clamped);
    if (typeof window !== "undefined") {
      localStorage.setItem("airctrl_hud_opacity", clamped.toString());
      document.documentElement.style.setProperty("--hud-opacity", clamped.toString());
    }
  };

  useEffect(() => {
    if (typeof window !== "undefined") {
      document.documentElement.style.setProperty("--hud-opacity", opacity.toString());
    }
  }, [opacity]);

  return { opacity, setOpacity };
}
