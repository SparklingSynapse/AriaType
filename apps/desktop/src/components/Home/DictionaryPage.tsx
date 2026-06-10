import { ChangeEvent, useCallback, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  BookOpenText,
  MagnifyingGlass,
  Plus,
  Trash,
  UploadSimple,
} from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { useEventListeners } from "@/hooks/useEventListeners";
import {
  dictionaryCommands,
  events,
  type DictionaryEntry,
} from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { showErrorToast, showToast } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { SettingsPageLayout } from "./SettingsPageLayout";

type DictionaryTab = "automatic" | "manual";

function formatLastSeen(
  timestamp: number,
  t: (key: string, options?: Record<string, unknown>) => string,
) {
  if (!timestamp) {
    return t("dictionary.meta.manual");
  }

  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
  }).format(new Date(timestamp));
}

interface DictionaryEntryRowProps {
  entry: DictionaryEntry;
  tab: DictionaryTab;
  onDelete: (entry: DictionaryEntry) => void;
}

function DictionaryEntryRow({ entry, tab, onDelete }: DictionaryEntryRowProps) {
  const { t } = useTranslation();
  const isManual = tab === "manual";

  return (
    <div
      className={cn(
        "group flex items-start justify-between gap-4 border-b border-border/40 px-2 py-3 last:border-0 md:px-3",
        "transition-colors hover:bg-secondary/30",
      )}
    >
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 flex-wrap items-center gap-2 text-[14px] leading-relaxed">
          <span className="break-words font-medium text-foreground">{entry.term}</span>
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <span>
            {isManual
              ? t("dictionary.meta.manual")
              : t("dictionary.meta.frequency", { count: entry.frequency })}
          </span>
          {!isManual && (
            <>
              <span className="text-muted-foreground/35">/</span>
              <span>{t("dictionary.meta.lastSeen", { value: formatLastSeen(entry.last_seen_at_ms, t) })}</span>
            </>
          )}
        </div>
      </div>

      {isManual && (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-8 w-8 shrink-0 opacity-0 transition-opacity group-hover:opacity-100"
          onClick={() => onDelete(entry)}
          title={t("dictionary.actions.delete")}
        >
          <Trash className="h-4 w-4" />
        </Button>
      )}
    </div>
  );
}

interface DictionaryEmptyStateProps {
  tab: DictionaryTab;
}

function DictionaryEmptyState({ tab }: DictionaryEmptyStateProps) {
  const { t } = useTranslation();
  const title =
    tab === "automatic"
      ? t("dictionary.empty.automatic.title")
      : t("dictionary.empty.manual.title");
  const description =
    tab === "automatic"
      ? t("dictionary.empty.automatic.description")
      : t("dictionary.empty.manual.description");

  return (
    <div className="flex flex-col items-center justify-center rounded-3xl border border-dashed border-border/50 bg-secondary/10 py-16 text-center">
      <div className="mb-4 rounded-full bg-secondary/50 p-4">
        <BookOpenText className="h-8 w-8 text-muted-foreground" />
      </div>
      <h3 className="mb-2 text-lg font-semibold text-foreground">
        {title}
      </h3>
      <p className="max-w-sm text-sm text-muted-foreground">
        {description}
      </p>
    </div>
  );
}

export function DictionaryPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<DictionaryTab>("automatic");
  const [autoEntries, setAutoEntries] = useState<DictionaryEntry[]>([]);
  const [manualEntries, setManualEntries] = useState<DictionaryEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [term, setTerm] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const refreshDictionary = useCallback(async () => {
    setIsLoading(true);
    try {
      const [automatic, manual] = await Promise.all([
        dictionaryCommands.getAutoEntries(),
        dictionaryCommands.getCustomEntries(),
      ]);
      setAutoEntries(automatic);
      setManualEntries(manual);
    } catch (error) {
      logger.error("failed_to_fetch_dictionary", { error: String(error) });
      showErrorToast(t("dictionary.messages.loadError"));
    } finally {
      setIsLoading(false);
    }
  }, [t]);

  useEventListeners(async () => {
    await refreshDictionary();
    return [
      await events.onCorrectionLearned(() => {
        void refreshDictionary();
      }),
      await events.onSettingsChanged(() => {
        void refreshDictionary();
      }),
    ];
  }, [refreshDictionary]);

  const visibleEntries = useMemo(() => {
    const entries = activeTab === "automatic" ? autoEntries : manualEntries;
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return entries;
    }

    return entries.filter((entry) => {
      return entry.term.toLowerCase().includes(query);
    });
  }, [activeTab, autoEntries, manualEntries, searchQuery]);

  const handleAdd = async () => {
    if (!term.trim()) {
      showErrorToast(t("dictionary.messages.missingTerm"));
      return;
    }

    setIsSaving(true);
    try {
      await dictionaryCommands.addCustomEntry(term);
      setTerm("");
      await refreshDictionary();
      showToast(t("dictionary.messages.added"));
    } catch (error) {
      logger.error("failed_to_add_dictionary_entry", { error: String(error) });
      showErrorToast(t("dictionary.messages.addError"));
    } finally {
      setIsSaving(false);
    }
  };

  const handleImportClick = () => {
    fileInputRef.current?.click();
  };

  const handleImport = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) {
      return;
    }

    setIsSaving(true);
    try {
      const csvContent = await file.text();
      const result = await dictionaryCommands.importCustomCsv(csvContent);
      await refreshDictionary();
      showToast(
        t("dictionary.messages.imported", {
          imported: result.imported,
          skipped: result.skipped,
        }),
      );
    } catch (error) {
      logger.error("failed_to_import_dictionary_csv", { error: String(error) });
      showErrorToast(t("dictionary.messages.importError"));
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async (entry: DictionaryEntry) => {
    setIsSaving(true);
    try {
      await dictionaryCommands.deleteCustomEntry(entry.term);
      await refreshDictionary();
      showToast(t("dictionary.messages.deleted"));
    } catch (error) {
      logger.error("failed_to_delete_dictionary_entry", { error: String(error) });
      showErrorToast(t("dictionary.messages.deleteError"));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <SettingsPageLayout
      title={t("dictionary.title")}
      description={t("dictionary.description")}
      testId="dictionary-page"
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <SegmentedControl
            value={activeTab}
            onChange={(value) => setActiveTab(value as DictionaryTab)}
            items={[
              { value: "automatic", label: t("dictionary.tabs.automatic") },
              { value: "manual", label: t("dictionary.tabs.manual") },
            ]}
            testId="dictionary-tabs"
          />

          <div className="flex items-center gap-2">
            <div className="relative w-full md:w-64">
              <MagnifyingGlass className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder={t("dictionary.search.placeholder")}
                className="h-10 rounded-full pl-9"
              />
            </div>
            {activeTab === "manual" && (
              <>
                <Button
                  type="button"
                  variant="outline"
                  className="h-10 gap-2 px-4"
                  disabled={isSaving}
                  onClick={handleImportClick}
                >
                  <UploadSimple className="h-4 w-4" />
                  <span>{t("dictionary.actions.importCsv")}</span>
                </Button>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept=".csv,text/csv"
                  className="hidden"
                  onChange={handleImport}
                />
              </>
            )}
          </div>
        </div>

        {activeTab === "manual" && (
          <Card className="border-border/70 p-4 shadow-sm">
            <div className="grid gap-3 md:grid-cols-[1fr_auto] md:items-center">
              <Input
                value={term}
                onChange={(event) => setTerm(event.target.value)}
                placeholder={t("dictionary.form.termPlaceholder")}
                className="h-10 rounded-full"
              />
              <Button
                type="button"
                className="h-10 gap-2 px-4"
                disabled={isSaving}
                onClick={handleAdd}
              >
                <Plus className="h-4 w-4" />
                <span>{t("dictionary.actions.add")}</span>
              </Button>
            </div>
          </Card>
        )}

        <Card className="relative flex min-h-[400px] flex-col">
          {isLoading ? (
            <div className="flex h-[360px] items-center justify-center text-sm text-muted-foreground">
              {t("dictionary.loading")}
            </div>
          ) : visibleEntries.length === 0 ? (
            <div className="p-3">
              <DictionaryEmptyState tab={activeTab} />
            </div>
          ) : (
            <div className="flex flex-col p-3" data-testid="dictionary-entries">
              {visibleEntries.map((entry) => (
                <DictionaryEntryRow
                  key={`${activeTab}:${entry.term}`}
                  entry={entry}
                  tab={activeTab}
                  onDelete={handleDelete}
                />
              ))}
            </div>
          )}
        </Card>
      </div>
    </SettingsPageLayout>
  );
}
