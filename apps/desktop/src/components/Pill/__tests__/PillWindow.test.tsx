import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PillWindow } from "../PillWindow";
import type { AppSettings, PillTooltipEvent } from "@/lib/tauri";

type Listener<T> = (event: T) => void;

const {
  getSettingsMock,
  hidePillMock,
  onPillTooltipMock,
  onSettingsChangedMock,
  showPillMock,
} = vi.hoisted(() => {
  const tooltipListeners = new Set<Listener<PillTooltipEvent>>();

  return {
    getSettingsMock: vi.fn(),
    hidePillMock: vi.fn(),
    onPillTooltipMock: vi.fn(async (callback: Listener<PillTooltipEvent>) => {
      tooltipListeners.add(callback);
      return () => tooltipListeners.delete(callback);
    }),
    onSettingsChangedMock: vi.fn(async () => () => undefined),
    showPillMock: vi.fn(),
    emitPillTooltip: (event: PillTooltipEvent) => {
      tooltipListeners.forEach((callback) => callback(event));
    },
  };
});

const mocks = vi.hoisted(() => ({
  emitPillTooltip: undefined as
    | undefined
    | ((event: PillTooltipEvent) => void),
}));

vi.mock("@/lib/tauri", () => ({
  events: {
    onPillTooltip: onPillTooltipMock,
    onSettingsChanged: onSettingsChangedMock,
  },
  settingsCommands: {
    getSettings: getSettingsMock,
  },
  windowCommands: {
    hidePill: hidePillMock,
    showPill: showPillMock,
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => undefined),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    startDragging: vi.fn(),
  }),
}));

vi.mock("border-beam", () => ({
  BorderBeam: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock("../AudioDots", () => ({
  AudioDots: () => <div data-testid="audio-dots" />,
}));

vi.mock("../SettingsButton", () => ({
  SettingsButton: () => <button type="button" aria-label="settings" />,
}));

vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

function settings(overrides: Partial<AppSettings> = {}): Partial<AppSettings> {
  return {
    pill_background_color: "#1d1d1d",
    pill_background_opacity: 1,
    pill_indicator_mode: "when_recording",
    pill_size: 2,
    ...overrides,
  };
}

describe("PillWindow backend tooltip", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    getSettingsMock.mockReset();
    hidePillMock.mockReset();
    onPillTooltipMock.mockClear();
    onSettingsChangedMock.mockClear();
    showPillMock.mockReset();
    hidePillMock.mockResolvedValue(undefined);
    showPillMock.mockResolvedValue(undefined);
    getSettingsMock.mockResolvedValue(settings());
    mocks.emitPillTooltip = (event: PillTooltipEvent) => {
      const calls = onPillTooltipMock.mock.calls as Array<[Listener<PillTooltipEvent>]>;
      calls.forEach(([callback]) => callback(event));
    };
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders task-scoped tooltip messages without requesting a native window show", async () => {
    render(<PillWindow />);

    await act(async () => {
      await Promise.resolve();
    });
    expect(onPillTooltipMock).toHaveBeenCalled();

    act(() => {
      mocks.emitPillTooltip?.({
        message: "ESC 取消，Enter 确认",
        duration_ms: 3200,
        task_id: 2,
      });
    });

    expect(showPillMock).not.toHaveBeenCalled();
    const tooltip = screen.getByText("ESC 取消，Enter 确认");
    expect(tooltip).toBeInTheDocument();
    expect(tooltip).toHaveClass("max-w-[calc(100vw-1rem)]");
  });

  it("hides backend-pushed tooltip messages after the backend duration", async () => {
    render(<PillWindow />);

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      mocks.emitPillTooltip?.({
        message: "已记录纠错词：搜题 -> sootie",
        duration_ms: 50,
        task_id: null,
      });
    });

    expect(showPillMock).toHaveBeenCalledTimes(1);
    const tooltip = screen.getByText("已记录纠错词：搜题 -> sootie");
    expect(tooltip).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(50);
    });

    expect(tooltip).not.toBeVisible();
  });
});
