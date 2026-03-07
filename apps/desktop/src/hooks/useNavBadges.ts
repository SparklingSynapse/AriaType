import { useState, useEffect, useCallback } from "react";
import { systemCommands } from "@/lib/tauri";
import { getVersion } from "@tauri-apps/api/app";
import { UPDATE_CHECK_URL } from "@ariatype/shared";
import { compareVersions, validate } from "compare-versions";

function isNewerVersion(latest: string, current: string): boolean {
  if (!validate(latest) || !validate(current)) {
    console.warn("Invalid version format", { latest, current });
    return false;
  }
  return compareVersions(latest, current) > 0;
}

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
      const [response, appVersion] = await Promise.all([
        fetch(UPDATE_CHECK_URL),
        getVersion(),
      ]);
      if (!response.ok) return;
      const data = await response.json();
      setAboutBadge(!!data.version && isNewerVersion(data.version, appVersion));
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
