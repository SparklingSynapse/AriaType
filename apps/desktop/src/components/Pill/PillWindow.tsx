import { useEffect, useState, useRef, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { AudioDots } from "./AudioDots";
import { SettingsButton } from "./SettingsButton";
import type { RecordingStatus } from "@/types";
import { logger } from "@/lib/logger";
import { settingsCommands, windowCommands, events, type AppSettings } from "@/lib/tauri";
import type { RecordingStateEvent } from "@/lib/tauri";

export function PillWindow() {
  const { t } = useTranslation();
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
            return;
          }
          latestTaskId.current = task_id;
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
      logger.error("failed_to_start_dragging", { error: String(e) });
    }
  }, []);

  const handleExitComplete = useCallback(async () => {
    // Only hide the native window when in when_recording mode; never mode is
    // handled entirely by the backend, always mode never hides.
    if (indicatorMode === "when_recording") {
      await windowCommands.hidePill();
    }
  }, [indicatorMode]);

  const statusTooltipKey =
    status === "recording"
      ? "pill.status.recording"
      : status === "transcribing" || status === "processing"
        ? "pill.status.transcribing"
        : status === "polishing"
          ? "pill.status.polishing"
          : null;

  return (
    <div
      className="fixed inset-0 flex items-start justify-center pt-1.5 select-none bg-transparent"
      onMouseDown={handleDrag}
      style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
    >
      <AnimatePresence initial={false} onExitComplete={handleExitComplete}>
        {showContent && (
          <motion.div
            key="pill"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="flex flex-col items-center"
          >
            <div
              className="relative flex items-center justify-center rounded-full bg-white dark:bg-zinc-900 shadow-[0_2px_4px_rgba(0,0,0,0.18)] ring-1 ring-black/10 dark:ring-white/20"
              style={{
                paddingLeft: isActive ? 16 : 12,
                paddingRight: isActive ? 16 : 12,
                paddingTop: isActive ? 7 : 5,
                paddingBottom: isActive ? 7 : 5,
                WebkitAppRegion: "no-drag",
              } as React.CSSProperties}
            >
              <AudioDots status={status} audioLevel={audioLevel} hasAudioActivity={hasAudioActivity} />
              <SettingsButton />
            </div>
            <AnimatePresence mode="wait">
              {statusTooltipKey && (
                <motion.div
                  key={statusTooltipKey}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.15, ease: "easeOut" }}
                  className="pointer-events-none mt-2 rounded-full bg-black/60 px-2 py-0.5 text-[9.5px] font-medium text-white/90 shadow-[0_2px_8px_rgba(0,0,0,0.15)] ring-1 ring-white/10 backdrop-blur-md whitespace-nowrap"
                >
                  {t(statusTooltipKey)}
                </motion.div>
              )}
            </AnimatePresence>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
