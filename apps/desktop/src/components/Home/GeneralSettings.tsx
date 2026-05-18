import { useEffect, useRef, useState } from "react";
import { getIdentifier } from "@tauri-apps/api/app";
import {
  autoUpdate,
  flip,
  FloatingFocusManager,
  FloatingPortal,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
  useRole,
} from "@floating-ui/react";
import { HexColorInput, HexColorPicker } from "react-colorful";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { MultiSwitch } from "@/components/ui/multi-switch";
import { systemCommands, settingsCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { showErrorToast } from "@/lib/toast";
import { useTranslation } from "react-i18next";
import { supportedLanguages } from "@/i18n";
import type { PillIndicatorMode, PresetPosition } from "@/types";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { SettingsPageLayout } from "./SettingsPageLayout";
import langCodes from "@/lib/lang-codes.json";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Slider } from "@/components/ui/slider";

function getLanguageLabel(code: string): string {
  return (langCodes as Record<string, string>)[code] || code;
}

const DEFAULT_PILL_BACKGROUND_COLOR = "#1d1d1d";
const DEFAULT_PILL_BACKGROUND_OPACITY = 1;

const PILL_BACKGROUND_COLOR_PRESETS = [
  { value: "#1d1d1d", label: "dark" },
  { value: "#26324a", label: "slate" },
  { value: "#2f3a32", label: "forest" },
  { value: "#472b39", label: "plum" },
  { value: "#4a3728", label: "copper" },
] as const;

type PillBackgroundColorLabel = (typeof PILL_BACKGROUND_COLOR_PRESETS)[number]["label"];

function normalizePillBackgroundColor(color: string | undefined): string {
  if (!color || !/^#[0-9a-f]{6}$/i.test(color)) {
    return DEFAULT_PILL_BACKGROUND_COLOR;
  }

  return color.toLowerCase();
}

function normalizePillBackgroundOpacity(opacity: number | undefined): number {
  if (typeof opacity !== "number" || !Number.isFinite(opacity)) {
    return DEFAULT_PILL_BACKGROUND_OPACITY;
  }

  return Math.min(1, Math.max(0.2, opacity));
}

export function GeneralSettings() {
  const { t, i18n } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [audioDevices, setAudioDevices] = useState<string[]>(["default"]);
  const [isMacOS, setIsMacOS] = useState(false);
  const [isInHouseBuild, setIsInHouseBuild] = useState(false);
  const [activeTab, setActiveTab] = useState<"general" | "transcription">("general");
  const [availableSubdomains, setAvailableSubdomains] = useState<string[]>([]);
  const [colorPickerOpen, setColorPickerOpen] = useState(false);
  const [isClearingCorrectionMemory, setIsClearingCorrectionMemory] = useState(false);
  const [isOpeningCorrectionMemoryDirectory, setIsOpeningCorrectionMemoryDirectory] = useState(false);
  const [correctionMemoryCleared, setCorrectionMemoryCleared] = useState(false);
  const correctionMemoryClearedTimer = useRef<number | undefined>(undefined);

  const { refs: colorPickerRefs, floatingStyles: colorPickerFloatingStyles, context: colorPickerContext } = useFloating({
    open: colorPickerOpen,
    onOpenChange: setColorPickerOpen,
    placement: "bottom-start",
    whileElementsMounted: autoUpdate,
    middleware: [
      offset(8),
      flip({ padding: 8 }),
      shift({ padding: 8 }),
    ],
  });

  const colorPickerClick = useClick(colorPickerContext);
  const colorPickerDismiss = useDismiss(colorPickerContext);
  const colorPickerRole = useRole(colorPickerContext, { role: "dialog" });
  const { getReferenceProps: getColorPickerReferenceProps, getFloatingProps: getColorPickerFloatingProps } = useInteractions([
    colorPickerClick,
    colorPickerDismiss,
    colorPickerRole,
  ]);

  // Helper functions with static i18n keys (for scanner detection)
  const getPillSizeLabel = (size: number): string => {
    switch (size) {
      case 1: return t("general.display.pillSize.small");
      case 2: return t("general.display.pillSize.default");
      case 3: return t("general.display.pillSize.medium");
      case 4: return t("general.display.pillSize.large");
      case 5: return t("general.display.pillSize.xlarge");
      default: return t("general.display.pillSize.default");
    }
  };

  const getPillBackgroundColorLabel = (label: PillBackgroundColorLabel): string => {
    switch (label) {
      case "dark": return t("general.display.pillBackgroundColor.dark");
      case "slate": return t("general.display.pillBackgroundColor.slate");
      case "forest": return t("general.display.pillBackgroundColor.forest");
      case "plum": return t("general.display.pillBackgroundColor.plum");
      case "copper": return t("general.display.pillBackgroundColor.copper");
    }
  };

  const getSubdomainLabel = (subdomain: string): string => {
    switch (subdomain) {
      case "general": return t("model.domain.subdomain_general", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "security": return t("model.domain.subdomain_security", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "hardware": return t("model.domain.subdomain_hardware", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "software": return t("model.domain.subdomain_software", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "web": return t("model.domain.subdomain_web", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "ai": return t("model.domain.subdomain_ai", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "civil": return t("model.domain.subdomain_civil", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "criminal": return t("model.domain.subdomain_criminal", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "corporate": return t("model.domain.subdomain_corporate", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "international": return t("model.domain.subdomain_international", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "pharmacy": return t("model.domain.subdomain_pharmacy", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "diagnostics": return t("model.domain.subdomain_diagnostics", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "cardiology": return t("model.domain.subdomain_cardiology", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      case "neurology": return t("model.domain.subdomain_neurology", subdomain.charAt(0).toUpperCase() + subdomain.slice(1));
      default: return subdomain.charAt(0).toUpperCase() + subdomain.slice(1);
    }
  };

  useEffect(() => {
    systemCommands.getAudioDevices().then(setAudioDevices).catch((err: unknown) => logger.error("failed_to_get_audio_devices", { error: String(err) }));
    systemCommands.getPlatform().then((platform) => {
      setIsMacOS(platform === "macos");
    }).catch((err: unknown) => logger.error("failed_to_get_platform", { error: String(err) }));
    getIdentifier()
      .then((identifier) => {
        setIsInHouseBuild(identifier.endsWith(".inhouse"));
      })
      .catch((err: unknown) => logger.error("failed_to_get_app_identifier", { error: String(err) }));
  }, []);

  useEffect(() => {
    return () => {
      if (correctionMemoryClearedTimer.current !== undefined) {
        window.clearTimeout(correctionMemoryClearedTimer.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!settings) return;
    if (settings.stt_engine_work_domain && settings.stt_engine_work_domain !== "general") {
      settingsCommands.getAvailableSubdomains(settings.stt_engine_work_domain)
        .then(setAvailableSubdomains)
        .catch((err: unknown) => logger.error("failed_to_get_available_subdomains", { error: String(err) }));
    }
  }, [settings?.stt_engine_work_domain]);

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

  const handlePillSizeChange = async (value: number) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "pill_size", value: String(value) });
    await updateSetting("pill_size", value);
  };

  const handlePillBackgroundColorChange = async (value: string) => {
    const color = normalizePillBackgroundColor(value);
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "pill_background_color", value: color });
    await updateSetting("pill_background_color", color);
  };

  const handlePillBackgroundOpacityChange = async (value: number) => {
    const opacity = normalizePillBackgroundOpacity(value);
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "pill_background_opacity", value: String(opacity) });
    await updateSetting("pill_background_opacity", opacity);
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

  const handleWindowContextChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "window_context_enabled", value: String(checked) });
    await updateSetting("window_context_enabled", checked);
  };

  const handleCorrectionMemoryChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "correction_memory_enabled", value: String(checked) });
    await updateSetting("correction_memory_enabled", checked);
  };

  const handleClearCorrectionMemory = async () => {
    setIsClearingCorrectionMemory(true);
    try {
      await settingsCommands.clearCorrectionMemory();
      analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "correction_memory_cleared" });
      setCorrectionMemoryCleared(true);
      if (correctionMemoryClearedTimer.current !== undefined) {
        window.clearTimeout(correctionMemoryClearedTimer.current);
      }
      correctionMemoryClearedTimer.current = window.setTimeout(() => {
        setCorrectionMemoryCleared(false);
        correctionMemoryClearedTimer.current = undefined;
      }, 2500);
    } catch (err) {
      logger.error("failed_to_clear_correction_memory", { error: String(err) });
      showErrorToast(t("general.privacy.correctionMemoryClearError"));
    } finally {
      setIsClearingCorrectionMemory(false);
    }
  };

  const handleOpenCorrectionMemoryDirectory = async () => {
    setIsOpeningCorrectionMemoryDirectory(true);
    try {
      await settingsCommands.openCorrectionMemoryDirectory();
      analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "correction_memory_directory_opened" });
    } catch (err) {
      logger.error("failed_to_open_correction_memory_directory", { error: String(err) });
      showErrorToast(t("general.privacy.correctionMemoryOpenError"));
    } finally {
      setIsOpeningCorrectionMemoryDirectory(false);
    }
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

  const pillBackgroundColor = normalizePillBackgroundColor(settings.pill_background_color);
  const pillBackgroundOpacity = normalizePillBackgroundOpacity(settings.pill_background_opacity);
  const pillBackgroundOpacityPercent = `${Math.round(pillBackgroundOpacity * 100)}%`;

  return (
    <SettingsPageLayout
      title={t("general.title")}
      description={t("general.description")}
      testId="settings-page"
    >
      <SegmentedControl
        items={[
          { value: "general", label: t("general.tabs.general") },
          { value: "transcription", label: t("general.tabs.transcription") },
        ]}
        value={activeTab}
        onChange={(v) => setActiveTab(v as "general" | "transcription")}
        size="sm"
      />

      {activeTab === "general" && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>{t("general.language.title")}</CardTitle>
              <CardDescription>{t("general.language.description")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
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
            <CardContent className="space-y-6">
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
            <CardContent className="space-y-6">
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
              <div className="space-y-2">
                <Label>{t("general.display.pillSize")}</Label>
                <p className="text-xs text-muted-foreground">
                  {t("general.display.pillSizeDesc")}
                </p>
                <div className="flex items-center gap-3">
                  <Slider
                    min={1}
                    max={5}
                    step={1}
                    value={settings.pill_size ?? 2}
                    onChange={(e) => handlePillSizeChange(Number(e.target.value))}
                    className="w-24"
                  />
                  <span className="text-xs text-muted-foreground">
                    {getPillSizeLabel(settings.pill_size ?? 2)}
                  </span>
                </div>
              </div>
              <div className="space-y-2">
                <Label>{t("general.display.pillBackgroundColor")}</Label>
                <p className="text-xs text-muted-foreground">
                  {t("general.display.pillBackgroundColorDesc")}
                </p>
                <div className="flex flex-wrap items-center gap-2">
                  {PILL_BACKGROUND_COLOR_PRESETS.map((preset) => {
                    const isSelected = pillBackgroundColor === preset.value;
                    const label = getPillBackgroundColorLabel(preset.label);

                    return (
                      <button
                        key={preset.value}
                        type="button"
                        aria-label={label}
                        title={label}
                        onClick={() => handlePillBackgroundColorChange(preset.value)}
                        className={`h-7 w-7 rounded-full border transition-all ${
                          isSelected
                            ? "border-foreground ring-2 ring-ring ring-offset-2 ring-offset-background"
                            : "border-border hover:border-foreground/50"
                        }`}
                        style={{ backgroundColor: preset.value }}
                      />
                    );
                  })}
                  <button
                    ref={colorPickerRefs.setReference}
                    type="button"
                    aria-label={t("general.display.pillBackgroundColor.custom")}
                    title={t("general.display.pillBackgroundColor.custom")}
                    className="relative block h-7 w-7 cursor-pointer overflow-hidden rounded-full border border-border transition-all hover:border-foreground/50 data-[state=open]:border-foreground data-[state=open]:ring-2 data-[state=open]:ring-ring data-[state=open]:ring-offset-2 data-[state=open]:ring-offset-background"
                    style={{ backgroundColor: pillBackgroundColor }}
                    data-state={colorPickerOpen ? "open" : "closed"}
                    {...getColorPickerReferenceProps()}
                  >
                    <span className="absolute inset-1 rounded-full border border-background/80" />
                  </button>
                  {colorPickerOpen && (
                    <FloatingPortal>
                      <FloatingFocusManager context={colorPickerContext} modal={false}>
                        <div
                          ref={colorPickerRefs.setFloating}
                          style={colorPickerFloatingStyles}
                          className="z-[9999] w-56 rounded-2xl border border-border bg-card p-3 shadow-lg outline-none"
                          {...getColorPickerFloatingProps()}
                        >
                          <HexColorPicker
                            color={pillBackgroundColor}
                            onChange={handlePillBackgroundColorChange}
                            className="!h-36 !w-full"
                          />
                          <HexColorInput
                            prefixed
                            color={pillBackgroundColor}
                            onChange={handlePillBackgroundColorChange}
                            className="mt-3 h-9 w-full rounded-2xl border border-border bg-background px-3 py-2 text-sm font-mono text-foreground outline-none transition-colors focus-visible:border-primary"
                          />
                        </div>
                      </FloatingFocusManager>
                    </FloatingPortal>
                  )}
                  <span className="text-xs font-mono text-muted-foreground">
                    {pillBackgroundColor.toUpperCase()}
                  </span>
                </div>
              </div>
              <div className="space-y-2">
                <Label>{t("general.display.pillBackgroundOpacity")}</Label>
                <p className="text-xs text-muted-foreground">
                  {t("general.display.pillBackgroundOpacityDesc")}
                </p>
                <div className="flex items-center gap-3">
                  <Slider
                    min={0.2}
                    max={1}
                    step={0.05}
                    value={pillBackgroundOpacity}
                    onChange={(e) => handlePillBackgroundOpacityChange(Number(e.target.value))}
                    className="w-24"
                  />
                  <span className="w-10 text-xs text-muted-foreground">
                    {pillBackgroundOpacityPercent}
                  </span>
                </div>
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
            <CardContent className="space-y-6">
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
              <div className="flex items-center justify-between space-x-4">
                <div>
                  <Label htmlFor="correction-memory-toggle">
                    {t("general.privacy.correctionMemory")}
                  </Label>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {t("general.privacy.correctionMemoryDesc")}
                  </p>
                </div>
                <Switch
                  id="correction-memory-toggle"
                  checked={settings.correction_memory_enabled ?? true}
                  onCheckedChange={handleCorrectionMemoryChange}
                />
              </div>
              <div className="flex items-center justify-between space-x-4">
                <div>
                  <Label>{t("general.privacy.correctionMemoryClear")}</Label>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {t("general.privacy.correctionMemoryClearDesc")}
                  </p>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={isClearingCorrectionMemory}
                  onClick={handleClearCorrectionMemory}
                >
                  {correctionMemoryCleared
                    ? t("general.privacy.correctionMemoryCleared")
                    : t("general.privacy.correctionMemoryClearAction")}
                </Button>
              </div>
              {isInHouseBuild && (
                <div className="flex items-center justify-between space-x-4">
                  <div>
                    <Label>{t("general.privacy.correctionMemoryOpen")}</Label>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {t("general.privacy.correctionMemoryOpenDesc")}
                    </p>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={isOpeningCorrectionMemoryDirectory}
                    onClick={handleOpenCorrectionMemoryDirectory}
                  >
                    {t("general.privacy.correctionMemoryOpenAction")}
                  </Button>
                </div>
              )}
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
            <CardContent className="space-y-6">
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
            <CardContent className="space-y-6">
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
            <CardContent className="space-y-6">
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

              <div className="flex items-center justify-between space-x-4">
                <div>
                  <Label htmlFor="window-context-toggle">{t("general.audio.contextAware")}</Label>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {t("general.audio.contextAwareDesc")}
                  </p>
                </div>
                <Switch
                  id="window-context-toggle"
                  checked={settings.window_context_enabled}
                  onCheckedChange={handleWindowContextChange}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("general.transcription.domainTitle")}</CardTitle>
              <CardDescription>{t("general.transcription.domainDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
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
                      label: getSubdomainLabel(sub),
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
    </SettingsPageLayout>
  );
}
