import { useEffect, useState, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { events, historyCommands, TranscriptionEntry, HistoryFilter } from "@/lib/tauri";
import { showToast } from "@/lib/toast";
import { cn } from "@/lib/utils";
import {
  Search,
  Clock,
  ChevronLeft,
  ChevronRight,
  X,
  Copy,
  AlertCircle,
  RotateCcw,
} from "lucide-react";
import { logger } from "@/lib/logger";

const PAGE_SIZE = 20;

type EngineFilter = "all" | "local" | "cloud";

function formatRelativeTime(timestamp: number, t: (key: string, options?: Record<string, unknown>) => string): string {
  const now = Date.now();
  const diff = now - timestamp;
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return t("history.justNow");
  if (minutes < 60) return t("history.minutesAgo", { captures: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("history.hoursAgo", { captures: hours });
  const days = Math.floor(hours / 24);
  if (days < 7) return t("history.daysAgo", { captures: days });
  return new Intl.DateTimeFormat().format(new Date(timestamp));
}

interface HistoryEntryCardProps {
  entry: TranscriptionEntry;
  t: (key: string, options?: Record<string, unknown>) => string;
  onRetry: (id: string) => void;
  retryingIds: Set<string>;
}

function HistoryEntryCard({ entry, t, onRetry, retryingIds }: HistoryEntryCardProps) {
  const [copied, setCopied] = useState(false);
  const isRetrying = retryingIds.has(entry.id);
  const isError = entry.status === "error";
  const isSuccess = entry.status === "success" && entry.final_text && entry.final_text.length > 0;

  const handleCopy = async () => {
    if (!entry.final_text) return;
    try {
      await navigator.clipboard.writeText(entry.final_text);
      setCopied(true);
      showToast(t("history.copied"));
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      logger.error("failed_to_copy", { error: String(err) });
    }
  };

  return (
    <div className={cn(
      "group flex items-start justify-between gap-4 py-3 px-2 md:px-3 border-b border-border/40 last:border-0",
      "hover:bg-secondary/30 transition-colors",
      isRetrying && "opacity-70"
    )}>
      {/* 文本列 (左侧) */}
      <div className="flex-1 min-w-0">
        {isError ? (
          <div className="flex items-start gap-2">
            <AlertCircle className="h-4 w-4 text-destructive shrink-0 mt-0.5" />
            <div className="flex flex-col gap-1">
              <p className="text-[14px] text-muted-foreground leading-relaxed">
                {t("history.error.failed")}
              </p>
              {entry.error && (
                <p className="text-xs text-destructive/70">
                  {entry.error}
                </p>
              )}
            </div>
          </div>
        ) : (
          <p className="text-[14px] text-foreground leading-relaxed break-words">
            {entry.final_text || t("history.empty.description")}
          </p>
        )}
      </div>

      {/* 操作列 (右侧) */}
      <div className="shrink-0 flex items-center gap-2">
        {/* 按钮 hover 时才显示 */}
        <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
          {isSuccess && (
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={handleCopy}
              title={t("history.copy")}
            >
              <Copy className={cn("h-3.5 w-3.5 pointer-events-none", copied && "text-primary")} />
            </Button>
          )}
          {isError && (
            <Button
              variant={isRetrying ? "outline" : "ghost"}
              size="sm"
              className={cn(
                "h-7 px-2 text-xs gap-1",
                isRetrying && "cursor-wait bg-accent/50"
              )}
              onClick={() => onRetry(entry.id)}
              disabled={isRetrying}
              title={t("history.retry")}
            >
              <RotateCcw className={cn("h-3.5 w-3.5 pointer-events-none", isRetrying && "animate-spin")} />
              <span className="pointer-events-none">
                {isRetrying ? t("history.retrying") : t("history.retry")}
              </span>
            </Button>
          )}
        </div>
        {/* 时间 */}
        <span className="text-[13px] font-medium font-mono tabular-nums tracking-tight text-muted-foreground/50">
          {formatRelativeTime(entry.created_at, t)}
        </span>
      </div>
    </div>
  );
}

interface EmptyStateProps {
  t: (key: string) => string;
}

function EmptyState({ t }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-20 text-center rounded-3xl border border-dashed border-border/50 bg-secondary/10">
      <div className="rounded-full bg-secondary/50 p-4 mb-4">
        <Clock className="h-8 w-8 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold text-foreground mb-2">
        {t("history.empty.title")}
      </h3>
      <p className="text-sm text-muted-foreground max-w-sm">
        {t("history.empty.description")}
      </p>
    </div>
  );
}

export function HistoryPage() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<TranscriptionEntry[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [engineFilter, setEngineFilter] = useState<EngineFilter>("all");
  const [currentPage, setCurrentPage] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [searchDebounceTimer, setSearchDebounceTimer] = useState<ReturnType<typeof setTimeout> | null>(null);
  const [pendingSearch, setPendingSearch] = useState("");
  const [retryingIds, setRetryingIds] = useState<Set<string>>(new Set());

  const fetchHistory = useCallback(async () => {
    setIsLoading(true);
    try {
      const filter: HistoryFilter = {
        limit: PAGE_SIZE,
        offset: currentPage * PAGE_SIZE,
      };

      if (pendingSearch.trim()) {
        filter.search = pendingSearch.trim();
      }

      if (engineFilter === "local") {
        filter.engine = "local";
      } else if (engineFilter === "cloud") {
        filter.engine = "cloud";
      }

      const [result] = await Promise.all([
        historyCommands.getHistory(filter),
      ]);
      
      setEntries(result);
      setTotalCount(result.length === PAGE_SIZE ? (currentPage + 2) * PAGE_SIZE : currentPage * PAGE_SIZE + result.length);
    } catch (err) {
      logger.error("failed_to_fetch_history", { error: String(err) });
    } finally {
      setIsLoading(false);
    }
  }, [currentPage, engineFilter, pendingSearch]);

  const handleRetry = useCallback(async (id: string) => {
    const startTime = Date.now();
    setRetryingIds(prev => new Set(prev).add(id));
    try {
      const result = await historyCommands.retryTranscription(id);
      logger.info("retry_success", { id, result });
      // Refresh the list
      await fetchHistory();
    } catch (err) {
      logger.error("retry_failed", { id, error: String(err) });
    } finally {
      // Ensure loading shows for at least 1 second
      const elapsed = Date.now() - startTime;
      const minDisplayTime = 1000;
      const remainingTime = Math.max(0, minDisplayTime - elapsed);
      
      if (remainingTime > 0) {
        await new Promise(resolve => setTimeout(resolve, remainingTime));
      }
      
      setRetryingIds(prev => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  }, [fetchHistory]);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  useEffect(() => {
    let unlistenRetryState: (() => void) | undefined;
    let unlistenRetryComplete: (() => void) | undefined;
    let unlistenRetryError: (() => void) | undefined;

    const setupListeners = async () => {
      unlistenRetryState = await events.onRetryStateChanged((event) => {
        logger.info("history_retry_state_changed", {
          entry_id: event.entry_id,
          task_id: event.task_id,
          status: event.status,
        });
      });

      unlistenRetryComplete = await events.onRetryComplete((event) => {
        logger.info("history_retry_completed", {
          entry_id: event.entry_id,
          task_id: event.task_id,
          text_len: event.text.length,
        });
        void fetchHistory();
      });

      unlistenRetryError = await events.onRetryError((event) => {
        logger.error("history_retry_failed_event", {
          entry_id: event.entry_id,
          task_id: event.task_id,
          error: event.error,
        });
        void fetchHistory();
      });
    };

    void setupListeners();

    return () => {
      unlistenRetryState?.();
      unlistenRetryComplete?.();
      unlistenRetryError?.();
    };
  }, [fetchHistory]);

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setSearchQuery(value);

    if (searchDebounceTimer) {
      clearTimeout(searchDebounceTimer);
    }

    const timer = setTimeout(() => {
      setPendingSearch(value);
      setCurrentPage(0);
    }, 300);

    setSearchDebounceTimer(timer);
  };

  const handleClearSearch = () => {
    setSearchQuery("");
    setPendingSearch("");
    setCurrentPage(0);
  };

  const handleEngineFilterChange = (filter: EngineFilter) => {
    setEngineFilter(filter);
    setCurrentPage(0);
  };

  const totalPages = Math.ceil(totalCount / PAGE_SIZE);
  const hasNextPage = entries.length === PAGE_SIZE;
  const hasPrevPage = currentPage > 0;

  const engineFilters: { value: EngineFilter; label: string }[] = useMemo(() => [
    { value: "all", label: t("history.filter.all") },
    { value: "local", label: t("history.filter.local") },
    { value: "cloud", label: t("history.filter.cloud") },
  ], [t]);

  return (
    <div className="mx-auto max-w-6xl p-10" data-testid="history-page">
      <div className="flex flex-col md:flex-row md:items-start justify-between gap-4 mb-8">
        <div>
          <h1 className="text-[1.7rem] font-semibold tracking-[-0.05em] text-foreground">{t("history.title")}</h1>
          <p className="text-muted-foreground mt-2">{t("history.description")}</p>
        </div>
      </div>

      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-6">
        <div className="inline-flex h-10 items-center justify-center rounded-full bg-secondary p-1 text-muted-foreground">
          {engineFilters.map((filter) => (
            <button
              key={filter.value}
              onClick={() => handleEngineFilterChange(filter.value)}
              className={cn(
                "inline-flex items-center justify-center rounded-full px-4 py-1.5 text-sm font-medium transition-all",
                engineFilter === filter.value
                  ? "bg-background text-foreground shadow-sm"
                  : "hover:text-foreground"
              )}
            >
              {filter.label}
            </button>
          ))}
        </div>
        <div className="relative w-full sm:w-72">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            type="text"
            placeholder={t("history.search.placeholder")}
            value={searchQuery}
            onChange={handleSearchChange}
            className="pl-10 pr-10 rounded-full bg-secondary/50 border-transparent focus-visible:bg-background focus-visible:border-primary"
          />
          {searchQuery && (
            <Button
              variant="ghost"
              size="icon"
              className="absolute right-1 top-1/2 -translate-y-1/2 h-7 w-7 rounded-full"
              onClick={handleClearSearch}
            >
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      <div className="relative min-h-[400px]">
        {isLoading && entries.length === 0 ? (
          <div className="space-y-1">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="flex items-center gap-3 py-4 px-3 border-b border-border/40">
                <div className="h-4 w-12 rounded bg-secondary/50 animate-pulse" />
                <div className="h-4 w-2/3 rounded bg-secondary/50 animate-pulse" />
              </div>
            ))}
          </div>
        ) : entries.length > 0 ? (
          <div className="flex flex-col" data-testid="history-entries">
            {entries.map((entry) => (
              <HistoryEntryCard 
                key={entry.id} 
                entry={entry} 
                t={t} 
                onRetry={handleRetry}
                retryingIds={retryingIds}
              />
            ))}
          </div>
        ) : (
          <EmptyState t={t} />
        )}
      </div>

      {!isLoading && entries.length > 0 && (
        <div className="flex items-center justify-between mt-6">
          <div className="text-sm text-muted-foreground">
            {t("history.pagination.page", {
              current: currentPage + 1,
              total: Math.max(1, totalPages),
            })}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setCurrentPage((p) => Math.max(0, p - 1))}
              disabled={!hasPrevPage}
            >
              <ChevronLeft className="h-4 w-4 mr-1" />
              {t("history.pagination.prev")}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setCurrentPage((p) => p + 1)}
              disabled={!hasNextPage}
            >
              {t("history.pagination.next")}
              <ChevronRight className="h-4 w-4 ml-1" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
