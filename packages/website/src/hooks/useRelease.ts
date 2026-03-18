import { useState, useEffect } from 'react';

export type MacArchitecture = 'aarch64' | 'x86_64' | 'universal' | 'unknown';

export interface ReleasePlatforms {
  mac?: {
    universal?: string;
    aarch64?: string;
    x86_64?: string;
  };
  windows?: {
    exe?: string;
    msi?: string;
  };
}

export interface LatestRelease {
  version: string;
  pub_date: string;
  notes: string;
  url: string;
  platforms?: ReleasePlatforms;
  files?: Array<{
    file: string;
    channel: string;
    url: string;
  }>;
}

export function getMacArchitecture(url: string): MacArchitecture {
  if (url.includes('_aarch64.') || url.includes('-arm64.')) return 'aarch64';
  if (url.includes('_universal.') || url.includes('-universal.')) return 'universal';
  if (url.includes('_x86_64.') || url.includes('-intel.') || url.includes('_x64.')) return 'x86_64';
  return 'unknown';
}

export function detectPlatform(): 'mac' | 'win' | 'other' {
  if (typeof window === 'undefined') return 'other';
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('mac')) return 'mac';
  if (ua.includes('win')) return 'win';
  return 'other';
}

async function detectMacArchitecture(): Promise<MacArchitecture> {
  if (typeof window === 'undefined') return 'unknown';

  const nav = navigator as Navigator & {
    userAgentData?: {
      getHighEntropyValues?: (hints: string[]) => Promise<{ architecture?: string }>;
    };
  };

  if (nav.userAgentData?.getHighEntropyValues) {
    try {
      const values = await nav.userAgentData.getHighEntropyValues(['architecture']);
      const architecture = (values?.architecture || '').toLowerCase();
      if (architecture.includes('arm')) return 'aarch64';
      if (architecture.includes('x86')) return 'x86_64';
    } catch {}
  }

  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('aarch64') || ua.includes('arm64') || ua.includes('apple silicon')) return 'aarch64';
  if (ua.includes('x86_64') || ua.includes('intel')) return 'x86_64';
  return 'unknown';
}

export function pickMacDownloadUrl(release: LatestRelease, preferredArch: MacArchitecture): string {
  const mac = release.platforms?.mac;
  const universal = mac?.universal || '';
  const arm = mac?.aarch64 || '';
  const intel = mac?.x86_64 || '';

  if (preferredArch === 'aarch64') return arm || universal || intel || release.url;
  if (preferredArch === 'x86_64') return intel || universal || arm || release.url;
  return universal || arm || intel || release.url;
}

export function useRelease() {
  const [release, setRelease] = useState<LatestRelease | null>(null);
  const [loading, setLoading] = useState(true);
  const [unavailable, setUnavailable] = useState(false);
  const [platform, setPlatform] = useState<'mac' | 'win' | 'other'>('other');
  const [macArch, setMacArch] = useState<MacArchitecture>('unknown');

  useEffect(() => {
    const currentPlatform = detectPlatform();
    setPlatform(currentPlatform);
    if (currentPlatform === 'mac') {
      detectMacArchitecture().then(setMacArch).catch(() => setMacArch('unknown'));
    }

    async function fetchRelease() {
      try {
        const res = await fetch('/release/latest.json');
        if (!res.ok) {
          setUnavailable(true);
          return;
        }

        const data: LatestRelease = await res.json();
        if (!data.url) {
          setUnavailable(true);
          return;
        }

        // Verify the download file is accessible.
        // Use no-cors so same-origin restriction doesn't block the check in dev.
        // An opaque response (type === 'opaque') means the server responded — treat as OK.
        const check = await fetch(data.url, { method: 'HEAD', mode: 'no-cors' });
        if (check.type === 'error') {
          setUnavailable(true);
          return;
        }

        setRelease(data);
      } catch {
        setUnavailable(true);
      } finally {
        setLoading(false);
      }
    }

    fetchRelease();
  }, []);

  return { release, loading, unavailable, platform, macArch };
}
