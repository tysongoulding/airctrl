import type { Config } from "tailwindcss";

export default {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        hud: {
          bg: "rgba(var(--hud-bg-rgb), var(--hud-opacity, 0.95))",
          border: "rgba(var(--hud-border-rgb), 0.2)",
          card: "rgba(var(--hud-card-rgb), var(--hud-opacity, 0.95))",
        },
      },
      animation: {
        "pulse-slow": "pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "radial-spin": "spin 20s linear infinite",
      },
    },
  },
  plugins: [],
} satisfies Config;
