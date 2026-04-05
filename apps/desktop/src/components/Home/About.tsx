import { useState, useEffect } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Mic,
  Keyboard,
  Type,
  Palette,
  Sparkles,
  Lock,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { useNavigate } from "react-router-dom";
import logo from "../../../assets/logo.png";
import appleLogo from "../../../assets/apple-logo.svg";
import microsoftLogo from "../../../assets/microsoft-logo.svg";
import { UpdateChecker } from "./UpdateChecker";
import { SettingsPageLayout } from "./SettingsPageLayout";

const platforms = [
  { name: "macOS", requirement: "12.0+", logo: appleLogo },
  { name: "Windows", requirement: "10+", logo: microsoftLogo },
];

export function About() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [version, setVersion] = useState("");

  const features = [
    {
      icon: Lock,
      title: t("about.feature.privacy.title"),
      description: t("about.feature.privacy.description"),
      highlight: true,
    },
    {
      icon: Mic,
      title: t("about.feature.voice.title"),
      description: t("about.feature.voice.description"),
    },
    {
      icon: Keyboard,
      title: t("about.feature.hotkey.title"),
      description: t("about.feature.hotkey.description"),
    },
    {
      icon: Type,
      title: t("about.feature.insert.title"),
      description: t("about.feature.insert.description"),
    },
    {
      icon: Sparkles,
      title: t("about.feature.polish.title"),
      description: t("about.feature.polish.description"),
    },
    {
      icon: Palette,
      title: t("about.feature.design.title"),
      description: t("about.feature.design.description"),
    },
  ];

  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion(""));
  }, []);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    const handlePointerDown = () => {
      timer = setTimeout(() => navigate("/logs"), 800);
    };
    const handlePointerUp = () => {
      clearTimeout(timer);
    };

    const copyright = document.getElementById("copyright-area");
    if (copyright) {
      copyright.addEventListener("pointerdown", handlePointerDown);
      copyright.addEventListener("pointerup", handlePointerUp);
      copyright.addEventListener("pointerleave", handlePointerUp);
    }

    return () => {
      if (copyright) {
        copyright.removeEventListener("pointerdown", handlePointerDown);
        copyright.removeEventListener("pointerup", handlePointerUp);
        copyright.removeEventListener("pointerleave", handlePointerUp);
      }
      clearTimeout(timer);
    };
  }, [navigate]);

  return (
    <SettingsPageLayout>
      <div className="text-center py-8">
        <div className="relative inline-block mb-6">
          <div className="relative">
            <img
              src={logo}
              alt="AriaType"
              className="h-24 w-24 rounded-3xl shadow-xl ring-1 ring-border"
            />
            <div className="absolute -bottom-2 -right-2 bg-primary rounded-2xl p-2">
              <Sparkles className="h-5 w-5 text-primary-foreground" />
            </div>
          </div>
        </div>
        <h1 className="text-3xl font-bold tracking-tight text-foreground font-serif italic">
          {t("app.name")}
        </h1>
        <p className="text-muted-foreground mt-2 text-lg">
          {t("app.tagline")}
        </p>
        <div className="flex items-center justify-center gap-2 mt-4">
          <span className="inline-flex items-center rounded-full bg-secondary px-3 py-1 text-sm text-secondary-foreground">
            v{version || "..."}
          </span>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("about.updates")}</CardTitle>
          <CardDescription>{t("about.updatesDesc")}</CardDescription>
        </CardHeader>
        <CardContent>
          <UpdateChecker />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("about.features")}</CardTitle>
          <CardDescription>{t("about.featuresDesc")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {features.map((feature, index) => (
              <div
                key={index}
                className="flex items-start gap-3 p-4 rounded-2xl bg-secondary/50 hover:bg-secondary transition-colors"
              >
                <div className="rounded-2xl bg-background p-2 shrink-0 shadow-sm">
                  <feature.icon className={`h-4 w-4 ${feature.highlight ? "text-green-600 dark:text-green-500" : "text-foreground"}`} />
                </div>
                <div>
                  <h3 className="text-sm font-medium">{feature.title}</h3>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {feature.description}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("about.platforms.title")}</CardTitle>
          <CardDescription>{t("about.platforms.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-3">
            {platforms.map((platform) => (
              <div
                key={platform.name}
                className="flex items-center gap-3 rounded-2xl border border-border bg-background px-4 py-3 shadow-sm"
              >
                <img
                  src={platform.logo}
                  alt={platform.name}
                  className={platform.name === "macOS" ? "h-6 w-6 dark:invert" : "h-6 w-6"}
                />
                <div>
                  <span className="text-sm font-medium text-foreground">{platform.name}</span>
                  <span className="text-xs text-muted-foreground ml-2">
                    {platform.requirement}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="text-center pt-4" id="copyright-area">
        <p className="text-xs text-muted-foreground">{t("about.copyright")}</p>
      </div>
    </SettingsPageLayout>
  );
}
