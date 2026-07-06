import { execSync } from 'child_process';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { basename, dirname, join, resolve, sep } from 'path';

export const ENSURE_WINDOWS_RUNTIME_COMMAND =
  'node ../../scripts/ensure-llama-server-runtime.mjs --platform windows';

export const PREPARE_WINDOWS_RUNTIME_COMMAND =
  'node ../../scripts/prepare-tauri-runtime-resources.mjs --platform windows --require-runtime';

export const RUN_TAURI_BUILD_WITH_UPDATER_SIGNING_COMMAND =
  'node ../../scripts/run-tauri-build-with-updater-signing.mjs --';

export const UPDATER_CONFIG = 'src-tauri/tauri.updater.conf.json';
export const NPM_TAURI_BUILD_COMMAND = 'npm run tauri -- build';

export const WINDOWS_NATIVE_BUILD_COMMAND =
  `${ENSURE_WINDOWS_RUNTIME_COMMAND} && ${PREPARE_WINDOWS_RUNTIME_COMMAND} && ${RUN_TAURI_BUILD_WITH_UPDATER_SIGNING_COMMAND} ${NPM_TAURI_BUILD_COMMAND} --config src-tauri/tauri.windows.conf.json --config ${UPDATER_CONFIG} --config src-tauri/tauri.runtime.generated.conf.json --target x86_64-pc-windows-msvc`;

export const WINDOWS_CROSS_BUILD_COMMAND =
  `${ENSURE_WINDOWS_RUNTIME_COMMAND} && ${PREPARE_WINDOWS_RUNTIME_COMMAND} && ${RUN_TAURI_BUILD_WITH_UPDATER_SIGNING_COMMAND} cargo tauri build --config src-tauri/tauri.windows.conf.json --config ${UPDATER_CONFIG} --config src-tauri/tauri.runtime.generated.conf.json --runner cargo-xwin --target x86_64-pc-windows-msvc`;

export const WINDOWS_NATIVE_UNSIGNED_BUILD_COMMAND =
  `${ENSURE_WINDOWS_RUNTIME_COMMAND} && ${PREPARE_WINDOWS_RUNTIME_COMMAND} && ${NPM_TAURI_BUILD_COMMAND} --config src-tauri/tauri.windows.conf.json --config src-tauri/tauri.runtime.generated.conf.json --target x86_64-pc-windows-msvc`;

export const WINDOWS_CROSS_UNSIGNED_BUILD_COMMAND =
  `${ENSURE_WINDOWS_RUNTIME_COMMAND} && ${PREPARE_WINDOWS_RUNTIME_COMMAND} && cargo tauri build --config src-tauri/tauri.windows.conf.json --config src-tauri/tauri.runtime.generated.conf.json --runner cargo-xwin --target x86_64-pc-windows-msvc`;

function write(log, level, message) {
  const method = log[level] ?? log.log;
  method.call(log, message);
}

function getErrorMessage(error) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function isRetryableNotarizationTimeout(error, description) {
  if (!description.startsWith('Building macOS')) {
    return false;
  }

  const message = getErrorMessage(error);
  return (
    message.includes('failed to notarize app')
    && (message.includes('HTTPClientError.deadlineExceeded') || message.includes('abortedUpload'))
  );
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

export function findLastBundledDmgPath(buildLog) {
  const pattern = /^\s*Bundling .+?\.dmg \((.+?\.dmg)\)$/gm;
  let found;
  let match;

  while ((match = pattern.exec(String(buildLog))) !== null) {
    found = match[1];
  }

  return found;
}

export function inferDmgVolumeName(dmgPath) {
  return basename(dmgPath, '.dmg').replace(/_[0-9][^_]*_(aarch64|x64|universal)$/, '');
}

export function createDmgTraceCommand(options) {
  const {
    scriptPath,
    dmgPath,
    sourceDir,
    traceDmgPath,
    traceLogPath,
    backgroundPath,
    windowSize,
  } = options;

  const args = [
    '--volname',
    inferDmgVolumeName(dmgPath),
  ];

  if (backgroundPath) {
    args.push('--background', backgroundPath);
  }

  if (windowSize) {
    args.push('--window-size', String(windowSize.width), String(windowSize.height));
  }

  args.push(traceDmgPath, sourceDir);

  return [
    'set -o pipefail;',
    `cd ${shellQuote(dirname(scriptPath))}`,
    '&&',
    'bash -x ./bundle_dmg.sh',
    args.map(shellQuote).join(' '),
    '2>&1',
    '|',
    'tee',
    shellQuote(traceLogPath),
  ].join(' ');
}

function normalizeVolumeName(value) {
  return basename(value).replace(/ \d+$/, '');
}

function normalizePath(value) {
  return resolve(value);
}

function pathIsInside(parent, child) {
  const normalizedParent = normalizePath(parent);
  const normalizedChild = normalizePath(child);
  return (
    normalizedChild === normalizedParent
    || normalizedChild.startsWith(`${normalizedParent}${sep}`)
  );
}

export function parseHdiutilMountedImages(hdiutilInfo) {
  const images = [];
  let current = null;

  for (const rawLine of String(hdiutilInfo).split(/\r?\n/)) {
    const line = rawLine.trimEnd();

    if (line.startsWith('image-path')) {
      if (current) {
        images.push(current);
      }
      current = {
        imagePath: line.slice(line.indexOf(':') + 1).trim(),
        mountPoints: [],
      };
      continue;
    }

    if (!current || !line.startsWith('/dev/')) {
      continue;
    }

    const volumeIndex = line.indexOf('/Volumes/');
    if (volumeIndex >= 0) {
      current.mountPoints.push(line.slice(volumeIndex).trim());
    }
  }

  if (current) {
    images.push(current);
  }

  return images;
}

export function findRepoDmgMounts(hdiutilInfo, options) {
  const {
    repoRoot,
    volumeNames,
  } = options;
  const allowedVolumeNames = new Set(volumeNames);

  return parseHdiutilMountedImages(hdiutilInfo)
    .filter((image) => image.imagePath.endsWith('.dmg'))
    .filter((image) => pathIsInside(repoRoot, image.imagePath))
    .flatMap((image) => image.mountPoints.map((mountPoint) => ({
      imagePath: image.imagePath,
      mountPoint,
      volumeName: normalizeVolumeName(mountPoint),
    })))
    .filter((mount) => allowedVolumeNames.has(mount.volumeName));
}

export function detachRepoDmgMounts(options) {
  const {
    repoRoot,
    volumeNames = ['AriaType', 'AriaType Inhouse'],
    exec = execSync,
    log = console,
  } = options;

  const info = exec('hdiutil info', { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  const mounts = findRepoDmgMounts(info, { repoRoot, volumeNames });

  for (const mount of mounts) {
    write(
      log,
      'warn',
      `Detaching stale ${mount.volumeName} DMG mount before packaging: ${mount.mountPoint}`
    );
    exec(`hdiutil detach ${shellQuote(mount.mountPoint)}`, { stdio: 'inherit' });
  }

  return mounts;
}

function appendEnvToken(value, token) {
  if (!value) {
    return token;
  }

  return value.split(/\s+/).includes(token) ? value : `${value} ${token}`;
}

export function windowsCrossBuildEnv(baseEnv = process.env) {
  return {
    ...baseEnv,
    LLAMA_STATIC_CRT: '1',
    STATIC_VCRUNTIME: 'false',
    RUSTFLAGS: appendEnvToken(baseEnv.RUSTFLAGS, '-Ctarget-feature=+crt-static'),
  };
}

export function createBundleArtifactPreserver(options) {
  const {
    targetDir,
    cacheDir = mkdtempSync(join(tmpdir(), 'ariatype-build-artifacts-')),
    log = console,
  } = options;
  const preserved = new Map();

  function bundleDirFor(targetTriple, rootDir = targetDir) {
    return resolve(rootDir, targetTriple, 'release/bundle');
  }

  return {
    cacheDir,

    preserve(targetTriple) {
      const source = bundleDirFor(targetTriple);
      if (!existsSync(source)) {
        write(log, 'warn', `No bundle artifacts found to preserve for ${targetTriple}: ${source}`);
        return false;
      }

      const destination = bundleDirFor(targetTriple, cacheDir);
      rmSync(destination, { recursive: true, force: true });
      mkdirSync(dirname(destination), { recursive: true });
      cpSync(source, destination, { recursive: true, force: true });
      preserved.set(targetTriple, destination);
      write(log, 'info', `   Preserved bundle artifacts for ${targetTriple}`);
      return true;
    },

    restore() {
      const restored = [];
      for (const [targetTriple, source] of preserved) {
        const destination = bundleDirFor(targetTriple);
        rmSync(destination, { recursive: true, force: true });
        mkdirSync(dirname(destination), { recursive: true });
        cpSync(source, destination, { recursive: true, force: true });
        restored.push(targetTriple);
      }
      return restored;
    },

    cleanup() {
      rmSync(cacheDir, { recursive: true, force: true });
    },
  };
}

export function collectReleaseAssetsFromTarget(options) {
  const {
    targetDir,
    releaseDir,
    version,
    log = console,
  } = options;
  const copied = [];
  const seen = new Set();

  function copyFile(source, fileName = basename(source)) {
    if (!existsSync(source)) {
      return false;
    }
    const destination = resolve(releaseDir, fileName);
    cpSync(source, destination, { force: true });
    if (!seen.has(fileName)) {
      copied.push(fileName);
      seen.add(fileName);
    }
    return true;
  }

  function copyFilesFrom(sourceDir, predicate) {
    if (!existsSync(sourceDir)) {
      return;
    }
    for (const file of readdirSync(sourceDir).sort()) {
      if (predicate(file)) {
        copyFile(resolve(sourceDir, file), file);
      }
    }
  }

  rmSync(releaseDir, { recursive: true, force: true });
  mkdirSync(releaseDir, { recursive: true });

  const macTargets = [
    { targetTriple: 'aarch64-apple-darwin', archiveSuffix: `${version}_aarch64` },
    { targetTriple: 'x86_64-apple-darwin', archiveSuffix: `${version}_x64` },
    { targetTriple: 'universal-apple-darwin', archiveSuffix: '' },
  ];
  for (const target of macTargets) {
    const bundleDir = resolve(targetDir, target.targetTriple, 'release/bundle');
    copyFilesFrom(resolve(bundleDir, 'dmg'), (file) => file.endsWith('.dmg'));

    const updaterArchive = resolve(bundleDir, 'macos/AriaType.app.tar.gz');
    const updaterArchiveName = target.archiveSuffix
      ? `AriaType_${target.archiveSuffix}.app.tar.gz`
      : 'AriaType.app.tar.gz';
    if (copyFile(updaterArchive, updaterArchiveName)) {
      copyFile(`${updaterArchive}.sig`, `${updaterArchiveName}.sig`);
    }
  }

  const windowsBundleDirs = [
    resolve(targetDir, 'x86_64-pc-windows-msvc/release/bundle/nsis'),
    resolve(targetDir, 'release/bundle/nsis'),
    resolve(targetDir, 'x86_64-pc-windows-msvc/release/bundle/msi'),
    resolve(targetDir, 'release/bundle/msi'),
  ];
  for (const bundleDir of windowsBundleDirs) {
    copyFilesFrom(bundleDir, (file) => (
      file.endsWith('.exe')
      || file.endsWith('.msi')
      || file.endsWith('.nsis.zip')
      || file.endsWith('.msi.zip')
      || file.endsWith('.sig')
    ));
  }

  write(log, 'info', `   Collected ${copied.length} GitHub release asset(s) in ${releaseDir}`);
  return copied;
}

export function runCommand(command, description, options = {}) {
  const {
    cwd,
    env,
    exec = execSync,
    log = console,
    logFile,
    maxAttempts = 1,
    onFailure,
  } = options;

  write(log, 'info', `\n${'═'.repeat(50)}`);
  write(log, 'info', `📦 ${description}`);
  write(log, 'info', `${'═'.repeat(50)}\n`);

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    const startTime = Date.now();

    try {
      const execOptions = {
        cwd,
        stdio: 'inherit',
        env,
      };
      const execCommand = logFile
        ? `set -o pipefail; (${command}) 2>&1 | tee ${shellQuote(logFile)}`
        : command;

      if (logFile) {
        execOptions.shell = '/bin/bash';
      }

      exec(execCommand, execOptions);

      const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      write(log, 'info', `\n✅ ${description} completed in ${elapsed}s\n`);
      return true;
    } catch (error) {
      if (attempt < maxAttempts && isRetryableNotarizationTimeout(error, description)) {
        write(
          log,
          'warn',
          `\n⚠️  Retrying after notarization upload timeout (${attempt}/${maxAttempts - 1})\n`
        );
        continue;
      }

      onFailure?.(error);
      write(log, 'error', `\n❌ ${description} failed\n`);
      return false;
    }
  }

  write(log, 'error', `\n❌ ${description} failed\n`);
  return false;
}

export function checkRequiredBuildTools(requiredTools, options = {}) {
  const {
    exec = execSync,
    log = console,
  } = options;
  let allAvailable = true;

  for (const tool of requiredTools) {
    try {
      exec(`${tool.command} --version`, { stdio: 'ignore' });
    } catch {
      allAvailable = false;
      write(log, 'error', `❌ Missing required build tool: ${tool.description}`);
      write(log, 'error', `   Install: ${tool.installHint}`);
    }
  }

  if (!allAvailable) {
    write(log, 'error', '\nInstall the missing tools above, then rerun the build.\n');
  }

  return allAvailable;
}
