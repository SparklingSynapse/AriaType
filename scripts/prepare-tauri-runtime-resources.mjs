#!/usr/bin/env node
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, posix as pathPosix, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultRoot = resolve(__dirname, '..');

export const GENERATED_RUNTIME_CONFIG =
  'apps/desktop/src-tauri/tauri.runtime.generated.conf.json';

const RUNTIME_RESOURCE_CANDIDATES = {
  macos: [
    'bin/apple-silicon/llama-server',
    'bin/intel/llama-server',
    'bin/universal/llama-server',
    'bin/macos/llama-server',
  ],
  windows: [
    'bin/windows/llama-server.exe',
    'bin/windows/llama-server',
  ],
  linux: [
    'bin/linux/llama-server',
    'bin/llama-server',
  ],
};

const RUNTIME_RESOURCE_DIRS = {
  macos: [
    'bin/apple-silicon',
    'bin/intel',
    'bin/universal',
    'bin/macos',
  ],
  windows: [
    'bin/windows',
  ],
  linux: [
    'bin/linux',
    'bin',
  ],
};

const RUNTIME_RESOURCE_DESTINATIONS = {
  macos: 'bin/universal/llama-server',
  windows: 'bin/windows/llama-server.exe',
  linux: 'bin/linux/llama-server',
};

const RUNTIME_SIDECAR_SPECS = {
  macos: [
    {
      pathEnv: 'ARIATYPE_LLAMA_SERVER_MACOS_ARM64_PATH',
      shaEnv: 'ARIATYPE_LLAMA_SERVER_MACOS_ARM64_SHA256',
      destinationResource: 'bin/apple-silicon/llama-server',
    },
    {
      pathEnv: 'ARIATYPE_LLAMA_SERVER_MACOS_X64_PATH',
      shaEnv: 'ARIATYPE_LLAMA_SERVER_MACOS_X64_SHA256',
      destinationResource: 'bin/intel/llama-server',
    },
    {
      pathEnv: 'ARIATYPE_LLAMA_SERVER_MACOS_PATH',
      shaEnv: 'ARIATYPE_LLAMA_SERVER_MACOS_SHA256',
      destinationResource: 'bin/universal/llama-server',
    },
  ],
  windows: [
    {
      pathEnv: 'ARIATYPE_LLAMA_SERVER_WINDOWS_X64_PATH',
      shaEnv: 'ARIATYPE_LLAMA_SERVER_WINDOWS_X64_SHA256',
      destinationResource: 'bin/windows/llama-server.exe',
    },
    {
      pathEnv: 'ARIATYPE_LLAMA_SERVER_WINDOWS_PATH',
      shaEnv: 'ARIATYPE_LLAMA_SERVER_WINDOWS_SHA256',
      destinationResource: 'bin/windows/llama-server.exe',
    },
  ],
  linux: [
    {
      pathEnv: 'ARIATYPE_LLAMA_SERVER_LINUX_X64_PATH',
      shaEnv: 'ARIATYPE_LLAMA_SERVER_LINUX_X64_SHA256',
      destinationResource: 'bin/linux/llama-server',
    },
    {
      pathEnv: 'ARIATYPE_LLAMA_SERVER_LINUX_PATH',
      shaEnv: 'ARIATYPE_LLAMA_SERVER_LINUX_SHA256',
      destinationResource: 'bin/linux/llama-server',
    },
  ],
};

const REQUIRED_RUNTIME_ENV = 'ARIATYPE_REQUIRE_LOCAL_POLISH_RUNTIME';

function tauriDir(rootDir) {
  return resolve(rootDir, 'apps/desktop/src-tauri');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function readConfig(rootDir, name) {
  return readJson(resolve(tauriDir(rootDir), name));
}

function unique(values) {
  return Array.from(new Set(values));
}

export function normalizePlatform(value = process.platform) {
  if (value === 'darwin') {
    return 'macos';
  }
  if (value === 'win32') {
    return 'windows';
  }
  if (value === 'linux') {
    return 'linux';
  }
  if (['macos', 'windows', 'linux'].includes(value)) {
    return value;
  }

  throw new Error(`Unsupported platform for runtime resources: ${value}`);
}

export function runtimeResourceCandidates(platformValue) {
  const platform = normalizePlatform(platformValue);
  return RUNTIME_RESOURCE_CANDIDATES[platform];
}

export function runtimeResourceDirs(platformValue) {
  const platform = normalizePlatform(platformValue);
  return RUNTIME_RESOURCE_DIRS[platform];
}

export function runtimeResourceDestination(platformValue) {
  const platform = normalizePlatform(platformValue);
  return RUNTIME_RESOURCE_DESTINATIONS[platform];
}

export function runtimeSidecarSpecs(platformValue) {
  const platform = normalizePlatform(platformValue);
  return RUNTIME_SIDECAR_SPECS[platform];
}

function envFlagEnabled(value) {
  return ['1', 'true', 'yes', 'on'].includes(String(value ?? '').trim().toLowerCase());
}

function sha256File(path) {
  const hash = createHash('sha256');
  hash.update(readFileSync(path));
  return hash.digest('hex');
}

function assertExecutableFile(path) {
  if (!existsSync(path)) {
    throw new Error(`Local polish runtime artifact not found: ${path}`);
  }

  const stats = statSync(path);
  if (!stats.isFile()) {
    throw new Error(`Local polish runtime artifact is not a file: ${path}`);
  }
}

function chmodExecutable(path) {
  try {
    // Keep user/group/world executable so Tauri resources work after packaging.
    // Windows ignores POSIX mode bits, but chmod is harmless there under Node.
    chmodSync(path, 0o755);
  } catch {
    // The subsequent spawn path will report a platform-specific error if the
    // file still cannot execute.
  }
}

function runtimeSupportFileMatcher(platformValue) {
  const platform = normalizePlatform(platformValue);
  if (platform === 'macos') {
    return (name) =>
      name === 'llama-server' ||
      name.endsWith('.dylib') ||
      name.endsWith('.metal') ||
      name.endsWith('.h');
  }
  if (platform === 'windows') {
    return (name) =>
      name === 'llama-server.exe' ||
      name === 'llama-server' ||
      name.toLowerCase().endsWith('.dll');
  }

  return (name) => name === 'llama-server' || /\.so(\.|$)/.test(name);
}

export function prepareRuntimeSidecarArtifact({
  platform = process.platform,
  rootDir = defaultRoot,
  sourcePath,
  expectedSha256,
  destinationResource = runtimeResourceDestination(platform),
  required = envFlagEnabled(process.env[REQUIRED_RUNTIME_ENV]),
} = {}) {
  const normalizedPlatform = normalizePlatform(platform);
  const destinationPath = resolve(tauriDir(rootDir), destinationResource);

  if (!sourcePath || !String(sourcePath).trim()) {
    if (required) {
      throw new Error(
        [
          `Missing local polish runtime artifact for ${normalizedPlatform}.`,
          'Set a matching ARIATYPE_LLAMA_SERVER_*_PATH environment variable',
          `or disable ${REQUIRED_RUNTIME_ENV}.`,
        ].join(' ')
      );
    }

    return {
      copied: false,
      reason: 'source_not_configured',
      destinationResource,
      destinationPath,
    };
  }

  const resolvedSource = resolve(rootDir, String(sourcePath).trim());
  assertExecutableFile(resolvedSource);

  if (expectedSha256 && String(expectedSha256).trim()) {
    const actualSha256 = sha256File(resolvedSource);
    if (actualSha256 !== String(expectedSha256).trim().toLowerCase()) {
      throw new Error(
        `Local polish runtime sha256 mismatch for ${resolvedSource}: expected ${expectedSha256}, got ${actualSha256}`
      );
    }
  }

  mkdirSync(dirname(destinationPath), { recursive: true });
  if (resolve(resolvedSource) !== resolve(destinationPath)) {
    copyFileSync(resolvedSource, destinationPath);
  }
  chmodExecutable(destinationPath);

  return {
    copied: true,
    sourcePath: resolvedSource,
    destinationResource,
    destinationPath,
  };
}

export function prepareRuntimeSidecarArtifacts({
  platform = process.platform,
  rootDir = defaultRoot,
  required = envFlagEnabled(process.env[REQUIRED_RUNTIME_ENV]),
  env = process.env,
  sidecarSourcePath,
  sidecarExpectedSha256,
} = {}) {
  if (sidecarSourcePath !== undefined) {
    return [
      prepareRuntimeSidecarArtifact({
        platform,
        rootDir,
        sourcePath: sidecarSourcePath,
        expectedSha256: sidecarExpectedSha256,
        required,
      }),
    ];
  }

  const copied = [];
  const skipped = [];
  const seenDestinations = new Set();

  for (const spec of runtimeSidecarSpecs(platform)) {
    if (seenDestinations.has(spec.destinationResource)) {
      continue;
    }

    const sourcePath = env[spec.pathEnv];
    if (!sourcePath || !String(sourcePath).trim()) {
      skipped.push({
        copied: false,
        reason: 'source_not_configured',
        pathEnv: spec.pathEnv,
        destinationResource: spec.destinationResource,
        destinationPath: resolve(tauriDir(rootDir), spec.destinationResource),
      });
      continue;
    }

    const result = prepareRuntimeSidecarArtifact({
      platform,
      rootDir,
      sourcePath,
      expectedSha256: env[spec.shaEnv],
      destinationResource: spec.destinationResource,
      required: true,
    });
    copied.push({ ...result, pathEnv: spec.pathEnv });
    seenDestinations.add(spec.destinationResource);
  }

  const existing = existingRuntimeResources(platform, rootDir);
  if (copied.length === 0 && existing.length > 0) {
    return existing.map((destinationResource) => ({
      copied: false,
      reason: 'already_present',
      destinationResource,
      destinationPath: resolve(tauriDir(rootDir), destinationResource),
    }));
  }

  if (copied.length === 0 && required) {
    const platformName = normalizePlatform(platform);
    throw new Error(
      [
        `Missing local polish runtime artifact for ${platformName}.`,
        `Set one of ${runtimeSidecarSpecs(platformName).map((spec) => spec.pathEnv).join(', ')}`,
        `or disable ${REQUIRED_RUNTIME_ENV}.`,
      ].join(' ')
    );
  }

  return copied.length > 0 ? copied : skipped.slice(0, 1);
}

export function baseResourcesForPlatform(platformValue, rootDir = defaultRoot) {
  const platform = normalizePlatform(platformValue);
  const baseResources = readConfig(rootDir, 'tauri.conf.json').bundle?.resources ?? [];

  if (platform === 'windows') {
    return readConfig(rootDir, 'tauri.windows.conf.json').bundle?.resources ?? baseResources;
  }

  return baseResources;
}

export function existingRuntimeResources(platformValue, rootDir = defaultRoot) {
  const rootTauriDir = tauriDir(rootDir);
  const matchesRuntimeSupportFile = runtimeSupportFileMatcher(platformValue);
  const executableResources = runtimeResourceCandidates(platformValue).filter((resource) =>
    existsSync(resolve(rootTauriDir, resource))
  );
  const executableDirs = new Set(
    executableResources.map((resource) => pathPosix.dirname(resource))
  );
  const resources = [...executableResources];

  for (const runtimeDir of runtimeResourceDirs(platformValue)) {
    if (!executableDirs.has(runtimeDir)) {
      continue;
    }

    const absoluteDir = resolve(rootTauriDir, runtimeDir);
    if (!existsSync(absoluteDir)) {
      continue;
    }

    const entries = readdirSync(absoluteDir, { withFileTypes: true }).sort((a, b) =>
      a.name.localeCompare(b.name)
    );

    for (const entry of entries) {
      if (!entry.isFile() && !entry.isSymbolicLink()) {
        continue;
      }
      if (!matchesRuntimeSupportFile(entry.name)) {
        continue;
      }

      resources.push(pathPosix.join(runtimeDir, entry.name));
    }
  }

  return unique(resources);
}

export function runtimeResourceConfig(platformValue, rootDir = defaultRoot) {
  const platform = normalizePlatform(platformValue);
  const resources = unique([
    ...baseResourcesForPlatform(platform, rootDir),
    ...existingRuntimeResources(platform, rootDir),
  ]);

  return {
    $schema: 'https://schema.tauri.app/config/2',
    bundle: {
      resources,
    },
  };
}

export function writeRuntimeResourceConfig({
  platform = process.platform,
  rootDir = defaultRoot,
  outputPath = resolve(rootDir, GENERATED_RUNTIME_CONFIG),
  requiredRuntime = envFlagEnabled(process.env[REQUIRED_RUNTIME_ENV]),
  sidecarSourcePath,
  sidecarExpectedSha256,
} = {}) {
  const sidecarOptions = {
    platform,
    rootDir,
    required: requiredRuntime,
  };
  if (sidecarSourcePath !== undefined) {
    sidecarOptions.sourcePath = sidecarSourcePath;
  }
  if (sidecarExpectedSha256 !== undefined) {
    sidecarOptions.expectedSha256 = sidecarExpectedSha256;
  }

  const sidecars = prepareRuntimeSidecarArtifacts(sidecarOptions);
  const config = runtimeResourceConfig(platform, rootDir);
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(config, null, 2)}\n`);
  return { config, outputPath, sidecar: sidecars[0], sidecars };
}

function parsePlatformArg(args) {
  const index = args.indexOf('--platform');
  if (index === -1) {
    return process.platform;
  }

  const value = args[index + 1];
  if (!value) {
    throw new Error('--platform requires a value');
  }

  return value;
}

function parseRequiredRuntimeArg(args) {
  return args.includes('--require-runtime') || envFlagEnabled(process.env[REQUIRED_RUNTIME_ENV]);
}

function main() {
  const args = process.argv.slice(2);
  const platform = parsePlatformArg(args);
  const requiredRuntime = parseRequiredRuntimeArg(args);
  const { config, outputPath, sidecars } = writeRuntimeResourceConfig({
    platform,
    requiredRuntime,
  });
  const runtimeResources = existingRuntimeResources(platform);

  console.log(`Prepared Tauri runtime resources: ${outputPath}`);
  const copiedSidecars = sidecars.filter((sidecar) => sidecar.copied);
  if (copiedSidecars.length > 0) {
    for (const sidecar of copiedSidecars) {
      console.log(
        `Prepared local polish runtime sidecar: ${sidecar.sourcePath} -> ${sidecar.destinationResource}`
      );
    }
  } else if (sidecars.some((sidecar) => sidecar.reason === 'source_not_configured')) {
    console.log('No local polish runtime sidecar source configured.');
  }
  if (runtimeResources.length === 0) {
    console.log('No bundled local polish runtime resources found; using base resources only.');
  } else {
    console.log(`Bundled local polish runtime resources: ${runtimeResources.join(', ')}`);
  }
  console.log(`Total Tauri resources: ${config.bundle.resources.length}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
