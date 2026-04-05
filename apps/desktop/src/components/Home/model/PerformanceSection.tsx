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
import { useTranslation } from "react-i18next";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useSettingsContext } from "@/contexts/SettingsContext";

export function PerformanceSection() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();

  if (!settings) return null;

  const handleGpuChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "gpu_acceleration", value: String(checked) });
    await updateSetting("gpu_acceleration", checked);
  };

  const handleModelResidentChange = async (checked: boolean) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "model_resident", value: String(checked) });
    await updateSetting("model_resident", checked);
  };

  const handleIdleUnloadChange = async (value: number) => {
    analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "idle_unload_minutes", value: String(value) });
    await updateSetting("idle_unload_minutes", value);
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>{t("model.performanceSection.title")}</CardTitle>
          <CardDescription>{t("model.performanceSection.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between space-x-4">
            <div>
              <Label htmlFor="gpu">
                {t("model.performance.gpuAcceleration")}
              </Label>
              <p className="text-xs text-muted-foreground">
                {t("model.performance.gpuDesc")}
              </p>
            </div>
            <Switch
              id="gpu"
              checked={settings.gpu_acceleration}
              onCheckedChange={handleGpuChange}
            />
          </div>

          <div className="flex items-center justify-between space-x-4 pt-4 border-t border-border">
            <div>
              <Label htmlFor="model-resident">
                {t("model.performance.modelResident")}
              </Label>
              <p className="text-xs text-muted-foreground">
                {t("model.performance.modelResidentDesc")}
              </p>
            </div>
            <Switch
              id="model-resident"
              checked={settings.model_resident}
              onCheckedChange={handleModelResidentChange}
            />
          </div>

          {settings.model_resident && (
            <div className="flex items-center justify-between space-x-4 pt-4 border-t border-border">
              <div>
                <Label htmlFor="idle-unload">
                  {t("model.performance.idleUnload")}
                </Label>
                <p className="text-xs text-muted-foreground">
                  {t("model.performance.idleUnloadDesc")}
                </p>
              </div>
              <Select
                value={String(settings.idle_unload_minutes ?? 5)}
                onChange={(e) =>
                  handleIdleUnloadChange(Number(e.target.value))
                }
                options={[
                  { value: "1", label: "1 min" },
                  { value: "3", label: "3 min" },
                  { value: "5", label: "5 min" },
                  { value: "10", label: "10 min" },
                  { value: "30", label: "30 min" },
                ]}
              />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}