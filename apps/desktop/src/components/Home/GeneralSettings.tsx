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
import { Switch } from "@/components/ui/switch";
import { systemCommands } from "@/lib/tauri";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useTranslation } from "react-i18next";
import { supportedLanguages } from "@/i18n";
import type { PillIndicatorMode, PresetPosition } from "@/types";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { SettingsPageLayout } from "./SettingsPageLayout";

export function GeneralSettings() {
  const { t, i18n } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [audioDevices, setAudioDevices] = useState<string[]>(["default"]);

  useEffect(() => {
    systemCommands.getAudioDevices().then(setAudioDevices).catch(console.error);
  }, []);

  if (!settings) return null;

  const handleAutoStartChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "auto_start", value: String(checked) });
    await updateSetting("auto_start", checked);
  };

  const handleBeepOnRecordChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "beep_on_record", value: String(checked) });
    await updateSetting("beep_on_record", checked);
  };

  const handlePositionChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "pill_position", value });
    await updateSetting("pill_position", value);
  };

  const handleIndicatorModeChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "pill_indicator_mode", value });
    await updateSetting("pill_indicator_mode", value);
  };

  const handleAppLanguageChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "language", value });
    if (value === "auto") {
      i18n.changeLanguage(navigator.language);
      localStorage.removeItem("app_language");
    } else {
      i18n.changeLanguage(value);
      localStorage.setItem("app_language", value);
    }
    await updateSetting("language", value);
  };

  const handleAudioDeviceChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "audio_device", value });
    await updateSetting("audio_device", value);
  };

  const handleDenoiseModeChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "denoise_mode", value });
    await updateSetting("denoise_mode", value);
  };

  const handleThemeModeChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "theme_mode", value });
    await updateSetting("theme_mode", value);
  };

  const handleAnalyticsChange = async (checked: boolean) => {
    await updateSetting("analytics_opt_in", checked);
    if (checked) {
      analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "analytics_enabled", value: "true" });
    }
  };

  return (
    <SettingsPageLayout
      title={t("general.title")}
      description={t("general.description")}
    >
      <Card>
        <CardHeader>
          <CardTitle>{t("general.language.title")}</CardTitle>
          <CardDescription>{t("general.language.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>{t("general.language.select")}</Label>
            <Select
              value={settings.language ?? "auto"}
              onChange={(e) => handleAppLanguageChange(e.target.value)}
              options={[
                { value: "auto", label: t("general.language.auto") },
                ...supportedLanguages.map((lang) => ({
                  value: lang.code,
                  label: `${lang.nativeName} (${lang.name})`,
                })),
              ]}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("general.startup.title")}</CardTitle>
          <CardDescription>{t("general.startup.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between space-x-4">
            <div>
              <Label htmlFor="auto-start">{t("general.startup.autoStart")}</Label>
              <p className="text-xs text-muted-foreground mt-0.5">
                {t("general.startup.autoStartDesc")}
              </p>
            </div>
            <Switch
              id="auto-start"
              checked={settings.auto_start}
              onCheckedChange={handleAutoStartChange}
            />
          </div>
          <div className="flex items-center justify-between space-x-4">
            <div>
              <Label htmlFor="beep-on-record">{t("general.startup.recordingSound")}</Label>
              <p className="text-xs text-muted-foreground mt-0.5">
                {t("general.startup.recordingSoundDesc")}
              </p>
            </div>
            <Switch
              id="beep-on-record"
              checked={settings.beep_on_record}
              onCheckedChange={handleBeepOnRecordChange}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("general.audio.title")}</CardTitle>
          <CardDescription>{t("general.audio.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>{t("general.audio.inputDevice")}</Label>
            <Select
              value={settings.audio_device ?? "default"}
              onChange={(e) => handleAudioDeviceChange(e.target.value)}
              options={audioDevices.map((d) => ({
                value: d,
                label: d === "default" ? t("general.audio.systemDefault") : d,
              }))}
            />
          </div>
          <div className="space-y-2">
            <Label>{t("general.audio.denoise")}</Label>
            <Select
              value={settings.denoise_mode ?? "auto"}
              onChange={(e) => handleDenoiseModeChange(e.target.value)}
              options={[
                { value: "auto", label: t("general.audio.denoiseAuto") },
                { value: "on", label: t("general.audio.denoiseOn") },
                { value: "off", label: t("general.audio.denoiseOff") },
              ]}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("general.display.title")}</CardTitle>
          <CardDescription>{t("general.display.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>{t("general.display.themeMode")}</Label>
            <Select
              value={settings.theme_mode ?? "system"}
              onChange={(e) => handleThemeModeChange(e.target.value)}
              options={[
                { value: "system", label: t("general.display.themeSystem") },
                { value: "light", label: t("general.display.themeLight") },
                { value: "dark", label: t("general.display.themeDark") },
              ]}
            />
          </div>
          <div className="space-y-2">
            <Label>{t("general.pill.title")}</Label>
            <Select
              value={settings.pill_position as PresetPosition}
              onChange={(e) => handlePositionChange(e.target.value)}
              options={[
                { value: "top-left", label: t("general.pill.topLeft") },
                { value: "top-center", label: t("general.pill.topCenter") },
                { value: "top-right", label: t("general.pill.topRight") },
                { value: "bottom-left", label: t("general.pill.bottomLeft") },
                { value: "bottom-center", label: t("general.pill.bottomCenter") },
                { value: "bottom-right", label: t("general.pill.bottomRight") },
              ]}
            />
          </div>
          <div className="space-y-2">
            <Label>{t("general.display.indicatorMode")}</Label>
            <Select
              value={settings.pill_indicator_mode as PillIndicatorMode}
              onChange={(e) => handleIndicatorModeChange(e.target.value)}
              options={[
                { value: "always", label: t("general.display.alwaysShow") },
                { value: "when_recording", label: t("general.display.showWhenRecording") },
                { value: "never", label: t("general.display.neverShow") },
              ]}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("general.privacy.title")}</CardTitle>
          <CardDescription>{t("general.privacy.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between space-x-4">
            <div>
              <Label htmlFor="analytics-toggle">
                {t("general.privacy.analytics")}
              </Label>
              <p className="text-xs text-muted-foreground mt-0.5">
                {t("general.privacy.analyticsDesc")}
              </p>
            </div>
            <Switch
              id="analytics-toggle"
              checked={settings.analytics_opt_in}
              onCheckedChange={handleAnalyticsChange}
            />
          </div>
        </CardContent>
      </Card>
    </SettingsPageLayout>
  );
}
