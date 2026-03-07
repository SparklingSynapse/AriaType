import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { useTranslation } from "react-i18next";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { HotkeyInput } from "@/components/ui/hotkey-input";
import { SettingsPageLayout } from "./SettingsPageLayout";

export function HotkeySettings() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();

  if (!settings) return null;

  const saveHotkey = async (value: string) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "hotkey", value });
    await updateSetting("hotkey", value);
  };

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
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>{t("hotkey.recording.globalHotkey")}</Label>
            <HotkeyInput
              value={settings.hotkey}
              onChange={saveHotkey}
              placeholder={t("hotkey.recording.pressKeys")}
              className="w-full px-3 py-2 text-sm rounded-md border border-input bg-background"
            />
            <p className="text-xs text-muted-foreground">
              {t("hotkey.recording.hint")}
            </p>
          </div>
        </CardContent>
      </Card>
    </SettingsPageLayout>
  );
}
