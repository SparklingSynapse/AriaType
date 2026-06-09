#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, posix as pathPosix, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultRoot = resolve(__dirname, '..');

const SIDECAR_TARGETS = {
  macos: [
    {
      key: 'macos-arm64',
      archiveMatchers: [/macos/i, /(arm64|aarch64)/i],
      binaryName: 'llama-server',
      destinationResource: 'bin/apple-silicon/llama-server',
    },
    {
      key: 'macos-x64',
      archiveMatchers: [/macos/i, /(x64|x86_64)/i],
      binaryName: 'llama-server',
      destinationResource: 'bin/intel/llama-server',
    },
  ],
  windows: [
    {
      key: 'windows-x64-cpu',
      archiveMatchers: [/(win|windows)/i, /cpu/i, /(x64|x86_64)/i],
      binaryName: 'llama-server.exe',
      destinationResource: 'bin/windows/llama-server.exe',
    },
  ],
  linux: [
    {
      key: 'linux-x64-cpu',
      archiveMatchers: [/(ubuntu|linux)/i, /cpu/i, /(x64|x86_64)/i],
      binaryName: 'llama-server',
      destinationResource: 'bin/linux/llama-server',
    },
  ],
};

function tauriDir(rootDir) {
  return resolve(rootDir, 'apps/desktop/src-tauri');
}

export function normalizePlatform(value = process.platform) {
  if (value === 'darwin') return 'macos';
  if (value === 'win32') return 'windows';
  if (value === 'linux') return 'linux';
  if (Object.hasOwn(SIDECAR_TARGETS, value)) return value;
  throw new Error(`Unsupported platform for llama-server assets: ${value}`);
}

export function sidecarTargets(platformValue) {
  return SIDECAR_TARGETS[normalizePlatform(platformValue)];
}

function archiveMatchesTarget(archivePath, target) {
  const fileName = basename(archivePath);
  return target.archiveMatchers.every((matcher) => matcher.test(fileName));
}

export function selectArchiveForTarget(archives, target) {
  return archives.find((archivePath) => archiveMatchesTarget(archivePath, target));
}

function listArchiveMembers(archivePath) {
  return execFileSync('tar', ['-tf', archivePath], { encoding: 'utf8' })
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
}

export function selectBinaryMember(members, binaryName) {
  return members.find((member) => pathPosix.basename(member) === binaryName);
}

function runtimeDependencyMatcher(platformValue) {
  const platform = normalizePlatform(platformValue);
  if (platform === 'macos') {
    return (name) =>
      name.endsWith('.dylib') ||
      name.endsWith('.metal') ||
      name.endsWith('.h');
  }
  if (platform === 'windows') {
    return (name) => name.toLowerCase().endsWith('.dll');
  }

  return (name) => /\.so(\.|$)/.test(name);
}

export function selectRuntimeDependencyMembers(members, binaryMember, platformValue) {
  const runtimeDir = pathPosix.dirname(binaryMember);
  const matchesRuntimeDependency = runtimeDependencyMatcher(platformValue);

  return members
    .filter((member) => {
      if (member === binaryMember) {
        return false;
      }
      if (pathPosix.dirname(member) !== runtimeDir) {
        return false;
      }
      return matchesRuntimeDependency(pathPosix.basename(member));
    })
    .sort((left, right) => pathPosix.basename(left).localeCompare(pathPosix.basename(right)));
}

function extractArchiveMember(archivePath, member, outputDir) {
  execFileSync('tar', ['-xf', archivePath, '-C', outputDir, member], {
    stdio: 'ignore',
  });
  return resolve(outputDir, member);
}

function copyExecutable(sourcePath, destinationPath) {
  mkdirSync(dirname(destinationPath), { recursive: true });
  copyFileSync(sourcePath, destinationPath);
  try {
    chmodSync(destinationPath, 0o755);
  } catch {
    // Windows ignores POSIX mode bits. Runtime spawn errors will be explicit.
  }
}

function copyRuntimeDependency(sourcePath, destinationPath) {
  mkdirSync(dirname(destinationPath), { recursive: true });
  copyFileSync(sourcePath, destinationPath);
}

export function prepareLlamaServerReleaseAssets({
  platform = process.platform,
  assetsDir,
  rootDir = defaultRoot,
  required = true,
} = {}) {
  const normalizedPlatform = normalizePlatform(platform);
  if (!assetsDir) {
    throw new Error('--assets-dir is required');
  }

  const resolvedAssetsDir = resolve(assetsDir);
  if (!existsSync(resolvedAssetsDir)) {
    throw new Error(`llama-server assets directory not found: ${resolvedAssetsDir}`);
  }

  const archives = readdirSync(resolvedAssetsDir)
    .map((entry) => resolve(resolvedAssetsDir, entry))
    .filter((path) => /\.(zip|tar\.gz|tgz)$/.test(path));
  const prepared = [];

  for (const target of sidecarTargets(normalizedPlatform)) {
    const archivePath = selectArchiveForTarget(archives, target);
    if (!archivePath) {
      if (required) {
        throw new Error(`Missing llama.cpp release archive for ${target.key}`);
      }
      continue;
    }

    const members = listArchiveMembers(archivePath);
    const member = selectBinaryMember(members, target.binaryName);
    if (!member) {
      throw new Error(`${archivePath} does not contain ${target.binaryName}`);
    }
    const dependencyMembers = selectRuntimeDependencyMembers(
      members,
      member,
      normalizedPlatform
    );

    const tempDir = mkdtempSync(join(tmpdir(), 'ariatype-llama-server-'));
    try {
      const extractedPath = extractArchiveMember(archivePath, member, tempDir);
      const destinationPath = resolve(tauriDir(rootDir), target.destinationResource);
      copyExecutable(extractedPath, destinationPath);

      const dependencyResources = [];
      const extractedDependencies = dependencyMembers.map((dependencyMember) => ({
        member: dependencyMember,
        path: extractArchiveMember(archivePath, dependencyMember, tempDir),
      }));

      for (const dependency of extractedDependencies) {
        const dependencyResource = pathPosix.join(
          pathPosix.dirname(target.destinationResource),
          pathPosix.basename(dependency.member)
        );
        const dependencyDestinationPath = resolve(tauriDir(rootDir), dependencyResource);
        copyRuntimeDependency(dependency.path, dependencyDestinationPath);
        dependencyResources.push(dependencyResource);
      }

      prepared.push({
        key: target.key,
        archivePath,
        member,
        destinationResource: target.destinationResource,
        destinationPath,
        dependencyResources,
      });
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  }

  return prepared;
}

function parseArgs(args) {
  const getValue = (name) => {
    const index = args.indexOf(name);
    return index === -1 ? undefined : args[index + 1];
  };

  return {
    platform: getValue('--platform') ?? process.platform,
    assetsDir: getValue('--assets-dir'),
    required: !args.includes('--optional'),
  };
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const prepared = prepareLlamaServerReleaseAssets(options);

  if (prepared.length === 0) {
    console.log('No llama-server release assets prepared.');
    return;
  }

  for (const item of prepared) {
    console.log(
      `Prepared ${item.key}: ${item.archivePath}#${item.member} -> ${item.destinationResource}`
    );
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
