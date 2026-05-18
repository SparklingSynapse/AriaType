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
import type { PillTooltipEvent, RecordingStateEvent } from "@/lib/tauri";

// Font-size scaling factor for pill size (1-5 levels)
// Applied to document root so Tailwind rem units scale proportionally
const PILL_SIZE_SCALE: Record<number, number> = {
  1: 0.875,  // 14px / 16px
  2: 1,      // default (16px)
  3: 1.125,  // 18px / 16px
  4: 1.25,   // 20px / 16px
  5: 1.375,  // 22px / 16px
};

const DEFAULT_PILL_BACKGROUND_COLOR = "#1d1d1d";
const DEFAULT_PILL_BACKGROUND_OPACITY = 1;

function normalizePillBackgroundColor(color: string | undefined): string {
  if (!color || !/^#[0-9a-f]{6}$/i.test(color)) {
    return DEFAULT_PILL_BACKGROUND_COLOR;
  }

  return color.toLowerCase();
}

function isLightHexColor(color: string): boolean {
  const normalized = normalizePillBackgroundColor(color);
  const red = Number.parseInt(normalized.slice(1, 3), 16);
  const green = Number.parseInt(normalized.slice(3, 5), 16);
  const blue = Number.parseInt(normalized.slice(5, 7), 16);
  const luminance = (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255;

  return luminance > 0.62;
}

function normalizePillBackgroundOpacity(opacity: number | undefined): number {
  if (typeof opacity !== "number" || !Number.isFinite(opacity)) {
    return DEFAULT_PILL_BACKGROUND_OPACITY;
  }

  return Math.min(1, Math.max(0.2, opacity));
}

function hexToRgba(color: string, opacity: number): string {
  const normalized = normalizePillBackgroundColor(color);
  const red = Number.parseInt(normalized.slice(1, 3), 16);
  const green = Number.parseInt(normalized.slice(3, 5), 16);
  const blue = Number.parseInt(normalized.slice(5, 7), 16);

  return `rgba(${red}, ${green}, ${blue}, ${normalizePillBackgroundOpacity(opacity)})`;
}

function shouldRequestNativeShowForTooltip(event: PillTooltipEvent): boolean {
  return event.task_id === null || event.task_id === undefined;
}

export function PillWindow() {
  const [status, setStatus] = useState<RecordingStatus>("idle");
  const [audioLevel, setAudioLevel] = useState(0);
  const [hasAudioActivity, setHasAudioActivity] = useState(false);
  const [indicatorMode, setIndicatorMode] = useState("always");
  const [pillSize, setPillSize] = useState(2);
  const [pillBackgroundColor, setPillBackgroundColor] = useState(DEFAULT_PILL_BACKGROUND_COLOR);
  const [pillBackgroundOpacity, setPillBackgroundOpacity] = useState(DEFAULT_PILL_BACKGROUND_OPACITY);
  const [tooltip, setTooltip] = useState<PillTooltipEvent | null>(null);
  const latestTaskId = useRef<number>(0);
  const tooltipTimer = useRef<number | undefined>(undefined);
  const indicatorModeRef = useRef(indicatorMode);

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
      const nextIndicatorMode = s.pill_indicator_mode ?? "always";
      indicatorModeRef.current = nextIndicatorMode;
      setIndicatorMode(nextIndicatorMode);
      setPillSize(s.pill_size ?? 2);
      setPillBackgroundColor(normalizePillBackgroundColor(s.pill_background_color));
      setPillBackgroundOpacity(normalizePillBackgroundOpacity(s.pill_background_opacity));
    });

    let unlisten: (() => void) | undefined;
    events.onSettingsChanged((s: AppSettings) => {
      const nextIndicatorMode = s.pill_indicator_mode ?? "always";
      indicatorModeRef.current = nextIndicatorMode;
      setIndicatorMode(nextIndicatorMode);
      setPillSize(s.pill_size ?? 2);
      setPillBackgroundColor(normalizePillBackgroundColor(s.pill_background_color));
      setPillBackgroundOpacity(normalizePillBackgroundOpacity(s.pill_background_opacity));
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, []);

  useEffect(() => {
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenLevel: UnlistenFn | undefined;
    let unlistenActivity: UnlistenFn | undefined;
    let unlistenTooltip: UnlistenFn | undefined;

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

      unlistenTooltip = await events.onPillTooltip((event) => {
        if (typeof event.task_id === "number" && event.task_id < latestTaskId.current) {
          return;
        }
        setTooltip(event);
        if (indicatorModeRef.current === "when_recording" && shouldRequestNativeShowForTooltip(event)) {
          void windowCommands.showPill().catch((error) => {
            logger.error("failed_to_show_pill_for_tooltip", { error: String(error) });
          });
        }
        if (tooltipTimer.current !== undefined) {
          window.clearTimeout(tooltipTimer.current);
        }
        tooltipTimer.current = window.setTimeout(() => {
          setTooltip(null);
          tooltipTimer.current = undefined;
        }, Math.max(0, event.duration_ms));
      });
    };

    setupListeners();

    return () => {
      unlistenStatus?.();
      unlistenLevel?.();
      unlistenActivity?.();
      unlistenTooltip?.();
      if (tooltipTimer.current !== undefined) {
        window.clearTimeout(tooltipTimer.current);
      }
    };
  }, []);

  const isActive = status !== "idle";

  // For "when_recording" mode: pill animates in/out and hides the OS window after exit.
  // For "always" mode: pill content is always rendered, no show/hide animation.
  const showPillBody = indicatorMode === "always" || isActive;
  const showTooltip = indicatorMode !== "never" && tooltip !== null;
  const showContent = showPillBody || showTooltip;

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
  const isLightBackground = isLightHexColor(pillBackgroundColor);
  const idleDotColor = isLightBackground ? "rgba(39,39,42,0.72)" : "rgba(255,255,255,0.7)";
  const pillBackground = hexToRgba(pillBackgroundColor, pillBackgroundOpacity);

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
            {showPillBody && (
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
                    backgroundColor: pillBackground,
                    paddingLeft: "0.75rem",
                    paddingRight: "0.75rem",
                    paddingTop: "0.3125rem",
                    paddingBottom: "0.3125rem",
                    WebkitAppRegion: "no-drag",
                  } as React.CSSProperties}
                >
                  <AudioDots
                    status={status}
                    audioLevel={audioLevel}
                    hasAudioActivity={hasAudioActivity}
                    idleColor={idleDotColor}
                  />
                  <SettingsButton isLightBackground={isLightBackground} />
                </div>
              </BorderBeam>
            )}
            <AnimatePresence mode="wait">
              {showTooltip && (
                <motion.div
                  key={`${tooltip.task_id ?? "global"}:${tooltip.message}`}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.15, ease: "easeOut" }}
                  className={`pointer-events-none ${showPillBody ? "mt-2" : "mt-0"} max-w-[calc(100vw-1rem)] truncate rounded-full bg-black/60 px-2.5 py-0.5 text-[9.5px] font-medium text-white/90 shadow-[0_2px_8px_rgba(0,0,0,0.15)] ring-1 ring-white/10 backdrop-blur-md`}
                >
                  {tooltip.message}
                </motion.div>
              )}
            </AnimatePresence>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
