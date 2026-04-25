import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');

test('tauri security config preserves style-src for runtime inline styles', () => {
  const tauriConfig = JSON.parse(
    readFileSync(resolve(root, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8')
  );

  assert.deepEqual(
    tauriConfig.app.security.dangerousDisableAssetCspModification,
    ['style-src']
  );
});
