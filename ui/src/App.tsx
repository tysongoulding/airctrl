import { useState, useEffect } from "react";
import { RadialDial } from "./components/radial/RadialDial";
import { SettingsHud } from "./components/hud/SettingsHud";

export function App() {
  const [route, setRoute] = useState<"dial" | "hud">(() => {
    if (typeof window !== "undefined" && window.location.hash === "#dial") {
      return "dial";
    }
    return "hud";
  });

  useEffect(() => {
    const handleHashChange = () => {
      if (window.location.hash === "#dial") {
        setRoute("dial");
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

  return <SettingsHud />;
}

export default App;
