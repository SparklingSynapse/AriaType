import { useEffect, useState } from "react";
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
import { Check, CircleNotch, WarningCircle } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import {
  modelCommands,
  settingsCommands,
  type CloudConnectionCheckResult,
  type LocalPolishRuntimeSettings,
  type PolishModelInfo,
  type PolishModelStatus,
} from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useSettingsContext } from "@/contexts/SettingsContext";

const TEMPLATE_LABEL_KEYS: Record<string, string> = {
  filler: "model.polish.templateFiller",
  chat: "model.polish.templateChat",
  formal: "model.polish.templateFormal",
  concise: "model.polish.templateConcise",
  document: "model.polish.templateDocument",
  agent: "model.polish.templateAgent",
};

const LATENCY_LABEL_KEYS: Record<PolishModelInfo["latency_profile"]["class"], string> = {
  fast: "model.polish.latency.fast",
  balanced: "model.polish.latency.balanced",
  slow: "model.polish.latency.slow",
  heavy: "model.polish.latency.heavy",
};

const DEFAULT_LOCAL_RUNTIME: LocalPolishRuntimeSettings = {
  provider_type: "llama-server",
  base_url: "http://127.0.0.1:8000/v1",
  api_key: "",
  server_command: "",
  server_args_json: "",
  ready_timeout_secs: 20,
};

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

function getTemplateNames(templateIds: string[], t: TFunction): string {
  return templateIds
    .map((id) => t(TEMPLATE_LABEL_KEYS[id] ?? id))
    .join(", ");
}

function getLatencyLabel(model: PolishModelInfo, t: TFunction): string {
  return t(LATENCY_LABEL_KEYS[model.latency_profile.class]);
}

function getLatencySummary(model: PolishModelInfo, t: TFunction): string {
  return t("model.polish.latency.bestFor", {
    templates: getTemplateNames(model.latency_profile.recommended_templates, t),
  });
}

function getLatencyCaution(model: PolishModelInfo, t: TFunction): string | null {
  if (model.latency_profile.caution_templates.length === 0) return null;

  return t("model.polish.latency.slowerFor", {
    templates: getTemplateNames(model.latency_profile.caution_templates, t),
  });
}

function getLocalRuntimeProviderLabel(providerType: string, t: TFunction): string {
  switch (providerType) {
    case "llama-server":
      return t("model.polish.localRuntime.llamaServer");
    case "lm-studio":
      return t("model.polish.localRuntime.lmStudio");
    case "ollama":
      return t("model.polish.localRuntime.ollama");
    case "custom":
      return t("model.polish.localRuntime.custom");
    default:
      return providerType;
  }
}

function getRuntimeCheckMessage(
  result: CloudConnectionCheckResult | null,
  checking: boolean,
  t: TFunction,
): string | null {
  if (checking) return t("cloud.check.checking");
  if (!result) return null;

  switch (result.kind) {
    case "ok":
      return t("cloud.check.ok");
    case "disabled":
      return t("cloud.check.disabled");
    case "missing_required":
      return t("cloud.check.missing_required");
    case "invalid_url":
      return t("cloud.check.invalid_url");
    case "unsupported_provider":
      return t("cloud.check.unsupported_provider");
    case "auth_failed":
      return t("cloud.check.auth_failed");
    case "model_failed":
      return t("cloud.check.model_failed");
    case "network_failed":
      return t("cloud.check.network_failed");
    case "timeout":
      return t("cloud.check.timeout");
    case "provider_error":
      return t("cloud.check.provider_error");
  }
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
  const [runtimeCheckResult, setRuntimeCheckResult] = useState<CloudConnectionCheckResult | null>(null);
  const [checkingRuntime, setCheckingRuntime] = useState(false);
  const [polishModelStatus, setPolishModelStatus] = useState<PolishModelStatus | null>(null);

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

  const localRuntime = settings?.local_polish_runtime ?? DEFAULT_LOCAL_RUNTIME;

  const refreshPolishModelStatus = async () => {
    try {
      const status = await modelCommands.getPolishModelStatus();
      setPolishModelStatus(status);
    } catch (err) {
      logger.error("failed_to_load_polish_model_status", { error: String(err) });
      setPolishModelStatus(null);
    }
  };

  const handlePolishModelSelect = async (modelId: string) => {
    setSelectedPolishModel(modelId);
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_model", value: modelId });
    await updateSetting("polish_model", modelId);
    await refreshPolishModelStatus();
  };

  useEffect(() => {
    if (!settings) return;

    void refreshPolishModelStatus();
  }, [
    settings,
    selectedPolishModel,
    polishModels,
    localRuntime.provider_type,
    localRuntime.base_url,
    localRuntime.server_command,
    localRuntime.server_args_json,
  ]);

  const handleRuntimeCheck = async () => {
    setCheckingRuntime(true);
    try {
      const result = await settingsCommands.checkLocalPolishRuntimeConfig();
      setRuntimeCheckResult(result);
      await refreshPolishModelStatus();
    } catch (err) {
      logger.error("failed_to_check_local_polish_runtime", { error: String(err) });
      setRuntimeCheckResult({
        ok: false,
        kind: "provider_error",
        message: String(err),
        duration_ms: 0,
      });
    } finally {
      setCheckingRuntime(false);
    }
  };

  if (!settings) return null;

  const downloadedPolishModels = polishModels.filter((m) => m.downloaded);
  const selectedPolishModelInfo = polishModels.find((m) => m.id === selectedPolishModel);
  const selectedCompatibilityWarning = getCompatibilityWarning(selectedPolishModelInfo, t);
  const selectedLatencyCaution = selectedPolishModelInfo
    ? getLatencyCaution(selectedPolishModelInfo, t)
    : null;
  const selectedModelStatusMatches =
    polishModelStatus?.current_model === selectedPolishModel;
  const selectedRuntimeReady = selectedModelStatusMatches
    ? polishModelStatus.runtime_ready
    : false;
  const selectedModelDownloaded = selectedModelStatusMatches
    ? polishModelStatus.is_downloaded
    : Boolean(selectedPolishModelInfo?.downloaded);
  const selectedModelLoaded = selectedModelStatusMatches
    ? polishModelStatus.is_loaded
    : false;
  const showRuntimeNotReadyWarning =
    Boolean(selectedPolishModelInfo) && selectedModelDownloaded && !selectedRuntimeReady;
  const runtimeProviderLabel = getLocalRuntimeProviderLabel(localRuntime.provider_type, t);
  const runtimeStatusText = selectedModelLoaded
    ? t("model.polish.localRuntime.readyStatus")
    : showRuntimeNotReadyWarning
      ? t("model.polish.localRuntime.notReadyStatus")
      : t("model.polish.localRuntime.fileOnlyStatus");
  const runtimeCheckMessage = getRuntimeCheckMessage(runtimeCheckResult, checkingRuntime, t);

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
                label: `${m.name} · ${m.size} · ${getLatencyLabel(m, t)}`,
              }))}
              placeholder={
                downloadedPolishModels.length === 0
                  ? t("model.active.noModels")
                  : undefined
              }
            />
            {downloadedPolishModels.length === 0 && (
              <p className="text-xs text-muted-foreground">
                {t("model.active.noModels")}
              </p>
            )}
            {selectedCompatibilityWarning && (
              <p className="mt-2 flex items-start gap-1 text-xs leading-5 text-muted-foreground">
                <WarningCircle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                <span>{selectedCompatibilityWarning}</span>
              </p>
            )}
            {selectedPolishModelInfo && (
              <div className="mt-2 space-y-1 text-xs leading-5 text-muted-foreground">
                <p>
                  {getLatencyLabel(selectedPolishModelInfo, t)} ·{" "}
                  {getLatencySummary(selectedPolishModelInfo, t)}
                </p>
                {selectedLatencyCaution && <p>{selectedLatencyCaution}</p>}
              </div>
            )}
          </div>

          <div className="space-y-4 rounded-2xl border border-border/60 p-4">
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0 space-y-1">
                <Label>{t("model.polish.localRuntime.title")}</Label>
                <p className="text-xs leading-5 text-muted-foreground">
                  {t("model.polish.localRuntime.description")}
                </p>
              </div>
              <div className="shrink-0 text-xs font-medium text-muted-foreground">
                {runtimeProviderLabel}
              </div>
            </div>

            <div className="grid gap-3 border-t border-border/60 pt-3 sm:grid-cols-2">
              <div className="min-w-0 space-y-1">
                <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                  {t("model.polish.localRuntime.provider")}
                </p>
                <p className="truncate text-sm text-foreground">{runtimeProviderLabel}</p>
              </div>
              <div className="min-w-0 space-y-1">
                <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                  {t("model.polish.localRuntime.baseUrl")}
                </p>
                <p className="truncate font-mono text-xs text-foreground/80">
                  {localRuntime.base_url}
                </p>
              </div>
            </div>

            <div className="flex items-start justify-between gap-3 border-t border-border/70 pt-3">
              <div className="min-w-0 space-y-1">
                <p className="text-sm font-medium text-foreground">{runtimeStatusText}</p>
                {runtimeCheckMessage && (
                  <p className="text-xs leading-5 text-muted-foreground">
                    {runtimeCheckMessage}
                  </p>
                )}
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8 shrink-0 gap-1.5 px-3 text-xs"
                onClick={handleRuntimeCheck}
                disabled={checkingRuntime}
              >
                {checkingRuntime ? (
                  <CircleNotch className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Check className="h-3.5 w-3.5" />
                )}
                {t("cloud.check.button")}
              </Button>
            </div>

            {showRuntimeNotReadyWarning && (
              <p className="flex items-start gap-1 text-xs leading-5 text-muted-foreground">
                <WarningCircle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                <span>{t("model.polish.localRuntime.notReadyHelp")}</span>
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
                      {m.downloaded && m.id === selectedPolishModel && selectedModelLoaded && (
                        <Check className="h-4 w-4 text-muted-foreground" />
                      )}
                    </div>
                    <div className="text-xs text-muted-foreground mt-0.5">
                      {m.size}
                    </div>
                    <div className="mt-2 space-y-1 text-xs leading-5 text-muted-foreground">
                      <p>
                        {getLatencyLabel(m, t)} · {getLatencySummary(m, t)}
                      </p>
                    </div>
                    {compatibilityWarning && (
                      <p className="mt-2 flex items-start gap-1 text-xs leading-5 text-muted-foreground">
                        <WarningCircle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
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
