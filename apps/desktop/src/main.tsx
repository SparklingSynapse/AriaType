import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { events, windowCommands, systemCommands } from "./lib/tauri";
import { initAnalytics } from "./lib/analytics";
import { SettingsProvider, useSettingsContext } from "./contexts/SettingsContext";
import { ConfirmProvider, setGlobalConfirm, useConfirm } from "./components/ui/confirm";
import "./index.css";
import "./i18n";
import { useTranslation } from "react-i18next";

initAnalytics();

type ThemeMode = "system" | "light" | "dark";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyTheme(mode: ThemeMode) {
  const isDark = mode === "system" ? getSystemTheme() === "dark" : mode === "dark";
  document.documentElement.classList.toggle("dark", isDark);
  try { localStorage.setItem("ariatype-theme", isDark ? "dark" : "light"); } catch {}
}

function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { settings } = useSettingsContext();

  useEffect(() => {
    const mode = (settings?.theme_mode as ThemeMode) || "system";
    applyTheme(mode);
  }, [settings?.theme_mode]);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleSystemThemeChange = () => {
      if ((settings?.theme_mode || "system") === "system") applyTheme("system");
    };
    mediaQuery.addEventListener("change", handleSystemThemeChange);
    return () => mediaQuery.removeEventListener("change", handleSystemThemeChange);
  }, [settings?.theme_mode]);

  return <>{children}</>;
}

function GlobalConfirmSetup() {
  const confirm = useConfirm();
  useEffect(() => {
    setGlobalConfirm(confirm);
  }, [confirm]);
  return null;
}

function PermissionNotice() {
  const { t } = useTranslation();
  const [showPermissionPrompt, setShowPermissionPrompt] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    events.onShortcutRegistrationFailed((error) => {
      console.error("Shortcut registration failed:", error);
      setShowPermissionPrompt(true);
      windowCommands.showToast(t("permission.description"));
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [t]);

  const handleOpenSettings = async () => {
    try {
      await systemCommands.applyPermission("accessibility");
    } catch (err) {
      console.error("Failed to open settings:", err);
    }
  };

  if (!showPermissionPrompt) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-card border border-border rounded-xl p-6 max-w-sm mx-4 shadow-lg">
        <h3 className="text-lg font-semibold mb-2">{t("permission.title")}</h3>
        <p className="text-muted-foreground text-sm mb-4">
          {t("permission.description")}
        </p>
        <div className="flex gap-3">
          <button
            onClick={() => setShowPermissionPrompt(false)}
            className="flex-1 px-4 py-2 text-sm border border-input rounded-lg hover:bg-secondary transition-colors"
          >
            {t("permission.later")}
          </button>
          <button
            onClick={handleOpenSettings}
            className="flex-1 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-lg hover:opacity-90 transition-opacity"
          >
            {t("permission.openSystemSettings")}
          </button>
        </div>
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <SettingsProvider>
        <ThemeProvider>
          <ConfirmProvider>
            <GlobalConfirmSetup />
            <PermissionNotice />
            <App />
          </ConfirmProvider>
        </ThemeProvider>
      </SettingsProvider>
    </BrowserRouter>
  </React.StrictMode>
);
