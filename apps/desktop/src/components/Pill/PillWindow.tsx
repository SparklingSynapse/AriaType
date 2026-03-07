import { useEffect, useState, useRef, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { AudioDots } from "./AudioDots";
import { SettingsButton } from "./SettingsButton";
import type { RecordingStatus } from "@/types";
import { settingsCommands, windowCommands, events, type AppSettings } from "@/lib/tauri";
import type { RecordingStateEvent } from "@/lib/tauri";

export function PillWindow() {
  const [status, setStatus] = useState<RecordingStatus>("idle");
  const [audioLevel, setAudioLevel] = useState(0);
  const [hasAudioActivity, setHasAudioActivity] = useState(false);
  const [indicatorMode, setIndicatorMode] = useState("always");
  const latestTaskId = useRef<number>(0);

  useEffect(() => {
    settingsCommands.getSettings().then((s) => {
      setIndicatorMode(s.pill_indicator_mode ?? "always");
    });

    let unlisten: (() => void) | undefined;
    events.onSettingsChanged((s: AppSettings) => {
      setIndicatorMode(s.pill_indicator_mode ?? "always");
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, []);

  useEffect(() => {
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenLevel: UnlistenFn | undefined;
    let unlistenActivity: UnlistenFn | undefined;

    const setupListeners = async () => {
      unlistenStatus = await listen<RecordingStateEvent>(
        "recording-state-changed",
        (event: { payload: RecordingStateEvent }) => {
          const { status, task_id } = event.payload;
          if (task_id < latestTaskId.current) {
            console.log(`[PillWindow] Ignoring stale event (task_id=${task_id}, latest=${latestTaskId.current})`);
            return;
          }
          latestTaskId.current = task_id;
          console.log(`[PillWindow] Received status: ${status} (task_id=${task_id})`);
          const next = status as RecordingStatus;
          if (next === "recording") {
            setHasAudioActivity(false);
          }
          setStatus(next);
        }
      );

      unlistenLevel = await listen<number>(
        "audio-level",
        (event: { payload: number }) => {
          setAudioLevel(event.payload);
        }
      );

      unlistenActivity = await listen<boolean>(
        "audio-activity",
        (event: { payload: boolean }) => {
          setHasAudioActivity(event.payload);
        }
      );
    };

    setupListeners();

    return () => {
      unlistenStatus?.();
      unlistenLevel?.();
      unlistenActivity?.();
    };
  }, []);

  const isActive = status !== "idle";

  // For "when_recording" mode: pill animates in/out and hides the OS window after exit.
  // For "always" mode: pill content is always rendered, no show/hide animation.
  const showContent = indicatorMode === "always" ? true : isActive;

  const handleDrag = useCallback(async () => {
    try {
      await getCurrentWindow().startDragging();
    } catch (e) {
      console.error("Failed to start dragging:", e);
    }
  }, []);

  const handleExitComplete = useCallback(async () => {
    // Only hide the native window when in when_recording mode; never mode is
    // handled entirely by the backend, always mode never hides.
    if (indicatorMode === "when_recording") {
      await windowCommands.hidePill();
    }
  }, [indicatorMode]);

  return (
    <div
      className="fixed inset-0 flex items-center justify-center select-none bg-transparent"
      onMouseDown={handleDrag}
      style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
    >
      <AnimatePresence onExitComplete={handleExitComplete}>
        {showContent && (
          <motion.div
            key="pill"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="relative flex items-center justify-center rounded-full bg-white dark:bg-zinc-900 shadow-[0_2px_4px_rgba(0,0,0,0.18)] ring-1 ring-black/10 dark:ring-white/20"
            style={{
              paddingLeft: isActive ? 16 : 12,
              paddingRight: isActive ? 16 : 12,
              paddingTop: isActive ? 9 : 7,
              paddingBottom: isActive ? 9 : 7,
              WebkitAppRegion: "no-drag",
            } as React.CSSProperties}
          >
            <AudioDots status={status} audioLevel={audioLevel} hasAudioActivity={hasAudioActivity} />
            <SettingsButton />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
