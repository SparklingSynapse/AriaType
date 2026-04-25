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
