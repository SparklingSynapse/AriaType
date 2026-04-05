import { settingsCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";

let _optIn: boolean | null = null;
let _trackEventModule: ((eventName: string, props?: Record<string, string | number>) => Promise<void>) | null = null;

function isTauriEnvironment(): boolean {
  try {
    return (
      typeof window !== "undefined" &&
      typeof (window as any).__TAURI_IPC__ === "function" &&
      typeof (window as any).__TAURI_INTERNALS__ === "object"
    );
  } catch {
    return false;
  }
}

async function getTrackEvent(): Promise<typeof _trackEventModule> {
  if (_trackEventModule) return _trackEventModule;
  if (!isTauriEnvironment()) return null;
  
  try {
    const module = await import("@aptabase/tauri");
    _trackEventModule = module.trackEvent;
    return _trackEventModule;
  } catch {
    return null;
  }
}

export async function initAnalytics() {
  try {
    const settings = await settingsCommands.getSettings();
    _optIn = settings.analytics_opt_in ?? false;
  } catch {
    _optIn = false;
  }
}

export function isAnalyticsEnabled(): boolean {
  return _optIn === true;
}

export async function setAnalyticsEnabled(enabled: boolean) {
  _optIn = enabled;
  await settingsCommands.updateSettings("analytics_opt_in", enabled);
}

export const analytics = {
  track: async (event: string, props?: Record<string, string | number>) => {
    if (!_optIn) {
      return;
    }
    if (!isTauriEnvironment()) {
      return;
    }
    try {
      const trackEvent = await getTrackEvent();
      if (trackEvent) {
        await trackEvent(event, props);
      }
    } catch (err) {
      logger.error("failed_to_track_event", { error: String(err) });
    }
  },
};
