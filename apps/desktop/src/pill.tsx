import React from "react";
import ReactDOM from "react-dom/client";
import { PillWindow } from "./components/Pill/PillWindow";
import { settingsCommands, events } from "./lib/tauri";
import "./index.css";

type ThemeMode = "system" | "light" | "dark";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyTheme(mode: ThemeMode) {
  const isDark = mode === "system" ? getSystemTheme() === "dark" : mode === "dark";
  document.documentElement.classList.toggle("dark", isDark);
}

let currentMode: ThemeMode = "system";

// Apply initial theme from settings, fall back to system preference
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
