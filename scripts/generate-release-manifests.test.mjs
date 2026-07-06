import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

function withTempReleaseDir(run) {
  const releaseDir = mkdtempSync(join(tmpdir(), 'ariatype-release-'));
  try {
    return run(releaseDir);
  } finally {
    rmSync(releaseDir, { recursive: true, force: true });
  }
}

function runGenerateReleaseManifests(args) {
  return spawnSync(process.execPath, ['scripts/generate-release-manifests.mjs', ...args], {
    cwd: process.cwd(),
    encoding: 'utf8',
  });
}

test('fails when updater manifest is required but no signed updater artifacts exist', () => {
  withTempReleaseDir((releaseDir) => {
    writeFileSync(join(releaseDir, 'AriaType_1.0.5_x64-setup.exe'), 'installer');

    const result = runGenerateReleaseManifests([
      '--release-dir',
      releaseDir,
      '--version',
      '1.0.5',
      '--require-updater',
    ]);

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /latest\.updater\.json was not generated/);
  });
});

test('generates updater manifest when signed updater artifacts are present', () => {
  withTempReleaseDir((releaseDir) => {
    writeFileSync(join(releaseDir, 'AriaType.app.tar.gz'), 'mac updater archive');
    writeFileSync(join(releaseDir, 'AriaType.app.tar.gz.sig'), 'mac-signature\n');
    writeFileSync(join(releaseDir, 'AriaType_1.0.5_x64-setup.exe'), 'windows installer');
    writeFileSync(join(releaseDir, 'AriaType_1.0.5_x64-setup.exe.sig'), 'windows-signature\n');

    const result = runGenerateReleaseManifests([
      '--release-dir',
      releaseDir,
      '--version',
      '1.0.5',
      '--base-url',
      'https://github.com/joe223/AriaType/releases/download/v1.0.5',
      '--require-updater',
      '--require-updater-platform',
      'darwin-aarch64',
      '--require-updater-platform',
      'darwin-x86_64',
      '--require-updater-platform',
      'windows-x86_64',
    ]);

    assert.equal(result.status, 0, result.stderr);
    const latest = JSON.parse(readFileSync(join(releaseDir, 'latest.json'), 'utf8'));
    assert.equal(
      latest.platforms.windows.exe,
      'https://github.com/joe223/AriaType/releases/download/v1.0.5/AriaType_1.0.5_x64-setup.exe',
    );

    const manifest = JSON.parse(readFileSync(join(releaseDir, 'latest.updater.json'), 'utf8'));
    assert.deepEqual(manifest.platforms, {
      'darwin-aarch64': {
        url: 'https://github.com/joe223/AriaType/releases/download/v1.0.5/AriaType.app.tar.gz',
        signature: 'mac-signature',
      },
      'darwin-x86_64': {
        url: 'https://github.com/joe223/AriaType/releases/download/v1.0.5/AriaType.app.tar.gz',
        signature: 'mac-signature',
      },
      'windows-x86_64': {
        url: 'https://github.com/joe223/AriaType/releases/download/v1.0.5/AriaType_1.0.5_x64-setup.exe',
        signature: 'windows-signature',
      },
    });
  });
});

test('fails when a required updater platform is missing', () => {
  withTempReleaseDir((releaseDir) => {
    writeFileSync(join(releaseDir, 'AriaType.app.tar.gz'), 'mac updater archive');
    writeFileSync(join(releaseDir, 'AriaType.app.tar.gz.sig'), 'mac-signature\n');

    const result = runGenerateReleaseManifests([
      '--release-dir',
      releaseDir,
      '--version',
      '1.0.5',
      '--require-updater',
      '--require-updater-platform',
      'windows-x86_64',
    ]);

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Missing required updater platform\(s\): windows-x86_64/);
  });
});
