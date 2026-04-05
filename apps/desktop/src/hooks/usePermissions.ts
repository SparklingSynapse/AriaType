import { useState, useEffect, useCallback, useRef } from "react";
import { systemCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";

export type MicrophoneStatus = "granted" | "denied" | "not_determined" | null;

export interface PermissionsState {
  accessibilityGranted: boolean | null;
  inputMonitoringGranted: boolean | null;
  microphoneStatus: MicrophoneStatus;
  isLoading: boolean;
  checkPermissions: () => void;
  handleApplyPermission: (kind: "accessibility" | "input_monitoring" | "microphone") => Promise<void>;
}

export function usePermissions(): PermissionsState {
  const [accessibilityGranted, setAccessibilityGranted] = useState<boolean | null>(null);
  const [inputMonitoringGranted, setInputMonitoringGranted] = useState<boolean | null>(null);
  const [microphoneStatus, setMicrophoneStatus] = useState<MicrophoneStatus>(null);
  const [isLoading, setIsLoading] = useState(true);
  const mounted = useRef(true);

  const checkPermissions = useCallback(() => {
    setIsLoading(true);
    Promise.all([
      systemCommands.checkPermission("accessibility").catch(() => null),
      systemCommands.checkPermission("input_monitoring").catch(() => null),
      systemCommands.checkPermission("microphone").catch(() => null),
    ]).then(([accessibility, inputMonitoring, microphone]) => {
      if (mounted.current) {
        setAccessibilityGranted(accessibility === "granted");
        setInputMonitoringGranted(inputMonitoring === "granted");
        setMicrophoneStatus(microphone as MicrophoneStatus);
        setIsLoading(false);
      }
    });
  }, []);

  useEffect(() => {
    mounted.current = true;
    checkPermissions();
    return () => {
      mounted.current = false;
    };
  }, [checkPermissions]);

  useEffect(() => {
    const onFocus = () => checkPermissions();
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onFocus);
    };
  }, [checkPermissions]);

  const handleApplyPermission = async (
    kind: "accessibility" | "input_monitoring" | "microphone"
  ) => {
    analytics.track(AnalyticsEvents.PERMISSION_GRANT_REQUESTED, {
      permission: kind,
    });
    try {
      await systemCommands.applyPermission(kind);
      checkPermissions();
    } catch (err) {
      logger.error("failed_to_apply_permission", { kind, error: String(err) });
    }
  };

  return {
    accessibilityGranted,
    inputMonitoringGranted,
    microphoneStatus,
    isLoading,
    checkPermissions,
    handleApplyPermission,
  };
}
