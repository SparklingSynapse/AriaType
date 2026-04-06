import { useState, useEffect, useRef, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { systemCommands } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { useTranslation } from "react-i18next";
import { analytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { SettingsPageLayout } from "./SettingsPageLayout";
import { OverlayScrollbarsComponent } from "overlayscrollbars-react";
import "overlayscrollbars/overlayscrollbars.css";

const LINE_COUNT = 500;

const LEVEL_COLORS: Record<string, string> = {
  ERROR: "text-red-400 dark:text-red-300",
  WARN:  "text-yellow-400 dark:text-yellow-300",
  INFO:  "text-blue-300 dark:text-blue-200",
  DEBUG: "text-gray-400 dark:text-gray-400",
};

function colorLine(line: string): { color: string; text: string } {
  for (const [level, color] of Object.entries(LEVEL_COLORS)) {
    if (line.includes(` ${level} `) || line.startsWith(level)) {
      return { color, text: line };
    }
  }
  return { color: "text-gray-300 dark:text-gray-400", text: line };
}

export function LogViewer() {
  const { t } = useTranslation();
  const [content, setContent] = useState<string>("");
  const [filter, setFilter] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    try {
      const text = await systemCommands.getLogContent(LINE_COUNT);
      setContent(text);
    } catch (err) {
      logger.error("failed_to_load_logs", { error: String(err) });
    }
  }, []);

  // Initial load
  useEffect(() => {
    load();
  }, [load]);

  // Scroll to bottom when content updates
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [content]);

  const lines = content
    .split("\n")
    .filter((l) => !filter || l.toLowerCase().includes(filter.toLowerCase()));

  return (
    <SettingsPageLayout
      title={t("logs.title")}
      description={t("logs.description")}
      className="flex flex-col h-full"
    >
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder={t("logs.filter")}
          className="flex-1 h-10 rounded-2xl border border-border bg-background px-4 text-sm focus-visible:border-primary focus-visible:outline-none"
        />
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            load();
            analytics.track(AnalyticsEvents.LOGS_REFRESHED);
          }}
        >
          {t("logs.refresh")}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            systemCommands.openLogFolder();
            analytics.track(AnalyticsEvents.LOG_FOLDER_OPENED);
          }}
        >
          {t("logs.openFolder")}
        </Button>
      </div>

      <OverlayScrollbarsComponent
        defer
        className="flex-1 rounded-2xl border border-border bg-zinc-900 dark:bg-zinc-950 text-zinc-300 dark:text-zinc-400 font-mono text-xs p-4 min-h-0 max-h-[calc(100vh-220px)]"
        options={{
          showNativeOverlaidScrollbars: false,
          scrollbars: {
            theme: "os-theme-dark",
            visibility: "auto",
            autoHide: "scroll",
            autoHideDelay: 300,
            autoHideSuspend: false,
          },
        }}
      >
        {lines.length === 0 ? (
          <span className="text-gray-500 dark:text-gray-400">{t("logs.empty")}</span>
        ) : (
          lines.map((line, i) => {
            const { color, text } = colorLine(line);
            return (
              <div key={i} className={`whitespace-pre-wrap break-all leading-5 ${color}`}>
                {text}
              </div>
            );
          })
        )}
        <div ref={bottomRef} />
      </OverlayScrollbarsComponent>
    </SettingsPageLayout>
  );
}
