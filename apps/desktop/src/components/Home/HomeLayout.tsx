import { Outlet, NavLink, useLocation } from "react-router-dom";
import {
  Settings,
  Keyboard,
  Brain,
  CloudCog,
  Info,
  Shield,
  MessageSquare,
  ExternalLink,
  ArrowUpCircle,
  LayoutDashboard,
  History,
  Sparkles,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";
import logo from "../../../assets/logo.png";
import { modelCommands, events, systemCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useEffect, useState, useCallback } from "react";
import { useOnboarding } from "@/hooks/useOnboarding";
import { OnboardingGuide } from "./OnboardingGuide";
import { useNavBadges } from "@/hooks/useNavBadges";
import { OverlayScrollbarsComponent } from "overlayscrollbars-react";
import "overlayscrollbars/overlayscrollbars.css";

const FEEDBACK_URL = "https://github.com/SparklingSynapse/AriaType/issues/new";

export function HomeLayout() {
  const { t } = useTranslation();
  const [hasModel, setHasModel] = useState(true);
  const { isOpen, closeOnboarding } = useOnboarding();
  const badges = useNavBadges();
  const location = useLocation();

  useEffect(() => {
    analytics.track(AnalyticsEvents.SCREEN_VIEW, {
      screen_name: location.pathname,
    });
  }, [location]);

  const navItems = [
    { to: "/", icon: LayoutDashboard, label: t("nav.dashboard") },
    { to: "/settings", icon: Settings, label: t("nav.general") },
    { to: "/hotkey", icon: Keyboard, label: t("nav.hotkey") },
    { to: "/private-ai", icon: Brain, label: t("nav.privateAi"), badge: !hasModel },
    { to: "/cloud", icon: CloudCog, label: t("cloud.title") },
    { to: "/polish-templates", icon: Sparkles, label: t("nav.polishTemplates") },
    { to: "/permission", icon: Shield, label: t("nav.permission"), badge: badges.permission },
    { to: "/history", icon: History, label: t("nav.history") },
    { to: "/about", icon: Info, label: t("nav.about"), badge: badges.about },
    { type: "external" as const, icon: MessageSquare, label: t("nav.feedback"), href: FEEDBACK_URL },
  ];

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

  useEffect(() => {
    checkModel();

    let unlistenComplete: (() => void) | undefined;
    let unlistenDeleted: (() => void) | undefined;
    const setup = async () => {
      unlistenComplete = await events.onModelDownloadComplete(() => checkModel());
      unlistenDeleted = await events.onModelDeleted(() => checkModel());
    };
    setup();

    return () => {
      unlistenComplete?.();
      unlistenDeleted?.();
    };
  }, [checkModel]);

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Drag region for the overlay title bar */}
      <div
        className="h-7 flex-shrink-0 top-0 left-0 right-0 absolute"
        data-tauri-drag-region
      />
      <div className="flex flex-1 overflow-hidden ">
        <OnboardingGuide isOpen={isOpen} onClose={handleOnboardingClose} />
        <aside className="w-56 border-r border-border bg-card pt-7">
          <div className="px-6 py-6 border-b border-border flex items-center gap-3.5">
            <img
              src={logo}
              alt="AriaType"
              className="h-10 w-10 rounded-2xl shadow-sm ring-1 ring-border"
            />
            <span className="text-2xl font-bold tracking-tight text-foreground font-serif italic">
              {t("app.name")}
            </span>
          </div>
          <nav className="p-4 flex flex-col h-[calc(100%-4.5rem)]">
            <div className="space-y-1 flex-1">
              {navItems.filter(i => i.type !== "external").map((item) => (
                <NavLink
                  key={(item as { to: string }).to}
                  to={(item as { to: string }).to}
                  end={(item as { to: string }).to === "/"}
                  className={({ isActive }) =>
                    cn(
                      "flex items-center gap-3 rounded-full px-4 py-2.5 text-sm transition-all duration-200",
                      isActive
                        ? "bg-primary text-primary-foreground font-medium"
                        : "text-muted-foreground hover:bg-secondary/80 hover:text-secondary-foreground",
                    )
                  }
                >
                  <item.icon className="h-4 w-4" />
                  {item.label}
                  {"badge" in item && item.badge && (
                    <ArrowUpCircle className="h-4 w-4 text-green-500 ml-auto" />
                  )}
                </NavLink>
              ))}
            </div>
            <div className="border-t border-border py-3 space-y-1">
              {navItems.filter(i => i.type === "external").map((item) => (
                <a
                  key={(item as { href: string }).href}
                  href={(item as { href: string }).href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-3 rounded-full px-4 py-2.5 text-sm text-muted-foreground hover:bg-secondary/80 hover:text-secondary-foreground transition-all duration-200"
                >
                  <item.icon className="h-4 w-4" />
                  {item.label}
                  <ExternalLink className="h-3 w-3 ml-auto opacity-50" />
                </a>
              ))}
            </div>
          </nav>
        </aside>
        <main className="flex-1">
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
            <Outlet />
          </OverlayScrollbarsComponent>
        </main>
      </div>
    </div>
  );
}
