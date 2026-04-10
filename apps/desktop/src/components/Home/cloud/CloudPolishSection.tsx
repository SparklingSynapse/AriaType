import { useState, useCallback, useEffect } from "react";
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
import { AlertCircle, ExternalLink } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { settingsCommands, CloudProviderConfig, CloudProviderSchemas } from "@/lib/tauri";
import polishSvg from "@/assets/illustrations/cloud/polish.png";

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
  const [schemas, setSchemas] = useState<CloudProviderSchemas | null>(null);
  const [polishErrors, setPolishErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    settingsCommands.getCloudProviderSchemas().then(setSchemas).catch(console.error);
  }, []);

  const validateUrl = (url: string) => {
    if (!url) return true;
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const activeProviderId = settings?.active_cloud_polish_provider ?? "anthropic";
  const activeSchema = schemas?.polish.find((s) => s.id === activeProviderId);
  const currentConfig = settings?.cloud_polish_configs?.[activeProviderId] ?? getDefaultConfig(activeProviderId);
  const configRecord = currentConfig as unknown as Record<string, string>;
  const isEnabled = settings?.cloud_polish_enabled ?? false;

  const updateProviderConfig = useCallback(async (provider: string, updates: Partial<CloudProviderConfig>) => {
    const configs = { ...(settings?.cloud_polish_configs ?? {}) };
    const existingConfig = configs[provider] ?? getDefaultConfig(provider);
    configs[provider] = { ...existingConfig, ...updates, provider_type: provider };
    await updateSetting("cloud_polish_configs", configs);
  }, [settings?.cloud_polish_configs, updateSetting]);

  const handleFieldChange = async (key: string, value: string) => {
    if (key === "base_url") {
      if (!validateUrl(value)) {
        setPolishErrors((prev) => ({ ...prev, baseUrl: t("cloud.validation.invalidUrl", "Invalid URL format") }));
      } else {
        setPolishErrors((prev) => ({ ...prev, baseUrl: "" }));
      }
    } else if (value && isEnabled) {
      const field = activeSchema?.fields.find((f) => f.key === key);
      if (field?.required) {
        setPolishErrors((prev) => ({ ...prev, [key]: "" }));
      }
    }
    await updateProviderConfig(activeProviderId, { [key]: value });
  };

  const handleSwitchChange = async (key: string, checked: boolean) => {
    await updateProviderConfig(activeProviderId, { [key]: checked });
  };

  const handleEnabledChange = async (enabled: boolean) => {
    if (enabled && activeSchema) {
      const errors: Record<string, string> = {};
      for (const field of activeSchema.fields) {
        if (field.required) {
          const value = configRecord[field.key] ?? "";
          if (!value.trim()) {
            errors[field.key] = t("cloud.validation.fieldRequired", `${field.name} is required`);
          }
        }
      }
      if (configRecord.base_url && !validateUrl(configRecord.base_url)) {
        errors.baseUrl = t("cloud.validation.invalidUrl", "Invalid URL format");
      }
      setPolishErrors(errors);
    } else {
      setPolishErrors({});
    }
    await updateSetting("cloud_polish_enabled", enabled);
  };

  const handleProviderChange = async (newProviderId: string) => {
    await updateSetting("active_cloud_polish_provider", newProviderId);
    setPolishErrors({});
  };

  if (!settings || !schemas) return null;

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
            <a
              href="https://github.com/joe223/AriaType/discussions/4"
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-primary hover:underline flex items-center gap-1"
            >
              {t("cloud.polish.modelGuide", "Mainstream model information")}
              <ExternalLink className="w-3 h-3" />
            </a>

            <div className="space-y-2">
              <Label>{t("model.polish.cloud.provider")}</Label>
              <Select
                value={activeProviderId}
                onChange={(e) => handleProviderChange(e.target.value)}
                options={schemas.polish.map((s) => ({
                  value: s.id,
                  label: s.name,
                }))}
              />
              <p className="text-xs text-muted-foreground">
                {t("cloud.polish.providerHint", "Supports any subscription service compatible with OpenAI or Anthropic API format.")}
              </p>
            </div>

            {activeSchema?.fields.map((field) => (
              <div key={field.key} className="space-y-2">
                <Label htmlFor={`cloud-polish-${field.key}`} required={field.required}>
                  {field.name}
                </Label>
                <input
                  id={`cloud-polish-${field.key}`}
                  type={field.secret ? "password" : "text"}
                  className={`flex h-10 w-full rounded-2xl border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${polishErrors[field.key] ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive" : "border-border focus-visible:border-primary"}`}
                  value={configRecord[field.key] ?? ""}
                  onChange={(e) => handleFieldChange(field.key, e.target.value)}
                  placeholder={field.example || undefined}
                />
                {polishErrors[field.key] && (
                  <p className="text-[13px] text-destructive flex items-center mt-1">
                    <AlertCircle className="w-3.5 h-3.5 mr-1" />
                    {polishErrors[field.key]}
                  </p>
                )}
              </div>
            ))}

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
                onCheckedChange={(checked) => handleSwitchChange("enable_thinking", checked)}
              />
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
