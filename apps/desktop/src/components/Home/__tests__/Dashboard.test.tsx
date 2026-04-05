import type { PropsWithChildren } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Dashboard } from "@/components/Home/Dashboard";
import i18n from "@/i18n";
import { historyCommands } from "@/lib/tauri";

vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: PropsWithChildren) => <div>{children}</div>,
  ComposedChart: ({ children }: PropsWithChildren) => <div>{children}</div>,
  CartesianGrid: () => null,
  Area: () => null,
  Line: () => null,
  XAxis: () => null,
  YAxis: () => null,
  Tooltip: () => null,
}));

vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

vi.mock("@/lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("@/lib/tauri")>("@/lib/tauri");
  return {
    ...actual,
    historyCommands: {
      getDashboardStats: vi.fn(),
      getDailyUsage: vi.fn(),
      getEngineUsage: vi.fn(),
    },
  };
});

describe("Dashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    void i18n.changeLanguage("en");
  });

  it("renders redesigned dashboard sections with live data", async () => {
    vi.mocked(historyCommands.getDashboardStats).mockResolvedValue({
      total_count: 14,
      today_count: 2,
      total_chars: 280,
      total_output_units: 72,
      total_audio_ms: 180_000,
      avg_stt_ms: 640,
      avg_audio_ms: 12_000,
      avg_output_units: 5.1,
      local_count: 9,
      cloud_count: 5,
      polish_count: 8,
      active_days: 6,
      current_streak_days: 4,
      longest_streak_days: 5,
      last_7_days_count: 10,
      last_7_days_audio_ms: 126_000,
      last_7_days_output_units: 54,
    });
    vi.mocked(historyCommands.getDailyUsage).mockResolvedValue([
      { date: "2026-04-02", count: 3, audio_ms: 21_000, output_units: 12 },
      { date: "2026-04-03", count: 2, audio_ms: 24_000, output_units: 14 },
      { date: "2026-04-04", count: 5, audio_ms: 81_000, output_units: 28 },
    ]);
    vi.mocked(historyCommands.getEngineUsage).mockResolvedValue([
      { engine: "Whisper", count: 9, avg_stt_ms: 800 },
      { engine: "Volcengine", count: 5, avg_stt_ms: 420 },
    ]);

    render(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText("Recent pattern")).toBeInTheDocument();
    });

    expect(screen.getAllByText("Current habits").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Current Streak").length).toBeGreaterThan(0);
    expect(screen.getByText("4 days")).toBeInTheDocument();
    expect(screen.getAllByText("Recognition mix").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Total Captures").length).toBeGreaterThan(0);
    expect(screen.getByText("14")).toBeInTheDocument();
    expect(screen.getByText("Time to Recognize")).toBeInTheDocument();
    expect(screen.getByText("640ms")).toBeInTheDocument();
    expect(screen.getAllByText("Whisper").length).toBeGreaterThan(0);
    expect(screen.getByText("Captures Polished")).toBeInTheDocument();
    expect(screen.getByText("57%")).toBeInTheDocument();
    expect(
      screen.getByText("Which engines have been carrying most of the work lately"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("See how your dictation sessions have been settling into place"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/10 captures, 2.1m spoken, and 54 text units/i),
    ).toBeInTheDocument();
  });

  it("shows a no-data fallback for missing duration metrics", async () => {
    vi.mocked(historyCommands.getDashboardStats).mockResolvedValue({
      total_count: 3,
      today_count: 1,
      total_chars: 60,
      total_output_units: 18,
      total_audio_ms: 45_000,
      avg_stt_ms: null,
      avg_audio_ms: null,
      avg_output_units: 6,
      local_count: 2,
      cloud_count: 1,
      polish_count: 1,
      active_days: 2,
      current_streak_days: 1,
      longest_streak_days: 2,
      last_7_days_count: 3,
      last_7_days_audio_ms: 45_000,
      last_7_days_output_units: 18,
    });
    vi.mocked(historyCommands.getDailyUsage).mockResolvedValue([
      { date: "2026-04-03", count: 1, audio_ms: 15_000, output_units: 6 },
      { date: "2026-04-04", count: 2, audio_ms: 30_000, output_units: 12 },
    ]);
    vi.mocked(historyCommands.getEngineUsage).mockResolvedValue([
      { engine: "Whisper", count: 3, avg_stt_ms: null },
    ]);

    render(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText("Time to Recognize")).toBeInTheDocument();
    });

    expect(screen.getByText("1 day")).toBeInTheDocument();
    expect(screen.getAllByText("No data").length).toBeGreaterThan(0);
  });

  it("shows the sample data notice when no history exists", async () => {
    vi.mocked(historyCommands.getDashboardStats).mockResolvedValue({
      total_count: 0,
      today_count: 0,
      total_chars: 0,
      total_output_units: 0,
      total_audio_ms: 0,
      avg_stt_ms: null,
      avg_audio_ms: null,
      avg_output_units: null,
      local_count: 0,
      cloud_count: 0,
      polish_count: 0,
      active_days: 0,
      current_streak_days: 0,
      longest_streak_days: 0,
      last_7_days_count: 0,
      last_7_days_audio_ms: 0,
      last_7_days_output_units: 0,
    });
    vi.mocked(historyCommands.getDailyUsage).mockResolvedValue([]);
    vi.mocked(historyCommands.getEngineUsage).mockResolvedValue([]);

    render(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText(/Sample data/i)).toBeInTheDocument();
    });
  });
});
