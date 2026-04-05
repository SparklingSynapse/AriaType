import { useState, useCallback } from "react";
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
import { AlertCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { CloudSttConfig } from "@/lib/tauri";
import sttSvg from "@/assets/illustrations/cloud/stt.png";

type CloudSttProvider = "volcengine-streaming" | "qwen-omni-realtime" | "elevenlabs";

const STT_PROVIDERS: { value: CloudSttProvider; labelKey: string; descKey: string }[] = [
  { value: "volcengine-streaming", labelKey: "cloud.stt.provider.volcengineStreaming", descKey: "cloud.stt.provider.volcengineStreamingDesc" },
  { value: "qwen-omni-realtime", labelKey: "cloud.stt.provider.qwenOmniRealtime", descKey: "cloud.stt.provider.qwenOmniRealtimeDesc" },
  { value: "elevenlabs", labelKey: "cloud.stt.provider.elevenlabs", descKey: "cloud.stt.provider.elevenlabsDesc" },
];

function isVolcengineProvider(provider: string): boolean {
  return provider === "volcengine-streaming";
}

function getApiKeyLabel(provider: CloudSttProvider, t: (key: string, fallback: string) => string): string {
  if (isVolcengineProvider(provider)) {
    return t("model.stt.cloud.accessToken", "Access Token");
  }
  return t("model.stt.cloud.apiKey", "API Key");
}

function getModelPlaceholder(provider: CloudSttProvider): string {
  switch (provider) {
    case "volcengine-streaming":
      return "volc.bigasr.sauc.duration";
    case "qwen-omni-realtime":
      return "qwen3-asr-flash-realtime";
    case "elevenlabs":
      return "scribe_v2_realtime";
    default:
      return "model-name";
  }
}

function getBaseUrlPlaceholder(provider: CloudSttProvider): string {
  switch (provider) {
    case "volcengine-streaming":
      return "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream";
    case "qwen-omni-realtime":
      return "wss://dashscope.aliyuncs.com/api-ws/v1/realtime";
    case "elevenlabs":
      return "wss://api.elevenlabs.io/v1/speech-to-text/realtime";
    default:
      return "https://api.example.com/v1/transcribe";
  }
}

function getDefaultConfig(provider: string): CloudSttConfig {
  return {
    enabled: false,
    provider_type: provider,
    api_key: "",
    app_id: "",
    base_url: "",
    model: "",
    language: "",
  };
}

export function CloudSttSection() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [sttErrors, setSttErrors] = useState<{ apiKey?: string; baseUrl?: string; appId?: string }>({});

  const validateUrl = (url: string) => {
    if (!url) return true;
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const activeProvider = (settings?.active_cloud_stt_provider ?? "volcengine-streaming") as CloudSttProvider;
  const currentConfig = settings?.cloud_stt_configs?.[activeProvider] ?? getDefaultConfig(activeProvider);
  const isEnabled = settings?.cloud_stt_enabled ?? false;

  const updateProviderConfig = useCallback(async (provider: string, updates: Partial<CloudSttConfig>) => {
    const configs = { ...(settings?.cloud_stt_configs ?? {}) };
    const existingConfig = configs[provider] ?? getDefaultConfig(provider);
    configs[provider] = { ...existingConfig, ...updates, provider_type: provider };
    await updateSetting("cloud_stt_configs", configs);
  }, [settings?.cloud_stt_configs, updateSetting]);

  const handleFieldChange = async (key: keyof CloudSttConfig, value: string | boolean) => {
    if (key === "base_url" && typeof value === "string") {
      if (!validateUrl(value)) {
        setSttErrors((prev) => ({ ...prev, baseUrl: t("cloud.validation.invalidUrl", "Invalid URL format") }));
      } else {
        setSttErrors((prev) => ({ ...prev, baseUrl: undefined }));
      }
    }

    if (key === "api_key" && typeof value === "string") {
      if (isEnabled && !value.trim()) {
        setSttErrors((prev) => ({ ...prev, apiKey: t("cloud.validation.apiKeyRequired", "API Key is required when enabled") }));
      } else {
        setSttErrors((prev) => ({ ...prev, apiKey: undefined }));
      }
    }

    if (key === "app_id" && typeof value === "string") {
      if (isEnabled && isVolcengineProvider(activeProvider) && !value.trim()) {
        setSttErrors((prev) => ({ ...prev, appId: t("cloud.validation.appIdRequired", "App ID is required for Volcengine") }));
      } else {
        setSttErrors((prev) => ({ ...prev, appId: undefined }));
      }
    }

    await updateProviderConfig(activeProvider, { [key]: value });
  };

  const handleEnabledChange = async (enabled: boolean) => {
    if (enabled) {
      const config = currentConfig;
      if (!config.api_key?.trim()) {
        setSttErrors((prev) => ({ ...prev, apiKey: t("cloud.validation.apiKeyRequired", "API Key is required when enabled") }));
      }
      if (isVolcengineProvider(activeProvider) && !config.app_id?.trim()) {
        setSttErrors((prev) => ({ ...prev, appId: t("cloud.validation.appIdRequired", "App ID is required for Volcengine") }));
      }
      if (config.base_url && !validateUrl(config.base_url)) {
        setSttErrors((prev) => ({ ...prev, baseUrl: t("cloud.validation.invalidUrl", "Invalid URL format") }));
      }
    } else {
      setSttErrors({});
    }
    await updateSetting("cloud_stt_enabled", enabled);
  };

  const handleProviderChange = async (newProvider: string) => {
    await updateSetting("active_cloud_stt_provider", newProvider);
    setSttErrors({});
  };

  if (!settings) return null;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-4">
          <img src={sttSvg} alt="STT" className="w-32 h-auto drop-shadow-sm" />
          <div>
            <CardTitle className="text-base">{t("cloud.stt.title", "Cloud STT")}</CardTitle>
            <CardDescription className="text-sm">{t("cloud.stt.description", "Use cloud-based STT engines for transcription")}</CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between space-x-4">
          <div>
            <Label htmlFor="cloud-stt">{t("model.stt.cloud.enable", "Enable Cloud STT")}</Label>
            <p className="text-xs text-muted-foreground">
              {t("model.stt.cloud.enableDesc", "Use cloud API for transcription instead of local models.")}
            </p>
          </div>
          <Switch
            id="cloud-stt"
            checked={isEnabled}
            onCheckedChange={handleEnabledChange}
          />
        </div>

        {isEnabled && (
          <div className="space-y-4 pt-4 border-t border-border">
            <div className="space-y-2">
              <Label>{t("model.stt.cloud.provider", "Provider")}</Label>
              <Select
                value={activeProvider}
                onChange={(e) => handleProviderChange(e.target.value)}
                options={STT_PROVIDERS.map((p) => ({
                  value: p.value,
                  label: t(p.labelKey, p.value),
                }))}
              />
              <p className="text-xs text-muted-foreground">
                {t(STT_PROVIDERS.find((p) => p.value === activeProvider)?.descKey ?? "", "")}
              </p>
            </div>

            {isVolcengineProvider(activeProvider) && (
              <div className="space-y-2">
                <Label htmlFor="cloud-stt-app-id">{t("model.stt.cloud.appId", "App ID")}</Label>
                <input
                  id="cloud-stt-app-id"
                  type="text"
                  className={`flex h-10 w-full rounded-2xl border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${sttErrors.appId ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive" : "border-border focus-visible:border-primary"}`}
                  value={currentConfig.app_id ?? ""}
                  onChange={(e) => handleFieldChange("app_id", e.target.value)}
                  placeholder="123456789"
                />
                {sttErrors.appId && (
                  <p className="text-[13px] text-destructive flex items-center mt-1">
                    <AlertCircle className="w-3.5 h-3.5 mr-1" />
                    {sttErrors.appId}
                  </p>
                )}
              </div>
            )}

            <div className="space-y-2">
              <Label htmlFor="cloud-stt-api-key">{getApiKeyLabel(activeProvider, t)}</Label>
              <input
                id="cloud-stt-api-key"
                type="password"
                className={`flex h-10 w-full rounded-2xl border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${sttErrors.apiKey ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive" : "border-border focus-visible:border-primary"}`}
                value={currentConfig.api_key ?? ""}
                onChange={(e) => handleFieldChange("api_key", e.target.value)}
                placeholder="sk-..."
              />
              {sttErrors.apiKey && (
                <p className="text-[13px] text-destructive flex items-center mt-1">
                  <AlertCircle className="w-3.5 h-3.5 mr-1" />
                  {sttErrors.apiKey}
                </p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="cloud-stt-base-url">{t("model.stt.cloud.baseUrl", "Base URL")}</Label>
              <input
                id="cloud-stt-base-url"
                type="text"
                className={`flex h-10 w-full rounded-2xl border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${sttErrors.baseUrl ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive" : "border-border focus-visible:border-primary"}`}
                value={currentConfig.base_url ?? ""}
                onChange={(e) => handleFieldChange("base_url", e.target.value)}
                placeholder={getBaseUrlPlaceholder(activeProvider)}
              />
              {sttErrors.baseUrl && (
                <p className="text-[13px] text-destructive flex items-center mt-1">
                  <AlertCircle className="w-3.5 h-3.5 mr-1" />
                  {sttErrors.baseUrl}
                </p>
              )}
            </div>

            {isVolcengineProvider(activeProvider) && (
              <div className="space-y-2">
                <Label htmlFor="cloud-stt-model">
                  {t("model.stt.cloud.resourceId", "Resource ID")}
                </Label>
                <input
                  id="cloud-stt-model"
                  type="text"
                  className="flex h-10 w-full rounded-2xl border border-border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                  value={currentConfig.model ?? ""}
                  onChange={(e) => handleFieldChange("model", e.target.value)}
                  placeholder={getModelPlaceholder(activeProvider)}
                />
                <p className="text-xs text-muted-foreground">
                  {t("model.stt.cloud.resourceIdDesc", "火山引擎资源ID，如: volc.bigasr.sauc.duration")}
                </p>
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
