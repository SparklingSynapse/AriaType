import { Outlet, NavLink, useLocation } from "react-router-dom";
import {
  Settings,
  Keyboard,
  Brain,
  Info,
  Shield,
  MessageSquare,
  ExternalLink,
  ArrowUpCircle,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";
import logoLight from "../../../assets/ariatype-light.png";
import logoDark from "../../../assets/ariatype-dark.png";
import { modelCommands, events, systemCommands } from "@/lib/tauri";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { useEffect, useState, useCallback } from "react";
import { useOnboarding } from "@/hooks/useOnboarding";
import { OnboardingGuide } from "./OnboardingGuide";
import { useNavBadges } from "@/hooks/useNavBadges";
import { OverlayScrollbarsComponent } from "overlayscrollbars-react";
import "overlayscrollbars/overlayscrollbars.css";

const FEEDBACK_URL = "https://admitted-wave-091.notion.site/312c21a6f899804093ddf0f8db898318?pvs=105";

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
    { to: "/", icon: Settings, label: t("nav.general") },
    { to: "/hotkey", icon: Keyboard, label: t("nav.hotkey") },
    { to: "/private-ai", icon: Brain, label: t("nav.privateAi"), badge: !hasModel },
    { to: "/permission", icon: Shield, label: t("nav.permission"), badge: badges.permission },
    { to: "/about", icon: Info, label: t("nav.about"), badge: badges.about },
    { type: "external" as const, icon: MessageSquare, label: t("nav.feedback"), href: FEEDBACK_URL },
  ];

  const handleOnboardingClose = useCallback(async () => {
    closeOnboarding();
    const micStatus = await systemCommands.checkPermission("microphone").catch(() => null);
    if (micStatus === "not_determined") {
      systemCommands.applyPermission("microphone").catch(console.error);
    }
    const axStatus = await systemCommands.checkPermission("accessibility").catch(() => "granted");
    if (axStatus !== "granted") {
      systemCommands.applyPermission("accessibility").catch(console.error);
    }
  }, [closeOnboarding]);

  const checkModel = useCallback(async () => {
    try {
      const models = await modelCommands.getModels();
      setHasModel(models.some((m) => m.downloaded));
    } catch (err) {
      console.error("Failed to check models:", err);
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
        <aside className="w-56 border-r border-border bg-card  pt-7">
          <div className="pr-16 pl-4 py-4 border-b border-border flex items-center">
            <img src={logoLight} alt="AriaType" className="w-full dark:hidden" />
            <img src={logoDark} alt="AriaType" className="w-full hidden dark:block" />
          </div>
          <nav className="p-4 flex flex-col h-[calc(100%-4rem)]">
            <div className="space-y-1 flex-1">
              {navItems.filter(i => i.type !== "external").map((item) => (
                <NavLink
                  key={(item as { to: string }).to}
                  to={(item as { to: string }).to}
                  end={(item as { to: string }).to === "/"}
                  className={({ isActive }) =>
                    cn(
                      "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-all duration-200",
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
            <div className="border-t border-border pt-3 space-y-1">
              {navItems.filter(i => i.type === "external").map((item) => (
                <a
                  key={(item as { href: string }).href}
                  href={(item as { href: string }).href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm text-muted-foreground hover:bg-secondary/80 hover:text-secondary-foreground transition-all duration-200"
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
              scrollbars: {
                theme: "os-theme-dark",
                autoHide: "leave",
                autoHideDelay: 300,
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
