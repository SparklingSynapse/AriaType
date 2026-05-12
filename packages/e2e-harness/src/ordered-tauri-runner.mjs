import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { execSync, spawn, spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

function logStep(message) {
  console.log(`[e2e-runner] ${new Date().toISOString()} ${message}`);
}

function formatCommand(command, args = []) {
  return [command, ...args].join(' ');
}

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

function cleanupPaths(paths = [], label = 'paths') {
  if (!paths.length) {
    logStep(`cleanup ${label}: no paths`);
    return;
  }

  for (const path of paths) {
    logStep(`cleanup ${label}: ${path}`);
    rmSync(path, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function batchHasMatchingTests(specs, extraArgs, config) {
  logStep(`enumerate tests: ${specs.join(', ')}`);
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
  const args = ['exec', 'playwright', 'test', '--config', config.playwrightConfig, ...specs, ...normalizedExtraArgs];
  logStep(`run playwright: ${formatCommand('pnpm', args)}`);
  const result = spawnSync(
    'pnpm',
    args,
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
  logStep(`prepare runtime: remove socket ${config.socketPath}`);
  rmSync(config.socketPath, { force: true });
  logStep(`prepare runtime: reset ${config.runtimeRoot}`);
  rmSync(config.runtimeRoot, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  logStep('prepare runtime: create xdg-data, xdg-cache, xdg-config');
  mkdirSync(join(config.runtimeRoot, 'xdg-data'), { recursive: true });
  mkdirSync(join(config.runtimeRoot, 'xdg-cache'), { recursive: true });
  mkdirSync(join(config.runtimeRoot, 'xdg-config'), { recursive: true });

  if (config.seedFiles?.length) {
    for (const { src, dest } of config.seedFiles) {
      const destPath = join(config.runtimeRoot, 'xdg-data', dest);
      logStep(`seed runtime file: ${src} -> ${destPath}`);
      mkdirSync(dirname(destPath), { recursive: true });
      copyFileSync(src, destPath);
    }
  }

  if (config.seedDataFiles?.length) {
    for (const { src, dest } of config.seedDataFiles) {
      logStep(`seed data file: ${src} -> ${dest}`);
      mkdirSync(dirname(dest), { recursive: true });
      copyFileSync(src, dest);
    }
  }
}

function cleanupSystemDataPaths(config) {
  cleanupPaths(config.systemDataPaths ?? [], 'system data');
}

function cleanupExternalRuntime(config) {
  logStep(`cleanup external runtime: ${config.killCommand}`);
  try {
    execSync(config.killCommand, { stdio: 'ignore' });
  } catch {}

  logStep(`cleanup external runtime: socket ${config.socketPath}`);
  rmSync(config.socketPath, { force: true });
  logStep(`cleanup external runtime: runtime ${config.runtimeRoot}`);
  rmSync(config.runtimeRoot, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  cleanupSystemDataPaths(config);
  cleanupCapabilities(config);
}

function prepareCapabilities(config) {
  if (!config.capabilityFiles?.length) {
    return;
  }

  for (const { src, dest } of config.capabilityFiles) {
    logStep(`prepare capability: ${src} -> ${dest}`);
    mkdirSync(dirname(dest), { recursive: true });
    copyFileSync(src, dest);
  }
}

function cleanupCapabilities(config) {
  if (!config.capabilityFiles?.length) {
    return;
  }

  for (const { dest } of config.capabilityFiles) {
    logStep(`cleanup capability: ${dest}`);
    rmSync(dest, { force: true });
  }
}

function registerCleanupSignalHandlers(cleanup) {
  const signalExitCodes = new Map([
    ['SIGINT', 130],
    ['SIGTERM', 143],
  ]);
  const handlers = [];

  const unregister = () => {
    for (const [signal, handler] of handlers) {
      process.off(signal, handler);
    }
  };

  for (const [signal, exitCode] of signalExitCodes) {
    const handler = () => {
      unregister();
      cleanup();
      process.exit(exitCode);
    };

    process.once(signal, handler);
    handlers.push([signal, handler]);
  }

  return unregister;
}

async function startExternalRuntime(config, onStarted) {
  logStep('start external runtime: prepare paths and capabilities');
  prepareRuntimePaths(config);
  prepareCapabilities(config);

  const tauriExecutable = config.tauriExecutable
    ? (isAbsolute(config.tauriExecutable) ? config.tauriExecutable : resolve(config.projectRoot, config.tauriExecutable))
    : 'pnpm';
  if (config.tauriExecutable && !existsSync(tauriExecutable)) {
    throw new Error(`Tauri executable does not exist: ${tauriExecutable}`);
  }

  const args = [
    ...config.tauriCommand,
  ];

  if (config.tauriFeatures?.length) {
    args.push('--features', config.tauriFeatures.join(','));
  }

  const env = {
    ...process.env,
    XDG_DATA_HOME: join(config.runtimeRoot, 'xdg-data'),
    XDG_CACHE_HOME: join(config.runtimeRoot, 'xdg-cache'),
    XDG_CONFIG_HOME: join(config.runtimeRoot, 'xdg-config'),
    TAURI_PLAYWRIGHT_SOCKET: config.socketPath,
    CARGO_TERM_COLOR: process.env.CARGO_TERM_COLOR ?? 'always',
    FORCE_COLOR: process.env.FORCE_COLOR ?? '1',
  };

  logStep(`start tauri: ${formatCommand(tauriExecutable, args)}`);

  const child = spawn(tauriExecutable, args, {
    cwd: config.projectRoot,
    stdio: 'inherit',
    env,
    detached: true,
  });
  logStep(`tauri process spawned: pid ${child.pid ?? 'unknown'}`);

  const processManager = {
    stop() {
      logStep(`stop tauri process group: pid ${child.pid ?? 'unknown'}`);
      if (child.pid) {
        try {
          process.kill(-child.pid, 'SIGTERM');
        } catch {
          child.kill('SIGTERM');
        }
      }
    },
  };
  onStarted?.(processManager);

  await waitForProcessSocket(child, config.socketPath, config.startTimeoutSeconds);
  logStep(`tauri runtime ready: socket ${config.socketPath}`);

  return processManager;
}

async function waitForHttpReady(url, timeoutMs = 30000) {
  logStep(`wait for HTTP ready: ${url} timeout=${timeoutMs}ms`);
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        logStep(`HTTP ready: ${url}`);
        return;
      }
    } catch {}

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw new Error(`Server ${url} did not become ready within ${timeoutMs}ms`);
}

async function waitForProcessSocket(child, socketPath, timeoutSeconds = 120) {
  logStep(`wait for tauri process to expose playwright socket: timeout=${timeoutSeconds}s`);
  const timeoutMs = timeoutSeconds * 1000;

  return new Promise((resolve, reject) => {
    const startedAt = Date.now();
    let lastHeartbeatSecond = 0;
    const timeout = setTimeout(() => {
      if (child.pid) {
        try {
          process.kill(-child.pid, 'SIGTERM');
        } catch {
          child.kill('SIGTERM');
        }
      }
      reject(new Error(`Tauri app did not expose ${socketPath} within ${timeoutSeconds}s`));
    }, timeoutMs);

    const poll = setInterval(() => {
      if (existsSync(socketPath)) {
        clearTimeout(timeout);
        cleanup();
        resolve();
        return;
      }

      const elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
      if (elapsedSeconds > 0 && elapsedSeconds % 10 === 0 && elapsedSeconds !== lastHeartbeatSecond) {
        lastHeartbeatSecond = elapsedSeconds;
        logStep(`still waiting for tauri socket after ${elapsedSeconds}s: ${socketPath}`);
      }
    }, 1000);

    const onExit = (code) => {
      clearTimeout(timeout);
      cleanup();
      reject(new Error(`Tauri process exited before playwright socket was ready with code ${code}`));
    };
    const onError = (error) => {
      clearTimeout(timeout);
      cleanup();
      reject(error);
    };
    const cleanup = () => {
      clearInterval(poll);
      child.off('exit', onExit);
      child.off('error', onError);
    };

    child.once('exit', onExit);
    child.once('error', onError);
  });
}

function prepareDevServer(config) {
  cleanupPaths(config.devServerResetPaths ?? [], 'dev server reset');

  if (!config.devServerPrepareCommand?.length) {
    logStep('prepare dev server: no prepare command');
    return;
  }

  logStep(`prepare dev server: ${formatCommand('pnpm', config.devServerPrepareCommand)}`);
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
  logStep('start dev server: prepare');
  prepareDevServer(config);

  logStep(`start dev server: ${formatCommand('pnpm', config.devServerCommand)}`);
  const devServer = spawn('pnpm', config.devServerCommand, {
    cwd: config.projectRoot,
    stdio: 'inherit',
    env: process.env,
  });

  await waitForHttpReady(config.devServerUrl, config.devServerReadyTimeoutMs);
  return devServer;
}

export async function runOrderedTauriSuite(config, extraArgs) {
  logStep('runner start');
  const specBatches = resolveSpecOrder(config);
  logStep(`resolved spec batches: ${specBatches.map((specs) => specs.join(',')).join(' | ')}`);
  const filteredSpecBatches = hasBatchFilterArgs(extraArgs)
    ? specBatches.filter((specs) => batchHasMatchingTests(specs, extraArgs, config))
    : specBatches;
  logStep(`filtered spec batches: ${filteredSpecBatches.map((specs) => specs.join(',')).join(' | ')}`);
  let exitCode = 0;
  let devServer;
  let processManager;
  let cleanupDone = false;
  const cleanup = () => {
    if (cleanupDone) {
      logStep('cleanup skipped: already done');
      return;
    }

    logStep('cleanup start');
    cleanupDone = true;
    processManager?.stop();
    devServer?.kill('SIGTERM');
    cleanupExternalRuntime(config);
    logStep('cleanup complete');
  };
  const unregisterSignalHandlers = registerCleanupSignalHandlers(cleanup);

  try {
    logStep('phase: cleanup stale runtime');
    cleanupExternalRuntime(config);
    logStep('phase: start dev server');
    devServer = await startDevServer(config);
    logStep('phase: start tauri external runtime');
    processManager = await startExternalRuntime(config, (manager) => {
      processManager = manager;
    });

    for (const specs of filteredSpecBatches) {
      logStep(`phase: run spec batch ${specs.join(', ')}`);
      exitCode = runPlaywright(specs, extraArgs, config);
      if (exitCode !== 0) {
        logStep(`spec batch failed with exit code ${exitCode}: ${specs.join(', ')}`);
        break;
      }
      logStep(`spec batch passed: ${specs.join(', ')}`);
    }
  } finally {
    logStep('phase: final cleanup');
    unregisterSignalHandlers();
    cleanup();
  }

  if (exitCode !== 0) {
    logStep(`runner exit with code ${exitCode}`);
    process.exit(exitCode);
  }

  logStep('runner complete');
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
