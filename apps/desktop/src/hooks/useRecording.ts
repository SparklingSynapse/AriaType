import { useState, useEffect, useCallback, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { audioCommands, settingsCommands } from "@/lib/tauri";
import { showToast } from "@/lib/toast";
import { logger } from "@/lib/logger";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import type { RecordingStatus } from "@/types";

export function useRecording() {
  const [status, setStatus] = useState<RecordingStatus>("idle");
  const [audioLevel, setAudioLevel] = useState(0);
  const [hotkey, setHotkey] = useState("shift+space");
  const recordingStartTime = useRef<number | null>(null);
  const prevStatusRef = useRef<RecordingStatus>("idle");
  const latestTaskIdRef = useRef(0);

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const settings = await settingsCommands.getSettings();
        const defaultHotkey =
          settings?.shortcut_profiles?.dictate?.hotkey ||
          "shift+space";
        setHotkey(defaultHotkey);
      } catch (err) {
        logger.error("failed_to_load_settings", { error: String(err) });
      }
    };
    loadSettings();
  }, []);

  useEffect(() => {
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenLevel: UnlistenFn | undefined;

    const setupListeners = async () => {
      unlistenStatus = await listen<{ status: RecordingStatus; task_id: number }>(
        "recording-state-changed",
        (event) => {
          const { status: newStatus, task_id: taskId } = event.payload;
          if (taskId < latestTaskIdRef.current) {
            return;
          }

          latestTaskIdRef.current = taskId;
          const prevStatus = prevStatusRef.current;

          if (newStatus === "recording" && prevStatus !== "recording") {
            recordingStartTime.current = Date.now();
            analytics.track(AnalyticsEvents.RECORDING_STARTED);
          } else if (newStatus === "transcribing" && prevStatus === "recording") {
            analytics.track(AnalyticsEvents.RECORDING_STATE_CHANGED, { state: "transcribing" });
          } else if (newStatus === "processing" && prevStatus === "transcribing") {
            analytics.track(AnalyticsEvents.RECORDING_STATE_CHANGED, { state: "processing" });
          } else if (newStatus === "polishing" && prevStatus !== "polishing") {
            analytics.track(AnalyticsEvents.RECORDING_STATE_CHANGED, { state: "polishing" });
          } else if (newStatus === "error" && prevStatus !== "error") {
            analytics.track(AnalyticsEvents.RECORDING_ERROR);
          }

          prevStatusRef.current = newStatus;
          setStatus(newStatus);
        }
      );

      unlistenLevel = await listen<number>(
        "audio-level",
        (event: { payload: number }) => {
          setAudioLevel(event.payload);
        }
      );
    };

    setupListeners();

    return () => {
      unlistenStatus?.();
      unlistenLevel?.();
    };
  }, []);

  const startRecording = useCallback(async () => {
    try {
      await audioCommands.startRecording();
    } catch (err) {
      logger.error("failed_to_start_recording", { error: String(err) });
      analytics.track(AnalyticsEvents.RECORDING_ERROR, { reason: "start_failed" });
    }
  }, []);

  const stopRecording = useCallback(async () => {
    try {
      const outputPath = await audioCommands.stopRecording();
      if (outputPath) {
        const duration = recordingStartTime.current
          ? Math.round((Date.now() - recordingStartTime.current) / 1000)
          : 0;
        analytics.track(AnalyticsEvents.RECORDING_STOPPED, { duration });
        recordingStartTime.current = null;
        showToast("Recording saved");
      }
    } catch (err) {
      logger.error("failed_to_stop_recording", { error: String(err) });
      analytics.track(AnalyticsEvents.RECORDING_ERROR, { reason: "stop_failed" });
    }
  }, []);

  return {
    status,
    audioLevel,
    hotkey,
    startRecording,
    stopRecording,
  };
}
