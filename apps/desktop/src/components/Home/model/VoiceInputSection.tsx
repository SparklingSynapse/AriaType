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
import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  type ModelInfo,
} from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useSettingsContext } from "@/contexts/SettingsContext";

function getModelLanguageHint(modelName: string, t: (key: string) => string): string {
  if (modelName === "sense-voice-small-q4_k" || modelName === "sense-voice-small-q8_0") {
    return " · " + t("model.hint.cjkBest");
  }
  if (modelName === "large-v3-turbo-q8_0") {
    return " · " + t("model.hint.multiLang");
  }
  if (modelName === "small-q8_0") {
    return " · " + t("model.hint.enCjk");
  }
  if (modelName === "medium-q5_0") {
    return " · " + t("model.hint.enCjkEu");
  }
  if (modelName === "tiny" || modelName === "base") {
    return " · " + t("model.hint.enFocused");
  }
return "";
}

interface VoiceInputSectionProps {
  models: ModelInfo[];
  downloading: Set<string>;
  downloadProgress: Record<string, number>;
  onDownload: (modelName: string) => void;
  onCancel: (modelName: string) => void;
  onDelete: (modelName: string) => void;
}

export function VoiceInputSection({
  models,
  downloading,
  downloadProgress,
  onDownload,
  onCancel,
  onDelete,
}: VoiceInputSectionProps) {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();

  useEffect(() => {
    if (models.length === 0 || !settings) return;

    const downloadedModels = models.filter((m) => m.downloaded);
    const currentModelIsValid = downloadedModels.some((m) => m.name === settings.model);

    if (!currentModelIsValid && downloadedModels.length > 0) {
      updateSetting("model", downloadedModels[0].name).catch((err: unknown) => logger.error("failed_to_update_model", { error: String(err) }));
    }
  }, [models, settings?.model]);

  const handleModelChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "model", value });
    await updateSetting("model", value);
  };

  if (!settings) return null;

  const downloadedModels = models.filter((m) => m.downloaded);
  const selectedModel = models.find((m) => m.name === settings.model);

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>{t("model.active.title")}</CardTitle>
          <CardDescription>{t("model.active.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>{t("model.active.model")}</Label>
            <Select
              value={downloadedModels.length === 0 ? "" : settings.model}
              onChange={(e) => handleModelChange(e.target.value)}
              options={downloadedModels.map((m) => ({
                value: m.name,
                label: m.display_name,
              }))}
              placeholder={
                downloadedModels.length === 0
                  ? t("model.active.noModels")
                  : undefined
              }
            />
            {downloadedModels.length === 0 && (
              <p className="text-xs text-amber-500">
                {t("model.active.noModels")}
              </p>
            )}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("model.available.title")}</CardTitle>
          <CardDescription>
            {t("model.available.description")}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {models.map((m) => {
            const isDownloading = downloading.has(m.name);
            const progress = downloadProgress[m.name];
            return (
              <div
                key={m.name}
                className="flex items-center justify-between space-x-4 p-4 rounded-2xl border border-border"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm">
                      {m.display_name}
                    </span>
                    {m.downloaded && m.name === settings.model && (
                      <Check className="h-4 w-4 text-green-500" />
                    )}
                  </div>
                  <div className="text-xs text-muted-foreground mt-0.5">
                    {m.size_mb}MB · {t("model.available.speed")}:{" "}
                    {m.speed_score}/10 · {t("model.available.accuracy")}:{" "}
                    {m.accuracy_score}/10{getModelLanguageHint(m.name, t)}
                  </div>
                  {isDownloading && (
                    <div className="mt-2">
                      <div className="h-1.5 bg-secondary rounded-full overflow-hidden border border-border">
                        <div
                          className="h-full bg-primary transition-all"
                          style={{ width: `${progress ?? 0}%` }}
                        />
                      </div>
                      <span className="text-xs text-muted-foreground">
                        {progress ?? 0}%
                      </span>
                    </div>
                  )}
                </div>
                <div className="ml-3 flex gap-2">
                  {m.downloaded ? (
                    <Button
                      variant="outline"
                      size="sm"
                      className="w-24"
                      onClick={() => onDelete(m.name)}
                      disabled={isDownloading}
                    >
                      {t("model.available.delete")}
                    </Button>
                  ) : isDownloading ? (
                    <Button
                      variant="outline"
                      size="sm"
                      className="w-24"
                      onClick={() => onCancel(m.name)}
                    >
                      {t("model.available.cancel")}
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      className="w-24"
                      onClick={() => onDownload(m.name)}
                    >
                      {t("model.available.download")}
                    </Button>
                  )}
                </div>
              </div>
            );
          })}
        </CardContent>
      </Card>

      {selectedModel && (
        <Card>
          <CardHeader>
            <CardTitle>{t("model.info.title")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("model.info.model")}
                </span>
                <span>{selectedModel.display_name}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("model.info.size")}
                </span>
                <span>{selectedModel.size_mb}MB</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("model.info.status")}
                </span>
                <span
                  className={
                    selectedModel.downloaded
                      ? "text-green-600"
                      : "text-amber-600"
                  }
                >
                  {selectedModel.downloaded
                    ? t("model.info.ready")
                    : t("model.info.notDownloaded")}
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}