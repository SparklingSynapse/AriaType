import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { events, windowCommands, systemCommands } from "./lib/tauri";
import { logger } from "./lib/logger";
import { initAnalytics } from "./lib/analytics";
import { applyInitialTheme, applyTheme, type ThemeMode } from "./lib/theme";
import { SettingsProvider, useSettingsContext } from "./contexts/SettingsContext";
import { ConfirmProvider, setGlobalConfirm, useConfirm } from "./components/ui/confirm";
import "./index.css";
import "./i18n";
import { useTranslation } from "react-i18next";

initAnalytics();

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
      logger.error("shortcut_registration_failed", { error });
      setShowPermissionPrompt(true);
      windowCommands.showToast(t("permission.description"));
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
      <div className="bg-card border border-border rounded-3xl p-8 max-w-sm mx-4 shadow-lg">
        <h3 className="text-lg font-semibold mb-2">{t("permission.title")}</h3>
        <p className="text-muted-foreground text-sm mb-4">
          {t("permission.description")}
        </p>
        <div className="flex gap-3">
          <button
            onClick={() => setShowPermissionPrompt(false)}
            className="flex-1 px-5 py-2 text-sm border border-border rounded-full hover:bg-secondary transition-colors"
          >
            {t("permission.later")}
          </button>
          <button
            onClick={handleOpenSettings}
            className="flex-1 px-5 py-2 text-sm bg-primary text-primary-foreground rounded-full hover:opacity-90 transition-opacity"
          >
            {t("permission.openSystemSettings")}
          </button>
        </div>
      </div>
    </div>
  );
}

// Prime the initial theme before the first React paint to avoid a flash of opaque
// light surfaces before the cached or persisted theme is restored.
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
