'use client';

import { ArrowRight } from 'lucide-react';
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
        onClick={trackDownload}
        className={`inline-flex items-center gap-2 px-8 py-4 rounded-lg bg-primary text-primary-foreground font-medium hover:bg-primary/90 transition-colors ${className}`}
      >
        {t('hero.cta')}
        <ArrowRight className="w-4 h-4" />
      </a>
    );
  }

  // Fallback: go to download page (also shown while loading)
  return (
    <Link
      href={`/${lang}/download`}
      onClick={handleLinkClick}
      className="inline-flex items-center gap-2 px-8 py-4 rounded-lg bg-primary text-primary-foreground font-medium hover:bg-primary/90 transition-colors"
    >
      {t('hero.cta')}
      <ArrowRight className="w-4 h-4" />
    </Link>
  );
}
