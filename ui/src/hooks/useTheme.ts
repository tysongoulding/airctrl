import { useState, useEffect } from "react";

export type ThemeMode = "dark" | "light" | "system";

export function useTheme(initialTheme: ThemeMode = "dark") {
  const [theme, setTheme] = useState<ThemeMode>(() => {
    const saved = typeof window !== "undefined" ? (localStorage.getItem("airctrl_theme") as ThemeMode | null) : null;
    return saved && ["dark", "light", "system"].includes(saved) ? saved : initialTheme;
  });

  useEffect(() => {
    if (typeof window === "undefined") return;
    const root = document.documentElement;

    const applyTheme = (isDark: boolean) => {
      if (isDark) {
        root.classList.add("dark");
      } else {
        root.classList.remove("dark");
      }
    };

    if (theme === "system") {
      const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
      applyTheme(mediaQuery.matches);

      const handler = (e: MediaQueryListEvent) => applyTheme(e.matches);
      mediaQuery.addEventListener("change", handler);
      return () => mediaQuery.removeEventListener("change", handler);
    } else {
      applyTheme(theme === "dark");
    }

    localStorage.setItem("airctrl_theme", theme);
  }, [theme]);

  return { theme, setTheme };
}
