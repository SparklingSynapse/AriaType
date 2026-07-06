import { useState, useEffect } from 'react';

export const GITHUB_RELEASES_URL = 'https://github.com/joe223/AriaType/releases';
export const GITHUB_LATEST_RELEASE_URL = `${GITHUB_RELEASES_URL}/latest`;

export type MacArchitecture = 'aarch64' | 'x86_64' | 'universal' | 'unknown';
export type WebsitePlatform = 'mac' | 'win' | 'other';

export interface LatestRelease {
  version: string;
  pub_date: string;
  notes: string;
  url: string;
}

export function getMacArchitecture(url: string): MacArchitecture {
  const value = url.toLowerCase();
  if (value.includes('_aarch64.') || value.includes('-arm64.') || value.includes('_arm64.')) return 'aarch64';
  if (value.includes('_universal.') || value.includes('-universal.')) return 'universal';
  if (value.includes('_x86_64.') || value.includes('-intel.') || value.includes('_x64.')) return 'x86_64';
  return 'unknown';
}

export function detectPlatform(): WebsitePlatform {
  if (typeof window === 'undefined') return 'other';
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('mac')) return 'mac';
  if (ua.includes('win')) return 'win';
  return 'other';
}

function detectMacArchitectureFromUserAgent(userAgent: string): MacArchitecture {
  const ua = userAgent.toLowerCase();
  if (ua.includes('aarch64') || ua.includes('arm64') || ua.includes('apple silicon')) return 'aarch64';
  if (ua.includes('x86_64') || ua.includes('intel')) return 'x86_64';
  return 'unknown';
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
    } catch {
      return detectMacArchitectureFromUserAgent(navigator.userAgent);
    }
  }

  return detectMacArchitectureFromUserAgent(navigator.userAgent);
}

const LATEST_RELEASE: LatestRelease = {
  version: '',
  pub_date: '',
  notes: '',
  url: GITHUB_LATEST_RELEASE_URL,
};

export function useRelease() {
  const [platform, setPlatform] = useState<WebsitePlatform>('other');
  const [macArch, setMacArch] = useState<MacArchitecture>('unknown');

  useEffect(() => {
    const currentPlatform = detectPlatform();
    setPlatform(currentPlatform);
    if (currentPlatform === 'mac') {
      detectMacArchitecture().then(setMacArch).catch(() => setMacArch('unknown'));
    }
  }, []);

  return {
    release: LATEST_RELEASE,
    loading: false,
    unavailable: false,
    platform,
    macArch,
  };
}
