import { useEffect, useRef } from "react";

type CleanupFn = () => void;
type SetupFn = () => Promise<CleanupFn[]>;

export function useEventListeners(setup: SetupFn, deps: React.DependencyList = []) {
  const cleanupRef = useRef<CleanupFn[]>([]);

  useEffect(() => {
    let mounted = true;

    setup().then((cleanups) => {
      if (mounted) {
        cleanupRef.current = cleanups;
      } else {
        cleanups.forEach((fn) => fn());
      }
    });

    return () => {
      mounted = false;
      cleanupRef.current.forEach((fn) => fn());
      cleanupRef.current = [];
    };
  }, deps);
}