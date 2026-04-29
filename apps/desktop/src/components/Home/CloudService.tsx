import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsPageLayout } from "./SettingsPageLayout";
import { CloudSttSection } from "./cloud/CloudSttSection";
import { CloudPolishSection } from "./cloud/CloudPolishSection";
import { SegmentedControl } from "@/components/ui/segmented-control";

export function CloudService() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"stt" | "polish">("stt");

  return (
    <SettingsPageLayout
      title={t("cloud.title")}
      description={t("cloud.description")}
      testId="cloud-page"
    >
      <SegmentedControl
        items={[
          { value: "stt", label: t("cloud.tabs.stt", "Cloud STT") },
          { value: "polish", label: t("cloud.tabs.polish", "Cloud Polish") },
        ]}
        value={activeTab}
        onChange={(v) => setActiveTab(v as "stt" | "polish")}
      />
      <div>
        {activeTab === "stt" && <CloudSttSection />}
        {activeTab === "polish" && <CloudPolishSection />}
      </div>
    </SettingsPageLayout>
  );
}