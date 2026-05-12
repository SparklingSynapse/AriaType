#!/usr/bin/env node
import { rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));

const e2eCapabilityPath = join(
  scriptDir,
  '..',
  'apps',
  'desktop',
  'src-tauri',
  'capabilities',
  'e2e.json',
);

console.log(`[clean-e2e-capability] removing ${e2eCapabilityPath}`);
rmSync(e2eCapabilityPath, { force: true });
console.log('[clean-e2e-capability] done');
