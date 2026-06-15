import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  Brain,
  Cloud,
  GearSix,
  Keyboard,
  ShieldCheck,
  Waveform,
  type Icon,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import { modelCommands, events } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { useNavBadges } from "@/hooks/useNavBadges";
import { useEventListeners } from "@/hooks/useEventListeners";
import { GeneralSettings } from "./GeneralSettings";
import { HotkeySettings } from "./HotkeySettings";
import { ModelSettings } from "./ModelSettings";
import { CloudService } from "./CloudService";
import { PermissionSettings } from "./PermissionSettings";
import {
  ModalFrame,
  MODAL_NAV_WIDTH_CLASS,
  ModalCloseButton,
} from "./ModalShell";
import {
  NavigationAttentionBadge,
  NavigationGroup,
  NavigationItemButton,
} from "./NavigationItem";

export type SettingsModalSection =
  | "basics"
  | "recording"
  | "transcription"
  | "models"
  | "cloud"
  | "permissions";

interface SettingsModalProps {
  open: boolean;
  initialSection?: SettingsModalSection;
  onOpenChange: (open: boolean) => void;
}

interface SettingsSectionConfig {
  id: SettingsModalSection;
  icon: Icon;
  title: string;
  description: string;
}

export function SettingsModal({
  open,
  initialSection = "basics",
  onOpenChange,
}: SettingsModalProps) {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] =
    useState<SettingsModalSection>(initialSection);
  const [hasModel, setHasModel] = useState(true);
  const badges = useNavBadges();

  const checkModel = useCallback(async () => {
    try {
      const models = await modelCommands.getModels();
      setHasModel(models.some((m) => m.downloaded));
    } catch (err) {
      logger.error("failed_to_check_models", { error: String(err) });
    }
  }, []);

  useEffect(() => {
    if (open) {
      checkModel();
    }
  }, [checkModel, open]);

  useEventListeners(async () => {
    return [
      await events.onModelDownloadComplete(() => checkModel()),
      await events.onModelDeleted(() => checkModel()),
    ];
  }, [checkModel]);

  const sections = useMemo<SettingsSectionConfig[]>(
    () => [
      {
        id: "basics",
        icon: GearSix,
        title: t("settingsModal.basics.title"),
        description: t("settingsModal.basics.description"),
      },
      {
        id: "recording",
        icon: Keyboard,
        title: t("settingsModal.recording.title"),
        description: t("settingsModal.recording.description"),
      },
      {
        id: "transcription",
        icon: Waveform,
        title: t("settingsModal.transcription.title"),
        description: t("settingsModal.transcription.description"),
      },
      {
        id: "models",
        icon: Brain,
        title: t("settingsModal.models.title"),
        description: t("settingsModal.models.description"),
      },
      {
        id: "cloud",
        icon: Cloud,
        title: t("settingsModal.cloud.title"),
        description: t("settingsModal.cloud.description"),
      },
      {
        id: "permissions",
        icon: ShieldCheck,
        title: t("settingsModal.permissions.title"),
        description: t("settingsModal.permissions.description"),
      },
    ],
    [t],
  );

  useEffect(() => {
    if (open) {
      setActiveSection(initialSection);
    }
  }, [initialSection, open]);

  const activeConfig = useMemo(
    () =>
      sections.find((section) => section.id === activeSection) ??
      sections[0],
    [activeSection, sections],
  );

  const content = (() => {
    switch (activeSection) {
      case "basics":
        return <GeneralSettings variant="modal" fixedTab="general" />;
      case "recording":
        return <HotkeySettings variant="modal" />;
      case "transcription":
        return <GeneralSettings variant="modal" fixedTab="transcription" />;
      case "models":
        return <ModelSettings variant="modal" />;
      case "cloud":
        return <CloudService variant="modal" />;
      case "permissions":
        return <PermissionSettings variant="modal" />;
    }
  })();

  return (
    <ModalFrame
      onOpenChange={onOpenChange}
      open={open}
      testId="settings-modal"
      viewportClassName="bottom-8 top-12"
    >
      <aside
        className={cn(
          "flex shrink-0 flex-col border-r border-border/70 bg-background/70",
          MODAL_NAV_WIDTH_CLASS,
        )}
        data-testid="settings-modal-nav"
      >
        <div className="px-5 pb-6 pt-6">
          <Dialog.Title className="text-xl font-semibold text-foreground">
            {t("settingsModal.title")}
          </Dialog.Title>
          <Dialog.Description className="sr-only">
            {t("settingsModal.description")}
          </Dialog.Description>
        </div>
        <NavigationGroup id="settings-modal-navigation">
          <div className="flex-1 space-y-1 overflow-y-auto px-3 pb-4 pt-1">
            {sections.map((section) => {
              const isActive = section.id === activeSection;
              const needsAttention =
                (section.id === "models" && !hasModel) ||
                (section.id === "permissions" && badges.permission);

              return (
                <NavigationItemButton
                  active={isActive}
                  activeIndicatorLayoutId="settings-modal-active-section"
                  badge={needsAttention ? <NavigationAttentionBadge /> : undefined}
                  data-testid={`settings-modal-section-${section.id}`}
                  icon={section.icon}
                  key={section.id}
                  label={section.title}
                  onClick={() => setActiveSection(section.id)}
                />
              );
            })}
          </div>
        </NavigationGroup>
      </aside>

      <section className="flex min-w-0 flex-1 flex-col bg-card">
        <div className="flex shrink-0 items-center justify-between gap-5 border-b border-border/70 px-8 py-5">
          <div className="min-w-0">
            <h2 className="text-xl font-semibold leading-7 text-foreground">
              {activeConfig.title}
            </h2>
            <p className="mt-1 max-w-xl text-sm leading-6 text-muted-foreground">
              {activeConfig.description}
            </p>
          </div>
          <Dialog.Close asChild>
            <ModalCloseButton
              aria-label={t("settingsModal.close")}
            />
          </Dialog.Close>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-7 py-5">
          {content}
        </div>
      </section>
    </ModalFrame>
  );
}
