import test from 'node:test';
import assert from 'node:assert/strict';

const { runCommand } = await import('./build-all-platforms-lib.mjs');

test('retries once when notarization upload times out', () => {
  let attempts = 0;

  const exec = () => {
    attempts += 1;
    if (attempts === 1) {
      throw new Error(
        'failed to notarize app: Error: abortedUpload(error: HTTPClientError.deadlineExceeded)'
      );
    }
  };

  const logs = [];
  const success = runCommand('pnpm tauri build', 'Building macOS Intel', {
    exec,
    log: {
      info(message) {
        logs.push(message);
      },
      error(message) {
        logs.push(message);
      },
      warn(message) {
        logs.push(message);
      },
    },
    maxAttempts: 2,
  });

  assert.equal(success, true);
  assert.equal(attempts, 2);
  assert.ok(logs.some((message) => message.includes('Retrying after notarization upload timeout')));
});

test('does not retry unrelated build failures', () => {
  let attempts = 0;
  const logs = [];

  const exec = () => {
    attempts += 1;
    throw new Error('cargo build failed');
  };

  const success = runCommand('cargo build', 'Building macOS Intel', {
    exec,
    log: {
      info(message) {
        logs.push(message);
      },
      error(message) {
        logs.push(message);
      },
      warn(message) {
        logs.push(message);
      },
    },
    maxAttempts: 2,
  });

  assert.equal(success, false);
  assert.equal(attempts, 1);
});

test('calls failure hook when final attempt fails', () => {
  const failure = new Error('bundle_dmg.sh failed');
  let observedError;

  const success = runCommand('pnpm tauri build', 'Building macOS ARM', {
    exec() {
      throw failure;
    },
    log: {
      info() {},
      error() {},
      warn() {},
    },
    onFailure(error) {
      observedError = error;
    },
  });

  assert.equal(success, false);
  assert.equal(observedError, failure);
});

test('mirrors command output to a build log when requested', () => {
  let observedCommand;
  let observedOptions;

  const success = runCommand('pnpm tauri build', 'Building macOS ARM', {
    exec(command, options) {
      observedCommand = command;
      observedOptions = options;
    },
    log: {
      info() {},
      error() {},
      warn() {},
    },
    logFile: '/tmp/ariatype build.log',
  });

  assert.equal(success, true);
  assert.equal(
    observedCommand,
    "set -o pipefail; (pnpm tauri build) 2>&1 | tee '/tmp/ariatype build.log'",
  );
  assert.equal(observedOptions.shell, '/bin/bash');
  assert.equal(observedOptions.stdio, 'inherit');
});
