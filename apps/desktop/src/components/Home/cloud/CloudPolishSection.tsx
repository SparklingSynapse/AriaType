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
import { CloudProviderConfig } from "@/lib/tauri";
import polishSvg from "@/assets/illustrations/cloud/polish.png";

type CloudPolishProvider = "anthropic" | "openai" | "custom";

const POLISH_PROVIDERS: { value: CloudPolishProvider; labelKey: string }[] = [
  { value: "anthropic", labelKey: "model.polish.cloud.providerAnthropic" },
  { value: "openai", labelKey: "model.polish.cloud.providerOpenAI" },
  { value: "custom", labelKey: "model.polish.cloud.providerCustom" },
];

function getDefaultConfig(provider: string): CloudProviderConfig {
  return {
    enabled: false,
    provider_type: provider,
    api_key: "",
    base_url: "",
    model: "",
    enable_thinking: false,
  };
}

export function CloudPolishSection() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [polishErrors, setPolishErrors] = useState<{ apiKey?: string; baseUrl?: string }>({});

  const validateUrl = (url: string) => {
    if (!url) return true;
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const activeProvider = (settings?.active_cloud_polish_provider ?? "anthropic") as CloudPolishProvider;
  const currentConfig = settings?.cloud_polish_configs?.[activeProvider] ?? getDefaultConfig(activeProvider);
  const isEnabled = settings?.cloud_polish_enabled ?? false;

  const updateProviderConfig = useCallback(async (provider: string, updates: Partial<CloudProviderConfig>) => {
    const configs = { ...(settings?.cloud_polish_configs ?? {}) };
    const existingConfig = configs[provider] ?? getDefaultConfig(provider);
    configs[provider] = { ...existingConfig, ...updates, provider_type: provider };
    await updateSetting("cloud_polish_configs", configs);
  }, [settings?.cloud_polish_configs, updateSetting]);

  const handleFieldChange = async (key: keyof CloudProviderConfig, value: string | boolean) => {
    if (key === "base_url" && typeof value === "string") {
      if (!validateUrl(value)) {
        setPolishErrors((prev) => ({ ...prev, baseUrl: t("cloud.validation.invalidUrl", "Invalid URL format") }));
      } else {
        setPolishErrors((prev) => ({ ...prev, baseUrl: undefined }));
      }
    }

    if (key === "api_key" && typeof value === "string") {
      if (isEnabled && !value.trim()) {
        setPolishErrors((prev) => ({ ...prev, apiKey: t("cloud.validation.apiKeyRequired", "API Key is required when enabled") }));
      } else {
        setPolishErrors((prev) => ({ ...prev, apiKey: undefined }));
      }
    }

    await updateProviderConfig(activeProvider, { [key]: value });
  };

  const handleEnabledChange = async (enabled: boolean) => {
    if (enabled) {
      const config = currentConfig;
      if (!config.api_key?.trim()) {
        setPolishErrors((prev) => ({ ...prev, apiKey: t("cloud.validation.apiKeyRequired", "API Key is required when enabled") }));
      }
      if (config.base_url && !validateUrl(config.base_url)) {
        setPolishErrors((prev) => ({ ...prev, baseUrl: t("cloud.validation.invalidUrl", "Invalid URL format") }));
      }
    } else {
      setPolishErrors({});
    }
    await updateSetting("cloud_polish_enabled", enabled);
  };

  const handleProviderChange = async (newProvider: string) => {
    await updateSetting("active_cloud_polish_provider", newProvider);
    setPolishErrors({});
  };

  if (!settings) return null;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-4">
          <img src={polishSvg} alt="Polish" className="w-32 h-auto drop-shadow-sm" />
          <div>
            <CardTitle className="text-base">{t("cloud.polish.title")}</CardTitle>
            <CardDescription className="text-sm">{t("cloud.polish.description")}</CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between space-x-4">
          <div>
            <Label htmlFor="cloud-polish">{t("model.polish.cloud.enable")}</Label>
            <p className="text-xs text-muted-foreground">
              {t("model.polish.cloud.enableDesc")}
            </p>
          </div>
          <Switch
            id="cloud-polish"
            checked={isEnabled}
            onCheckedChange={handleEnabledChange}
          />
        </div>

        {isEnabled && (
          <div className="space-y-4 pt-4 border-t border-border">
            <div className="space-y-2">
              <Label>{t("model.polish.cloud.provider")}</Label>
              <Select
                value={activeProvider}
                onChange={(e) => handleProviderChange(e.target.value)}
                options={POLISH_PROVIDERS.map((p) => ({
                  value: p.value,
                  label: t(p.labelKey, p.value),
                }))}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="cloud-api-key">{t("model.polish.cloud.apiKey")}</Label>
              <input
                id="cloud-api-key"
                type="password"
                className={`flex h-10 w-full rounded-2xl border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${polishErrors.apiKey ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive" : "border-border focus-visible:border-primary"}`}
                value={currentConfig.api_key ?? ""}
                onChange={(e) => handleFieldChange("api_key", e.target.value)}
                placeholder={t("model.polish.cloud.apiKeyPlaceholder")}
              />
              {polishErrors.apiKey && (
                <p className="text-[13px] text-destructive flex items-center mt-1">
                  <AlertCircle className="w-3.5 h-3.5 mr-1" />
                  {polishErrors.apiKey}
                </p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="cloud-base-url">{t("model.polish.cloud.baseUrl")}</Label>
              <input
                id="cloud-base-url"
                type="text"
                className={`flex h-10 w-full rounded-2xl border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${polishErrors.baseUrl ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive" : "border-border focus-visible:border-primary"}`}
                value={currentConfig.base_url ?? ""}
                onChange={(e) => handleFieldChange("base_url", e.target.value)}
                placeholder={t("model.polish.cloud.baseUrlPlaceholder")}
              />
              {polishErrors.baseUrl ? (
                <p className="text-[13px] text-destructive flex items-center mt-1">
                  <AlertCircle className="w-3.5 h-3.5 mr-1" />
                  {polishErrors.baseUrl}
                </p>
              ) : (
                <p className="text-xs text-muted-foreground">
                  {t("model.polish.cloud.baseUrlDesc")}
                </p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="cloud-model">{t("model.polish.cloud.model")}</Label>
              <input
                id="cloud-model"
                type="text"
                className="flex h-10 w-full rounded-2xl border border-border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                value={currentConfig.model ?? ""}
                onChange={(e) => handleFieldChange("model", e.target.value)}
                placeholder={t("model.polish.cloud.modelPlaceholder")}
              />
            </div>

            <div className="flex items-center justify-between space-x-4 pt-4 border-t border-border">
              <div>
                <Label htmlFor="cloud-thinking">{t("model.polish.cloud.enableThinking")}</Label>
                <p className="text-xs text-muted-foreground">
                  {t("model.polish.cloud.enableThinkingDesc")}
                </p>
              </div>
              <Switch
                id="cloud-thinking"
                checked={currentConfig.enable_thinking ?? false}
                onCheckedChange={(checked) => handleFieldChange("enable_thinking", checked)}
              />
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}