import { useRelease, getMacArchitecture } from './useRelease';
import { useAnalytics } from '@/lib/analytics';
import { AnalyticsEvents } from '@/lib/events';

export function useDownload(location: string) {
  const { release, loading, unavailable, platform } = useRelease();
  const { trackEvent } = useAnalytics();

  const isMac = platform === 'mac';
  const canDirectDownload = !!(release && isMac);
  const downloadUrl = canDirectDownload ? release?.url : null;

  const trackDownload = () => {
    if (!release) return;
    trackEvent(AnalyticsEvents.DOWNLOAD_CLICK, {
      platform,
      url: release.url,
      version: release.version,
      arch: getMacArchitecture(release.url) || 'unknown',
      location,
    });
  };

  return {
    release,
    loading,
    unavailable,
    platform,
    isMac,
    canDirectDownload,
    downloadUrl,
    trackDownload,
  };
}
