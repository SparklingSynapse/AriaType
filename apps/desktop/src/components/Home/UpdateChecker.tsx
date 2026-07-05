import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { ArrowsClockwise, CheckCircle, ArrowSquareOut, WarningCircle, DownloadSimple } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-shell";
import { relaunch } from "@tauri-apps/plugin-process";
import { Channel } from "@tauri-apps/api/core";
import { DOWNLOAD_URL } from "@ariatype/shared";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { updateCommands, type AppUpdateInfo, type UpdateInstallEvent } from "@/lib/tauri";
import { checkAppUpdate } from "@/lib/updateCheck";

export function UpdateChecker() {
  const { t } = useTranslation();
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<AppUpdateInfo | null>(null);
  const [lastChecked, setLastChecked] = useState<Date | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentVersion, setCurrentVersion] = useState("");
  const [autoInstallAvailable, setAutoInstallAvailable] = useState(false);
  const [downloaded, setDownloaded] = useState(0);
  const [contentLength, setContentLength] = useState<number | null>(null);

  const updateAvailable = updateInfo !== null;
  const progressPercent =
    contentLength && contentLength > 0
      ? Math.min(100, Math.round((downloaded / contentLength) * 100))
      : null;

  const checkForUpdates = async () => {
    setChecking(true);
    setError(null);
    setDownloaded(0);
    setContentLength(null);
    setAutoInstallAvailable(false);
    analytics.track(AnalyticsEvents.UPDATE_CHECK_STARTED);
    try {
      const result = await checkAppUpdate();
      setCurrentVersion(result.currentVersion);
      setUpdateInfo(result.update);
      setAutoInstallAvailable(result.autoInstallAvailable);

      if (result.update) {
        analytics.track(AnalyticsEvents.UPDATE_CHECK_COMPLETED, {
          status: "available",
          version: result.update.version,
          source: result.source,
        });
      } else {
        analytics.track(AnalyticsEvents.UPDATE_CHECK_COMPLETED, {
          status: "up_to_date",
          version: result.currentVersion,
          source: result.source,
        });
      }
      setLastChecked(new Date());
    } catch {
      setError(t("update.checkFailed"));
      analytics.track(AnalyticsEvents.UPDATE_CHECK_COMPLETED, {
        status: "failed",
      });
    } finally {
      setChecking(false);
    }
  };

  const installUpdate = async () => {
    if (!updateInfo) return;

    if (!autoInstallAvailable) {
      openDownloadPage();
      return;
    }

    setInstalling(true);
    setError(null);
    setDownloaded(0);
    setContentLength(null);

    const onEvent = new Channel<UpdateInstallEvent>();
    onEvent.onmessage = (event) => {
      if (event.event === "started") {
        setContentLength(event.data.contentLength ?? null);
      }
      if (event.event === "progress") {
        setDownloaded(event.data.downloaded);
        setContentLength(event.data.contentLength ?? null);
      }
    };

    try {
      await updateCommands.install(onEvent);
      analytics.track(AnalyticsEvents.UPDATE_CHECK_COMPLETED, {
        status: "installed",
        version: updateInfo.version,
      });
      await relaunch();
    } catch (err) {
      logger.error("update_install_failed", { error: String(err) });
      setError(t("update.installFailed"));
      analytics.track(AnalyticsEvents.UPDATE_CHECK_COMPLETED, {
        status: "install_failed",
        version: updateInfo.version,
      });
    } finally {
      setInstalling(false);
    }
  };

  const openDownloadPage = () =>
    open(DOWNLOAD_URL).catch((err: unknown) => logger.error("failed_to_open_download_url", { error: String(err) }));

  useEffect(() => { checkForUpdates(); }, []);

  return (
    <div className="space-y-4">
      {/* Status row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {checking || installing ? (
            <ArrowsClockwise className="h-4 w-4 text-muted-foreground shrink-0 animate-spin" />
          ) : error ? (
            <WarningCircle className="h-4 w-4 text-destructive shrink-0" />
          ) : updateAvailable ? (
            <DownloadSimple className="h-4 w-4 text-green-500 shrink-0" />
          ) : (
            <CheckCircle className="h-4 w-4 text-green-500 shrink-0" />
          )}
          <span className="text-sm font-medium">
            {installing
              ? t("update.installing")
              : checking
              ? t("update.checking")
              : error
              ? error
              : updateAvailable
              ? t("update.available")
              : t("update.upToDate")}
          </span>
          {currentVersion && !updateAvailable && !error && !checking && !installing && (
            <span className="text-xs text-muted-foreground">· v{currentVersion}</span>
          )}
        </div>

        <Button
          variant="ghost"
          size="sm"
          onClick={checkForUpdates}
          disabled={checking || installing}
          className="h-7 px-2 text-xs text-muted-foreground"
        >
          <ArrowsClockwise className="h-3 w-3 mr-1.5" />
          {t("update.checkNow")}
        </Button>
      </div>

      {/* Update detail — hidden while checking to avoid visual overlap */}
      {!checking && updateAvailable && updateInfo && (
        <div className="rounded-2xl border border-green-500/20 bg-green-500/5 p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <p className="text-sm font-medium">
                {t("update.newVersion")}:{" "}
                <span className="text-green-600 dark:text-green-500">v{updateInfo.version}</span>
              </p>
              <p className="text-xs text-muted-foreground">
                {t("update.currentVersion")}: v{currentVersion}
              </p>
              {installing && (
                <p className="text-xs text-muted-foreground">
                  {progressPercent !== null
                    ? t("update.downloadProgress", { percent: progressPercent })
                    : t("update.installing")}
                </p>
              )}
            </div>
            <Button size="sm" onClick={installUpdate} disabled={installing}>
              <DownloadSimple className="h-3.5 w-3.5 mr-1.5" />
              {installing
                ? t("update.installing")
                : autoInstallAvailable
                ? t("update.install")
                : t("update.download")}
            </Button>
          </div>
          {installing && progressPercent !== null && (
            <div className="h-1.5 overflow-hidden rounded-full bg-green-500/15">
              <div
                className="h-full rounded-full bg-green-500 transition-[width]"
                style={{ width: `${progressPercent}%` }}
              />
            </div>
          )}
          {error && (
            <div className="flex justify-end border-t border-border pt-3">
              <Button size="sm" variant="outline" onClick={openDownloadPage}>
                <ArrowSquareOut className="h-3.5 w-3.5 mr-1.5" />
                {t("update.download")}
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Last checked */}
      {lastChecked && !checking && !installing && (
        <p className="text-xs text-muted-foreground">
          {t("update.lastChecked")}: {lastChecked.toLocaleString()}
        </p>
      )}
    </div>
  );
}
