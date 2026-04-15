import { useState, useEffect, useRef } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { motion, AnimatePresence } from "framer-motion";
import { X, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { OverlayScrollbarsComponent } from "overlayscrollbars-react";

const CHANGELOG_URL =
  "https://raw.githubusercontent.com/joe223/AriaType/refs/heads/master/CHANGELOG.md";

interface ChangelogModalProps {
  isOpen: boolean;
  onClose: () => void;
}

function parseMarkdownToHtml(markdown: string): string {
  const lines = markdown.split("\n");
  const htmlLines: string[] = [];

  for (const line of lines) {
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

export function ChangelogModal({ isOpen, onClose }: ChangelogModalProps) {
  const { t } = useTranslation();
  const [changelog, setChangelog] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fetchingRef = useRef(false);

  useEffect(() => {
    if (!isOpen) return;
    if (changelog || fetchingRef.current) return;

    fetchingRef.current = true;
    setLoading(true);
    setError(null);

    fetch(CHANGELOG_URL)
      .then((response) => {
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }
        return response.text();
      })
      .then((text) => {
        setChangelog(text);
        setLoading(false);
      })
      .catch((err) => {
        const message = err instanceof Error ? err.message : "Unknown error";
        logger.error("changelog_fetch_failed", { error: message });
        setError(t("about.changelog.error"));
        setLoading(false);
        fetchingRef.current = false;
      });
  }, [isOpen, changelog, t]);

  const handleRetry = () => {
    fetchingRef.current = false;
    setError(null);
    setChangelog(null);
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <AnimatePresence>
        {isOpen && (
          <Dialog.Portal forceMount>
            <Dialog.Overlay asChild>
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.15 }}
                className="fixed inset-0 z-50 bg-black/50"
              />
            </Dialog.Overlay>
            <div className="fixed inset-0 z-50 flex items-center justify-center pointer-events-none p-4">
              <Dialog.Content asChild>
                <motion.div
                  initial={{ opacity: 0, scale: 0.95 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                  transition={{ duration: 0.15 }}
                  className="bg-card border border-border rounded-3xl max-w-2xl w-full max-h-[80vh] shadow-lg pointer-events-auto flex flex-col"
                >
                  <div className="flex items-center justify-between p-6 pb-4">
                    <Dialog.Title className="text-lg font-semibold tracking-tight">
                      {t("about.changelog.title")}
                    </Dialog.Title>
                    <Dialog.Close asChild>
                      <button
                        className="rounded-2xl p-2 hover:bg-secondary transition-colors"
                        aria-label={t("about.changelog.close")}
                      >
                        <X className="h-4 w-4" />
                      </button>
                    </Dialog.Close>
                  </div>

                  <div className="flex-1 overflow-hidden px-6 pb-6">
                    {loading && (
                      <div className="flex items-center justify-center h-64">
                        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                      </div>
                    )}

                    {error && !loading && (
                      <div className="flex flex-col items-center justify-center h-64 gap-4">
                        <p className="text-sm text-destructive">{error}</p>
                        <button
                          onClick={handleRetry}
                          className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                        >
                          {t("about.changelog.retry")}
                        </button>
                      </div>
                    )}

                    {changelog && !loading && !error && (
                      <OverlayScrollbarsComponent defer className="h-full">
                        <div
                          className="prose prose-sm dark:prose-invert max-w-none"
                          dangerouslySetInnerHTML={{
                            __html: parseMarkdownToHtml(changelog),
                          }}
                        />
                      </OverlayScrollbarsComponent>
                    )}
                  </div>
                </motion.div>
              </Dialog.Content>
            </div>
          </Dialog.Portal>
        )}
      </AnimatePresence>
    </Dialog.Root>
  );
}