'use client';

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Download, Loader2, AlertCircle, ChevronDown } from 'lucide-react';
import { motion } from 'framer-motion';
import { useDownload } from '@/hooks/useDownload';

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

function DownloadOptionButton({
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
      className={`flex min-h-14 items-center justify-between gap-4 rounded-2xl border px-4 py-4 text-left text-sm transition-colors ${
        option.recommended
          ? 'border-foreground/10 bg-foreground/[0.03] hover:bg-foreground/[0.05]'
          : 'border-border bg-background/70 hover:bg-secondary/60'
      }`}
    >
      <span className="flex items-center gap-3">
        <span className="font-medium text-foreground">{option.label}</span>
        {option.recommended && (
          <span className="rounded-full bg-secondary px-2.5 py-1 text-[11px] font-medium uppercase tracking-[0.16em] text-foreground">
            {recommendedLabel}
          </span>
        )}
      </span>
      <Download className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
    </a>
  );
}

function MetaRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="text-sm font-medium text-foreground">{value}</dd>
    </div>
  );
}

function PlatformBlock({
  title,
  requirements,
  options,
  emptyText,
  recommendedLabel,
  onTrack,
}: {
  title: string;
  requirements: string;
  options: DownloadOption[];
  emptyText: string;
  recommendedLabel: string;
  onTrack: (url: string) => void;
}) {
  return (
    <div className="rounded-3xl border border-border bg-card p-6 md:p-8">
      <div className="flex items-baseline justify-between gap-4">
        <h2 className="text-xl font-semibold tracking-[-0.03em] text-foreground">{title}</h2>
        <span className="text-sm text-muted-foreground">{requirements}</span>
      </div>

      <div className="mt-6 space-y-3">
        {options.length > 0 ? (
          options.map((option) => (
            <DownloadOptionButton
              key={option.url}
              option={option}
              onClick={() => onTrack(option.url)}
              recommendedLabel={recommendedLabel}
            />
          ))
        ) : (
          <p className="text-sm leading-7 text-muted-foreground">{emptyText}</p>
        )}
      </div>
    </div>
  );
}

export default function DownloadClient() {
  const { t } = useTranslation();
  const { release, loading, unavailable, platform, defaultMacUrl, trackDownload } = useDownload('download_page');
  const macUniversalUrl = release?.platforms?.mac?.universal || '';
  const macArmUrl = release?.platforms?.mac?.aarch64 || '';
  const macIntelUrl = release?.platforms?.mac?.x86_64 || '';
  const macOptions = [
    { label: t('download.universal'), url: macUniversalUrl, recommended: macUniversalUrl === defaultMacUrl },
    { label: t('download.macArm'), url: macArmUrl, recommended: macArmUrl === defaultMacUrl },
    { label: t('download.macIntel'), url: macIntelUrl, recommended: macIntelUrl === defaultMacUrl },
  ].filter((item) => item.url);
  const windowsExeUrl = release?.platforms?.windows?.exe || '';
  const windowsMsiUrl = release?.platforms?.windows?.msi || '';
  const windowsOptions = [
    { label: t('download.windowsExe'), url: windowsExeUrl },
    { label: t('download.windowsMsi'), url: windowsMsiUrl },
  ].filter((item) => item.url);
  const recommendedMacOption = macOptions.find((item) => item.recommended) ?? macOptions[0];
  const recommendedWindowsOption = windowsOptions[0];
  const primaryDownloadOption =
    platform === 'mac'
      ? recommendedMacOption
      : platform === 'win'
        ? recommendedWindowsOption
        : recommendedMacOption ?? recommendedWindowsOption;
  const primaryMeta = useMemo(() => {
    if (!primaryDownloadOption) {
      return '';
    }

    if (platform === 'mac') {
      return `${t('download.primaryMetaDetected')}: ${primaryDownloadOption.label}`;
    }

    if (platform === 'win') {
      return `${t('download.primaryMetaDetected')}: ${t('download.windows')}`;
    }

    return t('download.primaryMetaFallback');
  }, [platform, primaryDownloadOption, t]);
  const requirementsNote =
    platform === 'win' ? t('download.requirementsWin') : t('download.requirementsMac');
  const detectedHint = useMemo(() => {
    if (platform === 'mac') {
      return t('download.detectedMacHint');
    }

    if (platform === 'win') {
      return t('download.detectedWindowsHint');
    }

    return t('download.detectedOtherHint');
  }, [platform, t]);
  const detectedPlatform = useMemo(() => {
    if (platform === 'mac') {
      return t('download.macos');
    }

    if (platform === 'win') {
      return t('download.windows');
    }

    return t('download.otherPlatform');
  }, [platform, t]);
  const topFacts = [
    t('download.subtitle'),
    release ? `${t('download.currentVersion')}: v${release.version}` : null,
    requirementsNote,
  ].filter(Boolean);

  return (
    <div className="min-h-screen">
      <section className="pb-16 pt-24 md:pb-20 md:pt-32">
        <div className="mx-auto max-w-5xl px-6">
          <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_22rem] lg:items-start">
            <motion.div
              variants={reveal}
              initial="hidden"
              animate="visible"
              transition={{ ...transition, duration: 0.7 }}
              className="space-y-6"
            >
              <SectionLabel>{t('download.eyebrow')}</SectionLabel>
              <div className="space-y-4">
                <h1 className="max-w-2xl text-[clamp(2.5rem,5vw,4.5rem)] font-bold leading-[1.05] tracking-tight text-foreground">
                  {t('download.title')}
                </h1>
                <p className="max-w-xl text-base leading-8 text-muted-foreground">
                  {t('download.description')}
                </p>
              </div>

              <div className="flex flex-wrap gap-3 pt-1">
                {topFacts.map((fact) => (
                  <span
                    key={fact}
                    className="rounded-full border border-border bg-card px-4 py-2 text-sm text-muted-foreground"
                  >
                    {fact}
                  </span>
                ))}
              </div>
            </motion.div>

            <motion.div
              variants={reveal}
              initial="hidden"
              animate="visible"
              transition={{ ...transition, duration: 0.7, delay: 0.12 }}
              className="rounded-3xl border border-border bg-card p-6 md:p-8"
            >
              <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
                {t('download.detectedLabel')}
              </p>

              {loading && (
                <div className="flex min-h-60 flex-col items-center justify-center gap-4 text-center">
                  <Loader2 className="h-7 w-7 animate-spin text-muted-foreground" />
                  <p className="text-sm text-muted-foreground">{t('download.description')}</p>
                </div>
              )}

              {!loading && unavailable && (
                <div className="flex min-h-60 flex-col items-center justify-center gap-4 text-center">
                  <AlertCircle className="h-7 w-7 text-muted-foreground" />
                  <p className="max-w-xs text-sm leading-7 text-muted-foreground">{t('download.noRelease')}</p>
                </div>
              )}

              {!loading && release && primaryDownloadOption && (
                <div className="mt-5 space-y-6">
                  <div className="space-y-3">
                    <p className="text-sm text-muted-foreground">{primaryMeta}</p>
                    <h2 className="text-[clamp(1.6rem,3vw,2rem)] font-semibold leading-tight tracking-[-0.04em] text-foreground">
                      {primaryDownloadOption.label}
                    </h2>
                    <p className="text-sm leading-7 text-muted-foreground">{detectedHint}</p>
                  </div>

                  <a
                    href={primaryDownloadOption.url}
                    onClick={() => trackDownload(primaryDownloadOption.url)}
                    className="flex min-h-14 items-center justify-between gap-4 rounded-full bg-primary px-5 py-3 text-sm text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    <span className="font-medium">
                      {t('download.primaryCta')} {primaryDownloadOption.label}
                    </span>
                    <Download className="h-4 w-4 flex-shrink-0" />
                  </a>

                  <dl className="space-y-3 border-t border-border pt-5">
                    <MetaRow label={t('download.currentVersion')} value={`v${release.version}`} />
                    <MetaRow label={t('download.detectedLabel')} value={detectedPlatform} />
                    <MetaRow label={t('download.secondaryCta')} value={requirementsNote} />
                  </dl>
                </div>
              )}
            </motion.div>
          </div>
        </div>
      </section>

      {!loading && release && (
        <section className="border-t border-border/70 pb-20 md:pb-24">
          <div className="mx-auto max-w-5xl px-6 pt-12 md:pt-16">
            <motion.div
              variants={reveal}
              initial="hidden"
              animate="visible"
              transition={{ ...transition, delay: 0.16 }}
              className="space-y-10"
            >
              <div className="max-w-2xl space-y-4">
                <SectionLabel>{t('download.chooseEyebrow')}</SectionLabel>
                <h2 className="text-[clamp(1.8rem,3vw,2.4rem)] font-semibold leading-[1.08] tracking-[-0.04em] text-foreground">
                  {t('download.chooseTitle')}
                </h2>
                <p className="text-base leading-8 text-muted-foreground">
                  {t('download.chooseDescription')}
                </p>
              </div>

              <div className="grid gap-6 md:grid-cols-2">
                <PlatformBlock
                  title={t('download.macos')}
                  requirements={t('download.requirementsMac')}
                  options={
                    macOptions.length > 0
                      ? macOptions
                      : [
                          {
                            label: t('download.macos'),
                            url: release.url,
                            recommended: platform === 'mac',
                          },
                        ]
                  }
                  emptyText={t('download.comingSoon')}
                  recommendedLabel={t('download.recommendedBadge')}
                  onTrack={trackDownload}
                />
                <PlatformBlock
                  title={t('download.windows')}
                  requirements={t('download.requirementsWin')}
                  options={windowsOptions}
                  emptyText={t('download.comingSoon')}
                  recommendedLabel={t('download.recommendedBadge')}
                  onTrack={trackDownload}
                />
              </div>

              {release.notes && (
                <details className="group rounded-3xl border border-border bg-card px-6 py-5 md:px-8">
                  <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-sm font-medium text-foreground">
                    <div>
                      <span>{t('download.releaseNotes')}</span>
                      <p className="mt-1 text-sm font-normal leading-7 text-muted-foreground">
                        {t('download.releaseNotesDescription')}
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
