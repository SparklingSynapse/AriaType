import React from "react";
import ReactDOM from "react-dom/client";
import { PillWindow } from "./components/Pill/PillWindow";
import { settingsCommands, events } from "./lib/tauri";
import { applyInitialTheme, applyTheme, type ThemeMode } from "./lib/theme";
import "./index.css";
import "./i18n";

let currentMode: ThemeMode = "system";

applyInitialTheme();

// Apply initial theme from settings, fall back to the primed local or system theme
settingsCommands.getSettings().then((settings) => {
  currentMode = (settings.theme_mode as ThemeMode) || "system";
  applyTheme(currentMode);
}).catch(() => {
  applyTheme("system");
});

// Re-apply when system preference changes (only relevant when mode is "system")
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  if (currentMode === "system") applyTheme("system");
});

// Follow app theme setting changes in real time
events.onSettingsChanged((settings) => {
  currentMode = (settings.theme_mode as ThemeMode) || "system";
  applyTheme(currentMode);
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <PillWindow />
  </React.StrictMode>
);
