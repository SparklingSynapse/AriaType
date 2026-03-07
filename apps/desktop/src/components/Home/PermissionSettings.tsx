import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";
import { usePermissions } from "@/hooks/usePermissions";
import { SettingsPageLayout } from "./SettingsPageLayout";

export function PermissionSettings() {
  const { t } = useTranslation();
  const { accessibilityGranted, microphoneStatus, isLoading, handleApplyPermission } =
    usePermissions();

  return (
    <SettingsPageLayout
      title={t("general.permissions.title")}
      description={t("general.permissions.description")}
    >
      <Card>
        <CardHeader>
          <CardTitle>{t("general.permissions.title")}</CardTitle>
          <CardDescription>
            {t("general.permissions.description")}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="flex items-center justify-between space-x-4">
            <div>
              <Label>{t("general.permissions.microphone")}</Label>
              <p className="text-xs text-muted-foreground mt-0.5">
                {t("general.permissions.microphoneDesc")}
                {microphoneStatus === "granted" && (
                  <span className="text-green-500 ml-1">
                    · {t("general.permissions.granted")}
                  </span>
                )}
                {microphoneStatus === "denied" && (
                  <span className="text-destructive ml-1">
                    · {t("general.permissions.notGranted")}
                  </span>
                )}
                {microphoneStatus === "not_determined" && (
                  <span className="text-amber-500 ml-1">
                    · {t("general.permissions.notGranted")}
                  </span>
                )}
              </p>
            </div>
            <div className="flex gap-2">
              <Button
                variant={isLoading ? "outline" : microphoneStatus === "granted" ? "outline" : "default"}
                size="sm"
                onClick={() => handleApplyPermission("microphone")}
                disabled={isLoading}
              >
                {isLoading
                  ? "..."
                  : microphoneStatus === "granted"
                    ? t("general.permissions.openSettings")
                    : t("general.permissions.grantPermission")}
              </Button>
            </div>
          </div>

          <div className="flex items-center justify-between space-x-4">
            <div>
              <Label>{t("general.permissions.accessibility")}</Label>
              <p className="text-xs text-muted-foreground mt-0.5">
                {t("general.permissions.accessibilityDesc")}
                {accessibilityGranted === true && (
                  <span className="text-green-500 ml-1">
                    · {t("general.permissions.granted")}
                  </span>
                )}
                {accessibilityGranted === false && (
                  <span className="text-destructive ml-1">
                    · {t("general.permissions.notGranted")}
                  </span>
                )}
              </p>
            </div>
            <Button
              variant={isLoading ? "outline" : accessibilityGranted === false ? "default" : "outline"}
              size="sm"
              onClick={() => handleApplyPermission("accessibility")}
              disabled={isLoading}
            >
              {isLoading
                ? "..."
                : accessibilityGranted === false
                  ? t("general.permissions.grantPermission")
                  : t("general.permissions.openSettings")}
            </Button>
          </div>
</CardContent>
      </Card>
    </SettingsPageLayout>
  );
}
