import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsPageLayout } from "./SettingsPageLayout";
import { CloudSttSection } from "./cloud/CloudSttSection";
import { CloudPolishSection } from "./cloud/CloudPolishSection";
import { cn } from "@/lib/utils";

export function CloudService() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"stt" | "polish">("stt");

  return (
    <SettingsPageLayout
      title={t("cloud.title")}
      description={t("cloud.description")}
    >
      <div className="inline-flex h-11 items-center justify-center rounded-full bg-secondary p-1.5 text-muted-foreground">
        <button
          onClick={() => setActiveTab("stt")}
          className={cn(
            "inline-flex h-full items-center justify-center whitespace-nowrap rounded-full px-5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "stt"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground hover:bg-background/40"
          )}
        >
          {t("cloud.tabs.stt", "Cloud STT")}
        </button>
        <button
          onClick={() => setActiveTab("polish")}
          className={cn(
            "inline-flex h-full items-center justify-center whitespace-nowrap rounded-full px-5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            activeTab === "polish"
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground hover:bg-background/40"
          )}
        >
          {t("cloud.tabs.polish", "Cloud Polish")}
        </button>
      </div>
      <div>
        {activeTab === "stt" && <CloudSttSection />}
        {activeTab === "polish" && <CloudPolishSection />}
      </div>
    </SettingsPageLayout>
  );
}