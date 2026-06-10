import { Outlet, NavLink, useLocation } from "react-router-dom";
import {
  CirclesFour,
  GearSix,
  MagicWand,
  ClockCounterClockwise,
  ChatCircleText,
  ArrowSquareOut,
  ArrowCircleUp,
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
import { OverlayScrollbarsComponent } from "overlayscrollbars-react";
import "overlayscrollbars/overlayscrollbars.css";
import { SettingsModal, type SettingsModalSection } from "./SettingsModal";
import { MODAL_NAV_WIDTH_CLASS } from "./ModalShell";

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

function getNavItemClass(isActive: boolean) {
  return cn(
    "flex w-full items-center gap-3 rounded-[22px] px-3 py-2.5 text-left text-sm transition-colors",
    isActive
      ? "bg-white text-zinc-950 shadow-sm ring-1 ring-zinc-200 dark:bg-zinc-800 dark:text-zinc-100 dark:ring-zinc-700"
      : "text-muted-foreground hover:bg-white/70 hover:text-foreground dark:hover:bg-zinc-800/70",
  );
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

  const primaryNavItems: PrimaryNavItem[] = [
    { to: "/", icon: CirclesFour, label: t("nav.dashboard") },
    { to: "/history", icon: ClockCounterClockwise, label: t("nav.history") },
    { to: "/polish-templates", icon: MagicWand, label: t("nav.polishTemplates") },
  ];
  const settingsNeedsAttention = !hasModel || badges.permission;
  const settingsRouteActive = SETTINGS_ROUTES.has(location.pathname);

  const openSettingsModal = (section: SettingsModalSection = "basics") => {
    setSettingsInitialSection(section);
    setSettingsModalOpen(true);
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
              className="h-10 w-10 rounded-[20px] shadow-sm ring-1 ring-border/80"
            />
            <span className="text-[22px] font-bold text-foreground font-serif italic">
              {t("app.name")}
            </span>
          </div>
          <nav className="px-4 py-4 flex flex-col h-[calc(100%-4.5rem)]">
            <div className="space-y-1">
              {primaryNavItems.map((item) => {
                const to = item.to;
                return (
                  <NavLink
                    key={to}
                    to={to}
                    end={to === "/"}
                    className={({ isActive }) => getNavItemClass(isActive)}
                  >
                    {({ isActive }) => (
                      <>
                        <item.icon
                          className="h-5 w-5 shrink-0"
                          weight={isActive ? "duotone" : "regular"}
                        />
                        <span className="block min-w-0 text-sm font-medium">
                          {item.label}
                        </span>
                      </>
                    )}
                  </NavLink>
                );
              })}

              <button
                type="button"
                onClick={() => openSettingsModal(getSettingsSectionForPath(location.pathname))}
                className={getNavItemClass(settingsModalOpen || settingsRouteActive)}
                data-testid="open-settings-modal"
              >
                <GearSix
                  className="h-5 w-5 shrink-0"
                  weight={settingsModalOpen || settingsRouteActive ? "duotone" : "regular"}
                />
                <span className="block min-w-0 text-sm font-medium">
                  {t("nav.settings")}
                </span>
                {settingsNeedsAttention && (
                  <ArrowCircleUp className="ml-auto h-4 w-4 text-green-500" weight="fill" />
                )}
              </button>

              <NavLink
                to="/about"
                className={({ isActive }) => getNavItemClass(isActive)}
                data-testid="nav-about"
              >
                {({ isActive }) => (
                  <>
                    <Info
                      className="h-5 w-5 shrink-0"
                      weight={isActive ? "duotone" : "regular"}
                    />
                    <span className="block min-w-0 text-sm font-medium">
                      {t("nav.about")}
                    </span>
                    {badges.about && (
                      <ArrowCircleUp className="ml-auto h-4 w-4 text-green-500" weight="fill" />
                    )}
                  </>
                )}
              </NavLink>
            </div>
            <div className="mt-auto border-t border-border/70 py-3 space-y-1">
              <a
                href={showGithubSupportLink ? GITHUB_SUPPORT_URL : FEEDBACK_URL}
                target="_blank"
                rel="noopener noreferrer"
                className={getNavItemClass(false)}
                data-testid={showGithubSupportLink ? "nav-github-support" : "nav-feedback"}
              >
                {showGithubSupportLink ? (
                  <GithubLogo className="h-5 w-5 shrink-0" weight="regular" />
                ) : (
                  <ChatCircleText className="h-5 w-5 shrink-0" weight="regular" />
                )}
                <span className="block min-w-0 text-sm font-medium">
                  {showGithubSupportLink ? t("nav.githubSupport") : t("nav.feedback")}
                </span>
                <ArrowSquareOut className="ml-auto h-3 w-3 opacity-50" weight="fill" />
              </a>
            </div>
          </nav>
        </aside>
        <main className="flex-1 relative">
          <OverlayScrollbarsComponent
            defer
            className="h-full"
            options={{
              showNativeOverlaidScrollbars: false,
              scrollbars: {
                theme: "os-theme-dark",
                visibility: "auto",
                autoHide: "scroll",
                autoHideDelay: 300,
                autoHideSuspend: false,
              },
            }}
          >
            <div className="min-h-full px-2">
              <Outlet />
            </div>
          </OverlayScrollbarsComponent>
        </main>
      </div>
    </div>
  );
}
