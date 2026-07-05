import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const desktopTauriDir = join(import.meta.dirname, '../apps/desktop/src-tauri');

function readConfig(file) {
  return JSON.parse(readFileSync(join(desktopTauriDir, file), 'utf8'));
}

test('dev Tauri config initializes updater plugin with inert defaults', () => {
  const config = readConfig('tauri.dev.conf.json');

  assert.equal(typeof config.plugins?.updater, 'object');
  assert.notEqual(config.plugins.updater, null);
  assert.equal(config.plugins.updater.pubkey, '');
  assert.deepEqual(config.plugins.updater.endpoints, []);
});

test('release updater overlay provides signed-update configuration', () => {
  const config = readConfig('tauri.updater.conf.json');

  assert.equal(typeof config.plugins?.updater, 'object');
  assert.notEqual(config.plugins.updater, null);
  assert.equal(config.plugins.updater.pubkey, '${TAURI_UPDATER_PUBKEY}');
  assert.deepEqual(config.plugins.updater.endpoints, [
    'https://github.com/joe223/AriaType/releases/latest/download/latest.updater.json',
  ]);
});
