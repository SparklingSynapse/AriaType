import { createTauriGlobalSetup } from './playwright-hooks';

const runtimeRoot = process.env.E2E_HARNESS_RUNTIME_ROOT;
const socketPath = process.env.E2E_HARNESS_SOCKET_PATH;

if (!runtimeRoot || !socketPath) {
  throw new Error('Missing E2E_HARNESS_RUNTIME_ROOT or E2E_HARNESS_SOCKET_PATH for Playwright setup');
}

export default createTauriGlobalSetup({
  killCommand: process.env.E2E_HARNESS_KILL_COMMAND || undefined,
  runtimeRoot,
  socketPath,
});
