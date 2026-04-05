import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { useTranslation } from "react-i18next";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { HotkeyInput } from "@/components/ui/hotkey-input";
import { MultiSwitch } from "@/components/ui/multi-switch";
import { SettingsPageLayout } from "./SettingsPageLayout";

export function HotkeySettings() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();

  if (!settings) return null;

  const saveHotkey = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "hotkey", value });
    await updateSetting("hotkey", value);
  };

  const saveRecordingMode = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "recording_mode", value });
    await updateSetting("recording_mode", value);
  };

  const recordingModes = [
    { value: "hold", label: t("hotkey.recording.modeHold") },
    { value: "toggle", label: t("hotkey.recording.modeToggle") },
  ];

  return (
    <SettingsPageLayout
      title={t("hotkey.title")}
      description={t("hotkey.description")}
    >
      <Card>
        <CardHeader>
          <CardTitle>{t("hotkey.recording.title")}</CardTitle>
          <CardDescription>{t("hotkey.recording.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <Label>{t("hotkey.recording.globalHotkey")}</Label>
            <HotkeyInput
              value={settings.hotkey}
              onChange={saveHotkey}
              placeholder={t("hotkey.recording.pressKeys")}
              className="w-full px-4 py-2 h-10 text-sm rounded-2xl border border-border bg-background"
            />
            <p className="text-xs text-muted-foreground">
              {t("hotkey.recording.hint")}
            </p>
          </div>

          <div className="space-y-4">
            <div className="text-sm font-medium">{t("hotkey.recording.modeTitle")}</div>
            <MultiSwitch
              options={recordingModes}
              value={settings.recording_mode}
              onChange={saveRecordingMode}
            />
          </div>
          <p className="text-xs text-muted-foreground">
            {t("hotkey.recording.modeDesc")}
          </p>
        </CardContent>
      </Card>
    </SettingsPageLayout>
  );
}
