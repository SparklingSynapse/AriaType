#!/usr/bin/env node
import { loadRunnerConfigModule, normalizeRunnerCliArgs, runOrderedTauriSuite } from './ordered-tauri-runner.mjs';

const { configPath, extraArgs } = normalizeRunnerCliArgs(process.argv.slice(2));
const config = await loadRunnerConfigModule(configPath);

await runOrderedTauriSuite(config, extraArgs);
