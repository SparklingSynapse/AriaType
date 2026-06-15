import { Outlet, useLocation } from "react-router-dom";
import {
  CirclesFour,
  GearSix,
  MagicWand,
  ClockCounterClockwise,
  BookOpenText,
  ChatCircleText,
  ArrowSquareOut,
  GithubLogo,
  Info,
  type Icon,
} from "@phosphor-icons/react";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";
import logo from "../../../assets/logo.png";
import { modelCommands, events, systemCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useEffect, useState, useCallback, useRef } from "react";
import { useOnboarding } from "@/hooks/useOnboarding";
import { useEventListeners } from "@/hooks/useEventListeners";
import { OnboardingGuide } from "./OnboardingGuide";
import { useNavBadges } from "@/hooks/useNavBadges";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SettingsModal, type SettingsModalSection } from "./SettingsModal";
import { MODAL_NAV_WIDTH_CLASS } from "./ModalShell";
import {
  NavigationAttentionBadge,
  NavigationGroup,
  NavigationItemAnchor,
  NavigationItemButton,
  NavigationItemLink,
} from "./NavigationItem";

const FEEDBACK_URL = "https://github.com/joe223/AriaType/issues/new";
const GITHUB_SUPPORT_URL = "https://github.com/joe223/AriaType";
const SETTINGS_ROUTES = new Set([
  "/settings",
  "/hotkey",
  "/private-ai",
  "/cloud",
  "/permission",
]);

interface PrimaryNavItem {
  to: string;
  icon: Icon;
  label: string;
}

function getSettingsSectionForPath(pathname: string): SettingsModalSection {
  switch (pathname) {
    case "/hotkey":
      return "recording";
    case "/private-ai":
      return "models";
    case "/cloud":
      return "cloud";
    case "/permission":
      return "permissions";
    case "/settings":
    default:
      return "basics";
  }
}

export function HomeLayout() {
  const { t } = useTranslation();
  const [hasModel, setHasModel] = useState(true);
  const [settingsModalOpen, setSettingsModalOpen] = useState(false);
  const [settingsInitialSection, setSettingsInitialSection] =
    useState<SettingsModalSection>("basics");
  const [showGithubSupportLink, setShowGithubSupportLink] = useState(false);
  const { isOpen, closeOnboarding } = useOnboarding();
  const badges = useNavBadges();
  const location = useLocation();
  const supportLinkPathRef = useRef(location.pathname);
  const settingsOpenFrameRef = useRef<number | null>(null);

  useEffect(() => {
    analytics.track(AnalyticsEvents.SCREEN_VIEW, {
      screen_name: location.pathname,
    });
  }, [location]);

  useEffect(() => {
    if (supportLinkPathRef.current === location.pathname) {
      return;
    }

    supportLinkPathRef.current = location.pathname;
    setShowGithubSupportLink((value) => !value);
  }, [location.pathname]);

  useEffect(() => {
    const rotationTimer = window.setInterval(() => {
      setShowGithubSupportLink((value) => !value);
    }, 12000);

    return () => window.clearInterval(rotationTimer);
  }, []);

  useEffect(() => {
    return () => {
      if (settingsOpenFrameRef.current !== null) {
        window.cancelAnimationFrame(settingsOpenFrameRef.current);
      }
    };
  }, []);

  const primaryNavItems: PrimaryNavItem[] = [
    { to: "/", icon: CirclesFour, label: t("nav.dashboard") },
    { to: "/history", icon: ClockCounterClockwise, label: t("nav.history") },
    { to: "/dictionary", icon: BookOpenText, label: t("nav.dictionary") },
    { to: "/polish-templates", icon: MagicWand, label: t("nav.polishTemplates") },
  ];
  const settingsNeedsAttention = !hasModel || badges.permission;
  const settingsRouteActive = SETTINGS_ROUTES.has(location.pathname);

  const openSettingsModal = (section: SettingsModalSection = "basics") => {
    setSettingsInitialSection(section);
    if (settingsOpenFrameRef.current !== null) {
      window.cancelAnimationFrame(settingsOpenFrameRef.current);
    }

    settingsOpenFrameRef.current = window.requestAnimationFrame(() => {
      settingsOpenFrameRef.current = null;
      setSettingsModalOpen(true);
    });
  };

  const handleOnboardingClose = useCallback(async () => {
    closeOnboarding();
    const micStatus = await systemCommands.checkPermission("microphone").catch(() => null);
    if (micStatus === "not_determined") {
      systemCommands.applyPermission("microphone").catch((err: unknown) => logger.error("failed_to_apply_microphone_permission", { error: String(err) }));
    }
    const axStatus = await systemCommands.checkPermission("accessibility").catch(() => "granted");
    if (axStatus !== "granted") {
      systemCommands.applyPermission("accessibility").catch((err: unknown) => logger.error("failed_to_apply_accessibility_permission", { error: String(err) }));
    }
  }, [closeOnboarding]);

  const checkModel = useCallback(async () => {
    try {
      const models = await modelCommands.getModels();
      setHasModel(models.some((m) => m.downloaded));
    } catch (err) {
      logger.error("failed_to_check_models", { error: String(err) });
    }
  }, []);

  useEventListeners(async () => {
    return [
      await events.onModelDownloadComplete(() => checkModel()),
      await events.onModelDeleted(() => checkModel()),
    ];
  }, [checkModel]);

  return (
    <div className="flex flex-col h-screen bg-background">
      <div className="flex flex-1 overflow-hidden ">
        <OnboardingGuide isOpen={isOpen} onClose={handleOnboardingClose} />
        <SettingsModal
          open={settingsModalOpen}
          initialSection={settingsInitialSection}
          onOpenChange={setSettingsModalOpen}
        />
        <aside
          className={cn(
            MODAL_NAV_WIDTH_CLASS,
            "border-r border-border/70 bg-background/70 pt-7",
          )}
          data-testid="home-sidebar"
        >
          <div className="px-5 py-5 border-b border-border/70 flex items-center gap-3">
            <img
              src={logo}
              alt="AriaType"
              className="h-10 w-10 rounded-lg shadow-sm ring-1 ring-border/80"
            />
            <span className="text-[22px] font-bold text-foreground font-serif italic">
              {t("app.name")}
            </span>
          </div>
          <nav className="px-4 py-4 flex flex-col h-[calc(100%-4.5rem)]">
            <NavigationGroup id="home-sidebar-navigation">
              <div className="space-y-1">
                {primaryNavItems.map((item) => {
                  const to = item.to;

                  return (
                    <NavigationItemLink
                      activeIndicatorLayoutId="home-sidebar-active-item"
                      activeWhen={(isActive) => isActive && !settingsModalOpen}
                      end={to === "/"}
                      icon={item.icon}
                      key={to}
                      label={item.label}
                      to={to}
                    />
                  );
                })}

                <NavigationItemButton
                  active={settingsModalOpen || settingsRouteActive}
                  activeIndicatorLayoutId="home-sidebar-active-item"
                  badge={settingsNeedsAttention ? <NavigationAttentionBadge /> : undefined}
                  data-testid="open-settings-modal"
                  icon={GearSix}
                  label={t("nav.settings")}
                  onClick={() => openSettingsModal(getSettingsSectionForPath(location.pathname))}
                />

                <NavigationItemLink
                  activeIndicatorLayoutId="home-sidebar-active-item"
                  activeWhen={(isActive) => isActive && !settingsModalOpen}
                  badge={badges.about ? <NavigationAttentionBadge /> : undefined}
                  data-testid="nav-about"
                  icon={Info}
                  label={t("nav.about")}
                  to="/about"
                />
              </div>
            </NavigationGroup>
            <div className="mt-auto border-t border-border/70 py-3 space-y-1">
              <NavigationItemAnchor
                data-testid={showGithubSupportLink ? "nav-github-support" : "nav-feedback"}
                href={showGithubSupportLink ? GITHUB_SUPPORT_URL : FEEDBACK_URL}
                icon={showGithubSupportLink ? GithubLogo : ChatCircleText}
                label={showGithubSupportLink ? t("nav.githubSupport") : t("nav.feedback")}
                rel="noopener noreferrer"
                target="_blank"
                trailing={
                  <ArrowSquareOut
                    className="h-3 w-3 shrink-0 opacity-50"
                    weight="fill"
                  />
                }
              />
            </div>
          </nav>
        </aside>
        <main className="flex-1 relative">
          <ScrollArea
            defer
            className="h-full"
          >
            <div className="min-h-full px-2">
              <Outlet />
            </div>
          </ScrollArea>
        </main>
      </div>
    </div>
  );
}
