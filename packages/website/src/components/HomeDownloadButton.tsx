'use client';

import Link from 'next/link';
import { useTranslation } from 'react-i18next';
import { useDownload } from '@/hooks/useDownload';
import { useAnalytics } from '@/lib/analytics';
import { AnalyticsEvents } from '@/lib/events';

interface HomeDownloadButtonProps {
  lang: string;
  className?: string;
}

export function HomeDownloadButton({ lang , className}: HomeDownloadButtonProps) {
  const { t } = useTranslation();
  const { platform, canDirectDownload, downloadUrl, trackDownload } = useDownload('home_hero');
  const { trackEvent } = useAnalytics();
  const baseClassName = `inline-flex h-11 items-center justify-center rounded-full bg-primary px-6 text-sm font-medium text-primary-foreground transition-all hover:bg-primary/90 ${className ?? ''}`;

  // Always show the button immediately - don't wait for release data
  const handleLinkClick = () => {
    trackEvent(AnalyticsEvents.CTA_CLICK, {
      location: 'hero',
      label: 'download',
      platform,
    });
  };

  // If we have a release and user is on Mac, show direct download
  if (canDirectDownload && downloadUrl) {
    return (
      <a
        href={downloadUrl}
        onClick={() => trackDownload(downloadUrl)}
        className={baseClassName}
      >
        {t('hero.cta')}
      </a>
    );
  }

  // Fallback: go to download page (also shown while loading)
  return (
    <Link
      href={`/${lang}/download`}
      onClick={handleLinkClick}
      className={baseClassName}
    >
      {t('hero.cta')}
    </Link>
  );
}
