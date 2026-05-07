import { createTauriPlaywrightConfig } from '@ariatype/e2e-harness/playwright';
import config from './e2e.config.mjs';

const snapshotDir = new URL('./snapshots/', import.meta.url).pathname;
export default createTauriPlaywrightConfig(config, { snapshotDir });
