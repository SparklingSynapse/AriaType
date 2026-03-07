import { useState, useEffect } from 'react';

export interface LatestRelease {
  version: string;
  pub_date: string;
  notes: string;
  url: string;
}

export function getMacArchitecture(url: string): string {
  if (url.includes('_aarch64.') || url.includes('-arm64.')) return 'aarch64';
  if (url.includes('_universal.') || url.includes('-universal.')) return 'universal';
  if (url.includes('_x86_64.') || url.includes('-intel.')) return 'x86_64';
  return '';
}

export function detectPlatform(): 'mac' | 'win' | 'other' {
  if (typeof window === 'undefined') return 'other';
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('mac')) return 'mac';
  if (ua.includes('win')) return 'win';
  return 'other';
}

export function useRelease() {
  const [release, setRelease] = useState<LatestRelease | null>(null);
  const [loading, setLoading] = useState(true);
  const [unavailable, setUnavailable] = useState(false);
  const [platform, setPlatform] = useState<'mac' | 'win' | 'other'>('other');

  useEffect(() => {
    setPlatform(detectPlatform());

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

  return { release, loading, unavailable, platform };
}
