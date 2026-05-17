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
import { WarningCircle, ArrowSquareOut } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { settingsCommands } from "@/lib/tauri";
import type {
  CloudConnectionCheckResult,
  CloudProviderSchemas,
  CloudSttConfig,
} from "@/lib/tauri";
import { CloudConnectionCheckRow } from "./CloudConnectionCheckRow";
import { CloudProviderFieldInput } from "./CloudProviderFieldInput";
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
  const [checkResult, setCheckResult] = useState<CloudConnectionCheckResult | null>(null);
  const [checking, setChecking] = useState(false);

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

  const collectValidationErrors = () => {
    const errors: Record<string, string> = {};
    if (activeSchema) {
      for (const field of activeSchema.fields) {
        if (field.required) {
          const value = configRecord[field.key] ?? "";
          if (!value.trim()) {
            errors[field.key] = t("cloud.validation.fieldRequired", { fieldName: field.name });
          }
        }
      }
    }
    if (configRecord.base_url && !validateUrl(configRecord.base_url)) {
      errors.base_url = t("cloud.validation.invalidUrl", "Invalid URL format");
    }
    return errors;
  };

  const hasValidationErrors = (errors: Record<string, string>) =>
    Object.values(errors).some((error) => error.trim().length > 0);

  const handleFieldChange = async (key: string, value: string) => {
    setCheckResult(null);
    if (key === "base_url") {
      if (!validateUrl(value)) {
        setSttErrors((prev) => ({ ...prev, base_url: t("cloud.validation.invalidUrl", "Invalid URL format") }));
      } else {
        setSttErrors((prev) => ({ ...prev, base_url: "" }));
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
    setCheckResult(null);
    if (enabled && activeSchema) {
      setSttErrors(collectValidationErrors());
    } else {
      setSttErrors({});
    }
    await updateSetting("cloud_stt_enabled", enabled);
  };

  const handleProviderChange = async (newProviderId: string) => {
    await updateSetting("active_cloud_stt_provider", newProviderId);
    setSttErrors({});
    setCheckResult(null);
  };

  const handleCheck = async () => {
    const errors = collectValidationErrors();
    setSttErrors(errors);

    if (hasValidationErrors(errors)) {
      setCheckResult({
        ok: false,
        kind: errors.base_url ? "invalid_url" : "missing_required",
        message: "",
        duration_ms: 0,
      });
      return;
    }

    setChecking(true);
    setCheckResult(null);
    try {
      setCheckResult(await settingsCommands.checkActiveCloudSttConfig());
    } catch (error) {
      setCheckResult({
        ok: false,
        kind: "provider_error",
        message: String(error),
        duration_ms: 0,
      });
    } finally {
      setChecking(false);
    }
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
              <ArrowSquareOut className="w-3 h-3" />
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
                <CloudProviderFieldInput
                  id={`cloud-stt-${field.key}`}
                  secret={field.secret}
                  invalid={Boolean(sttErrors[field.key])}
                  value={configRecord[field.key] ?? ""}
                  onChange={(value) => handleFieldChange(field.key, value)}
                  placeholder={field.example || undefined}
                />
                {sttErrors[field.key] && (
                  <p className="text-[13px] text-destructive flex items-center mt-1">
                    <WarningCircle className="w-3.5 h-3.5 mr-1" />
                    {sttErrors[field.key]}
                  </p>
                )}
              </div>
            ))}

            <CloudConnectionCheckRow
              result={checkResult}
              checking={checking}
              onCheck={handleCheck}
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
