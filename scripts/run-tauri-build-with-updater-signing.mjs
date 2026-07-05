#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import {
  normalizeUpdaterSigningEnv,
  printUpdaterSigningEnvError,
  printUpdaterSigningMismatchError,
  printUpdaterSigningWarnings,
} from './ensure-updater-signing-env.mjs';

const UPDATER_CONFIG = 'src-tauri/tauri.updater.conf.json';
const GENERATED_UPDATER_CONFIG = 'src-tauri/tauri.updater.generated.conf.json';

function samePath(cwd, left, right) {
  return resolve(cwd, left) === resolve(cwd, right);
}

export function writeUpdaterConfigWithPubkey({
  cwd = process.cwd(),
  env = process.env,
  sourceConfig = UPDATER_CONFIG,
  outputConfig = GENERATED_UPDATER_CONFIG,
} = {}) {
  const sourcePath = resolve(cwd, sourceConfig);
  if (!existsSync(sourcePath)) {
    return null;
  }

  const config = JSON.parse(readFileSync(sourcePath, 'utf8'));
  config.plugins = config.plugins || {};
  config.plugins.updater = config.plugins.updater || {};
  config.plugins.updater.pubkey = env.TAURI_UPDATER_PUBKEY.trim();

  const outputPath = resolve(cwd, outputConfig);
  writeFileSync(outputPath, `${JSON.stringify(config, null, 2)}\n`);
  return outputConfig;
}

export function rewriteUpdaterConfigArgs(commandArgs, {
  cwd = process.cwd(),
  sourceConfig = UPDATER_CONFIG,
  generatedConfig = GENERATED_UPDATER_CONFIG,
} = {}) {
  const rewritten = [...commandArgs];

  for (let index = 0; index < rewritten.length; index += 1) {
    const arg = rewritten[index];
    if (arg === '--config' && rewritten[index + 1] && samePath(cwd, rewritten[index + 1], sourceConfig)) {
      rewritten[index + 1] = generatedConfig;
      index += 1;
      continue;
    }

    const prefix = '--config=';
    if (arg.startsWith(prefix) && samePath(cwd, arg.slice(prefix.length), sourceConfig)) {
      rewritten[index] = `${prefix}${generatedConfig}`;
    }
  }

  return rewritten;
}

function withSigningKeyArgs(args, privateKeyPath) {
  return privateKeyPath
    ? [...args, '--private-key-path', privateKeyPath]
    : args;
}

export function buildTauriSignerProbeCommand(commandArgs, probePath, { privateKeyPath = '' } = {}) {
  if (commandArgs[0] === 'npm' && commandArgs[1] === 'run' && commandArgs[2] === 'tauri') {
    return ['npm', 'run', 'tauri', '--', ...withSigningKeyArgs(['signer', 'sign'], privateKeyPath), probePath];
  }

  if (commandArgs[0] === 'tauri') {
    return ['tauri', ...withSigningKeyArgs(['signer', 'sign'], privateKeyPath), probePath];
  }

  if (commandArgs[0] === 'cargo' && commandArgs[1] === 'tauri') {
    return ['cargo', 'tauri', ...withSigningKeyArgs(['signer', 'sign'], privateKeyPath), probePath];
  }

  return null;
}

export function verifyUpdaterSigningKeyCanSign({
  commandArgs,
  cwd = process.cwd(),
  env = process.env,
  privateKeyPath = '',
  spawn = spawnSync,
} = {}) {
  const dir = mkdtempSync(join(tmpdir(), 'ariatype-updater-sign-'));
  const probePath = join(dir, 'probe.txt');

  try {
    writeFileSync(probePath, 'ariatype updater signing probe');

    const signerCommand = buildTauriSignerProbeCommand(commandArgs, probePath, { privateKeyPath });
    if (!signerCommand) {
      return { ok: true, skipped: true };
    }

    const [command, ...args] = signerCommand;
    const probeEnv = { ...env };
    if (privateKeyPath) {
      delete probeEnv.TAURI_SIGNING_PRIVATE_KEY;
      delete probeEnv.TAURI_SIGNING_PRIVATE_KEY_PATH;
    }

    const result = spawn(command, args, {
      cwd,
      env: probeEnv,
      encoding: 'utf8',
      stdio: 'pipe',
    });

    if (result.error) {
      return { ok: false, error: `Failed to run ${command}: ${result.error.message}` };
    }

    if (result.status !== 0) {
      return {
        ok: false,
        error: (result.stderr || result.stdout || `Updater signing probe failed with exit code ${result.status}`).trim(),
      };
    }

    return { ok: true, skipped: false };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

function run() {
  const args = process.argv.slice(2);
  const commandArgs = args[0] === '--' ? args.slice(1) : args;

  if (commandArgs.length === 0) {
    console.error('Usage: node scripts/run-tauri-build-with-updater-signing.mjs -- <command> [...args]');
    process.exit(1);
  }

  const signingEnv = normalizeUpdaterSigningEnv(process.env);
  if (!signingEnv.ok) {
    printUpdaterSigningEnvError(signingEnv.missing);
    printUpdaterSigningMismatchError(signingEnv.mismatches);
    process.exit(1);
  }
  printUpdaterSigningWarnings(signingEnv.warnings);

  const signingProbe = verifyUpdaterSigningKeyCanSign({
    commandArgs,
    env: signingEnv.env,
    privateKeyPath: signingEnv.privateKeyPath,
  });
  if (!signingProbe.ok) {
    console.error('Invalid updater signing key or password.');
    console.error(signingProbe.error);
    process.exit(1);
  }

  const generatedConfig = writeUpdaterConfigWithPubkey({ env: signingEnv.env });
  const finalCommandArgs = generatedConfig
    ? rewriteUpdaterConfigArgs(commandArgs, { generatedConfig })
    : commandArgs;

  const [command, ...rest] = finalCommandArgs;
  const result = spawnSync(command, rest, {
    cwd: process.cwd(),
    env: signingEnv.env,
    shell: false,
    stdio: 'inherit',
  });

  if (result.error) {
    console.error(`Failed to run ${command}: ${result.error.message}`);
    process.exit(1);
  }

  process.exit(result.status ?? 1);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  run();
}
