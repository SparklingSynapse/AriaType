import { useState, useEffect, useCallback } from "react";
import { systemCommands } from "@/lib/tauri";
import { checkAppUpdate } from "@/lib/updateCheck";

export interface NavBadges {
  permission: boolean;
  about: boolean;
}

export function useNavBadges(): NavBadges {
  const [permissionBadge, setPermissionBadge] = useState(false);
  const [aboutBadge, setAboutBadge] = useState(false);

  const checkPermissions = useCallback(async () => {
    try {
      const [mic, ax] = await Promise.all([
        systemCommands.checkPermission("microphone"),
        systemCommands.checkPermission("accessibility"),
      ]);
      setPermissionBadge(mic !== "granted" || ax !== "granted");
    } catch {
      // ignore
    }
  }, []);

  const checkUpdate = useCallback(async () => {
    try {
      const result = await checkAppUpdate();
      setAboutBadge(Boolean(result.update));
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    checkPermissions();
    checkUpdate();

    const onFocus = () => checkPermissions();
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onFocus);
    };
  }, [checkPermissions, checkUpdate]);

  return { permission: permissionBadge, about: aboutBadge };
}
