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
import { settingsCommands, CloudSttConfig, CloudProviderSchemas } from "@/lib/tauri";
import sttSvg from "@/assets/illustrations/cloud/stt.png";

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
  const [schemas, setSchemas] = useState<CloudProviderSchemas | null>(null);
  const [sttErrors, setSttErrors] = useState<Record<string, string>>({});

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

  const activeProviderId = settings?.active_cloud_stt_provider ?? "volcengine-streaming";
  const activeSchema = schemas?.stt.find((s) => s.id === activeProviderId);
  const currentConfig = settings?.cloud_stt_configs?.[activeProviderId] ?? getDefaultConfig(activeProviderId);
  const configRecord = currentConfig as unknown as Record<string, string>;
  const isEnabled = settings?.cloud_stt_enabled ?? false;

  const updateProviderConfig = useCallback(async (provider: string, updates: Partial<CloudSttConfig>) => {
    const configs = { ...(settings?.cloud_stt_configs ?? {}) };
    const existingConfig = configs[provider] ?? getDefaultConfig(provider);
    configs[provider] = { ...existingConfig, ...updates, provider_type: provider };
    await updateSetting("cloud_stt_configs", configs);
  }, [settings?.cloud_stt_configs, updateSetting]);

  const handleFieldChange = async (key: string, value: string) => {
    if (key === "base_url") {
      if (!validateUrl(value)) {
        setSttErrors((prev) => ({ ...prev, baseUrl: t("cloud.validation.invalidUrl", "Invalid URL format") }));
      } else {
        setSttErrors((prev) => ({ ...prev, baseUrl: "" }));
      }
    } else if (value && isEnabled) {
      // Required field changed - clear its error if filled
      const field = activeSchema?.fields.find((f) => f.key === key);
      if (field?.required) {
        setSttErrors((prev) => ({ ...prev, [key]: "" }));
      }
    }
    await updateProviderConfig(activeProviderId, { [key]: value });
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
      setSttErrors(errors);
    } else {
      setSttErrors({});
    }
    await updateSetting("cloud_stt_enabled", enabled);
  };

  const handleProviderChange = async (newProviderId: string) => {
    await updateSetting("active_cloud_stt_provider", newProviderId);
    setSttErrors({});
  };

  if (!settings || !schemas) return null;

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
            <a
              href="https://github.com/joe223/AriaType/discussions/3"
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-primary hover:underline flex items-center gap-1"
            >
              {t("cloud.stt.providerGuide", "How to get API credentials")}
              <ExternalLink className="w-3 h-3" />
            </a>

            <div className="space-y-2">
              <Label>{t("model.stt.cloud.provider", "Provider")}</Label>
              <Select
                value={activeProviderId}
                onChange={(e) => handleProviderChange(e.target.value)}
                options={schemas.stt.map((s) => ({
                  value: s.id,
                  label: s.name,
                }))}
              />
            </div>

            {activeSchema?.fields.map((field) => (
              <div key={field.key} className="space-y-2">
                <Label htmlFor={`cloud-stt-${field.key}`} required={field.required}>
                  {field.name}
                </Label>
                <input
                  id={`cloud-stt-${field.key}`}
                  type={field.secret ? "password" : "text"}
                  className={`flex h-10 w-full rounded-2xl border bg-background px-4 py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${sttErrors[field.key] ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive" : "border-border focus-visible:border-primary"}`}
                  value={configRecord[field.key] ?? ""}
                  onChange={(e) => handleFieldChange(field.key, e.target.value)}
                  placeholder={field.example || undefined}
                />
                {sttErrors[field.key] && (
                  <p className="text-[13px] text-destructive flex items-center mt-1">
                    <AlertCircle className="w-3.5 h-3.5 mr-1" />
                    {sttErrors[field.key]}
                  </p>
                )}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
