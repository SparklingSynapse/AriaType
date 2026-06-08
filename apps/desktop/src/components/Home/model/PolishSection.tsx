import { useEffect } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Check, WarningCircle } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { type PolishModelInfo } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useSettingsContext } from "@/contexts/SettingsContext";

interface PolishSectionProps {
  polishModels: PolishModelInfo[];
  selectedPolishModel: string;
  setSelectedPolishModel: (id: string) => void;
  polishDownloadingId: string | null;
  polishProgress: number | null;
  onDownload: (modelId: string) => void;
  onCancel: (modelId: string) => void;
  onDelete: (modelId: string) => void;
}

function formatMemory(mb: number | null | undefined): string | null {
  if (!mb) return null;
  if (mb >= 1024) {
    const value = mb / 1024;
    return `${Number.isInteger(value) ? value : value.toFixed(1)}GB`;
  }
  return `${mb}MB`;
}

function getCompatibilityWarning(model: PolishModelInfo | undefined, t: TFunction): string | null {
  const compatibility = model?.compatibility;
  if (!compatibility || compatibility.level === "smooth") return null;

  const deviceMemory =
    formatMemory(compatibility.device_memory_mb) ?? t("model.polish.compat.unknownMemory");
  const minimumMemory = formatMemory(compatibility.minimum_memory_mb) ?? "";
  const recommendedMemory = formatMemory(compatibility.recommended_memory_mb) ?? "";

  if (compatibility.code === "memory_below_minimum") {
    return t("model.polish.compat.memoryBelowMinimum", {
      deviceMemory,
      minimumMemory,
    });
  }

  if (compatibility.code === "memory_below_recommended") {
    return t("model.polish.compat.memoryBelowRecommended", {
      deviceMemory,
      recommendedMemory,
    });
  }

  if (compatibility.code === "cpu_threads_low") {
    return t("model.polish.compat.cpuThreadsLow", {
      cpuThreads: compatibility.logical_cpu_count,
    });
  }

  return t("model.polish.compat.memoryUnknown", {
    recommendedMemory,
  });
}

function getCompatibilityTextTone(model: PolishModelInfo | undefined): string {
  if (model?.compatibility?.level === "unsupported") {
    return "text-destructive";
  }
  return "text-amber-500";
}

export function PolishSection({
  polishModels,
  selectedPolishModel,
  setSelectedPolishModel,
  polishDownloadingId,
  polishProgress,
  onDownload,
  onCancel,
  onDelete,
}: PolishSectionProps) {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();

  useEffect(() => {
    if (polishModels.length === 0 || !settings) return;

    const downloadedModels = polishModels.filter((m) => m.downloaded);
    const isValid = downloadedModels.some((m) => m.id === selectedPolishModel);

    if (!isValid && downloadedModels.length > 0) {
      const first = downloadedModels[0].id;
      setSelectedPolishModel(first);
      updateSetting("polish_model", first).catch((err: unknown) => logger.error("failed_to_update_polish_model", { error: String(err) }));
    }
  }, [polishModels, selectedPolishModel, settings]);

  const handlePolishModelSelect = async (modelId: string) => {
    setSelectedPolishModel(modelId);
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_model", value: modelId });
    await updateSetting("polish_model", modelId);
  };

  if (!settings) return null;

  const downloadedPolishModels = polishModels.filter((m) => m.downloaded);
  const selectedPolishModelInfo = polishModels.find((m) => m.id === selectedPolishModel);
  const selectedCompatibilityWarning = getCompatibilityWarning(selectedPolishModelInfo, t);

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>{t("model.polishSection.title")}</CardTitle>
          <CardDescription>{t("model.polishSection.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>{t("model.polish.selectModel")}</Label>
            <Select
              value={
                downloadedPolishModels.length === 0 ? "" : selectedPolishModel
              }
              onChange={(e) => handlePolishModelSelect(e.target.value)}
              options={downloadedPolishModels.map((m) => ({
                value: m.id,
                label:
                  m.compatibility.level === "smooth"
                    ? `${m.name} · ${m.size}`
                    : `${m.name} · ${m.size} · ${t("model.polish.compat.warningTag")}`,
              }))}
              placeholder={
                downloadedPolishModels.length === 0
                  ? t("model.active.noModels")
                  : undefined
              }
            />
            {downloadedPolishModels.length === 0 && (
              <p className="text-xs text-amber-500">
                {t("model.active.noModels")}
              </p>
            )}
            {selectedCompatibilityWarning && (
              <p
                className={`mt-2 flex items-start gap-1 text-xs leading-5 ${getCompatibilityTextTone(selectedPolishModelInfo)}`}
              >
                <WarningCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                <span>{selectedCompatibilityWarning}</span>
              </p>
            )}
          </div>

          <div className="space-y-3">
            {polishModels.map((m) => {
              const isDownloading = polishDownloadingId === m.id;
              const compatibilityWarning = getCompatibilityWarning(m, t);
              return (
                <div
                  key={m.id}
                  className="flex items-center justify-between space-x-4 p-4 rounded-2xl border border-border"
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-sm">{m.name}</span>
                      {m.downloaded && m.id === selectedPolishModel && (
                        <Check className="h-4 w-4 text-green-500" />
                      )}
                    </div>
                    <div className="text-xs text-muted-foreground mt-0.5">
                      {m.size}
                    </div>
                    {compatibilityWarning && (
                      <p
                        className={`mt-2 flex items-start gap-1 text-xs leading-5 ${getCompatibilityTextTone(m)}`}
                      >
                        <WarningCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                        <span>{compatibilityWarning}</span>
                      </p>
                    )}
                    {isDownloading && polishProgress !== null && (
                      <div className="mt-2">
                        <div className="h-1.5 bg-secondary rounded-full overflow-hidden border border-border">
                          <div
                            className="h-full bg-primary transition-all"
                            style={{ width: `${polishProgress}%` }}
                          />
                        </div>
                        <span className="text-xs text-muted-foreground">
                          {polishProgress}%
                        </span>
                      </div>
                    )}
                  </div>
                  <div className="ml-3">
                    {m.downloaded ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-24"
                        onClick={() => onDelete(m.id)}
                        disabled={isDownloading}
                      >
                        {t("model.available.delete")}
                      </Button>
                    ) : isDownloading ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-24"
                        onClick={() => onCancel(m.id)}
                      >
                        {t("model.available.cancel")}
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        className="w-24"
                        onClick={() => onDownload(m.id)}
                        disabled={polishDownloadingId !== null}
                      >
                        {t("model.available.download")}
                      </Button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>


        </CardContent>
      </Card>
    </div>
  );
}
