import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { events, systemCommands } from "./lib/tauri";
import { showToast } from "./lib/toast";
import { logger } from "./lib/logger";
import { initAnalytics } from "./lib/analytics";
import { applyInitialTheme, applyTheme, type ThemeMode } from "./lib/theme";
import { SettingsProvider, useSettingsContext } from "./contexts/SettingsContext";
import { ConfirmProvider, setGlobalConfirm, useConfirm } from "./components/ui/confirm";
import "./index.css";
import "./i18n";
import { useTranslation } from "react-i18next";

initAnalytics();

let lastRegistrationError: string | null = null;
let lastRegistrationErrorTime = 0;

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
    events.onShortcutRegistrationFailed((payload) => {
      const now = Date.now();
      if (lastRegistrationError === payload.error && now - lastRegistrationErrorTime < 1000) return;
      lastRegistrationError = payload.error;
      lastRegistrationErrorTime = now;
      logger.error("shortcut_registration_failed", { error: payload.error, profile_id: payload.profile_id });
      
      const isPermissionError = payload.error.toLowerCase().includes("permission") 
        || payload.error.toLowerCase().includes("accessibility");
      
      if (isPermissionError) {
        setShowPermissionPrompt(true);
        showToast(t("permission.description"));
      } else {
        showToast(t("hotkey.registrationFailed", "Hotkey registration failed"));
      }
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [t]);

  const handleOpenSettings = async () => {
    try {
      await systemCommands.applyPermission("accessibility");
    } catch (err) {
      logger.error("failed_to_open_settings", { error: String(err) });
    }
  };

  if (!showPermissionPrompt) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-card border border-border rounded-3xl p-8 w-[420px] mx-4 shadow-lg text-center">
        <h3 className="text-lg font-semibold mb-2">{t("permission.title")}</h3>
        <p className="text-muted-foreground text-sm mb-6">
          {t("permission.description")}
        </p>
        <div className="grid grid-cols-2 gap-3 max-w-[280px] mx-auto">
          <button
            onClick={() => setShowPermissionPrompt(false)}
            className="px-4 py-2.5 text-sm border border-border rounded-full hover:bg-secondary transition-colors whitespace-nowrap"
          >
            {t("permission.later")}
          </button>
          <button
            onClick={handleOpenSettings}
            className="px-4 py-2.5 text-sm bg-primary text-primary-foreground rounded-full hover:opacity-90 transition-opacity whitespace-nowrap"
          >
            {t("permission.openSystemSettings")}
          </button>
        </div>
      </div>
    </div>
  );
}

applyInitialTheme();

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
