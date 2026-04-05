'use client';

import { useTranslation } from 'react-i18next';
import Link from 'next/link';
import { useParams } from 'next/navigation';
import { useAnalytics } from '@/lib/analytics';
import { AnalyticsEvents } from '@/lib/events';

export default function Footer() {
  const { trackEvent } = useAnalytics();
  const { t } = useTranslation();
  const params = useParams();
  const lang = (params?.lang as string) || 'en';

  return (
    <footer className="border-t border-border/50">
      <div className="max-w-5xl mx-auto px-6 py-6">
        <div className="flex flex-col md:flex-row items-center justify-between gap-4 text-xs tracking-wide">
          <p className="text-foreground/50">{t('footer.copyright')}</p>
          <div className="flex items-center gap-6">
            <Link
              href={`/${lang}/privacy`}
              onClick={() => trackEvent(AnalyticsEvents.FOOTER_LINK_CLICK, { label: 'privacy' })}
              className="text-foreground/50 hover:text-foreground transition-colors"
            >
              {t('footer.privacy')}
            </Link>
            <Link
              href={`/${lang}/terms`}
              onClick={() => trackEvent(AnalyticsEvents.FOOTER_LINK_CLICK, { label: 'terms' })}
              className="text-foreground/50 hover:text-foreground transition-colors"
            >
              {t('footer.terms')}
            </Link>
          </div>
        </div>
      </div>
    </footer>
  );
}
