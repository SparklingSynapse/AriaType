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
import { MultiSwitch } from "@/components/ui/multi-switch";
import { systemCommands, settingsCommands, modelCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useTranslation } from "react-i18next";
import { supportedLanguages } from "@/i18n";
import type { PillIndicatorMode, PresetPosition } from "@/types";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { SettingsPageLayout } from "./SettingsPageLayout";
import langCodes from "@/lib/lang-codes.json";
import { cn } from "@/lib/utils";
import { TemplateSelector } from "./TemplateSelector";
import type { CustomPolishTemplate } from "@/lib/tauri";

function getLanguageLabel(code: string): string {
  return (langCodes as Record<string, string>)[code] || code;
}

export function GeneralSettings() {
  const { t, i18n } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [audioDevices, setAudioDevices] = useState<string[]>(["default"]);
  const [isMacOS, setIsMacOS] = useState(false);
  const [activeTab, setActiveTab] = useState<"general" | "transcription" | "polish">("general");
  const [availableSubdomains, setAvailableSubdomains] = useState<string[]>([]);
  const [polishSelectedTemplate, setPolishSelectedTemplate] = useState<string>("filler");
  const [polishCustomTemplates, setPolishCustomTemplates] = useState<CustomPolishTemplate[]>([]);

  useEffect(() => {
    systemCommands.getAudioDevices().then(setAudioDevices).catch((err: unknown) => logger.error("failed_to_get_audio_devices", { error: String(err) }));
    systemCommands.getPlatform().then((platform) => {
      setIsMacOS(platform === "macos");
    }).catch((err: unknown) => logger.error("failed_to_get_platform", { error: String(err) }));
  }, []);

  useEffect(() => {
    if (!settings) return;
    if (settings.stt_engine_work_domain && settings.stt_engine_work_domain !== "general") {
      settingsCommands.getAvailableSubdomains(settings.stt_engine_work_domain)
        .then(setAvailableSubdomains)
        .catch((err: unknown) => logger.error("failed_to_get_available_subdomains", { error: String(err) }));
    }
  }, [settings?.stt_engine_work_domain]);

  useEffect(() => {
    if (!settings) return;
    setPolishSelectedTemplate(settings.polish_selected_template || "filler");
    setPolishCustomTemplates(settings.polish_custom_templates || []);
  }, [settings?.polish_selected_template, settings?.polish_custom_templates]);

  if (!settings) return null;

  const WHISPER_LANGUAGE_PROMPTS: Record<string, string> = {
    "zh-CN": "This is a Mandarin speech-to-text result. Please output in Simplified Chinese characters. Do not use Traditional Chinese. The speaker is from mainland China.",
    "zh-TW": "This is a Mandarin transcription. Use Traditional Chinese characters. The speaker is from Taiwan. Please output all content in Traditional Chinese.",
  };

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

  const handleStayInTrayChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "stay_in_tray", value: String(checked) });
    await updateSetting("stay_in_tray", checked);
  };

  const handleAudioDeviceChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "audio_device", value });
    await updateSetting("audio_device", value);
  };

  const handleDenoiseModeChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "denoise_mode", value });
    await updateSetting("denoise_mode", value);
  };

  const handleVadChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "vad_enabled", value: String(checked) });
    await updateSetting("vad_enabled", checked);
  };

  const handleSttLanguageChange = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "stt_engine_language", value });
    const prompt = WHISPER_LANGUAGE_PROMPTS[value] ?? "";
    await updateSetting("stt_engine_language", value);
    await updateSetting("stt_engine_initial_prompt", prompt);
  };

  const handleDomainChange = async (domain: string) => {
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

  const handleSubdomainChange = async (subdomain: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "stt_engine_work_subdomain", value: subdomain });
    await updateSetting("stt_engine_work_subdomain", subdomain);
  };

  const handleGlossaryChange = async (value: string) => {
    await updateSetting("stt_engine_user_glossary", value);
  };

  const handlePolishTemplateChange = async (templateId: string) => {
    setPolishSelectedTemplate(templateId);
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "polish_template", value: templateId });
    try {
      await modelCommands.selectPolishTemplate(templateId);
    } catch (err) {
      logger.error("failed_to_select_template", { error: String(err) });
    }
  };

  const handlePolishTemplatesChange = async () => {
    try {
      const templates = await modelCommands.getPolishCustomTemplates();
      setPolishCustomTemplates(templates);
    } catch (err) {
      logger.error("failed_to_get_custom_templates", { error: String(err) });
    }
  };

  return (
    <SettingsPageLayout
      title={t("general.title")}
      description={t("general.description")}
    >
      <div className="inline-flex h-10 items-center justify-center rounded-full bg-secondary p-1 text-muted-foreground">
        <button
          onClick={() => setActiveTab("general")}
          className={cn(
            "inline-flex items-center justify-center whitespace-nowrap rounded-full px-4 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "general"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground"
          )}
        >
          {t("general.tabs.general")}
        </button>
        <button
          onClick={() => setActiveTab("transcription")}
          className={cn(
            "inline-flex items-center justify-center whitespace-nowrap rounded-full px-4 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "transcription"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground"
          )}
        >
          {t("general.tabs.transcription")}
        </button>
        <button
          onClick={() => setActiveTab("polish")}
          className={cn(
            "inline-flex items-center justify-center whitespace-nowrap rounded-full px-4 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "polish"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground"
          )}
        >
          {t("general.tabs.polish")}
        </button>
      </div>

      {activeTab === "general" && (
        <>
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
              {isMacOS && (
                <div className="flex items-center justify-between space-x-4">
                  <div>
                    <Label htmlFor="stay-in-tray">{t("general.startup.stayInTray")}</Label>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {t("general.startup.stayInTrayDesc")}
                    </p>
                  </div>
                  <Switch
                    id="stay-in-tray"
                    checked={settings.stay_in_tray ?? false}
                    onCheckedChange={handleStayInTrayChange}
                  />
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("general.display.title")}</CardTitle>
              <CardDescription>{t("general.display.description")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-4">
                <div className="text-sm font-medium">{t("general.display.themeMode")}</div>
                <MultiSwitch
                  value={settings.theme_mode ?? "system"}
                  onChange={handleThemeModeChange}
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
              <div className="space-y-4">
                <div className="text-sm font-medium">{t("general.display.indicatorMode")}</div>
                <MultiSwitch
                  value={settings.pill_indicator_mode as PillIndicatorMode}
                  onChange={handleIndicatorModeChange}
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
        </>
      )}

      {activeTab === "transcription" && (
        <>
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
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("general.transcription.outputLanguage")}</CardTitle>
              <CardDescription>{t("general.transcription.outputLanguageDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>{t("model.active.language")}</Label>
                <Select
                  value={settings.stt_engine_language ?? "auto"}
                  onChange={(e) => handleSttLanguageChange(e.target.value)}
                  options={Object.keys(langCodes).map((code) => ({
                    value: code,
                    label: getLanguageLabel(code),
                  }))}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("general.audio.processing", "Audio Processing")}</CardTitle>
              <CardDescription>{t("general.audio.processingDesc", "Configure noise reduction and silence trimming to optimize recognition speed and quality.")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-4">
                <div className="text-sm font-medium">{t("general.audio.denoise")}</div>
                <MultiSwitch
                  value={settings.denoise_mode ?? "off"}
                  onChange={handleDenoiseModeChange}
                  options={[
                    { value: "auto", label: t("general.audio.denoiseAuto") },
                    { value: "on", label: t("general.audio.denoiseOn") },
                    { value: "off", label: t("general.audio.denoiseOff") },
                  ]}
                />
              </div>

              <div className="flex items-center justify-between space-x-4">
                <div>
                  <Label htmlFor="vad-toggle">{t("general.audio.vad", "Silence Trimming (VAD)")}</Label>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {t("general.audio.vadDesc", "Automatically trim silence at the start/end and collapse long pauses to speed up transcription. Turn off if you speak very quietly.")}
                  </p>
                </div>
                <Switch
                  id="vad-toggle"
                  checked={settings.vad_enabled ?? false}
                  onCheckedChange={handleVadChange}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("general.transcription.domainTitle")}</CardTitle>
              <CardDescription>{t("general.transcription.domainDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>{t("model.domain.domain")}</Label>
                <Select
                  value={settings.stt_engine_work_domain ?? "general"}
                  onChange={(e) => handleDomainChange(e.target.value)}
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
                    onChange={(e) => handleSubdomainChange(e.target.value)}
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
                  className="flex min-h-[80px] w-full rounded-2xl border border-border bg-background px-4 py-3 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                  value={settings.stt_engine_user_glossary ?? ""}
                  onChange={(e) => handleGlossaryChange(e.target.value)}
                  placeholder={t("model.domain.glossaryPlaceholder")}
                />
                <p className="text-xs text-muted-foreground">
                  {t("model.domain.glossaryDesc")}
                </p>
              </div>
            </CardContent>
          </Card>
        </>
      )}

      {activeTab === "polish" && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>{t("general.polish.templateTitle")}</CardTitle>
              <CardDescription>{t("general.polish.templateDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>{t("model.polish.template")}</Label>
                <TemplateSelector
                  selectedTemplate={polishSelectedTemplate}
                  customTemplates={polishCustomTemplates}
                  onSelect={handlePolishTemplateChange}
                  onTemplatesChange={handlePolishTemplatesChange}
                />
              </div>
              <div className="pt-2">
                <a
                  href="/polish-templates"
                  className="text-sm text-primary hover:underline"
                >
                  {t("polishTemplates.manageTemplates")} →
                </a>
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </SettingsPageLayout>
  );
}