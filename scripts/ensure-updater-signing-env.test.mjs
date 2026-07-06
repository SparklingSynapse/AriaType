import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const { normalizeUpdaterSigningEnv } = await import('./ensure-updater-signing-env.mjs');

function runWithEnv(env) {
  return spawnSync(process.execPath, ['scripts/ensure-updater-signing-env.mjs'], {
    cwd: new URL('..', import.meta.url),
    env: {
      PATH: process.env.PATH,
      ...env,
    },
    encoding: 'utf8',
  });
}

test('accepts a signing private key path with updater public key', () => {
  const result = runWithEnv({
    TAURI_SIGNING_PRIVATE_KEY_PATH: '/Users/example/.tauri/ariatype-updater.key',
    TAURI_UPDATER_PUBKEY: 'public-key',
  });

  assert.equal(result.status, 0);
});

test('normalizes private key path into the Tauri signing environment variable', () => {
  const result = normalizeUpdaterSigningEnv({
    PATH: '/usr/bin',
    TAURI_SIGNING_PRIVATE_KEY_PATH: '/Users/example/.tauri/ariatype-updater.key',
    TAURI_UPDATER_PUBKEY: 'public-key',
  });

  assert.equal(result.ok, true);
  assert.equal(
    result.env.TAURI_SIGNING_PRIVATE_KEY,
    '/Users/example/.tauri/ariatype-updater.key',
  );
  assert.equal(result.env.PATH, '/usr/bin');
  assert.equal(result.privateKeyPath, '/Users/example/.tauri/ariatype-updater.key');
});

test('prefers an explicit private key path over a stale private key value', () => {
  const result = normalizeUpdaterSigningEnv({
    TAURI_SIGNING_PRIVATE_KEY: '/Users/example/.tauri/ariatype.key',
    TAURI_SIGNING_PRIVATE_KEY_PATH: '/Users/example/.tauri/ariatype-updater.key',
    TAURI_UPDATER_PUBKEY: 'public-key',
  });

  assert.equal(result.ok, true);
  assert.equal(result.env.TAURI_SIGNING_PRIVATE_KEY, '/Users/example/.tauri/ariatype-updater.key');
  assert.equal(result.privateKeyPath, '/Users/example/.tauri/ariatype-updater.key');
});

test('accepts signing private key contents with updater public key', () => {
  const result = runWithEnv({
    TAURI_SIGNING_PRIVATE_KEY: 'private-key',
    TAURI_UPDATER_PUBKEY: 'public-key',
  });

  assert.equal(result.status, 0);
});

test('fails when both private key forms are missing', () => {
  const result = runWithEnv({
    TAURI_UPDATER_PUBKEY: 'public-key',
  });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /TAURI_SIGNING_PRIVATE_KEY or TAURI_SIGNING_PRIVATE_KEY_PATH/);
});

test('uses private key path sibling public key when updater public key is missing', () => {
  const dir = mkdtempSync(join(tmpdir(), 'ariatype-updater-key-'));
  const privateKeyPath = join(dir, 'ariatype-updater.key');
  writeFileSync(privateKeyPath, 'private-key');
  writeFileSync(`${privateKeyPath}.pub`, 'actual-public-key\n');

  const result = runWithEnv({
    TAURI_SIGNING_PRIVATE_KEY_PATH: privateKeyPath,
  });

  assert.equal(result.status, 0);
});

test('prefers private key path sibling public key over stale updater public key', () => {
  const dir = mkdtempSync(join(tmpdir(), 'ariatype-updater-key-'));
  const privateKeyPath = join(dir, 'ariatype-updater.key');
  writeFileSync(privateKeyPath, 'private-key');
  writeFileSync(`${privateKeyPath}.pub`, 'actual-public-key\n');

  const result = normalizeUpdaterSigningEnv({
    TAURI_SIGNING_PRIVATE_KEY_PATH: privateKeyPath,
    TAURI_UPDATER_PUBKEY: 'different-public-key',
  });

  assert.equal(result.ok, true);
  assert.equal(result.env.TAURI_UPDATER_PUBKEY, 'actual-public-key');
  assert.deepEqual(result.mismatches, []);
  assert.deepEqual(result.warnings, [`TAURI_UPDATER_PUBKEY was replaced with ${privateKeyPath}.pub`]);
});
