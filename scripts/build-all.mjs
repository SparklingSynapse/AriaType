#!/usr/bin/env node

import { spawnSync } from 'child_process';

const NPM_CONFIG_FLAGS = ['cross-win', 'skip-mac-arm', 'skip-mac-intel', 'skip-win', 'unsigned'];

function normalizeArgs(argv, env) {
  const normalized = [...argv];
  const present = new Set(normalized);

  for (const flag of NPM_CONFIG_FLAGS) {
    const envKey = `npm_config_${flag.replaceAll('-', '_')}`;
    const cliFlag = `--${flag}`;
    if (env[envKey] === 'true' && !present.has(cliFlag)) {
      normalized.push(cliFlag);
      present.add(cliFlag);
    }
  }

  return normalized;
}

const args = normalizeArgs(process.argv.slice(2), process.env);

function run(command, commandArgs, description) {
  const result = spawnSync(command, commandArgs, {
    cwd: process.cwd(),
    env: process.env,
    stdio: 'inherit',
    shell: false,
  });

  if (result.error) {
    console.error(`\n❌ ${description} failed: ${result.error.message}\n`);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

run('node', ['scripts/build-all-platforms.mjs', ...args], 'Desktop multi-platform build');
run('npm', ['run', 'build:website'], 'Website build');
