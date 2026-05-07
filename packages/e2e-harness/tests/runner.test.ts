import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { afterEach, describe, expect, it } from 'vitest';

import { loadRunnerConfigModule, normalizeRunnerCliArgs } from '../src/ordered-tauri-runner.mjs';

const tempDirs: string[] = [];

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => rm(dir, { recursive: true, force: true })));
});

describe('normalizeRunnerCliArgs', () => {
  it('extracts the config path and strips a leading separator', () => {
    expect(normalizeRunnerCliArgs(['tests/e2e/e2e.config.mjs', '--', '--grep', 'journey'])).toEqual({
      configPath: 'tests/e2e/e2e.config.mjs',
      extraArgs: ['--grep', 'journey'],
    });
  });
});

describe('loadRunnerConfigModule', () => {
  it('loads a default-exported runner config from a relative module path', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'e2e-harness-runner-'));
    tempDirs.push(tempDir);
    const configPath = join(tempDir, 'e2e.config.mjs');

    await writeFile(
      configPath,
      'export default { projectRoot: "/repo", playwrightConfig: "tests/e2e/playwright.config.ts" };',
      'utf8',
    );

    await expect(loadRunnerConfigModule('./e2e.config.mjs', tempDir)).resolves.toEqual({
      projectRoot: '/repo',
      playwrightConfig: 'tests/e2e/playwright.config.ts',
    });
  });
});
