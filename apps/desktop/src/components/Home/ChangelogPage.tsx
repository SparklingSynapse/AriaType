import { useState, useEffect, useRef } from "react";
import { Loader2, WifiOff } from "lucide-react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { OverlayScrollbarsComponent } from "overlayscrollbars-react";
import { Button } from "@/components/ui/button";
import "overlayscrollbars/overlayscrollbars.css";

const CHANGELOG_URL =
  "https://raw.githubusercontent.com/joe223/AriaType/refs/heads/master/CHANGELOG.md";

function parseMarkdownToHtml(markdown: string): string {
  const lines = markdown.split("\n");
  const htmlLines: string[] = [];
  let skipSection = false;

  for (const line of lines) {
    if (line.startsWith("## ")) {
      const sectionTitle = line.slice(3).trim().toLowerCase();
      skipSection = sectionTitle === "unreleased";
      if (skipSection) continue;
    }

    if (skipSection) continue;

    if (line.startsWith("# ")) {
      htmlLines.push(`<h1 class="text-xl font-bold tracking-tight mb-4">${line.slice(2)}</h1>`);
    } else if (line.startsWith("## ")) {
      htmlLines.push(`<h2 class="text-lg font-semibold tracking-tight mt-6 mb-2">${line.slice(3)}</h2>`);
    } else if (line.startsWith("### ")) {
      htmlLines.push(`<h3 class="text-sm font-medium uppercase tracking-wider text-muted-foreground mt-4 mb-2">${line.slice(4)}</h3>`);
    } else if (line.startsWith("- ")) {
      const content = line.slice(2);
      const withHash = content.replace(
        /\(([a-f0-9]{7})\)/g,
        '<span class="text-muted-foreground text-xs ml-2">($1)</span>'
      );
      htmlLines.push(`<li class="text-sm leading-7 mb-1 pl-4">${withHash}</li>`);
    } else if (line.trim() === "") {
      continue;
    } else {
      htmlLines.push(`<p class="text-sm text-muted-foreground leading-7">${line}</p>`);
    }
  }

  return htmlLines.join("\n");
}

export function ChangelogPage() {
  const { t } = useTranslation();
  const [changelog, setChangelog] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const fetchingRef = useRef(false);

  const handleRetry = () => {
    fetchingRef.current = false;
    setError(null);
    setChangelog(null);
    setLoading(true);
  };

  useEffect(() => {
    if (!loading || changelog || fetchingRef.current) return;

    fetchingRef.current = true;
    const startTime = Date.now();

    fetch(CHANGELOG_URL)
      .then((response) => {
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }
        return response.text();
      })
      .then((text) => {
        const elapsed = Date.now() - startTime;
        const minDelay = 1000;
        const remainingDelay = Math.max(0, minDelay - elapsed);

        setTimeout(() => {
          setChangelog(text);
          setLoading(false);
        }, remainingDelay);
      })
      .catch((err) => {
        const elapsed = Date.now() - startTime;
        const minDelay = 1000;
        const remainingDelay = Math.max(0, minDelay - elapsed);

        setTimeout(() => {
          const message = err instanceof Error ? err.message : "Unknown error";
          logger.error("changelog_fetch_failed", { error: message });
          setError(t("about.changelog.error"));
          setLoading(false);
          fetchingRef.current = false;
        }, remainingDelay);
      });
  }, [loading, changelog, t]);

  const parsedHtml = changelog ? parseMarkdownToHtml(changelog) : "";
  const isEmpty = parsedHtml.trim().length === 0;

  return (
    <div className="h-full p-6" data-testid="changelog-page">
      <div className="h-full rounded-2xl border border-border bg-card p-6 overflow-hidden">
        {loading && (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        )}

        {error && !loading && (
          <div className="flex flex-col items-center justify-center h-full gap-3">
            <WifiOff className="h-6 w-6 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">{error}</p>
            <Button variant="outline" size="sm" onClick={handleRetry}>
              {t("about.changelog.retry")}
            </Button>
          </div>
        )}

        {isEmpty && !loading && !error && (
          <div className="flex items-center justify-center h-full">
            <p className="text-sm text-muted-foreground">{t("about.changelog.empty")}</p>
          </div>
        )}

        {!isEmpty && !loading && !error && (
          <OverlayScrollbarsComponent
            defer
            className="h-full"
            options={{
              showNativeOverlaidScrollbars: false,
              scrollbars: {
                theme: "os-theme-dark",
                visibility: "auto",
                autoHide: "scroll",
                autoHideDelay: 300,
              },
            }}
          >
            <div
              className="prose prose-sm dark:prose-invert max-w-none"
              dangerouslySetInnerHTML={{
                __html: parsedHtml,
              }}
            />
          </OverlayScrollbarsComponent>
        )}
      </div>
    </div>
  );
}