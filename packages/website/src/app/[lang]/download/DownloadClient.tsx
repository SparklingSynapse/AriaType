"use client";

import { useTranslation } from "react-i18next";
import { Download, Loader2, AlertCircle, ChevronDown } from "lucide-react";
import { motion } from "framer-motion";
import { useDownload } from "@/hooks/useDownload";

const reveal = {
  hidden: { opacity: 0, y: 16 },
  visible: { opacity: 1, y: 0 },
};

const transition = {
  duration: 0.6,
  ease: [0.16, 1, 0.3, 1],
};

type DownloadOption = {
  label: string;
  url: string;
  recommended?: boolean;
};

function SectionLabel({ children }: { children: string }) {
  return (
    <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
      {children}
    </p>
  );
}

function DownloadOptionRow({
  option,
  onClick,
  recommendedLabel,
}: {
  option: DownloadOption;
  onClick: () => void;
  recommendedLabel: string;
}) {
  return (
    <a
      href={option.url}
      onClick={onClick}
      className={`group flex items-center justify-between gap-4 rounded-2xl px-4 py-4 text-left transition-colors ${
        option.recommended
          ? "bg-foreground/[0.035] hover:bg-foreground/[0.05]"
          : "hover:bg-foreground/[0.025]"
      }`}
    >
      <span className="flex min-w-0 items-center gap-3">
        <span className="min-w-0 font-medium text-foreground">
          {option.label}
        </span>
        {option.recommended && (
          <span className="rounded-full bg-background px-2.5 py-1 text-[11px] font-medium uppercase tracking-[0.16em] text-foreground/80">
            {recommendedLabel}
          </span>
        )}
      </span>
      <Download className="h-4 w-4 flex-shrink-0 text-muted-foreground transition-transform duration-200 group-hover:translate-y-[1px]" />
    </a>
  );
}

function PlatformBlock({
  title,
  description,
  requirements,
  options,
  emptyText,
  recommendedLabel,
  onTrack,
}: {
  title: string;
  description: string;
  requirements: string;
  options: DownloadOption[];
  emptyText: string;
  recommendedLabel: string;
  onTrack: (url: string) => void;
}) {
  return (
    <section className="rounded-[2rem] border border-border/70 bg-background/80 p-6 md:p-7">
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-4">
          <h2 className="text-xl font-semibold tracking-[-0.03em] text-foreground">
            {title}
          </h2>
          <span className="text-sm text-muted-foreground">{requirements}</span>
        </div>
        <p className="max-w-xl text-sm leading-7 text-muted-foreground">
          {description}
        </p>
      </div>

      <div className="mt-5 rounded-2xl ">
        {options.length > 0 ? (
          options.map((option) => (
            <DownloadOptionRow
              key={option.url}
              option={option}
              onClick={() => onTrack(option.url)}
              recommendedLabel={recommendedLabel}
            />
          ))
        ) : (
          <p className="px-4 py-5 text-sm leading-7 text-muted-foreground">
            {emptyText}
          </p>
        )}
      </div>
    </section>
  );
}

export default function DownloadClient() {
  const { t, i18n } = useTranslation();
  const {
    release,
    loading,
    unavailable,
    platform,
    defaultMacUrl,
    trackDownload,
  } = useDownload("download_page");
  const macUniversalUrl = release?.platforms?.mac?.universal || "";
  const macArmUrl = release?.platforms?.mac?.aarch64 || "";
  const macIntelUrl = release?.platforms?.mac?.x86_64 || "";
  const macOptions = [
    {
      label: t("download.universal"),
      url: macUniversalUrl,
      recommended: macUniversalUrl === defaultMacUrl,
    },
    {
      label: t("download.macArm"),
      url: macArmUrl,
      recommended: macArmUrl === defaultMacUrl,
    },
    {
      label: t("download.macIntel"),
      url: macIntelUrl,
      recommended: macIntelUrl === defaultMacUrl,
    },
  ].filter((item) => item.url);
  const windowsExeUrl = release?.platforms?.windows?.exe || "";
  const windowsMsiUrl = release?.platforms?.windows?.msi || "";
  const windowsOptions = [
    { label: t("download.windowsExe"), url: windowsExeUrl },
    { label: t("download.windowsMsi"), url: windowsMsiUrl },
  ].filter((item) => item.url);
  const recommendedMacOption =
    macOptions.find((item) => item.recommended) ?? macOptions[0];
  const recommendedWindowsOption = windowsOptions[0];
  const primaryDownloadOption =
    platform === "mac"
      ? recommendedMacOption
      : platform === "win"
        ? recommendedWindowsOption
        : (recommendedMacOption ?? recommendedWindowsOption);
  const requirementsNote =
    platform === "win"
      ? t("download.requirementsWin")
      : t("download.requirementsMac");
  const topFacts = [
    t("download.subtitle"),
    release ? `${t("download.currentVersion")}: v${release.version}` : null,
    requirementsNote,
  ].filter(Boolean);
  const primaryButtonLabel = (() => {
    if (!primaryDownloadOption) {
      return t("download.primaryCta");
    }

    if (primaryDownloadOption.url === macArmUrl) {
      return t("download.primaryCtaMacArm");
    }

    if (primaryDownloadOption.url === macIntelUrl) {
      return t("download.primaryCtaMacIntel");
    }

    if (primaryDownloadOption.url === macUniversalUrl) {
      return t("download.primaryCtaMacUniversal");
    }

    if (primaryDownloadOption.url === windowsExeUrl) {
      return t("download.primaryCtaWindowsExe");
    }

    if (primaryDownloadOption.url === windowsMsiUrl) {
      return t("download.primaryCtaWindowsMsi");
    }

    return i18n.language.startsWith("zh")
      ? `${t("download.primaryCta")}${primaryDownloadOption.label}`
      : `${t("download.primaryCta")} ${primaryDownloadOption.label}`;
  })();

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top,rgba(120,113,108,0.08),transparent_42%)]">
      <section className="pb-16 pt-24 md:pb-20 md:pt-32">
        <div className="mx-auto max-w-5xl px-6 text-center">
          <motion.div
            variants={reveal}
            initial="hidden"
            animate="visible"
            transition={{ ...transition, duration: 0.7 }}
            className="max-w-3xl space-y-6 mx-auto"
          >
            <div className="space-y-4">
              <h1 className="text-[clamp(2.7rem,5vw,4.8rem)] font-semibold leading-[1.02] tracking-[-0.05em] text-foreground">
                {t("download.title")}
              </h1>
              <p className="text-base leading-8 text-muted-foreground md:text-[1.05rem]">
                {t("download.description")}
              </p>
            </div>

            <div className="flex flex-wrap gap-3 pt-1 items-center justify-center">
              {topFacts.map((fact) => (
                <span
                  key={fact}
                  className="rounded-full border border-border/70 bg-background/80 px-4 py-2 text-sm text-muted-foreground"
                >
                  {fact}
                </span>
              ))}
            </div>
          </motion.div>

          <div className="mt-14">
            <motion.div
              variants={reveal}
              initial="hidden"
              animate="visible"
              transition={{ ...transition, delay: 0.08 }}
            >
              {loading && (
                <div className="flex min-h-40 flex-col items-center justify-center gap-4 text-center">
                  <Loader2 className="h-7 w-7 animate-spin text-muted-foreground" />
                  <p className="text-sm text-muted-foreground">
                    {t("download.description")}
                  </p>
                </div>
              )}

              {!loading && unavailable && (
                <div className="flex min-h-40 flex-col items-center justify-center gap-4 text-center">
                  <AlertCircle className="h-7 w-7 text-muted-foreground" />
                  <p className="max-w-xs text-sm leading-7 text-muted-foreground">
                    {t("download.noRelease")}
                  </p>
                </div>
              )}

              {!loading && release && primaryDownloadOption && (
                <div className="flex min-h-40 items-center justify-center">
                  <a
                    href={primaryDownloadOption.url}
                    onClick={() => trackDownload(primaryDownloadOption.url)}
                    className="inline-flex min-h-14 items-center justify-center gap-3 rounded-full bg-primary px-6 py-3 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    <span>{primaryButtonLabel}</span>
                    <Download className="h-4 w-4 flex-shrink-0" />
                  </a>
                </div>
              )}
            </motion.div>
          </div>
        </div>
      </section>

      {!loading && release && (
        <section className="border-t border-border/70 pb-20 md:pb-24 text-center">
          <div className="mx-auto max-w-5xl px-6 pt-14 md:pt-20">
            <motion.div
              variants={reveal}
              initial="hidden"
              animate="visible"
              transition={{ ...transition, delay: 0.16 }}
              className="space-y-10 mx-auto"
            >
              <div className="space-y-4">
                <SectionLabel>{t("download.chooseEyebrow")}</SectionLabel>
                <h2 className="text-[clamp(1.8rem,3vw,2.4rem)] font-semibold leading-[1.08] tracking-[-0.04em] text-foreground">
                  {t("download.chooseTitle")}
                </h2>
                <p className="text-base leading-8 text-muted-foreground">
                  {t("download.chooseDescription")}
                </p>
              </div>

              <div className="grid gap-6 md:grid-cols-2">
                <PlatformBlock
                  title={t("download.macos")}
                  description={t("download.platformMacDescription")}
                  requirements={t("download.requirementsMac")}
                  options={
                    macOptions.length > 0
                      ? macOptions
                      : [
                          {
                            label: t("download.macos"),
                            url: release.url,
                            recommended: platform === "mac",
                          },
                        ]
                  }
                  emptyText={t("download.comingSoon")}
                  recommendedLabel={t("download.recommendedBadge")}
                  onTrack={trackDownload}
                />
                <PlatformBlock
                  title={t("download.windows")}
                  description={t("download.platformWindowsDescription")}
                  requirements={t("download.requirementsWin")}
                  options={windowsOptions}
                  emptyText={t("download.comingSoon")}
                  recommendedLabel={t("download.recommendedBadge")}
                  onTrack={trackDownload}
                />
              </div>

              {release.notes && (
                <details className="group rounded-[2rem] border border-border/70 bg-background/80 px-6 py-5 md:px-8">
                  <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-sm font-medium text-foreground">
                    <div>
                      <span>{t("download.releaseNotes")}</span>
                      <p className="mt-1 text-sm font-normal leading-7 text-muted-foreground">
                        {t("download.releaseNotesDescription")}
                      </p>
                    </div>
                    <ChevronDown className="h-4 w-4 text-muted-foreground transition-transform group-open:rotate-180" />
                  </summary>
                  <div className="mt-4 max-h-72 overflow-y-auto border-t border-border pt-4">
                    <pre className="whitespace-pre-wrap text-sm leading-7 text-muted-foreground">
                      {release.notes}
                    </pre>
                  </div>
                </details>
              )}
            </motion.div>
          </div>
        </section>
      )}
    </div>
  );
}
