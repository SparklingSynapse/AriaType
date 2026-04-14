import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RecordingStatus } from "@/types";
import { AnalyticsEvents } from "@/lib/events";
import { useRecording } from "../useRecording";

type RecordingStateEvent = {
  status: RecordingStatus;
  task_id: number;
};

type Listener<T> = (event: { payload: T }) => void;

const { listeners, analyticsTrackMock, getSettingsMock, listenMock } = vi.hoisted(() => {
  const listeners = new Map<string, Set<Listener<unknown>>>();
  const analyticsTrackMock = vi.fn();
  const getSettingsMock = vi.fn();
  const listenMock = vi.fn(
    async <T,>(eventName: string, callback: Listener<T>) => {
      const callbacks = listeners.get(eventName) ?? new Set();
      callbacks.add(callback as Listener<unknown>);
      listeners.set(eventName, callbacks);

      return () => {
        callbacks.delete(callback as Listener<unknown>);
        if (callbacks.size === 0) {
          listeners.delete(eventName);
        }
      };
    }
  );

  return {
    listeners,
    analyticsTrackMock,
    getSettingsMock,
    listenMock,
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@/lib/tauri", () => ({
  audioCommands: {
    startRecording: vi.fn(),
    stopRecording: vi.fn(),
  },
  settingsCommands: {
    getSettings: getSettingsMock,
  },
}));

vi.mock("@/lib/toast", () => ({
  showToast: vi.fn(),
}));

vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

vi.mock("@/lib/analytics", () => ({
  analytics: {
    track: analyticsTrackMock,
  },
}));

function emitRecordingState(payload: RecordingStateEvent) {
  const callbacks = listeners.get("recording-state-changed");
  callbacks?.forEach((callback) => callback({ payload }));
}

describe("useRecording", () => {
  beforeEach(() => {
    listeners.clear();
    analyticsTrackMock.mockReset();
    getSettingsMock.mockReset();
    listenMock.mockClear();
    getSettingsMock.mockResolvedValue({ hotkey: "cmd+shift+space" });
  });

  it("ignores stale recording events from older tasks", async () => {
    const { result } = renderHook(() => useRecording());

    await waitFor(() => {
      expect(result.current.hotkey).toBe("cmd+shift+space");
    });

    act(() => {
      emitRecordingState({ status: "recording", task_id: 2 });
    });

    expect(result.current.status).toBe("recording");
    expect(analyticsTrackMock).toHaveBeenCalledWith(AnalyticsEvents.RECORDING_STARTED);

    act(() => {
      emitRecordingState({ status: "idle", task_id: 1 });
    });

    expect(result.current.status).toBe("recording");
    expect(analyticsTrackMock).toHaveBeenCalledTimes(1);

    act(() => {
      emitRecordingState({ status: "transcribing", task_id: 2 });
    });

    expect(result.current.status).toBe("transcribing");
    expect(analyticsTrackMock).toHaveBeenCalledWith(
      AnalyticsEvents.RECORDING_STATE_CHANGED,
      { state: "transcribing" }
    );
  });

  it("lets the newest task replace the previous task state", async () => {
    const { result } = renderHook(() => useRecording());

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
    });

    act(() => {
      emitRecordingState({ status: "error", task_id: 3 });
      emitRecordingState({ status: "recording", task_id: 4 });
      emitRecordingState({ status: "idle", task_id: 3 });
    });

    expect(result.current.status).toBe("recording");
    expect(analyticsTrackMock).toHaveBeenCalledWith(AnalyticsEvents.RECORDING_ERROR);
    expect(analyticsTrackMock).toHaveBeenCalledWith(AnalyticsEvents.RECORDING_STARTED);
  });
});
