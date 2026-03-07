import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { RefreshCw, CheckCircle2, ExternalLink, AlertCircle, Download } from "lucide-react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-shell";
import { getVersion } from "@tauri-apps/api/app";
import { UPDATE_CHECK_URL, DOWNLOAD_URL } from "@ariatype/shared";
import { compareVersions, validate } from "compare-versions";
import type { UpdateInfo } from "@ariatype/shared";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";

function isNewerVersion(latest: string, current: string): boolean {
  if (!validate(latest) || !validate(current)) return false;
  return compareVersions(latest, current) > 0;
}

export function UpdateChecker() {
  const { t } = useTranslation();
  const [checking, setChecking] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [lastChecked, setLastChecked] = useState<Date | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentVersion, setCurrentVersion] = useState("");

  const checkForUpdates = async () => {
    setChecking(true);
    setError(null);
    analytics.track(AnalyticsEvents.UPDATE_CHECK_STARTED);
    try {
      const [response, appVersion] = await Promise.all([
        fetch(UPDATE_CHECK_URL),
        getVersion(),
      ]);
      setCurrentVersion(appVersion);
      if (!response.ok) throw new Error("Failed to fetch update info");
      const data = await response.json();
      if (data.version && isNewerVersion(data.version, appVersion)) {
        setUpdateAvailable(true);
        setUpdateInfo({
          version: data.version,
          date: data.pub_date,
          notes: data.notes,
          url: data.url || DOWNLOAD_URL,
        });
        analytics.track(AnalyticsEvents.UPDATE_CHECK_COMPLETED, {
          status: "available",
          version: data.version,
        });
      } else {
        setUpdateAvailable(false);
        setUpdateInfo(null);
        analytics.track(AnalyticsEvents.UPDATE_CHECK_COMPLETED, {
          status: "up_to_date",
          version: appVersion,
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

  const openDownloadPage = () =>
    open(updateInfo?.url || DOWNLOAD_URL).catch(console.error);

  useEffect(() => { checkForUpdates(); }, []);

  return (
    <div className="space-y-4">
      {/* Status row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {checking ? (
            <RefreshCw className="h-4 w-4 text-muted-foreground shrink-0 animate-spin" />
          ) : error ? (
            <AlertCircle className="h-4 w-4 text-destructive shrink-0" />
          ) : updateAvailable ? (
            <Download className="h-4 w-4 text-green-500 shrink-0" />
          ) : (
            <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />
          )}
          <span className="text-sm font-medium">
            {checking
              ? t("update.checking")
              : error
              ? t("update.checkFailed")
              : updateAvailable
              ? t("update.available")
              : t("update.upToDate")}
          </span>
          {currentVersion && !updateAvailable && !error && !checking && (
            <span className="text-xs text-muted-foreground">· v{currentVersion}</span>
          )}
        </div>

        <Button
          variant="ghost"
          size="sm"
          onClick={checkForUpdates}
          disabled={checking}
          className="h-7 px-2 text-xs text-muted-foreground"
        >
          <RefreshCw className="h-3 w-3 mr-1.5" />
          {t("update.checkNow")}
        </Button>
      </div>

      {/* Update detail — hidden while checking to avoid visual overlap */}
      {!checking && updateAvailable && updateInfo && (
        <div className="rounded-lg border border-green-500/20 bg-green-500/5 p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <p className="text-sm font-medium">
                {t("update.newVersion")}:{" "}
                <span className="text-green-600 dark:text-green-500">v{updateInfo.version}</span>
              </p>
              <p className="text-xs text-muted-foreground">
                {t("update.currentVersion")}: v{currentVersion}
              </p>
            </div>
            <Button size="sm" onClick={openDownloadPage}>
              <ExternalLink className="h-3.5 w-3.5 mr-1.5" />
              {t("update.download")}
            </Button>
          </div>
          {updateInfo.notes && (
            <p className="text-xs text-muted-foreground whitespace-pre-wrap border-t border-border pt-3">
              {updateInfo.notes}
            </p>
          )}
        </div>
      )}

      {/* Last checked */}
      {lastChecked && !checking && (
        <p className="text-xs text-muted-foreground">
          {t("update.lastChecked")}: {lastChecked.toLocaleString()}
        </p>
      )}
    </div>
  );
}
