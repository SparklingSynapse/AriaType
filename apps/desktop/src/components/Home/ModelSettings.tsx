import { useState, useEffect, useCallback } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Cloud, Check } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  settingsCommands,
  modelCommands,
  events,
  type ModelInfo,
  type PolishModelInfo,
} from "@/lib/tauri";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import type { } from "@/types";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { SettingsPageLayout } from "./SettingsPageLayout";
import { confirm } from "@/components/ui/confirm";

const WHISPER_LANGUAGES = [
  { code: "auto", label: "Auto", prompt: "" },
  { code: "af", label: "Afrikaans", prompt: "" },
  { code: "sq", label: "Albanian", prompt: "" },
  { code: "am", label: "Amharic", prompt: "" },
  { code: "ar", label: "Arabic", prompt: "" },
  { code: "hy", label: "Armenian", prompt: "" },
  { code: "as", label: "Assamese", prompt: "" },
  { code: "az", label: "Azerbaijani", prompt: "" },
  { code: "ba", label: "Bashkir", prompt: "" },
  { code: "eu", label: "Basque", prompt: "" },
  { code: "be", label: "Belarusian", prompt: "" },
  { code: "bn", label: "Bengali", prompt: "" },
  { code: "bs", label: "Bosnian", prompt: "" },
  { code: "br", label: "Breton", prompt: "" },
  { code: "bg", label: "Bulgarian", prompt: "" },
  { code: "yue", label: "Cantonese", prompt: "" },
  { code: "ca", label: "Catalan", prompt: "" },
  // Chinese variants share ISO 639-1 code "zh"; a prompt steers the script.
  // This knowledge lives here so the transcription engine stays language-agnostic.
  {
    code: "zh",
    label: "Chinese (Simplified)",
    prompt:
      "This is a Mandarin speech-to-text result. Please output in Simplified Chinese characters. Do not use Traditional Chinese. The speaker is from mainland China.",
  },
  {
    code: "zh-TW",
    label: "Chinese (Traditional)",
    prompt:
      "This is a Mandarin transcription. Use Traditional Chinese characters. The speaker is from Taiwan. Please output all content in Traditional Chinese.",
  },
  { code: "hr", label: "Croatian", prompt: "" },
  { code: "cs", label: "Czech", prompt: "" },
  { code: "da", label: "Danish", prompt: "" },
  { code: "nl", label: "Dutch", prompt: "" },
  { code: "en", label: "English", prompt: "" },
  { code: "et", label: "Estonian", prompt: "" },
  { code: "fo", label: "Faroese", prompt: "" },
  { code: "fi", label: "Finnish", prompt: "" },
  { code: "fr", label: "French", prompt: "" },
  { code: "gl", label: "Galician", prompt: "" },
  { code: "ka", label: "Georgian", prompt: "" },
  { code: "de", label: "German", prompt: "" },
  { code: "el", label: "Greek", prompt: "" },
  { code: "gu", label: "Gujarati", prompt: "" },
  { code: "ht", label: "Haitian Creole", prompt: "" },
  { code: "ha", label: "Hausa", prompt: "" },
  { code: "haw", label: "Hawaiian", prompt: "" },
  { code: "he", label: "Hebrew", prompt: "" },
  { code: "hi", label: "Hindi", prompt: "" },
  { code: "hu", label: "Hungarian", prompt: "" },
  { code: "is", label: "Icelandic", prompt: "" },
  { code: "id", label: "Indonesian", prompt: "" },
  { code: "jw", label: "Javanese", prompt: "" },
  { code: "kn", label: "Kannada", prompt: "" },
  { code: "kk", label: "Kazakh", prompt: "" },
  { code: "km", label: "Khmer", prompt: "" },
  { code: "ko", label: "Korean", prompt: "" },
  { code: "lo", label: "Lao", prompt: "" },
  { code: "la", label: "Latin", prompt: "" },
  { code: "lv", label: "Latvian", prompt: "" },
  { code: "ln", label: "Lingala", prompt: "" },
  { code: "lt", label: "Lithuanian", prompt: "" },
  { code: "lb", label: "Luxembourgish", prompt: "" },
  { code: "mk", label: "Macedonian", prompt: "" },
  { code: "mg", label: "Malagasy", prompt: "" },
  { code: "ms", label: "Malay", prompt: "" },
  { code: "ml", label: "Malayalam", prompt: "" },
  { code: "mt", label: "Maltese", prompt: "" },
  { code: "mi", label: "Maori", prompt: "" },
  { code: "mr", label: "Marathi", prompt: "" },
  { code: "mn", label: "Mongolian", prompt: "" },
  { code: "my", label: "Myanmar", prompt: "" },
  { code: "ne", label: "Nepali", prompt: "" },
  { code: "no", label: "Norwegian", prompt: "" },
  { code: "nn", label: "Nynorsk", prompt: "" },
  { code: "oc", label: "Occitan", prompt: "" },
  { code: "ps", label: "Pashto", prompt: "" },
  { code: "fa", label: "Persian", prompt: "" },
  { code: "pl", label: "Polish", prompt: "" },
  { code: "pt", label: "Portuguese", prompt: "" },
  { code: "pa", label: "Punjabi", prompt: "" },
  { code: "ro", label: "Romanian", prompt: "" },
  { code: "ru", label: "Russian", prompt: "" },
  { code: "sa", label: "Sanskrit", prompt: "" },
  { code: "sr", label: "Serbian", prompt: "" },
  { code: "sn", label: "Shona", prompt: "" },
  { code: "sd", label: "Sindhi", prompt: "" },
  { code: "si", label: "Sinhala", prompt: "" },
  { code: "sk", label: "Slovak", prompt: "" },
  { code: "sl", label: "Slovenian", prompt: "" },
  { code: "so", label: "Somali", prompt: "" },
  { code: "es", label: "Spanish", prompt: "" },
  { code: "su", label: "Sundanese", prompt: "" },
  { code: "sw", label: "Swahili", prompt: "" },
  { code: "sv", label: "Swedish", prompt: "" },
  { code: "tl", label: "Tagalog", prompt: "" },
  { code: "tg", label: "Tajik", prompt: "" },
  { code: "ta", label: "Tamil", prompt: "" },
  { code: "tt", label: "Tatar", prompt: "" },
  { code: "te", label: "Telugu", prompt: "" },
  { code: "th", label: "Thai", prompt: "" },
  { code: "bo", label: "Tibetan", prompt: "" },
  { code: "tk", label: "Turkmen", prompt: "" },
  { code: "tr", label: "Turkish", prompt: "" },
  { code: "uk", label: "Ukrainian", prompt: "" },
  { code: "ur", label: "Urdu", prompt: "" },
  { code: "uz", label: "Uzbek", prompt: "" },
  { code: "vi", label: "Vietnamese", prompt: "" },
  { code: "cy", label: "Welsh", prompt: "" },
  { code: "yi", label: "Yiddish", prompt: "" },
  { code: "yo", label: "Yoruba", prompt: "" },
];

export function ModelSettings() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [polishModels, setPolishModels] = useState<PolishModelInfo[]>([]);
  const [selectedPolishModel, setSelectedPolishModel] = useState<string>("");
  const [polishDownloadingId, setPolishDownloadingId] = useState<string | null>(null);
  const [polishProgress, setPolishProgress] = useState<number | null>(null);
  const [polishTemplate, setPolishTemplate] = useState<string>("filler");
  const [availableSubdomains, setAvailableSubdomains] = useState<string[]>([]);

  const loadModels = useCallback(async () => {
    try {
      const list = await modelCommands.getModels();
      setModels(list);
    } catch (err) {
      console.error("Failed to load models:", err);
    }
  }, []);

  useEffect(() => {
    if (!settings) return;

    loadModels();

    modelCommands.getPolishModels().then((models) => {
      setPolishModels(models);
      if (settings.polish_model) setSelectedPolishModel(settings.polish_model);
    }).catch(console.error);

    if (settings.stt_engine_work_domain && settings.stt_engine_work_domain !== "general") {
      settingsCommands.getAvailableSubdomains(settings.stt_engine_work_domain)
        .then(setAvailableSubdomains)
        .catch(console.error);
    }
  }, [settings === null]); // only run once when settings first loads

  useEffect(() => {
    let unlistenComplete: (() => void) | undefined;
    let unlistenCancelled: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    let unlistenDeleted: (() => void) | undefined;
    let unlistenPolishProgress: (() => void) | undefined;
    let unlistenPolishComplete: (() => void) | undefined;
    let unlistenPolishCancelled: (() => void) | undefined;
    let unlistenPolishDeleted: (() => void) | undefined;

    const loadPolishModels = async () => {
      try {
        const models = await modelCommands.getPolishModels();
        setPolishModels(models);
      } catch (err) {
        console.error("Failed to load polish models:", err);
      }
    };

    const setup = async () => {
      unlistenComplete = await events.onModelDownloadComplete((model) => {
        setDownloading((prev) => {
          const next = new Set(prev);
          next.delete(model);
          return next;
        });
        setDownloadProgress((prev) => {
          const next = { ...prev };
          delete next[model];
          return next;
        });
        loadModels();
      });

      unlistenCancelled = await events.onModelDownloadCancelled((model) => {
        setDownloading((prev) => {
          const next = new Set(prev);
          next.delete(model);
          return next;
        });
        setDownloadProgress((prev) => {
          const next = { ...prev };
          delete next[model];
          return next;
        });
      });

      unlistenProgress = await events.onModelDownloadProgress((data) => {
        setDownloadProgress((prev) => ({
          ...prev,
          [data.model]: data.progress,
        }));
      });

      unlistenDeleted = await events.onModelDeleted(() => {
        loadModels();
      });

      unlistenPolishProgress = await events.onPolishModelDownloadProgress(
        (data) => {
          if (data.model_id === polishDownloadingId) {
            setPolishProgress(data.progress);
          }
        },
      );

      unlistenPolishComplete = await events.onPolishModelDownloadComplete(
        (modelId) => {
          if (modelId === polishDownloadingId) {
            setPolishDownloadingId(null);
            setPolishProgress(null);
            loadPolishModels();
            setSelectedPolishModel((prev) => {
              if (!prev) {
                settingsCommands
                  .updateSettings("polish_model", modelId)
                  .catch(console.error);
                return modelId;
              }
              return prev;
            });
          }
        },
      );

      unlistenPolishCancelled = await events.onPolishModelDownloadCancelled(
        (modelId) => {
          if (modelId === polishDownloadingId) {
            setPolishDownloadingId(null);
            setPolishProgress(null);
          }
        },
      );

      unlistenPolishDeleted = await events.onPolishModelDeleted(() => {
        loadPolishModels();
      });
    };
    setup();

    return () => {
      unlistenComplete?.();
      unlistenCancelled?.();
      unlistenProgress?.();
      unlistenDeleted?.();
      unlistenPolishProgress?.();
      unlistenPolishComplete?.();
      unlistenPolishCancelled?.();
      unlistenPolishDeleted?.();
    };
  }, [loadModels, polishDownloadingId]);

  useEffect(() => {
    if (models.length === 0 || !settings) return;

    const downloadedModels = models.filter((m) => m.downloaded);
    const currentModelIsValid = downloadedModels.some((m) => m.name === settings.model);

    if (!currentModelIsValid && downloadedModels.length > 0) {
      updateSetting("model", downloadedModels[0].name).catch(console.error);
    }
  }, [models, settings?.model]);

  useEffect(() => {
    if (polishModels.length === 0) return;

    const downloadedModels = polishModels.filter((m) => m.downloaded);
    const isValid = downloadedModels.some((m) => m.id === selectedPolishModel);

    if (!isValid && downloadedModels.length > 0) {
      const first = downloadedModels[0].id;
      setSelectedPolishModel(first);
      updateSetting("polish_model", first).catch(console.error);
    }
  }, [polishModels, selectedPolishModel]);

  const handleModelChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "model", value });
    await updateSetting("model", value);
  };

  const handleWhisperLanguageChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "stt_engine_language", value });
    const lang = WHISPER_LANGUAGES.find((l) => l.code === value);
    await updateSetting("stt_engine_language", value);
    await updateSetting("stt_engine_initial_prompt", lang?.prompt ?? "");
  };

  const handleGpuChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "gpu_acceleration", value: String(checked) });
    await updateSetting("gpu_acceleration", checked);
  };

  const handleModelResidentChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "model_resident", value: String(checked) });
    await updateSetting("model_resident", checked);
  };

  const handleIdleUnloadChange = async (value: number) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "idle_unload_minutes", value: String(value) });
    await updateSetting("idle_unload_minutes", value);
  };

  const handleDownload = async (modelName: string) => {
    if (downloading.has(modelName)) return;
    setDownloading((prev) => new Set(prev).add(modelName));
    try {
      await modelCommands.downloadModel(modelName);
    } catch (err) {
      console.error("Failed to download model:", err);
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(modelName);
        return next;
      });
      setDownloadProgress((prev) => {
        const next = { ...prev };
        delete next[modelName];
        return next;
      });
    }
  };

  const handleCancel = async (modelName: string) => {
    try {
      await modelCommands.cancelDownload(modelName);
    } catch (err) {
      console.error("Failed to cancel download:", err);
    }
  };

  const handleDelete = async (modelName: string) => {
    const confirmed = await confirm({
      title: "Delete Model",
      description: `Are you sure you want to delete the "${modelName}" model? This action cannot be undone.`,
      confirmText: "Delete",
      cancelText: "Cancel",
      variant: "danger",
    });
    if (!confirmed) return;

    try {
      await modelCommands.deleteModel(modelName);
      await loadModels();
    } catch (err) {
      console.error("Failed to delete model:", err);
    }
  };

  const handlePolishToggle = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_enabled", value: String(checked) });
    await updateSetting("polish_enabled", checked);
  };

  const handlePolishSystemPromptChange = async (value: string) => {
    setPolishTemplate("custom");
    await updateSetting("polish_system_prompt", value);
  };

  const handlePolishTemplateChange = async (template: string) => {
    setPolishTemplate(template as "filler" | "formal" | "concise" | "custom");
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_template", value: template });

    if (template !== "custom") {
      try {
        const prompt = await modelCommands.getPolishTemplatePrompt(template);
        await updateSetting("polish_system_prompt", prompt);
      } catch (err) {
        console.error("Failed to get template prompt:", err);
      }
    }
  };

  const handlePolishModelSelect = async (modelId: string) => {
    setSelectedPolishModel(modelId);
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_model", value: modelId });
    await updateSetting("polish_model", modelId);
  };

  const handlePolishDownload = async (modelId: string) => {
    setPolishDownloadingId(modelId);
    setPolishProgress(0);
    try {
      await modelCommands.downloadPolishModelById(modelId);
    } catch (err) {
      console.error("Failed to download polish model:", err);
      setPolishDownloadingId(null);
      setPolishProgress(null);
    }
  };

  const handlePolishCancel = async (modelId: string) => {
    try {
      await modelCommands.cancelPolishDownload(modelId);
    } catch (err) {
      console.error("Failed to cancel polish download:", err);
    }
  };

  const handleWhisperDomainChange = async (domain: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "stt_engine_work_domain", value: domain });
    await updateSetting("stt_engine_work_domain", domain);

    if (domain !== "general") {
      const subs = await settingsCommands.getAvailableSubdomains(domain);
      setAvailableSubdomains(subs);
      if (subs.includes("general")) {
        await updateSetting("stt_engine_work_subdomain", "general");
      }
    } else {
      setAvailableSubdomains([]);
      await updateSetting("stt_engine_work_subdomain", "general");
    }
  };

  const handleWhisperSubdomainChange = async (subdomain: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "stt_engine_work_subdomain", value: subdomain });
    await updateSetting("stt_engine_work_subdomain", subdomain);
  };

  const handleWhisperGlossaryChange = async (value: string) => {
    await updateSetting("stt_engine_user_glossary", value);
  };

  const handlePolishDelete = async (modelId: string) => {
    const confirmed = await confirm({
      title: "Delete Polish Model",
      description: `Are you sure you want to delete this polish model? This action cannot be undone.`,
      confirmText: "Delete",
      cancelText: "Cancel",
      variant: "danger",
    });
    if (!confirmed) return;

    try {
      await modelCommands.deletePolishModelById(modelId);
      setPolishModels((prev) =>
        prev.map((m) => (m.id === modelId ? { ...m, downloaded: false } : m)),
      );
      if (selectedPolishModel === modelId) {
        setSelectedPolishModel("");
      }
    } catch (err) {
      console.error("Failed to delete polish model:", err);
    }
  };

  if (!settings) return null;

  const downloadedModels = models.filter((m) => m.downloaded);
  const selectedModel = models.find((m) => m.name === settings.model);
  const downloadedPolishModels = polishModels.filter((m) => m.downloaded);

  return (
    <SettingsPageLayout
      title={t("model.title")}
      description={t("model.description")}
    >
      <div className="space-y-4">
        <Card className="border-dashed bg-primary/5">
          <CardContent className="pt-6">
            <div className="flex items-start gap-3">
              <div className="rounded-lg bg-primary/20 p-2 shrink-0">
                <Cloud className="h-5 w-5 text-primary" />
              </div>
              <div>
                <h3 className="text-sm font-semibold">
                  {t("model.cloud.title")}
                </h3>
                <p className="text-xs text-muted-foreground mt-1">
                  {t("model.cloud.description")}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

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

            <div className="space-y-2">
              <Label>{t("model.active.language")}</Label>
              <Select
                value={settings.stt_engine_language ?? "auto"}
                onChange={(e) => handleWhisperLanguageChange(e.target.value)}
                options={WHISPER_LANGUAGES.map((lang) => ({
                  value: lang.code,
                  label: lang.label,
                }))}
              />
              <p className="text-xs text-muted-foreground">
                {t("model.active.languageDesc")}
              </p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("model.domain.title")}</CardTitle>
            <CardDescription>{t("model.domain.description")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label>{t("model.domain.domain")}</Label>
              <Select
                value={settings.stt_engine_work_domain ?? "general"}
                onChange={(e) => handleWhisperDomainChange(e.target.value)}
                options={[
                  { value: "general", label: t("model.domain.domain_general") },
                  { value: "it", label: t("model.domain.domain_it") },
                  { value: "legal", label: t("model.domain.domain_legal") },
                  { value: "medical", label: t("model.domain.domain_medical") },
                ]}
              />
            </div>

            {settings.stt_engine_work_domain !== "general" && availableSubdomains.length > 0 && (
              <div className="space-y-2">
                <Label>{t("model.domain.subdomain")}</Label>
                <Select
                  value={settings.stt_engine_work_subdomain ?? "general"}
                  onChange={(e) => handleWhisperSubdomainChange(e.target.value)}
                  options={availableSubdomains.map((sub) => ({
                    value: sub,
                    label: t(
                      `model.domain.subdomain_${sub}`,
                      sub.charAt(0).toUpperCase() + sub.slice(1),
                    ),
                  }))}
                />
              </div>
            )}

            <div className="space-y-2">
              <Label>{t("model.domain.glossary")}</Label>
              <textarea
                className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 scrollbar-overlay"
                value={settings.stt_engine_user_glossary ?? ""}
                onChange={(e) => handleWhisperGlossaryChange(e.target.value)}
                placeholder={t("model.domain.glossaryPlaceholder")}
              />
              <p className="text-xs text-muted-foreground">
                {t("model.domain.glossaryDesc")}
              </p>
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
                  className="flex items-center justify-between space-x-4 p-3 rounded-lg border border-border"
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
                      {m.accuracy_score}/10
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
                        onClick={() => handleDelete(m.name)}
                        disabled={isDownloading}
                      >
                        {t("model.available.delete")}
                      </Button>
                    ) : isDownloading ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-24"
                        onClick={() => handleCancel(m.name)}
                      >
                        {t("model.available.cancel")}
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        className="w-24"
                        onClick={() => handleDownload(m.name)}
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

      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle>{t("model.performance.title")}</CardTitle>
            <CardDescription>
              {t("model.performance.description")}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between space-x-4">
              <div>
                <Label htmlFor="gpu">
                  {t("model.performance.gpuAcceleration")}
                </Label>
                <p className="text-xs text-muted-foreground">
                  {t("model.performance.gpuDesc")}
                </p>
              </div>
              <Switch
                id="gpu"
                checked={settings.gpu_acceleration}
                onCheckedChange={handleGpuChange}
              />
            </div>
            <div className="flex items-center justify-between space-x-4 pt-4">
              <div>
                <Label htmlFor="model-resident">
                  {t("model.performance.modelResident")}
                </Label>
                <p className="text-xs text-muted-foreground">
                  {t("model.performance.modelResidentDesc")}
                </p>
              </div>
              <Switch
                id="model-resident"
                checked={settings.model_resident}
                onCheckedChange={handleModelResidentChange}
              />
            </div>
            {settings.model_resident && (
              <div className="flex items-center justify-between space-x-4 pt-4">
                <div>
                  <Label htmlFor="idle-unload">
                    {t("model.performance.idleUnload")}
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    {t("model.performance.idleUnloadDesc")}
                  </p>
                </div>
                <Select
                  value={String(settings.idle_unload_minutes ?? 5)}
                  onChange={(e) =>
                    handleIdleUnloadChange(Number(e.target.value))
                  }
                  options={[
                    { value: "1", label: "1 min" },
                    { value: "3", label: "3 min" },
                    { value: "5", label: "5 min" },
                    { value: "10", label: "10 min" },
                    { value: "30", label: "30 min" },
                  ]}
                />
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle>{t("model.polish.title")}</CardTitle>
            <CardDescription>{t("model.polish.description")}</CardDescription>
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
                  label: `${m.name} · ${m.size}`,
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
            </div>

            <div className="space-y-3">
              {polishModels.map((m) => {
                const isDownloading = polishDownloadingId === m.id;
                return (
                  <div
                    key={m.id}
                    className="flex items-center justify-between space-x-4 p-3 rounded-lg border border-border"
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
                          onClick={() => handlePolishDelete(m.id)}
                          disabled={isDownloading}
                        >
                          {t("model.available.delete")}
                        </Button>
                      ) : isDownloading ? (
                        <Button
                          variant="outline"
                          size="sm"
                          className="w-24"
                          onClick={() => handlePolishCancel(m.id)}
                        >
                          {t("model.available.cancel")}
                        </Button>
                      ) : (
                        <Button
                          size="sm"
                          className="w-24"
                          onClick={() => handlePolishDownload(m.id)}
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

            <div className="flex items-center justify-between space-x-4">
              <div>
                <Label htmlFor="polish">{t("model.polish.enable")}</Label>
                <p className="text-xs text-muted-foreground">
                  {t("model.polish.enableDesc")}
                </p>
              </div>
              <Switch
                id="polish"
                checked={settings.polish_enabled}
                onCheckedChange={handlePolishToggle}
                disabled={downloadedPolishModels.length === 0}
              />
            </div>

            <div className="space-y-2">
              <Label>{t("model.polish.template")}</Label>
              <Select
                value={polishTemplate}
                onChange={(e) => handlePolishTemplateChange(e.target.value)}
                options={[
                  { value: "filler", label: t("model.polish.templateFiller") },
                  { value: "formal", label: t("model.polish.templateFormal") },
                  { value: "concise", label: t("model.polish.templateConcise") },
                  { value: "agent", label: t("model.polish.templateAgent") },
                  { value: "custom", label: t("model.polish.templateCustom") },
                ]}
              />
              <p className="text-xs text-muted-foreground">
                {polishTemplate === "filler" && t("model.polish.templateFillerDesc")}
                {polishTemplate === "formal" && t("model.polish.templateFormalDesc")}
                {polishTemplate === "concise" && t("model.polish.templateConciseDesc")}
                {polishTemplate === "agent" && t("model.polish.templateAgentDesc")}
                {polishTemplate === "custom" && t("model.polish.templateCustomDesc")}
              </p>
            </div>

            {polishTemplate === "custom" && (
              <div className="space-y-2">
                <Label htmlFor="polish-system-prompt">
                  {t("model.polish.prompt")}
                </Label>
                <textarea
                  id="polish-system-prompt"
                  className="flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 scrollbar-overlay"
                  value={settings.polish_system_prompt}
                  onChange={(e) =>
                    handlePolishSystemPromptChange(e.target.value)
                  }
                  placeholder={t("model.polish.promptPlaceholder")}
                  disabled={downloadedPolishModels.length === 0}
                />
                <p className="text-xs text-muted-foreground">
                  {t("model.polish.promptDesc")}
                </p>
              </div>
            )}
</CardContent>
        </Card>
      </div>
    </SettingsPageLayout>
  );
}
