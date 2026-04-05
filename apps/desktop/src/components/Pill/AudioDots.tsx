import { motion } from "framer-motion";
import { useState, useEffect } from "react";
import type { RecordingStatus } from "@/types";

function useIsDark() {
  const [isDark, setIsDark] = useState(() =>
    document.documentElement.classList.contains("dark")
  );
  useEffect(() => {
    const observer = new MutationObserver(() => {
      setIsDark(document.documentElement.classList.contains("dark"));
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  }, []);
  return isDark;
}

interface AudioDotsProps {
  status: RecordingStatus;
  audioLevel: number;
  hasAudioActivity?: boolean;
}

// Keep in sync with AUDIO_ACTIVITY_OFF_THRESHOLD in audio.rs
const AUDIO_ACTIVITY_THRESHOLD = 25;

const DOT_H = 5;
const IDLE_W = 10;
const ACTIVE_W = 5;
const TOTAL_W = IDLE_W * 3; // 30px — fixed bounding box, never changes
const R = DOT_H / 2; // 2.5px

// Idle: left-rounded | square | right-rounded
const IDLE_RADIUS = [
  `${R}px 0px 0px ${R}px`,
  `0px 0px 0px 0px`,
  `0px ${R}px ${R}px 0px`,
];
const ACTIVE_RADIUS = `${R}px ${R}px ${R}px ${R}px`;

/** Returns the fixed positioning style for each dot index. */
function dotPositioning(i: number) {
  if (i === 0) {
    // Left dot: fixed at left edge
    return { left: 0 };
  } else if (i === 1) {
    // Middle dot: centered with transform
    return { left: "50%", x: "-50%" };
  } else {
    // Right dot: fixed at right edge
    return { right: 0 };
  }
}

/** Returns the animated width for each dot. */
function dotWidth(isRecording: boolean) {
  return isRecording ? ACTIVE_W : IDLE_W;
}

/**
 * Per-property transitions with sequenced timing:
 *
 * Idle → Recording:  borderRadius first (fast), then width (slightly slower)
 * Recording → Idle:  width first (fast), then borderRadius (slightly slower)
 */
function dotTransition(i: number, isRecording: boolean) {
  // Stagger: left→right when entering, right→left when exiting
  const stagger = isRecording ? i * 0.04 : (2 - i) * 0.04;
  const OFFSET = 0.07; // delay between the "first" and "second" property group

  if (isRecording) {
    // borderRadius leads, width follows
    return {
      borderRadius: {
        duration: 0.13,
        ease: "easeOut" as const,
        delay: stagger,
      },
      width: {
        duration: 0.18,
        ease: "easeOut" as const,
        delay: stagger + OFFSET,
      },
      backgroundColor: { duration: 0.28, ease: "easeOut" as const },
    };
  } else {
    // width leads, borderRadius follows
    return {
      width: { duration: 0.13, ease: "easeOut" as const, delay: stagger },
      borderRadius: {
        duration: 0.18,
        ease: "easeOut" as const,
        delay: stagger + OFFSET,
      },
      backgroundColor: { duration: 0.28, ease: "easeOut" as const },
    };
  }
}

export function AudioDots({ status, audioLevel, hasAudioActivity }: AudioDotsProps) {
  const isDark = useIsDark();
  const isRecording = status === "recording";
  const isSttRunning = status === "transcribing" || status === "processing";
  const isPolishing = status === "polishing";
  const isError = status === "error";
  const hasAudio = hasAudioActivity ?? audioLevel > AUDIO_ACTIVITY_THRESHOLD;
  const showAnimatedState = isSttRunning || isPolishing;
  const processingColor = isPolishing
    ? ["rgb(72, 148, 255)", "rgb(214, 231, 255)", "rgb(72, 148, 255)"]
    : ["rgb(0, 170, 255)", "rgb(205, 219, 255)", "rgb(0, 170, 255)"];

  return (
    <div className="flex items-center justify-center w-8 h-4">
      <div style={{ position: "relative", width: TOTAL_W, height: DOT_H }}>
        {[0, 1, 2].map((i) => {
          const positioning = dotPositioning(i);
          const width = dotWidth(isRecording);
          const baseTransition = dotTransition(i, isRecording);
          return (
            <motion.div
              key={i}
              style={{
                position: "absolute",
                top: 0,
                ...positioning,
              }}
              animate={{
                width,
                height: DOT_H,
                borderRadius: isRecording ? ACTIVE_RADIUS : IDLE_RADIUS[i],
                backgroundColor: isError
                  ? "rgb(239, 68, 68)"
                  : showAnimatedState
                    ? processingColor
                    : isRecording
                      ? hasAudio
                        ? "rgb(3, 227, 52)"
                        : isDark ? "rgba(255,255,255,0.6)" : "rgba(0,0,0,0.15)"
                      : isDark ? "rgba(255,255,255,0.5)" : "rgba(0,0,0,0.15)",
              }}
              transition={
                showAnimatedState
                  ? {
                      ...baseTransition,
                      backgroundColor: {
                        duration: 1.4,
                        repeat: Infinity,
                        repeatType: "reverse",
                        ease: "easeInOut",
                      },
                    }
                  : baseTransition
              }
            />
          );
        })}
      </div>
    </div>
  );
}
