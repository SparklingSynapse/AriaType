import { copyFileSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { execSync, spawn, spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { TauriProcessManager } from '@srsholmes/tauri-playwright';

function compareDesc(a, b) {
  return b.localeCompare(a);
}

function resolveSpecOrder(config) {
  const pagesDir = join(config.projectRoot, config.pagesDir);
  const explicitSpecs = config.specOrder.map((spec) => `${config.specsPrefix}/${spec}`);
  const discoveredSpecs = readdirSync(pagesDir)
    .filter((file) => file.endsWith('.spec.ts'))
    .map((file) => `${config.specsPrefix}/${file}`)
    .filter((file) => !explicitSpecs.includes(file))
    .sort(compareDesc);

  return [...explicitSpecs.map((spec) => [spec]), ...discoveredSpecs.map((spec) => [spec])];
}

function hasBatchFilterArgs(extraArgs) {
  return extraArgs.some((arg) => arg === '--grep' || arg === '-g' || arg === '--grep-invert');
}

function normalizeExtraArgs(extraArgs) {
  const normalized = [];

  for (let index = 0; index < extraArgs.length; index += 1) {
    const arg = extraArgs[index];
    if (arg === '--') {
      continue;
    }

    normalized.push(arg);
  }

  return normalized;
}

function stripExecutionOnlyArgs(extraArgs) {
  return normalizeExtraArgs(extraArgs).filter((arg) => arg !== '--update-snapshots' && arg !== '-u');
}

function cleanupPaths(paths = []) {
  for (const path of paths) {
    rmSync(path, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function batchHasMatchingTests(specs, extraArgs, config) {
  const result = spawnSync(
    'pnpm',
    [
      'exec',
      'playwright',
      'test',
      '--list',
      '--config',
      config.playwrightConfig,
      ...specs,
      ...stripExecutionOnlyArgs(extraArgs),
    ],
    {
      cwd: config.projectRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        E2E_HARNESS_EXTERNAL_RUNTIME: '1',
        E2E_HARNESS_SOCKET_PATH: config.socketPath,
        E2E_HARNESS_SNAPSHOT_STABILIZATION_MS: String(config.snapshotStabilizationMs ?? 1000),
      },
    },
  );

  const combinedOutput = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;

  if (result.error) {
    throw result.error;
  }

  if (combinedOutput.includes('No tests found')) {
    return false;
  }

  if (typeof result.status === 'number' && result.status !== 0) {
    throw new Error(`Failed to enumerate tests for ${specs.join(', ')}`);
  }

  return combinedOutput.includes('[tauri]');
}

function runPlaywright(specs, extraArgs, config) {
  const normalizedExtraArgs = normalizeExtraArgs(extraArgs);
  const result = spawnSync(
    'pnpm',
    ['exec', 'playwright', 'test', '--config', config.playwrightConfig, ...specs, ...normalizedExtraArgs],
    {
      cwd: config.projectRoot,
      stdio: 'inherit',
      env: {
        ...process.env,
        E2E_HARNESS_EXTERNAL_RUNTIME: '1',
        E2E_HARNESS_SOCKET_PATH: config.socketPath,
        E2E_HARNESS_SNAPSHOT_STABILIZATION_MS: String(config.snapshotStabilizationMs ?? 1000),
      },
    },
  );

  if (typeof result.status === 'number' && result.status !== 0) {
    return result.status;
  }

  if (result.error) {
    throw result.error;
  }

  return 0;
}

function prepareRuntimePaths(config) {
  rmSync(config.socketPath, { force: true });
  rmSync(config.runtimeRoot, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  mkdirSync(join(config.runtimeRoot, 'xdg-data'), { recursive: true });
  mkdirSync(join(config.runtimeRoot, 'xdg-cache'), { recursive: true });
  mkdirSync(join(config.runtimeRoot, 'xdg-config'), { recursive: true });

  if (config.seedFiles?.length) {
    for (const { src, dest } of config.seedFiles) {
      const destPath = join(config.runtimeRoot, 'xdg-data', dest);
      mkdirSync(dirname(destPath), { recursive: true });
      copyFileSync(src, destPath);
    }
  }

  if (config.seedDataFiles?.length) {
    for (const { src, dest } of config.seedDataFiles) {
      mkdirSync(dirname(dest), { recursive: true });
      copyFileSync(src, dest);
    }
  }
}

function cleanupSystemDataPaths(config) {
  cleanupPaths(config.systemDataPaths ?? []);
}

function cleanupExternalRuntime(config) {
  try {
    execSync(config.killCommand, { stdio: 'ignore' });
  } catch {}

  rmSync(config.socketPath, { force: true });
  rmSync(config.runtimeRoot, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  cleanupSystemDataPaths(config);
  cleanupCapabilities(config);
}

function prepareCapabilities(config) {
  if (!config.capabilityFiles?.length) {
    return;
  }

  for (const { src, dest } of config.capabilityFiles) {
    copyFileSync(src, dest);
  }
}

function cleanupCapabilities(config) {
  if (!config.capabilityFiles?.length) {
    return;
  }

  for (const { dest } of config.capabilityFiles) {
    rmSync(dest, { force: true });
  }
}

async function startExternalRuntime(config) {
  prepareRuntimePaths(config);
  prepareCapabilities(config);

  const processManager = new TauriProcessManager({
    command: 'env',
    args: [
      `XDG_DATA_HOME=${join(config.runtimeRoot, 'xdg-data')}`,
      `XDG_CACHE_HOME=${join(config.runtimeRoot, 'xdg-cache')}`,
      `XDG_CONFIG_HOME=${join(config.runtimeRoot, 'xdg-config')}`,
      'pnpm',
      ...config.tauriCommand,
    ],
    cwd: config.projectRoot,
    features: config.tauriFeatures,
    socketPath: config.socketPath,
    startTimeout: config.startTimeoutSeconds,
  });

  await processManager.start();
  await processManager.waitForSocket(config.socketWaitMs);
  return processManager;
}

async function waitForHttpReady(url, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {}

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw new Error(`Server ${url} did not become ready within ${timeoutMs}ms`);
}

function prepareDevServer(config) {
  cleanupPaths(config.devServerResetPaths ?? []);

  if (!config.devServerPrepareCommand?.length) {
    return;
  }

  const result = spawnSync('pnpm', config.devServerPrepareCommand, {
    cwd: config.projectRoot,
    stdio: 'inherit',
    env: process.env,
  });

  if (typeof result.status === 'number' && result.status !== 0) {
    process.exit(result.status);
  }

  if (result.error) {
    throw result.error;
  }
}

async function startDevServer(config) {
  prepareDevServer(config);

  const devServer = spawn('pnpm', config.devServerCommand, {
    cwd: config.projectRoot,
    stdio: 'ignore',
    env: process.env,
  });

  await waitForHttpReady(config.devServerUrl, config.devServerReadyTimeoutMs);
  return devServer;
}

export async function runOrderedTauriSuite(config, extraArgs) {
  const specBatches = resolveSpecOrder(config);
  const filteredSpecBatches = hasBatchFilterArgs(extraArgs)
    ? specBatches.filter((specs) => batchHasMatchingTests(specs, extraArgs, config))
    : specBatches;
  let exitCode = 0;

  cleanupExternalRuntime(config);

  const devServer = await startDevServer(config);
  const processManager = await startExternalRuntime(config);

  try {
    for (const specs of filteredSpecBatches) {
      exitCode = runPlaywright(specs, extraArgs, config);
      if (exitCode !== 0) {
        break;
      }
    }
  } finally {
    processManager.stop();
    devServer.kill('SIGTERM');
    cleanupExternalRuntime(config);
  }

  if (exitCode !== 0) {
    process.exit(exitCode);
  }
}

export function normalizeRunnerCliArgs(argv) {
  const [configPath, ...rest] = argv;
  if (!configPath) {
    throw new Error('Missing runner config path');
  }

  const extraArgs = rest.filter((arg, index) => !(arg === '--' && index === 0));
  return { configPath, extraArgs };
}

export async function loadRunnerConfigModule(configPath, cwd = process.cwd()) {
  const absolutePath = isAbsolute(configPath) ? configPath : resolve(cwd, configPath);
  const module = await import(pathToFileURL(absolutePath).href);
  return module.default;
}

export function createRunnerConfig(config) {
  return config;
}

export function resolveHarnessDir(importMetaUrl) {
  return dirname(fileURLToPath(importMetaUrl));
}
