import { execSync } from 'node:child_process';
import { mkdirSync, rmSync } from 'node:fs';

const EXTERNAL_RUNTIME_ENV = 'E2E_HARNESS_EXTERNAL_RUNTIME';

export type TauriLocalRuntimeOptions = {
  killCommand?: string;
  runtimeRoot: string;
  socketPath: string;
};

function bestEffortKill(killCommand?: string): void {
  if (!killCommand) {
    return;
  }

  try {
    execSync(killCommand, { stdio: 'ignore' });
  } catch {
    // Best-effort cleanup only.
  }
}

function cleanupRuntimeRoot(runtimeRoot: string): void {
  rmSync(runtimeRoot, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}

export function cleanupTauriLocalArtifacts(
  options: TauriLocalRuntimeOptions,
  recreateRuntimeRoot: boolean,
): void {
  bestEffortKill(options.killCommand);
  rmSync(options.socketPath, { force: true });
  cleanupRuntimeRoot(options.runtimeRoot);

  if (recreateRuntimeRoot) {
    mkdirSync(options.runtimeRoot, { recursive: true });
  }
}

export function createTauriGlobalSetup(options: TauriLocalRuntimeOptions): () => Promise<void> {
  return async function globalSetup(): Promise<void> {
    if (process.env[EXTERNAL_RUNTIME_ENV] === '1') {
      return;
    }

    cleanupTauriLocalArtifacts(options, true);
  };
}

export function createTauriGlobalTeardown(options: TauriLocalRuntimeOptions): () => Promise<void> {
  return async function globalTeardown(): Promise<void> {
    if (process.env[EXTERNAL_RUNTIME_ENV] === '1') {
      return;
    }

    cleanupTauriLocalArtifacts(options, false);
  };
}
