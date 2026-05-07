export type SeedFileMapping = {
  src: string;
  dest: string;
};

export type CapabilityFileMapping = {
  src: string;
  dest: string;
};

export type OrderedTauriRunnerConfig = {
  projectRoot: string;
  pagesDir: string;
  specsPrefix: string;
  playwrightConfig: string;
  specOrder: string[];
  runtimeRoot: string;
  socketPath: string;
  killCommand: string;
  systemDataPaths?: string[];
  devServerResetPaths?: string[];
  devServerPrepareCommand?: string[];
  tauriCommand: string[];
  tauriFeatures?: string[];
  capabilityFiles?: CapabilityFileMapping[];
  seedFiles?: SeedFileMapping[];
  seedDataFiles?: SeedFileMapping[];
  startTimeoutSeconds?: number;
  socketWaitMs?: number;
  snapshotStabilizationMs?: number;
  devServerCommand: string[];
  devServerUrl: string;
  devServerReadyTimeoutMs?: number;
};

export function runOrderedTauriSuite(
  config: OrderedTauriRunnerConfig,
  extraArgs: string[],
): Promise<void>;

export function normalizeRunnerCliArgs(argv: string[]): {
  configPath: string;
  extraArgs: string[];
};

export function loadRunnerConfigModule(
  configPath: string,
  cwd?: string,
): Promise<OrderedTauriRunnerConfig>;

export function createRunnerConfig(config: OrderedTauriRunnerConfig): OrderedTauriRunnerConfig;

export function resolveHarnessDir(importMetaUrl: string): string;
