import { trackEvent } from "@aptabase/tauri";
import { settingsCommands } from "@/lib/tauri";

let _optIn: boolean | null = null;

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
  track: (event: string, props?: Record<string, string | number>) => {
    if (_optIn) {
      trackEvent(event, props).catch((err) => {
        console.error("Failed to track event:", err);
      });
    }
  },
};
