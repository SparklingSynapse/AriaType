import { useState, useEffect, useCallback } from "react";
import { settingsCommands, events, type AppSettings } from "@/lib/tauri";

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    settingsCommands.getSettings()
      .then((data) => { setSettings(data); setError(null); })
      .catch((err) => setError(err instanceof Error ? err.message : "Failed to load settings"))
      .finally(() => setLoading(false));

    let unlisten: (() => void) | undefined;
    events.onSettingsChanged((s) => setSettings(s)).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, []);

  const updateSetting = useCallback(async (key: string, value: unknown) => {
    try {
      await settingsCommands.updateSettings(key, value);
      // No need to reload — backend emits settings-changed event which updates state above
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update setting");
      throw err;
    }
  }, []);

  return { settings, loading, error, updateSetting };
}
