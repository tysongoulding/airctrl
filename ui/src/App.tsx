import { useState, useEffect } from "react";
import { RadialDial } from "./components/radial/RadialDial";
import { SettingsHud } from "./components/hud/SettingsHud";
import { SttOverlay } from "./components/stt/SttOverlay";

export function App() {
  const [route, setRoute] = useState<"dial" | "hud" | "stt-overlay">(() => {
    if (typeof window !== "undefined") {
      if (window.location.hash === "#dial") return "dial";
      if (window.location.hash === "#stt-overlay") return "stt-overlay";
    }
    return "hud";
  });

  useEffect(() => {
    const handleHashChange = () => {
      if (window.location.hash === "#dial") {
        setRoute("dial");
      } else if (window.location.hash === "#stt-overlay") {
        setRoute("stt-overlay");
      } else {
        setRoute("hud");
      }
    };

    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  if (route === "dial") {
    return <RadialDial />;
  }

  if (route === "stt-overlay") {
    return <SttOverlay />;
  }

  return <SettingsHud />;
}

export default App;

