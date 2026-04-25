import { execSync } from 'child_process';

function write(log, level, message) {
  const method = log[level] ?? log.log;
  method.call(log, message);
}

function getErrorMessage(error) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function isRetryableNotarizationTimeout(error, description) {
  if (!description.startsWith('Building macOS')) {
    return false;
  }

  const message = getErrorMessage(error);
  return (
    message.includes('failed to notarize app')
    && (message.includes('HTTPClientError.deadlineExceeded') || message.includes('abortedUpload'))
  );
}

export function runCommand(command, description, options = {}) {
  const {
    cwd,
    env,
    exec = execSync,
    log = console,
    maxAttempts = 1,
  } = options;

  write(log, 'info', `\n${'═'.repeat(50)}`);
  write(log, 'info', `📦 ${description}`);
  write(log, 'info', `${'═'.repeat(50)}\n`);

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    const startTime = Date.now();

    try {
      exec(command, {
        cwd,
        stdio: 'inherit',
        env,
      });

      const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      write(log, 'info', `\n✅ ${description} completed in ${elapsed}s\n`);
      return true;
    } catch (error) {
      if (attempt < maxAttempts && isRetryableNotarizationTimeout(error, description)) {
        write(
          log,
          'warn',
          `\n⚠️  Retrying after notarization upload timeout (${attempt}/${maxAttempts - 1})\n`
        );
        continue;
      }

      write(log, 'error', `\n❌ ${description} failed\n`);
      return false;
    }
  }

  write(log, 'error', `\n❌ ${description} failed\n`);
  return false;
}
