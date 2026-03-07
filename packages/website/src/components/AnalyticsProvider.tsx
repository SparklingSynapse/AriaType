'use client';

import { AptabaseProvider, useAptabase } from '@aptabase/react';
import { usePathname } from 'next/navigation';
import { useEffect } from 'react';
import { AnalyticsEvents } from '@/lib/events';

function AnalyticsTracker() {
  const { trackEvent } = useAptabase();
  const pathname = usePathname();

  useEffect(() => {
    if (pathname) {
      trackEvent(AnalyticsEvents.PAGE_VIEW, { path: pathname });
    }
  }, [pathname, trackEvent]);

  return null;
}

export function AnalyticsProvider({ children }: { children: React.ReactNode }) {
  return (
    <AptabaseProvider appKey="A-US-4787705900">
      <AnalyticsTracker />
      {children}
    </AptabaseProvider>
  );
}
