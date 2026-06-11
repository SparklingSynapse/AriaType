import { useEffect, useRef } from "react";
import { logger } from "@/lib/logger";

type CleanupFn = () => void | Promise<void>;
type SetupFn = () => Promise<CleanupFn[]>;

function runCleanup(fn: CleanupFn): void {
  try {
    void Promise.resolve(fn()).catch((error: unknown) => {
      logger.debug("event_listener_cleanup_failed", { error });
    });
  } catch (error) {
    logger.debug("event_listener_cleanup_failed", { error });
  }
}

export function useEventListeners(setup: SetupFn, deps: React.DependencyList = []) {
  const cleanupRef = useRef<CleanupFn[]>([]);

  useEffect(() => {
    let mounted = true;

    setup().then((cleanups) => {
      if (mounted) {
        cleanupRef.current = cleanups;
      } else {
        cleanups.forEach(runCleanup);
      }
    });

    return () => {
      mounted = false;
      cleanupRef.current.forEach(runCleanup);
      cleanupRef.current = [];
    };
  }, deps);
}
