import { useEffect, useState, useRef, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { BorderBeam } from "border-beam";
import { AudioDots } from "./AudioDots";
import { SettingsButton } from "./SettingsButton";
import type { RecordingStatus } from "@/types";
import { logger } from "@/lib/logger";
import { settingsCommands, windowCommands, events, type AppSettings } from "@/lib/tauri";
import type { RecordingStateEvent } from "@/lib/tauri";

// Font-size scaling factor for pill size (1-5 levels)
// Applied to document root so Tailwind rem units scale proportionally
const PILL_SIZE_SCALE: Record<number, number> = {
  1: 0.875,  // 14px / 16px
  2: 1,      // default (16px)
  3: 1.125,  // 18px / 16px
  4: 1.25,   // 20px / 16px
  5: 1.375,  // 22px / 16px
};

export function PillWindow() {
  const [status, setStatus] = useState<RecordingStatus>("idle");
  const [audioLevel, setAudioLevel] = useState(0);
  const [hasAudioActivity, setHasAudioActivity] = useState(false);
  const [indicatorMode, setIndicatorMode] = useState("always");
  const [pillSize, setPillSize] = useState(2);
  const latestTaskId = useRef<number>(0);

  // Apply font-size to document root for rem-based scaling
  useEffect(() => {
    const scale = PILL_SIZE_SCALE[pillSize] ?? 1;
    document.documentElement.style.fontSize = `${16 * scale}px`;
    return () => {
      document.documentElement.style.fontSize = "16px";
    };
  }, [pillSize]);

  useEffect(() => {
    settingsCommands.getSettings().then((s) => {
      setIndicatorMode(s.pill_indicator_mode ?? "always");
      setPillSize(s.pill_size ?? 2);
    });

    let unlisten: (() => void) | undefined;
    events.onSettingsChanged((s: AppSettings) => {
      setIndicatorMode(s.pill_indicator_mode ?? "always");
      setPillSize(s.pill_size ?? 2);
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

  const beamActive = status === "transcribing" || status === "processing" || status === "polishing";

  return (
    <div
      className="fixed inset-0 flex items-start justify-center pt-4 select-none bg-transparent"
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
            <BorderBeam
              active={beamActive}
              size="sm"
              borderRadius={9999}
              theme="dark"
              colorVariant="ocean"
              strength={0.5}
              brightness={2}
              duration={1}
              className="shadow-[0_0_10px_rgba(0,0,0,0.3),0_4px_12px_rgba(0,0,0,0.3)]"
            >
              <div
                className="relative flex items-center justify-center rounded-full bg-[#1d1d1d] shadow-[inset_0_0_0_1px_#2c2f3685,inset_0_0_50px_#ffffff05]"
                style={{
                  paddingLeft: "0.75rem",
                  paddingRight: "0.75rem",
                  paddingTop: "0.3125rem",
                  paddingBottom: "0.3125rem",
                  WebkitAppRegion: "no-drag",
                } as React.CSSProperties}
              >
                <AudioDots status={status} audioLevel={audioLevel} hasAudioActivity={hasAudioActivity} />
                <SettingsButton />
              </div>
            </BorderBeam>
            {/* TODO: Re-enable tooltip for future use */}
            {/* <AnimatePresence mode="wait">
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
            </AnimatePresence> */}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
