import { useState, useEffect } from "react";
import { RadialDial } from "./components/radial/RadialDial";
import { SettingsHud } from "./components/hud/SettingsHud";
import { SttOverlay } from "./components/stt/SttOverlay";
import { CenterSearchBar } from "./components/actions/CenterSearchBar";

export function App() {
  const [route, setRoute] = useState<"dial" | "hud" | "stt-overlay" | "command-palette">(() => {
    if (typeof window !== "undefined") {
      if (window.location.hash === "#dial") return "dial";
      if (window.location.hash === "#stt-overlay") return "stt-overlay";
      if (window.location.hash === "#command-palette") return "command-palette";
    }
    return "hud";
  });

  useEffect(() => {
    const handleHashChange = () => {
      if (window.location.hash === "#dial") {
        setRoute("dial");
      } else if (window.location.hash === "#stt-overlay") {
        setRoute("stt-overlay");
      } else if (window.location.hash === "#command-palette") {
        setRoute("command-palette");
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

  if (route === "command-palette") {
    return <CenterSearchBar />;
  }

  return <SettingsHud />;
}

export default App;

