import { execSync } from 'child_process';

export const WINDOWS_CROSS_BUILD_COMMAND =
  'cargo tauri build --config src-tauri/tauri.windows.conf.json --runner cargo-xwin --target x86_64-pc-windows-msvc';

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

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function appendEnvToken(value, token) {
  if (!value) {
    return token;
  }

  return value.split(/\s+/).includes(token) ? value : `${value} ${token}`;
}

export function windowsCrossBuildEnv(baseEnv = process.env) {
  return {
    ...baseEnv,
    LLAMA_STATIC_CRT: '1',
    STATIC_VCRUNTIME: 'false',
    RUSTFLAGS: appendEnvToken(baseEnv.RUSTFLAGS, '-Ctarget-feature=+crt-static'),
  };
}

export function runCommand(command, description, options = {}) {
  const {
    cwd,
    env,
    exec = execSync,
    log = console,
    logFile,
    maxAttempts = 1,
    onFailure,
  } = options;

  write(log, 'info', `\n${'═'.repeat(50)}`);
  write(log, 'info', `📦 ${description}`);
  write(log, 'info', `${'═'.repeat(50)}\n`);

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    const startTime = Date.now();

    try {
      const execOptions = {
        cwd,
        stdio: 'inherit',
        env,
      };
      const execCommand = logFile
        ? `set -o pipefail; (${command}) 2>&1 | tee ${shellQuote(logFile)}`
        : command;

      if (logFile) {
        execOptions.shell = '/bin/bash';
      }

      exec(execCommand, execOptions);

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

      onFailure?.(error);
      write(log, 'error', `\n❌ ${description} failed\n`);
      return false;
    }
  }

  write(log, 'error', `\n❌ ${description} failed\n`);
  return false;
}

export function checkRequiredBuildTools(requiredTools, options = {}) {
  const {
    exec = execSync,
    log = console,
  } = options;
  let allAvailable = true;

  for (const tool of requiredTools) {
    try {
      exec(`${tool.command} --version`, { stdio: 'ignore' });
    } catch {
      allAvailable = false;
      write(log, 'error', `❌ Missing required build tool: ${tool.description}`);
      write(log, 'error', `   Install: ${tool.installHint}`);
    }
  }

  if (!allAvailable) {
    write(log, 'error', '\nInstall the missing tools above, then rerun the build.\n');
  }

  return allAvailable;
}
