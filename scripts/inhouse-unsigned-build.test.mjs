import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');

test('desktop unsigned mac build script merges in-house and unsigned configs', () => {
  const packageJson = JSON.parse(
    readFileSync(resolve(root, 'apps/desktop/package.json'), 'utf8')
  );

  assert.equal(
    packageJson.scripts['tauri:build:mac:unsigned'],
    'env -u APPLE_SIGNING_IDENTITY -u APPLE_TEAM_ID -u APPLE_ID -u APPLE_PASSWORD tauri build --config src-tauri/tauri.dev.conf.json --config src-tauri/tauri.macos.unsigned.conf.json && pnpm copy-installer'
  );
});

test('multi-platform unsigned mac commands merge in-house and unsigned configs', () => {
  const script = readFileSync(resolve(root, 'scripts/build-all-platforms.mjs'), 'utf8');

  assert.match(
    script,
    /tauri\.dev\.conf\.json --config src-tauri\/tauri\.macos\.unsigned\.conf\.json --target aarch64-apple-darwin/
  );
  assert.match(
    script,
    /tauri\.dev\.conf\.json --config src-tauri\/tauri\.macos\.unsigned\.conf\.json --target x86_64-apple-darwin/
  );
});
