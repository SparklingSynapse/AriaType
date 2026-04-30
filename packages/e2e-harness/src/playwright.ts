import { execSync } from 'node:child_process';
import { mkdir, rm } from 'node:fs/promises';
import { defineConfig, type TestInfo } from '@playwright/test';
import { test as base } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import {
  PluginClient,
  TauriPage,
  TauriProcessManager,
  tauriExpect,
} from '@srsholmes/tauri-playwright';
import { captureStableScreenshot, sleep } from './snapshot';
import { loadRunnerConfigModule } from './ordered-tauri-runner.mjs';

export type RuntimePaths = {
  root: string;
  socketPath: string;
  xdgCacheHome: string;
  xdgConfigHome: string;
  xdgDataHome: string;
};

type TauriRuntime = {
  client: PluginClient;
  page: TauriPage;
  processManager?: TauriProcessManager;
  paths: RuntimePaths;
};

type WorkerFixtures = {
  tauriRuntime: TauriRuntime;
};

type TestFixtures = {
  tauriPage: TauriPage;
};

export type TauriFixtureOptions = {
  projectRoot: string;
  runtimeRoot: string;
  sharedRuntimeKey?: string;
  socketPathFactory?: (runtimeKey: string) => string;
  killCommand?: string;
  systemDataPaths?: string[];
  tauriCommand: string[];
  tauriFeatures?: string[];
  startTimeoutSeconds?: number;
  socketWaitMs?: number;
};

type RunnerBackedFixtureConfig = {
  projectRoot: string;
  runtimeRoot: string;
  socketPath: string;
  killCommand?: string;
  systemDataPaths?: string[];
  tauriCommand: string[];
  tauriFeatures?: string[];
  startTimeoutSeconds?: number;
  socketWaitMs?: number;
};

type RunnerBackedFixtureOverrides = {
  sharedRuntimeKey?: string;
};

type TauriPlaywrightConfigOptions = {
  snapshotDir: string;
  testDir?: string;
  timeout?: number;
  testMatch?: string;
};

const EXTERNAL_RUNTIME_ENV = 'E2E_HARNESS_EXTERNAL_RUNTIME';
const EXTERNAL_SOCKET_ENV = 'E2E_HARNESS_SOCKET_PATH';

function createRuntimePaths(
  runtimeRoot: string,
  runtimeKey: string,
  socketPathFactory?: (runtimeKey: string) => string,
): RuntimePaths {
  const root = `${runtimeRoot.replace(/\/?$/, '/')}${runtimeKey}/`;

  return {
    root,
    socketPath: socketPathFactory?.(runtimeKey) ?? `/tmp/tauri-e2e-${runtimeKey}.sock`,
    xdgDataHome: `${root}xdg-data`,
    xdgCacheHome: `${root}xdg-cache`,
    xdgConfigHome: `${root}xdg-config`,
  };
}

async function cleanupPaths(paths: string[]): Promise<void> {
  await Promise.all(
    paths.map((path) =>
      rm(path, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 }).catch(() => undefined),
    ),
  );
}

async function prepareRuntimePaths(paths: RuntimePaths, resetStorage: boolean, systemDataPaths: string[]): Promise<void> {
  await rm(paths.socketPath, { force: true }).catch(() => undefined);

  if (resetStorage) {
    await cleanupPaths([paths.root, ...systemDataPaths]);
  }

  await Promise.all([
    mkdir(paths.xdgDataHome, { recursive: true }),
    mkdir(paths.xdgCacheHome, { recursive: true }),
    mkdir(paths.xdgConfigHome, { recursive: true }),
  ]);
}

function cleanupStandaloneRuntimeResidue(killCommand?: string): void {
  if (!killCommand) {
    return;
  }

  try {
    execSync(killCommand, { stdio: 'ignore' });
  } catch {
    // Best effort cleanup before local Tauri test startup.
  }
}

async function startTauriRuntime(
  options: TauriFixtureOptions,
  runtimeKey: string,
  resetStorage = false,
): Promise<TauriRuntime> {
  const paths = createRuntimePaths(options.runtimeRoot, runtimeKey, options.socketPathFactory);
  cleanupStandaloneRuntimeResidue(options.killCommand);
  await prepareRuntimePaths(paths, resetStorage, options.systemDataPaths ?? []);

  const processManager = new TauriProcessManager({
    command: 'env',
    args: [
      `XDG_DATA_HOME=${paths.xdgDataHome}`,
      `XDG_CACHE_HOME=${paths.xdgCacheHome}`,
      `XDG_CONFIG_HOME=${paths.xdgConfigHome}`,
      'pnpm',
      ...options.tauriCommand,
    ],
    cwd: options.projectRoot,
    features: options.tauriFeatures ?? ['e2e-testing'],
    socketPath: paths.socketPath,
    startTimeout: options.startTimeoutSeconds ?? 180,
  });

  await processManager.start();
  await processManager.waitForSocket(options.socketWaitMs ?? 5000);

  const client = new PluginClient(paths.socketPath);
  await client.connect();

  const ping = await client.send({ type: 'ping' });
  if (!ping.ok) {
    client.disconnect();
    processManager.stop();
    throw new Error('Plugin ping failed');
  }

  return {
    client,
    page: new TauriPage(client),
    paths,
    processManager,
  };
}

async function connectExternalTauriRuntime(
  options: TauriFixtureOptions,
  runtimeKey: string,
  socketPath: string,
): Promise<TauriRuntime> {
  const client = new PluginClient(socketPath);
  await client.connect();

  const ping = await client.send({ type: 'ping' });
  if (!ping.ok) {
    client.disconnect();
    throw new Error('External plugin ping failed');
  }

  return {
    client,
    page: new TauriPage(client),
    paths: createRuntimePaths(options.runtimeRoot, runtimeKey, options.socketPathFactory),
  };
}

async function stopTauriRuntime(runtime: TauriRuntime): Promise<void> {
  runtime.client.disconnect();
  if (!runtime.processManager) {
    return;
  }

  runtime.processManager.stop();

  // The process manager does not wait for exit, so give the app time to release
  // the single-instance lock and socket before the next cold start.
  await sleep(1000);
  await rm(runtime.paths.socketPath, { force: true }).catch(() => undefined);
}

async function resetTauriPage(page: TauriPage): Promise<void> {
  await page.clearRoutes();
  await page.clearNetworkRequests();
  await page.clearDialogs();
}

function getScreenshotThreshold(testInfo: TestInfo): number {
  const thresholdAnnotation = testInfo.annotations.find(
    (annotation: { type: string; description?: string }) => annotation.type === 'screenshot-threshold',
  );
  const threshold = Number(thresholdAnnotation?.description ?? '0.1');

  return Number.isFinite(threshold) ? threshold : 0.1;
}

function getScreenshotMaxDiffPixels(testInfo: TestInfo): number | undefined {
  const annotation = testInfo.annotations.find(
    (entry: { type: string; description?: string }) => entry.type === 'screenshot-max-diff-pixels',
  );
  const value = Number(annotation?.description);

  return Number.isFinite(value) ? value : undefined;
}

function isAutoSnapshotDisabled(testInfo: TestInfo): boolean {
  return testInfo.annotations.some(
    (annotation: { type: string }) => annotation.type === 'disable-auto-snapshot',
  );
}

async function assertFinalSnapshot(page: TauriPage, testInfo: TestInfo): Promise<void> {
  const threshold = getScreenshotThreshold(testInfo);
  const maxDiffPixels = getScreenshotMaxDiffPixels(testInfo);
  const screenshot = await captureStableScreenshot(page, { captureMode: 'native-with-fallback' });
  tauriExpect(screenshot).toMatchSnapshot({
    threshold,
    maxDiffPixels,
  });
}

async function finalizeTauriPage(page: TauriPage, testInfo: TestInfo): Promise<void> {
  if (testInfo.status === testInfo.expectedStatus && !isAutoSnapshotDisabled(testInfo)) {
    await assertFinalSnapshot(page, testInfo);
  } else {
    try {
      const screenshot = await page.screenshot();
      if (screenshot.length > 0) {
        await testInfo.attach('native-screenshot', {
          body: screenshot,
          contentType: 'image/png',
        });
      }
    } catch {
      // Best-effort diagnostics only.
    }
  }

  await page.clearRoutes().catch(() => undefined);
  await page.clearNetworkRequests().catch(() => undefined);
  await page.clearDialogs().catch(() => undefined);
}

export function createTauriFixtures(options: TauriFixtureOptions) {
  const sharedRuntimeKey = options.sharedRuntimeKey ?? 'shared';
  const externalRuntimeSocketPath = process.env[EXTERNAL_SOCKET_ENV];
  const useExternalRuntime = process.env[EXTERNAL_RUNTIME_ENV] === '1';

  const test = base.extend<TestFixtures, WorkerFixtures>({
    tauriRuntime: [
      async ({}, use: (runtime: TauriRuntime) => Promise<void>) => {
        const runtime =
          useExternalRuntime && externalRuntimeSocketPath
            ? await connectExternalTauriRuntime(options, sharedRuntimeKey, externalRuntimeSocketPath)
            : await startTauriRuntime(options, sharedRuntimeKey, true);

        try {
          await use(runtime);
        } finally {
          await stopTauriRuntime(runtime);
        }
      },
      { scope: 'worker', timeout: 300000 },
    ],

    tauriPage: async (
      { tauriRuntime }: { tauriRuntime: TauriRuntime },
      use: (page: TauriPage) => Promise<void>,
      testInfo: TestInfo,
    ) => {
      const { page } = tauriRuntime;

      await resetTauriPage(page);

      try {
        await use(page);
      } finally {
        await finalizeTauriPage(page, testInfo);
      }
    },
  });

  return {
    test,
    expect: tauriExpect as typeof tauriExpect,
  };
}

export function createTauriFixturesOptionsFromRunnerConfig(
  config: RunnerBackedFixtureConfig,
  overrides: RunnerBackedFixtureOverrides = {},
): TauriFixtureOptions {
  return {
    projectRoot: config.projectRoot,
    runtimeRoot: config.runtimeRoot,
    sharedRuntimeKey: overrides.sharedRuntimeKey ?? 'shared',
    killCommand: config.killCommand,
    systemDataPaths: config.systemDataPaths,
    tauriCommand: config.tauriCommand,
    tauriFeatures: config.tauriFeatures,
    startTimeoutSeconds: config.startTimeoutSeconds,
    socketWaitMs: config.socketWaitMs,
  };
}

export function createTauriFixturesFromRunnerConfig(
  config: RunnerBackedFixtureConfig,
  overrides: RunnerBackedFixtureOverrides = {},
) {
  return createTauriFixtures(createTauriFixturesOptionsFromRunnerConfig(config, overrides));
}

export async function createTauriFixturesFromConfigModule(
  configPath: string,
  overrides: RunnerBackedFixtureOverrides = {},
  cwd?: string,
) {
  const config = await loadRunnerConfigModule(configPath, cwd);
  return createTauriFixturesFromRunnerConfig(config, overrides);
}

export function createTauriPlaywrightConfig(
  config: RunnerBackedFixtureConfig & {
    devServerCommand: string[];
    devServerUrl: string;
    snapshotStabilizationMs?: number;
  },
  options: TauriPlaywrightConfigOptions,
) {
  process.env.E2E_HARNESS_SNAPSHOT_STABILIZATION_MS ??= String(config.snapshotStabilizationMs ?? 1000);
  process.env.E2E_HARNESS_KILL_COMMAND ??= config.killCommand ?? '';
  process.env.E2E_HARNESS_RUNTIME_ROOT ??= config.runtimeRoot;
  process.env.E2E_HARNESS_SOCKET_PATH ??= config.socketPath;

  const globalSetupPath = fileURLToPath(new URL('./playwright-global-setup.ts', import.meta.url));
  const globalTeardownPath = fileURLToPath(new URL('./playwright-global-teardown.ts', import.meta.url));

  return defineConfig({
    testDir: options.testDir ?? './pages',
    snapshotDir: options.snapshotDir,
    fullyParallel: false,
    workers: 1,
    timeout: options.timeout ?? 120000,
    globalSetup: globalSetupPath,
    globalTeardown: globalTeardownPath,
    use: {
      screenshot: 'only-on-failure',
      trace: 'retain-on-failure',
      mode: 'tauri' as never,
    } as never,
    expect: {
      toHaveScreenshot: {
        threshold: 0.1,
      },
    },
    reporter: [['list'], ['html', { open: 'never' }]],
    webServer: {
      command: ['pnpm', ...config.devServerCommand].join(' '),
      cwd: config.projectRoot,
      url: config.devServerUrl,
      reuseExistingServer: true,
    },
    projects: [
      {
        name: 'tauri',
        use: {
          mode: 'tauri' as never,
        } as never,
        testMatch: options.testMatch ?? '**/*.spec.ts',
      },
    ],
  });
}

export async function createTauriPlaywrightConfigFromModule(
  configPath: string,
  options: TauriPlaywrightConfigOptions,
  cwd?: string,
) {
  const config = await loadRunnerConfigModule(configPath, cwd);
  return createTauriPlaywrightConfig(config, options);
}
