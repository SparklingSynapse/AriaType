import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Area,
  CartesianGrid,
  ComposedChart,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import {
  historyCommands,
  type DailyUsage,
  type DashboardStats,
  type EngineUsage,
} from "@/lib/tauri";
import i18n from "@/i18n";
import { logger } from "@/lib/logger";

type TrendPoint = DailyUsage & {
  short_date: string;
  avg_audio_seconds: number;
  avg_output_units: number;
};

interface TooltipPayloadItem {
  dataKey: string;
  value: number;
  payload: TrendPoint;
}

interface ChartPalette {
  primary: string;
  secondary: string;
  tertiary: string;
  textMuted: string;
  grid: string;
  panel: string;
  panelStrong: string;
  border: string;
}

interface ActivityRowProps {
  label: string;
  value: string;
}

function useDashboardPalette() {
  const [isDark, setIsDark] = useState(() =>
    typeof document !== "undefined"
      ? document.documentElement.classList.contains("dark")
      : false,
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

  return isDark
    ? {
        primary: "#93c5fd", // Blue-300 (Clear but soft blue)
        secondary: "#6ee7b7", // Emerald-300 (Clear but soft green)
        tertiary: "#c084fc", // Purple-400 (Clear but soft purple)
        textMuted: "#737373",
        grid: "rgba(255,255,255,0.06)",
        panel: "rgba(255,255,255,0.028)",
        panelStrong: "rgba(255,255,255,0.06)",
        border: "rgba(255,255,255,0.075)",
      }
    : {
        primary: "#3b82f6", // Blue-500 (Clear, visible blue)
        secondary: "#10b981", // Emerald-500 (Clear, visible green)
        tertiary: "#a855f7", // Purple-500 (Clear, visible purple)
        textMuted: "#a3a3a3",
        grid: "rgba(0,0,0,0.04)",
        panel: "rgba(17,17,17,0.022)",
        panelStrong: "rgba(17,17,17,0.045)",
        border: "rgba(17,17,17,0.07)",
      };
}

function ActivityRow({ label, value }: ActivityRowProps) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-border/70 py-3.5 first:pt-0 last:border-b-0 last:pb-0">
      <span className="text-[13px] leading-6 text-muted-foreground">
        {label}
      </span>
      <span className="text-sm font-medium tabular-nums text-foreground">
        {value}
      </span>
    </div>
  );
}

function LegendPill({
  color,
  dashed = false,
  label,
}: {
  color: string;
  dashed?: boolean;
  label: string;
}) {
  return (
    <div className="inline-flex items-center gap-2 rounded-full border border-border bg-background/80 px-3 py-1.5 text-xs text-muted-foreground">
      <span
        className="block h-px w-5"
        style={{
          backgroundColor: dashed ? "transparent" : color,
          borderTop: dashed ? `1px dashed ${color}` : undefined,
          height: dashed ? 0 : 1,
        }}
      />
      {label}
    </div>
  );
}

function RhythmTooltip({
  active,
  payload,
}: {
  active?: boolean;
  payload?: TooltipPayloadItem[];
}) {
  const { t } = useTranslation();

  if (!active || !payload || payload.length === 0) {
    return null;
  }

  const point = payload[0].payload;

  return (
    <div className="min-w-44 rounded-[20px] border border-border bg-card px-4 py-3 shadow-sm">
      <div className="text-xs font-medium text-foreground">{point.date}</div>
      <div className="mt-3 space-y-2">
        <div className="flex items-center justify-between gap-4 text-xs">
          <span className="text-muted-foreground">
            {t("dashboard.chart.captures")}
          </span>
          <span className="font-medium text-foreground">{point.count}</span>
        </div>
        <div className="flex items-center justify-between gap-4 text-xs">
          <span className="text-muted-foreground">
            {t("dashboard.chart.avgDuration")}
          </span>
          <span className="font-medium text-foreground">
            {point.avg_audio_seconds.toFixed(1)}s
          </span>
        </div>
        <div className="flex items-center justify-between gap-4 text-xs">
          <span className="text-muted-foreground">
            {t("dashboard.chart.avgOutput")}
          </span>
          <span className="font-medium text-foreground">
            {point.avg_output_units.toFixed(1)}
          </span>
        </div>
      </div>
    </div>
  );
}

function EngineUsageList({
  engines,
  totalCount,
  palette,
  formatDuration,
  shareLabel,
  latencyLabel,
}: {
  engines: EngineUsage[];
  totalCount: number;
  palette: ChartPalette;
  formatDuration: (value: number | null) => string;
  shareLabel: string;
  latencyLabel: string;
}) {
  return (
    <div className="divide-y divide-border/70">
      {engines.map((engine, index) => {
        const share = totalCount > 0 ? (engine.count / totalCount) * 100 : 0;
        const fill =
          index === 0
            ? palette.primary
            : index === 1
              ? palette.secondary
              : palette.tertiary;

        return (
          <div key={engine.engine} className="py-4 first:pt-0 last:pb-0">
            <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
              <div>
                <div className="text-[15px] font-medium tracking-[-0.03em] text-foreground">
                  {engine.engine}
                </div>
                <div className="mt-1 text-[13px] leading-6 text-muted-foreground">
                  {latencyLabel}: {formatDuration(engine.avg_stt_ms)}
                </div>
              </div>
              <div className="flex items-center gap-4 text-[13px] text-muted-foreground">
                <span className="tabular-nums">
                  {shareLabel}: {Math.round(share)}%
                </span>
                <span className="tabular-nums">
                  {formatCompactNumber(engine.count)}
                </span>
              </div>
            </div>
            <div className="mt-3 h-[3px] rounded-full bg-secondary/35">
              <div
                className="h-full rounded-full"
                style={{
                  width: `${Math.max(share, 6)}%`,
                  backgroundColor: fill,
                }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function Dashboard() {
  const { t } = useTranslation();
  const palette = useDashboardPalette();
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [dailyUsage, setDailyUsage] = useState<DailyUsage[]>([]);
  const [engineUsage, setEngineUsage] = useState<EngineUsage[]>([]);

  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [statsData, usageData, engineData] = await Promise.all([
          historyCommands.getDashboardStats(),
          historyCommands.getDailyUsage(30),
          historyCommands.getEngineUsage(),
        ]);
        setStats(statsData);
        setDailyUsage(usageData);
        setEngineUsage(engineData);
      } catch (error) {
        logger.error("dashboard_data_load_failed", { error: String(error) });
      } finally {
        setIsLoading(false);
      }
    };

    fetchData();
  }, []);

  const displayStats = stats ?? {
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
  };
  
  const displayDailyUsage = dailyUsage.length > 0 ? dailyUsage : Array.from({ length: 30 }, (_, index) => {
    const offset = 29 - index;
    const date = new Date(Date.now() - offset * 86_400_000);
    return {
      date: `${date.getFullYear()}-${`${date.getMonth() + 1}`.padStart(2, "0")}-${`${date.getDate()}`.padStart(2, "0")}`,
      count: 0,
      audio_ms: 0,
      output_units: 0,
    };
  });
  
  const displayEngineUsage = engineUsage.length > 0 ? engineUsage : [];

  const totalCount = Math.max(displayStats.total_count, 1);
  const polishRate = (displayStats.polish_count / totalCount) * 100;
  const trendData = useMemo<TrendPoint[]>(
    () =>
      displayDailyUsage.map((point) => ({
        ...point,
        short_date: formatShortDate(point.date),
        avg_audio_seconds:
          point.count > 0 ? point.audio_ms / point.count / 1000 : 0,
        avg_output_units:
          point.count > 0 ? point.output_units / point.count : 0,
      })),
    [displayDailyUsage],
  );

  const visibleTrendData = useMemo(() => {
    const activePoints = trendData.filter((point) => point.count > 0).length;

    if (trendData.length > 14 && activePoints <= 6) {
      return trendData.slice(-14);
    }

    return trendData;
  }, [trendData]);
  const isFocusedTrendWindow = visibleTrendData.length !== trendData.length;

  if (isLoading) {
    return <div className="mx-auto max-w-6xl p-10 min-h-[calc(100vh-4rem)]" />;
  }

  return (
    <div className="mx-auto max-w-6xl p-10" data-testid="dashboard-page">
      <div className="space-y-5 md:space-y-6">
        <section className="grid gap-6 xl:grid-cols-[minmax(0,1.42fr)_minmax(17rem,0.84fr)]" data-testid="dashboard-content">
          <div
            className="rounded-3xl border border-border bg-card px-5 py-5 md:px-6 md:py-6"
            style={{
              borderColor: palette.border,
              backgroundImage: `linear-gradient(180deg, ${palette.panel}, transparent 36%)`,
            }}
          >
            <div>
                <div className="text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
                  {t("dashboard.chart.usageTitle")}
                </div>
                <h3 className="mt-3 max-w-[20ch] text-[1.7rem] font-semibold  text-foreground">
                  {isFocusedTrendWindow
                    ? t("dashboard.chart.titleRecent")
                    : t("dashboard.chart.titleFull")}
                </h3>
                <div className="mt-4 flex flex-wrap gap-2">
                  <LegendPill
                    color={palette.primary}
                    label={t("dashboard.chart.captures")}
                  />
                  <LegendPill
                    color={palette.secondary}
                    label={t("dashboard.chart.avgDuration")}
                    dashed
                  />
                  <LegendPill
                    color={palette.tertiary}
                    label={t("dashboard.chart.avgOutput")}
                  />
                </div>
            </div>

            <div
              className="relative mt-8 overflow-hidden rounded-2xl border border-border bg-background/80 px-3 py-4 md:px-4"
              style={{ borderColor: palette.border }}
            >
              <div
                className="pointer-events-none absolute inset-x-10 top-0 h-24 rounded-full blur-3xl"
                style={{ backgroundColor: palette.panelStrong }}
              />
              <div className="relative h-[200px] md:h-[220px]">
                <ResponsiveContainer width="100%" height="100%">
                  <ComposedChart
                    data={visibleTrendData}
                    margin={{ top: 12, right: 6, left: -26, bottom: 0 }}
                  >
                    <defs>
                      <linearGradient
                        id="capturesFade"
                        x1="0"
                        y1="0"
                        x2="0"
                        y2="1"
                      >
                        <stop
                          offset="0%"
                          stopColor={palette.primary}
                          stopOpacity={0.2}
                        />
                        <stop
                          offset="100%"
                          stopColor={palette.primary}
                          stopOpacity={0}
                        />
                      </linearGradient>
                    </defs>
                    <CartesianGrid
                      strokeDasharray="4 8"
                      vertical={false}
                      stroke={palette.grid}
                      strokeOpacity={1}
                    />
                    <XAxis
                      dataKey="short_date"
                      tickLine={false}
                      axisLine={false}
                      tick={{ fill: palette.textMuted, fontSize: 11 }}
                      minTickGap={18}
                    />
                    <YAxis
                      tickLine={false}
                      axisLine={false}
                      allowDecimals={false}
                      tick={{ fill: palette.textMuted, fontSize: 11 }}
                    />
                    <Tooltip
                      content={<RhythmTooltip />}
                      cursor={{
                        stroke: palette.grid,
                        strokeDasharray: "3 6",
                        strokeOpacity: 1,
                      }}
                    />
                    <Area
                      type="monotone"
                      dataKey="count"
                      fill="url(#capturesFade)"
                      stroke="none"
                    />
                    <Line
                      type="monotone"
                      dataKey="count"
                      stroke={palette.primary}
                      strokeWidth={2.7}
                      dot={false}
                      activeDot={{
                        r: 4,
                        strokeWidth: 0,
                        fill: palette.primary,
                      }}
                    />
                    <Line
                      type="monotone"
                      dataKey="avg_audio_seconds"
                      stroke={palette.secondary}
                      strokeWidth={2}
                      dot={false}
                      strokeDasharray="8 7"
                      activeDot={{
                        r: 4,
                        strokeWidth: 0,
                        fill: palette.secondary,
                      }}
                    />
                    <Line
                      type="monotone"
                      dataKey="avg_output_units"
                      stroke={palette.tertiary}
                      strokeWidth={2.1}
                      dot={false}
                      activeDot={{
                        r: 4,
                        strokeWidth: 0,
                        fill: palette.tertiary,
                      }}
                    />
                  </ComposedChart>
                </ResponsiveContainer>
              </div>
            </div>
          </div>

          <div
            className="rounded-3xl border border-border bg-card px-5 py-5 md:px-6 md:py-6"
            style={{
              borderColor: palette.border,
              backgroundImage: `linear-gradient(180deg, ${palette.panelStrong}, transparent 36%)`,
            }}
          >
            <div className="text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              {t("dashboard.activity.title")}
            </div>

            <div className="mt-6">
              <ActivityRow
                label={t("dashboard.stats.activeDays")}
                value={formatDayCount(t, displayStats.active_days)}
              />
              <ActivityRow
                label={t("dashboard.activity.longestStreak")}
                value={formatDayCount(t, displayStats.longest_streak_days)}
              />
              <ActivityRow
                label={t("dashboard.stats.avgCaptureDuration")}
                value={formatCompactDuration(t, displayStats.avg_audio_ms)}
              />
              <ActivityRow
                label={t("dashboard.stats.polishRate")}
                value={`${Math.round(polishRate)}%`}
              />
            </div>

            <div className="mt-4">
              <div className="text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
                {t("dashboard.chart.engineTitle")}
              </div>
              <div className="mt-3">
                <EngineUsageList
                  engines={displayEngineUsage}
                  totalCount={totalCount}
                  palette={palette}
                  formatDuration={(value) => formatCompactDuration(t, value)}
                  shareLabel={t("dashboard.engine.share")}
                  latencyLabel={t("dashboard.engine.avgLatency")}
                />
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

function formatShortDate(dateString: string) {
  const date = new Date(dateString);
  return `${date.getMonth() + 1}/${date.getDate()}`;
}

function formatCompactNumber(value: number | null) {
  if (value === null || !Number.isFinite(value)) {
    return "0";
  }

  return new Intl.NumberFormat(undefined, {
    notation: value >= 1000 ? "compact" : "standard",
    maximumFractionDigits: value >= 1000 ? 1 : 0,
  }).format(value);
}

function formatCompactDuration(
  t: ReturnType<typeof useTranslation>["t"],
  milliseconds: number | null,
) {
  if (!milliseconds || milliseconds <= 0) {
    return t("dashboard.time.none");
  }

  if (milliseconds >= 1000) {
    return `${(milliseconds / 1000).toFixed(1)}s`;
  }

  return `${Math.round(milliseconds)}ms`;
}

function formatDayCount(
  t: ReturnType<typeof useTranslation>["t"],
  value: number | null,
) {
  if (i18n.resolvedLanguage?.startsWith("en") && value === 1) {
    return "1 day";
  }

  return t("dashboard.unit.days", {
    count: value ?? 0,
  });
}


