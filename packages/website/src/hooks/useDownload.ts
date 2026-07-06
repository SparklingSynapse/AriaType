import { useRelease, getMacArchitecture } from './useRelease';
import { useAnalytics } from '@/lib/analytics';
import { AnalyticsEvents } from '@/lib/events';

export function useDownload(location: string) {
  const { release, loading, unavailable, platform, macArch } = useRelease();
  const { trackEvent } = useAnalytics();

  const isMac = platform === 'mac';
  const defaultMacUrl = release.url;
  const downloadUrl = release.url;
  const canDirectDownload = !!downloadUrl;

  const trackDownload = (url?: string) => {
    const trackedUrl = url || downloadUrl || defaultMacUrl || release.url;
    trackEvent(AnalyticsEvents.DOWNLOAD_CLICK, {
      platform,
      url: trackedUrl,
      version: release.version,
      arch: getMacArchitecture(trackedUrl),
      detected_arch: macArch,
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
    defaultMacUrl,
    macArch,
    trackDownload,
  };
}
