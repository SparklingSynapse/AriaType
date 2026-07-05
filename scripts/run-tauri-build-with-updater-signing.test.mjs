import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const {
  buildTauriSignerProbeCommand,
  rewriteUpdaterConfigArgs,
  verifyUpdaterSigningKeyCanSign,
  writeUpdaterConfigWithPubkey,
} = await import('./run-tauri-build-with-updater-signing.mjs');

function withTempDesktop(run) {
  const cwd = mkdtempSync(join(tmpdir(), 'ariatype-updater-config-'));
  try {
    mkdirSync(join(cwd, 'src-tauri'), { recursive: true });
    writeFileSync(
      join(cwd, 'src-tauri/tauri.updater.conf.json'),
      `${JSON.stringify({
        bundle: { createUpdaterArtifacts: true },
        plugins: {
          updater: {
            pubkey: '${TAURI_UPDATER_PUBKEY}',
            endpoints: ['https://github.com/joe223/AriaType/releases/latest/download/latest.updater.json'],
          },
        },
      }, null, 2)}\n`,
    );
    return run(cwd);
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

test('writes updater config with resolved public key', () => {
  withTempDesktop((cwd) => {
    const generated = writeUpdaterConfigWithPubkey({
      cwd,
      env: {
        TAURI_UPDATER_PUBKEY: 'resolved-public-key',
      },
    });

    assert.equal(generated, 'src-tauri/tauri.updater.generated.conf.json');
    const config = JSON.parse(readFileSync(join(cwd, generated), 'utf8'));
    assert.equal(config.plugins.updater.pubkey, 'resolved-public-key');
  });
});

test('rewrites updater config argument to generated config', () => {
  withTempDesktop((cwd) => {
    assert.deepEqual(
      rewriteUpdaterConfigArgs(
        [
          'pnpm',
          'tauri',
          'build',
          '--config',
          'src-tauri/tauri.macos.conf.json',
          '--config',
          'src-tauri/tauri.updater.conf.json',
        ],
        { cwd },
      ),
      [
        'pnpm',
        'tauri',
        'build',
        '--config',
        'src-tauri/tauri.macos.conf.json',
        '--config',
        'src-tauri/tauri.updater.generated.conf.json',
      ],
    );
  });
});

test('rewrites equals-form updater config argument to generated config', () => {
  withTempDesktop((cwd) => {
    assert.deepEqual(
      rewriteUpdaterConfigArgs(
        [
          'tauri',
          'build',
          '--config=src-tauri/tauri.updater.conf.json',
        ],
        { cwd },
      ),
      [
        'tauri',
        'build',
        '--config=src-tauri/tauri.updater.generated.conf.json',
      ],
    );
  });
});

test('builds signer probe command for npm tauri builds', () => {
  const command = buildTauriSignerProbeCommand(['npm', 'run', 'tauri', '--', 'build'], '/tmp/probe.txt');

  assert.deepEqual(command, ['npm', 'run', 'tauri', '--', 'signer', 'sign', '/tmp/probe.txt']);
});

test('builds signer probe command with explicit private key path', () => {
  const command = buildTauriSignerProbeCommand(
    ['npm', 'run', 'tauri', '--', 'build'],
    '/tmp/probe.txt',
    { privateKeyPath: '/Users/example/.tauri/ariatype-updater.key' },
  );

  assert.deepEqual(command, [
    'npm',
    'run',
    'tauri',
    '--',
    'signer',
    'sign',
    '--private-key-path',
    '/Users/example/.tauri/ariatype-updater.key',
    '/tmp/probe.txt',
  ]);
});

test('builds signer probe command for direct tauri builds', () => {
  const command = buildTauriSignerProbeCommand(['tauri', 'build'], '/tmp/probe.txt');

  assert.deepEqual(command, ['tauri', 'signer', 'sign', '/tmp/probe.txt']);
});

test('builds signer probe command for cargo tauri builds', () => {
  const command = buildTauriSignerProbeCommand(['cargo', 'tauri', 'build'], '/tmp/probe.txt');

  assert.deepEqual(command, ['cargo', 'tauri', 'signer', 'sign', '/tmp/probe.txt']);
});

test('fails updater signing probe when signer command exits non-zero', () => {
  const calls = [];
  const result = verifyUpdaterSigningKeyCanSign({
    commandArgs: ['tauri', 'build'],
    env: { PATH: '/usr/bin' },
    spawn(command, args, options) {
      calls.push({ command, args, options });
      return { status: 1, stderr: 'incorrect updater private key password' };
    },
  });

  assert.equal(result.ok, false);
  assert.match(result.error, /incorrect updater private key password/);
  assert.equal(calls[0].command, 'tauri');
  assert.deepEqual(calls[0].args.slice(0, 2), ['signer', 'sign']);
  assert.equal(calls[0].options.env.PATH, '/usr/bin');
});

test('removes mutually exclusive private key env from explicit path signing probe', () => {
  const calls = [];
  const result = verifyUpdaterSigningKeyCanSign({
    commandArgs: ['tauri', 'build'],
    env: {
      PATH: '/usr/bin',
      TAURI_SIGNING_PRIVATE_KEY: '/Users/example/.tauri/old.key',
      TAURI_SIGNING_PRIVATE_KEY_PATH: '/Users/example/.tauri/new.key',
    },
    privateKeyPath: '/Users/example/.tauri/new.key',
    spawn(command, args, options) {
      calls.push({ command, args, options });
      return { status: 0, stderr: '' };
    },
  });

  assert.equal(result.ok, true);
  assert.equal(calls[0].options.env.TAURI_SIGNING_PRIVATE_KEY, undefined);
  assert.equal(calls[0].options.env.TAURI_SIGNING_PRIVATE_KEY_PATH, undefined);
  assert.deepEqual(calls[0].args.slice(0, 4), ['signer', 'sign', '--private-key-path', '/Users/example/.tauri/new.key']);
});


test('skips updater signing probe for unknown build commands', () => {
  const result = verifyUpdaterSigningKeyCanSign({
    commandArgs: ['node', 'script.mjs'],
    spawn() {
      throw new Error('spawn should not be called');
    },
  });

  assert.equal(result.ok, true);
  assert.equal(result.skipped, true);
});
