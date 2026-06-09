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
import { Input } from "@/components/ui/input";
import { Check, WarningCircle } from "@phosphor-icons/react";
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
import { CloudConnectionCheckRow } from "@/components/Home/cloud/CloudConnectionCheckRow";

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

const LOCAL_RUNTIME_PRESETS: Record<string, Pick<LocalPolishRuntimeSettings, "base_url">> = {
  "llama-server": { base_url: "http://127.0.0.1:8000/v1" },
  "lm-studio": { base_url: "http://127.0.0.1:1234/v1" },
  ollama: { base_url: "http://127.0.0.1:11434/v1" },
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

function getCompatibilityTextTone(model: PolishModelInfo | undefined): string {
  if (model?.compatibility?.level === "unsupported") {
    return "text-destructive";
  }
  return "text-amber-500";
}

function getTemplateNames(templateIds: string[], t: TFunction): string {
  return templateIds
    .map((id) => t(TEMPLATE_LABEL_KEYS[id] ?? id))
    .join(", ");
}

function getLatencyLabel(model: PolishModelInfo, t: TFunction): string {
  return t(LATENCY_LABEL_KEYS[model.latency_profile.class]);
}

function getLatencyTone(model: PolishModelInfo): string {
  switch (model.latency_profile.class) {
    case "fast":
      return "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400";
    case "balanced":
      return "border-sky-500/30 bg-sky-500/10 text-sky-600 dark:text-sky-400";
    case "slow":
      return "border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400";
    case "heavy":
      return "border-destructive/30 bg-destructive/10 text-destructive";
  }
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

  const updateLocalRuntime = async (next: LocalPolishRuntimeSettings) => {
    setRuntimeCheckResult(null);
    await updateSetting("local_polish_runtime", next);
    await refreshPolishModelStatus();
  };

  const handleRuntimeProviderSelect = async (providerType: string) => {
    const preset = LOCAL_RUNTIME_PRESETS[providerType];
    await updateLocalRuntime({
      ...localRuntime,
      provider_type: providerType,
      ...(preset ? { base_url: preset.base_url } : {}),
    });
  };

  const handleRuntimeFieldChange = async (
    key: keyof LocalPolishRuntimeSettings,
    value: string | number,
  ) => {
    await updateLocalRuntime({
      ...localRuntime,
      [key]: value,
    });
  };

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
                    ? `${m.name} · ${m.size} · ${getLatencyLabel(m, t)}`
                    : `${m.name} · ${m.size} · ${getLatencyLabel(m, t)} · ${t("model.polish.compat.warningTag")}`,
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
            {selectedPolishModelInfo && (
              <div className="mt-2 space-y-1 text-xs leading-5 text-muted-foreground">
                <p>
                  <span
                    className={`mr-2 inline-flex rounded-full border px-2 py-0.5 text-[11px] font-medium ${getLatencyTone(selectedPolishModelInfo)}`}
                  >
                    {getLatencyLabel(selectedPolishModelInfo, t)}
                  </span>
                  {getLatencySummary(selectedPolishModelInfo, t)}
                </p>
                {selectedLatencyCaution && <p>{selectedLatencyCaution}</p>}
                <p
                  className={
                    selectedModelLoaded
                      ? "text-emerald-600 dark:text-emerald-400"
                      : showRuntimeNotReadyWarning
                        ? "text-amber-500"
                        : "text-muted-foreground"
                  }
                >
                  {selectedModelLoaded
                    ? t("model.polish.localRuntime.readyStatus")
                    : showRuntimeNotReadyWarning
                      ? t("model.polish.localRuntime.notReadyStatus")
                      : t("model.polish.localRuntime.fileOnlyStatus")}
                </p>
              </div>
            )}
          </div>

          <div className="space-y-3 rounded-2xl border border-border p-4">
            <div className="space-y-1">
              <Label>{t("model.polish.localRuntime.title")}</Label>
              <p className="text-xs leading-5 text-muted-foreground">
                {t("model.polish.localRuntime.description")}
              </p>
            </div>
            <div className="grid gap-3 md:grid-cols-2">
              <div className="space-y-1.5">
                <Label>{t("model.polish.localRuntime.provider")}</Label>
                <Select
                  value={localRuntime.provider_type}
                  onChange={(e) => handleRuntimeProviderSelect(e.target.value)}
                  options={[
                    { value: "llama-server", label: t("model.polish.localRuntime.llamaServer") },
                    { value: "lm-studio", label: t("model.polish.localRuntime.lmStudio") },
                    { value: "ollama", label: t("model.polish.localRuntime.ollama") },
                    { value: "custom", label: t("model.polish.localRuntime.custom") },
                  ]}
                />
              </div>
              <div className="space-y-1.5">
                <Label>{t("model.polish.localRuntime.baseUrl")}</Label>
                <Input
                  value={localRuntime.base_url}
                  onChange={(e) => handleRuntimeFieldChange("base_url", e.target.value)}
                  placeholder="http://127.0.0.1:8000/v1"
                />
              </div>
              <div className="space-y-1.5">
                <Label>{t("model.polish.localRuntime.apiKey")}</Label>
                <Input
                  type="password"
                  value={localRuntime.api_key}
                  onChange={(e) => handleRuntimeFieldChange("api_key", e.target.value)}
                  placeholder={t("model.polish.localRuntime.optional")}
                />
              </div>
              <div className="space-y-1.5">
                <Label>{t("model.polish.localRuntime.timeout")}</Label>
                <Input
                  type="number"
                  min={1}
                  value={localRuntime.ready_timeout_secs}
                  onChange={(e) =>
                    handleRuntimeFieldChange(
                      "ready_timeout_secs",
                      Math.max(1, Number(e.target.value) || 1),
                    )
                  }
                />
              </div>
              <div className="space-y-1.5">
                <Label>{t("model.polish.localRuntime.serverCommand")}</Label>
                <Input
                  value={localRuntime.server_command}
                  onChange={(e) => handleRuntimeFieldChange("server_command", e.target.value)}
                  placeholder={t("model.polish.localRuntime.optional")}
                />
                <p className="text-xs leading-5 text-muted-foreground">
                  {t("model.polish.localRuntime.serverCommandHint")}
                </p>
              </div>
              <div className="space-y-1.5">
                <Label>{t("model.polish.localRuntime.serverArgs")}</Label>
                <Input
                  value={localRuntime.server_args_json}
                  onChange={(e) => handleRuntimeFieldChange("server_args_json", e.target.value)}
                  placeholder='["--model","{model_path}"]'
                />
              </div>
            </div>
            <CloudConnectionCheckRow
              result={runtimeCheckResult}
              checking={checkingRuntime}
              onCheck={handleRuntimeCheck}
            />
            {showRuntimeNotReadyWarning && (
              <p className="flex items-start gap-1 text-xs leading-5 text-amber-500">
                <WarningCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
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
                        <Check className="h-4 w-4 text-green-500" />
                      )}
                    </div>
                    <div className="text-xs text-muted-foreground mt-0.5">
                      {m.size}
                    </div>
                    <div className="mt-2 space-y-1 text-xs leading-5 text-muted-foreground">
                      <p>
                        <span
                          className={`mr-2 inline-flex rounded-full border px-2 py-0.5 text-[11px] font-medium ${getLatencyTone(m)}`}
                        >
                          {getLatencyLabel(m, t)}
                        </span>
                        {getLatencySummary(m, t)}
                      </p>
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
