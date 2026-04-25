import { useState, useEffect } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { useTranslation } from "react-i18next";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useSettingsContext } from "@/contexts/SettingsContext";
import { HotkeyInput } from "@/components/ui/hotkey-input";
import { MultiSwitch } from "@/components/ui/multi-switch";
import { SettingsPageLayout } from "./SettingsPageLayout";
import { hotkeyCommands, modelCommands, type ShortcutProfile, type PolishTemplate, type CustomPolishTemplate } from "@/lib/tauri";
import { Plus, Trash2 } from "lucide-react";
import { showErrorToast } from "@/lib/toast";

const RECORDING_MODES = [
  { value: "hold", label: "Hold" },
  { value: "toggle", label: "Toggle" },
] as const;

interface ProfileSectionProps {
  profileKey: string;
  profile?: ShortcutProfile;
  templates: (PolishTemplate | CustomPolishTemplate)[];
  canChangeTemplate: boolean;
  allowNullTemplate: boolean;
  onUpdate: (hotkey: string, templateId: string | null, triggerMode: "hold" | "toggle") => void;
  testId?: string;
}

function ProfileSection({
  profileKey,
  profile,
  templates,
  canChangeTemplate,
  allowNullTemplate,
  onUpdate,
  testId,
}: ProfileSectionProps) {
  const { t } = useTranslation();
  const templateId = profile?.action?.Record?.polish_template_id ?? null;
  const triggerMode = profile?.trigger_mode ?? "hold";

  const templateOptions = [
    ...(allowNullTemplate ? [{ value: "", label: t("hotkey.noPolish", "No Polish") }] : []),
    ...templates.map((tpl) => ({ value: tpl.id, label: tpl.name })),
  ];
  const recordingModes = RECORDING_MODES.map((option) => ({
    value: option.value,
    label:
      option.value === "hold"
        ? t("hotkey.recording.modeHold")
        : t("hotkey.recording.modeToggle"),
  }));

  return (
    <div className="space-y-4" data-testid={testId}>
      <div className="space-y-2">
        <Label>{t("hotkey.hotkey", "Hotkey")}</Label>
        <HotkeyInput
          profileKey={profileKey}
          value={profile?.hotkey || ""}
          onChange={(hotkey) => onUpdate(hotkey, templateId, triggerMode)}
          placeholder={t("hotkey.recording.pressKeys")}
          className="w-auto"
        />
      </div>

      <div className="space-y-4">
        <div className="text-sm font-medium">{t("hotkey.recording.modeTitle")}</div>
        <MultiSwitch
          options={recordingModes}
          value={triggerMode}
          onChange={(value) => onUpdate(profile?.hotkey || "", templateId, value as "hold" | "toggle")}
        />
      </div>

      {canChangeTemplate && (
        <div className="space-y-2">
          <Label>{t("hotkey.template", "Polish Template")}</Label>
          <Select
            value={templateId || ""}
            onChange={(e) =>
              onUpdate(
                profile?.hotkey || "",
                allowNullTemplate ? (e.target.value || null) : e.target.value,
                triggerMode,
              )
            }
            options={templateOptions}
            placeholder={t("hotkey.selectTemplate", "Select template")}
          />
        </div>
      )}
    </div>
  );
}

export function HotkeySettings() {
  const { t } = useTranslation();
  const { settings } = useSettingsContext();
  const [templates, setTemplates] = useState<(PolishTemplate | CustomPolishTemplate)[]>([]);

  useEffect(() => {
    loadTemplates();
  }, []);

  const loadTemplates = async () => {
    try {
      const [builtIn, custom] = await Promise.all([
        modelCommands.getPolishTemplates(),
        modelCommands.getPolishCustomTemplates(),
      ]);
      setTemplates([...builtIn, ...custom]);
    } catch (err) {
      console.error("Failed to load templates:", err);
    }
  };

  if (!settings) return null;

  const profiles = settings.shortcut_profiles;

  const handleUpdateDictate = async (
    hotkey: string,
    _: string | null,
    triggerMode: "hold" | "toggle",
  ) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "dictate_hotkey", value: hotkey });
    await hotkeyCommands.updateProfile("dictate", {
      hotkey,
      trigger_mode: triggerMode,
      action: { Record: { polish_template_id: null } },
    });
  };

  const handleUpdateChat = async (
    hotkey: string,
    templateId: string | null,
    triggerMode: "hold" | "toggle",
  ) => {
    if (!templateId) {
      showErrorToast(t("hotkey.chatTemplateRequired", "Chat profile requires a polish template"));
      return;
    }
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "chat_profile", value: `${hotkey}:${templateId}` });
    await hotkeyCommands.updateProfile("chat", {
      hotkey,
      trigger_mode: triggerMode,
      action: { Record: { polish_template_id: templateId } },
    });
  };

  const handleUpdateCustom = async (
    hotkey: string,
    templateId: string | null,
    triggerMode: "hold" | "toggle",
  ) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "custom_profile", value: `${hotkey}:${templateId ?? "none"}` });
    await hotkeyCommands.updateProfile("custom", {
      hotkey,
      trigger_mode: triggerMode,
      action: { Record: { polish_template_id: templateId } },
    });
  };

  const handleCreateCustom = async () => {
    const firstTemplateId = templates[0]?.id ?? "filler";
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "custom_profile_created" });
    await hotkeyCommands.createCustom({
      hotkey: "",
      trigger_mode: "toggle",
      action: { Record: { polish_template_id: firstTemplateId } },
    });
  };

  const handleDeleteCustom = async () => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "custom_profile_deleted" });
    await hotkeyCommands.deleteCustom();
  };

  return (
    <SettingsPageLayout
      title={t("hotkey.title")}
      description={t("hotkey.description")}
      testId="hotkey-page"
    >
      <Card>
        <CardHeader>
          <CardTitle>{t("hotkey.profiles.dictate", "Dictate")}</CardTitle>
          <CardDescription>{t("hotkey.profiles.dictateDesc", "Quick voice-to-text without polish")}</CardDescription>
        </CardHeader>
        <CardContent>
          <ProfileSection
            profileKey="dictate"
            profile={profiles?.dictate}
            templates={templates}
            canChangeTemplate={false}
            allowNullTemplate={false}
            onUpdate={handleUpdateDictate}
            testId="profile-dictate"
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("hotkey.profiles.chat", "Chat")}</CardTitle>
          <CardDescription>{t("hotkey.profiles.chatDesc", "Voice-to-text with polish formatting")}</CardDescription>
        </CardHeader>
        <CardContent>
          <ProfileSection
            profileKey="chat"
            profile={profiles?.chat}
            templates={templates}
            canChangeTemplate={true}
            allowNullTemplate={false}
            onUpdate={handleUpdateChat}
            testId="profile-chat"
          />
        </CardContent>
      </Card>

      {profiles?.custom ? (
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0">
            <div>
              <CardTitle>{t("hotkey.profiles.custom", "Custom")}</CardTitle>
              <CardDescription>{t("hotkey.profiles.customDesc", "Your custom profile")}</CardDescription>
            </div>
            <Button variant="ghost" size="icon" onClick={handleDeleteCustom} data-testid="delete-custom-profile">
              <Trash2 className="h-4 w-4" />
            </Button>
          </CardHeader>
          <CardContent>
            <ProfileSection
              profileKey="custom"
              profile={profiles.custom}
              templates={templates}
              canChangeTemplate={true}
              allowNullTemplate={true}
              onUpdate={handleUpdateCustom}
              testId="profile-custom"
            />
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>{t("hotkey.profiles.custom", "Custom")}</CardTitle>
            <CardDescription>{t("hotkey.profiles.customDesc", "Your custom profile")}</CardDescription>
          </CardHeader>
          <CardContent>
            <Button variant="outline" onClick={handleCreateCustom} data-testid="create-custom-profile">
              <Plus className="h-4 w-4 mr-2" />
              {t("hotkey.profiles.createCustom", "Create Custom Profile")}
            </Button>
          </CardContent>
        </Card>
      )}
    </SettingsPageLayout>
  );
}