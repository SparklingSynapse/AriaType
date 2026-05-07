import { join } from 'node:path';
import {
  createRunnerConfig,
  resolveHarnessDir,
} from '@ariatype/e2e-harness/runner';

const e2eDir = resolveHarnessDir(import.meta.url);
export const projectRoot = join(e2eDir, '..', '..');
export const runtimeKey = 'ordered-shared';
const userHome = process.env.HOME ?? '/Users/bytedance';
export const killCommand = 'pkill -f "target/debug/ariatype"';
export const systemDataPaths = [
  join(userHome, 'Library', 'Application Support', 'AriaType E2E'),
  join(userHome, 'Library', 'Application Support', 'com.ariatype.voicetotext.e2e'),
  join(userHome, 'Library', 'WebKit', 'app'),
  join(userHome, 'Library', 'WebKit', 'com.ariatype.voicetotext.e2e'),
  join(userHome, 'Library', 'WebKit', 'com.ariatype.voicetotext'),
  join(userHome, 'Library', 'WebKit', 'com.ariatype.voicetotext.inhouse'),
  join(userHome, 'Library', 'WebKit', 'com.notype.app'),
  join(userHome, 'Library', 'WebKit', 'com.notype.voicetotext'),
  join(userHome, 'Library', 'WebKit', 'ariatype'),
  join(userHome, 'Library', 'WebKit', 'notype'),
];
export const tauriCommand = [
  'tauri',
  'dev',
  '--config',
  'src-tauri/tauri.dev.conf.json',
  '--config',
  'src-tauri/tauri.e2e.conf.json',
];
export const tauriFeatures = ['e2e-testing'];
export const capabilityFiles = [
  {
    src: join(e2eDir, 'capabilities', 'e2e.json'),
    dest: join(projectRoot, 'src-tauri', 'capabilities', 'e2e.json'),
  },
];

export default createRunnerConfig({
  projectRoot,
  pagesDir: 'tests/e2e/pages',
  specsPrefix: 'tests/e2e/pages',
  playwrightConfig: 'tests/e2e/playwright.config.ts',
  specOrder: ['journey.spec.ts', 'history.spec.ts'],
  runtimeRoot: join(projectRoot, `tests/e2e/.runtime/${runtimeKey}`),
  socketPath: `/tmp/ariatype-pw-${runtimeKey}.sock`,
  killCommand,
  devServerResetPaths: [join(projectRoot, 'node_modules', '.vite')],
  devServerPrepareCommand: ['exec', 'vite', 'optimize', '--force'],
  systemDataPaths,
  tauriCommand,
  tauriFeatures,
  capabilityFiles,
  seedDataFiles: [
    {
      src: join(e2eDir, 'fixtures', 'settings-cloud-enabled.json'),
      // Use e2e-specific directory based on productName "AriaType E2E"
      dest: join(userHome, 'Library', 'Application Support', 'AriaType E2E', 'settings.json'),
    },
  ],
  startTimeoutSeconds: 180,
  socketWaitMs: 5000,
  snapshotStabilizationMs: 1000,
  devServerCommand: ['exec', 'vite', '--port', '1423', '--strictPort'],
  devServerUrl: 'http://localhost:1423',
  devServerReadyTimeoutMs: 30000,
});
