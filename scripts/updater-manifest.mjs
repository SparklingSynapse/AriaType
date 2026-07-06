const MACOS_PLATFORM_KEYS = ['darwin-aarch64', 'darwin-x86_64'];
const WINDOWS_PLATFORM_KEY = 'windows-x86_64';

function normalizedSearchText({ file, sourceDir = '' }) {
  return `${file} ${sourceDir}`.toLowerCase();
}

function unique(values) {
  return Array.from(new Set(values));
}

export function inferUpdaterPlatforms({ file, sourceDir = '' }) {
  const lowerFile = file.toLowerCase();
  const text = normalizedSearchText({ file, sourceDir });

  if (lowerFile.endsWith('.app.tar.gz')) {
    if (text.includes('universal-apple-darwin') || text.includes('universal')) {
      return MACOS_PLATFORM_KEYS;
    }
    if (text.includes('aarch64-apple-darwin') || text.includes('aarch64') || text.includes('arm64')) {
      return ['darwin-aarch64'];
    }
    if (text.includes('x86_64-apple-darwin') || text.includes('x86_64') || text.includes('x64') || text.includes('intel')) {
      return ['darwin-x86_64'];
    }
    return [];
  }

  if (
    lowerFile.endsWith('.nsis.zip')
    || lowerFile.endsWith('.msi.zip')
    || lowerFile.endsWith('.exe')
    || lowerFile.endsWith('.msi')
  ) {
    return [WINDOWS_PLATFORM_KEY];
  }

  return [];
}

export function isUpdaterArtifact({ file, sourceDir = '' }) {
  return !file.toLowerCase().endsWith('.sig')
    && inferUpdaterPlatforms({ file, sourceDir }).length > 0;
}

export function mergeUpdaterPlatforms({ existingLatest, version, nextPlatforms }) {
  if (existingLatest?.version !== version) {
    return { ...nextPlatforms };
  }

  return {
    ...(existingLatest.platforms || {}),
    ...nextPlatforms,
  };
}

export function buildUpdaterManifest({ version, pubDate, notes = '', platforms }) {
  const sortedPlatforms = {};
  for (const key of unique(Object.keys(platforms)).sort()) {
    sortedPlatforms[key] = platforms[key];
  }

  return {
    version,
    pub_date: pubDate,
    notes,
    platforms: sortedPlatforms,
  };
}
