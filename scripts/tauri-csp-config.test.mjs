import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');

function readTauriConfig() {
  return JSON.parse(
    readFileSync(resolve(root, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8')
  );
}

function readChangelogOrigin() {
  const source = readFileSync(
    resolve(root, 'apps/desktop/src/components/Home/ChangelogPage.tsx'),
    'utf8'
  );
  const match = source.match(/const CHANGELOG_URL =\s*\n\s*"([^"]+)";/);

  assert.ok(match, 'CHANGELOG_URL should stay as a readable string constant');

  return new URL(match[1]).origin;
}

function parseCsp(csp) {
  return Object.fromEntries(
    csp
      .split(';')
      .map((directive) => directive.trim().split(/\s+/))
      .filter(([name]) => name)
      .map(([name, ...values]) => [name, values])
  );
}

test('tauri security config preserves style-src for runtime inline styles', () => {
  const tauriConfig = readTauriConfig();

  assert.deepEqual(
    tauriConfig.app.security.dangerousDisableAssetCspModification,
    ['style-src']
  );
});

test('tauri CSP allows changelog fetch origin', () => {
  const tauriConfig = readTauriConfig();
  const csp = parseCsp(tauriConfig.app.security.csp);
  const changelogOrigin = readChangelogOrigin();

  assert.ok(
    csp['connect-src'].includes(changelogOrigin),
    `connect-src should include ${changelogOrigin} for the changelog page`
  );
});
