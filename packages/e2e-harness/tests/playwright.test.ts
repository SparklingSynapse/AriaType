import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';

const execSync = vi.fn();
const mkdirSync = vi.fn();
const rmSync = vi.fn();
const defineConfig = vi.fn((config) => config);

vi.mock('node:child_process', () => ({
  execSync,
}));

vi.mock('node:fs', () => ({
  mkdirSync,
  rmSync,
}));

vi.mock('@playwright/test', () => ({
  defineConfig,
  test: {
    extend: vi.fn(() => ({ test: 'test', expect: 'expect' })),
  },
  expect: vi.fn(),
}));

const tempDirs: string[] = [];

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => rm(dir, { recursive: true, force: true })));
});

const {
  createTauriFixturesOptionsFromRunnerConfig,
  createTauriPlaywrightConfig,
  createTauriFixturesFromConfigModule,
  createTauriPlaywrightConfigFromModule,
} = await import('../src/playwright');
const { createTauriGlobalSetup, createTauriGlobalTeardown } = await import('../src/playwright-hooks');

describe('createTauriFixturesOptionsFromRunnerConfig', () => {
  it('derives direct-playwright fixture options from ordered runner config', () => {
    const options = createTauriFixturesOptionsFromRunnerConfig(
      {
        projectRoot: '/repo/apps/desktop',
        runtimeRoot: '/repo/apps/desktop/tests/e2e/.runtime/ordered-shared',
        socketPath: '/tmp/app.sock',
        killCommand: 'pkill app',
        tauriCommand: ['tauri', 'dev'],
        tauriFeatures: ['e2e-testing'],
        systemDataPaths: ['/tmp/system-a'],
        startTimeoutSeconds: 123,
        socketWaitMs: 456,
      },
      { sharedRuntimeKey: 'shared' },
    );

    expect(options).toEqual({
      projectRoot: '/repo/apps/desktop',
      runtimeRoot: '/repo/apps/desktop/tests/e2e/.runtime/ordered-shared',
      sharedRuntimeKey: 'shared',
      killCommand: 'pkill app',
      tauriCommand: ['tauri', 'dev'],
      tauriFeatures: ['e2e-testing'],
      systemDataPaths: ['/tmp/system-a'],
      startTimeoutSeconds: 123,
      socketWaitMs: 456,
    });
  });
});

describe('createTauriPlaywrightConfig', () => {
  beforeEach(() => {
    defineConfig.mockClear();
    delete process.env.E2E_HARNESS_SNAPSHOT_STABILIZATION_MS;
    delete process.env.E2E_HARNESS_KILL_COMMAND;
    delete process.env.E2E_HARNESS_RUNTIME_ROOT;
    delete process.env.E2E_HARNESS_SOCKET_PATH;
  });

  it('derives direct playwright config from runner config without local wrapper files', () => {
    const playwrightConfig = createTauriPlaywrightConfig(
      {
        projectRoot: '/repo/apps/desktop',
        runtimeRoot: '/repo/apps/desktop/tests/e2e/.runtime/ordered-shared',
        socketPath: '/tmp/app.sock',
        killCommand: 'pkill app',
        tauriCommand: ['tauri', 'dev'],
        tauriFeatures: ['e2e-testing'],
        devServerCommand: ['exec', 'vite', '--port', '1423', '--strictPort'],
        devServerUrl: 'http://localhost:1423',
        snapshotStabilizationMs: 1500,
      },
      {
        snapshotDir: '/repo/apps/desktop/tests/e2e/snapshots',
      },
    );

    expect(defineConfig).toHaveBeenCalledTimes(1);
    expect(playwrightConfig.snapshotDir).toBe('/repo/apps/desktop/tests/e2e/snapshots');
    expect(playwrightConfig.globalSetup).toContain('playwright-global-setup.ts');
    expect(playwrightConfig.globalTeardown).toContain('playwright-global-teardown.ts');
    expect(playwrightConfig.use.mode).toBe('tauri');
    expect(playwrightConfig.projects).toHaveLength(1);
    expect(playwrightConfig.projects[0]).toMatchObject({
      name: 'tauri',
      testMatch: '**/*.spec.ts',
    });
    expect(process.env.E2E_HARNESS_SNAPSHOT_STABILIZATION_MS).toBe('1500');
    expect(process.env.E2E_HARNESS_KILL_COMMAND).toBe('pkill app');
    expect(process.env.E2E_HARNESS_RUNTIME_ROOT).toBe('/repo/apps/desktop/tests/e2e/.runtime/ordered-shared');
    expect(process.env.E2E_HARNESS_SOCKET_PATH).toBe('/tmp/app.sock');
    expect(playwrightConfig.webServer).toMatchObject({
      command: 'pnpm exec vite --port 1423 --strictPort',
      cwd: '/repo/apps/desktop',
      url: 'http://localhost:1423',
      reuseExistingServer: true,
    });
  });

  it('loads fixture options from a config module path', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'e2e-harness-playwright-'));
    tempDirs.push(tempDir);
    const configPath = join(tempDir, 'e2e.config.mjs');

    await writeFile(
      configPath,
      [
        'export default {',
        '  projectRoot: "/repo/app",',
        '  runtimeRoot: "/repo/app/tests/e2e/.runtime/shared",',
        '  socketPath: "/tmp/example.sock",',
        '  tauriCommand: ["tauri", "dev"],',
        '};',
      ].join('\n'),
      'utf8',
    );

    await expect(
      createTauriFixturesFromConfigModule('./e2e.config.mjs', { sharedRuntimeKey: 'worker' }, tempDir),
    ).resolves.toMatchObject({
      test: {
        test: 'test',
        expect: 'expect',
      },
    });
  });

  it('loads playwright config from a config module path', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'e2e-harness-playwright-config-'));
    tempDirs.push(tempDir);
    const configPath = join(tempDir, 'e2e.config.mjs');

    await writeFile(
      configPath,
      [
        'export default {',
        '  projectRoot: "/repo/app",',
        '  runtimeRoot: "/repo/app/tests/e2e/.runtime/shared",',
        '  socketPath: "/tmp/example.sock",',
        '  tauriCommand: ["tauri", "dev"],',
        '  devServerCommand: ["exec", "vite"],',
        '  devServerUrl: "http://localhost:1423",',
        '};',
      ].join('\n'),
      'utf8',
    );

    const config = await createTauriPlaywrightConfigFromModule(
      './e2e.config.mjs',
      { snapshotDir: '/repo/app/tests/e2e/snapshots' },
      tempDir,
    );

    expect(config.snapshotDir).toBe('/repo/app/tests/e2e/snapshots');
    expect(config.webServer).toMatchObject({
      command: 'pnpm exec vite',
      cwd: '/repo/app',
      url: 'http://localhost:1423',
    });
  });
});

describe('createTauriGlobalSetup', () => {
  beforeEach(() => {
    execSync.mockReset();
    mkdirSync.mockReset();
    rmSync.mockReset();
    delete process.env.E2E_HARNESS_EXTERNAL_RUNTIME;
  });

  it('cleans local runtime artifacts when not using an external runtime', async () => {
    const setup = createTauriGlobalSetup({
      killCommand: 'pkill app',
      runtimeRoot: '/repo/runtime',
      socketPath: '/tmp/app.sock',
    });

    await setup();

    expect(execSync).toHaveBeenCalledWith('pkill app', { stdio: 'ignore' });
    expect(rmSync).toHaveBeenCalledWith('/tmp/app.sock', { force: true });
    expect(rmSync).toHaveBeenCalledWith('/repo/runtime', {
      recursive: true,
      force: true,
      maxRetries: 5,
      retryDelay: 100,
    });
    expect(mkdirSync).toHaveBeenCalledWith('/repo/runtime', { recursive: true });
  });

  it('skips cleanup when the ordered runner provides the external runtime', async () => {
    process.env.E2E_HARNESS_EXTERNAL_RUNTIME = '1';
    const setup = createTauriGlobalSetup({
      killCommand: 'pkill app',
      runtimeRoot: '/repo/runtime',
      socketPath: '/tmp/app.sock',
    });

    await setup();

    expect(execSync).not.toHaveBeenCalled();
    expect(rmSync).not.toHaveBeenCalled();
    expect(mkdirSync).not.toHaveBeenCalled();
  });
});

describe('createTauriGlobalTeardown', () => {
  beforeEach(() => {
    execSync.mockReset();
    mkdirSync.mockReset();
    rmSync.mockReset();
    delete process.env.E2E_HARNESS_EXTERNAL_RUNTIME;
  });

  it('removes local runtime artifacts without recreating the directory', async () => {
    const teardown = createTauriGlobalTeardown({
      killCommand: 'pkill app',
      runtimeRoot: '/repo/runtime',
      socketPath: '/tmp/app.sock',
    });

    await teardown();

    expect(execSync).toHaveBeenCalledWith('pkill app', { stdio: 'ignore' });
    expect(rmSync).toHaveBeenCalledWith('/tmp/app.sock', { force: true });
    expect(rmSync).toHaveBeenCalledWith('/repo/runtime', {
      recursive: true,
      force: true,
      maxRetries: 5,
      retryDelay: 100,
    });
    expect(mkdirSync).not.toHaveBeenCalled();
  });
});
