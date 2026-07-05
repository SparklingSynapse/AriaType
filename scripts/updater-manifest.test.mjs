import test from 'node:test';
import assert from 'node:assert/strict';

const {
  buildUpdaterManifest,
  inferUpdaterPlatforms,
  mergeUpdaterPlatforms,
} = await import('./updater-manifest.mjs');

test('infers Tauri updater platform keys from artifact and bundle paths', () => {
  assert.deepEqual(
    inferUpdaterPlatforms({
      file: 'AriaType.app.tar.gz',
      sourceDir: '/repo/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos',
    }),
    ['darwin-aarch64'],
  );

  assert.deepEqual(
    inferUpdaterPlatforms({
      file: 'AriaType.app.tar.gz',
      sourceDir: '/repo/apps/desktop/src-tauri/target/x86_64-apple-darwin/release/bundle/macos',
    }),
    ['darwin-x86_64'],
  );

  assert.deepEqual(
    inferUpdaterPlatforms({
      file: 'AriaType.app.tar.gz',
      sourceDir: '/repo/apps/desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos',
    }),
    ['darwin-aarch64', 'darwin-x86_64'],
  );

  assert.deepEqual(
    inferUpdaterPlatforms({
      file: 'AriaType_1.0.4_x64-setup.nsis.zip',
      sourceDir: '/repo/apps/desktop/src-tauri/target/release/bundle/nsis',
    }),
    ['windows-x86_64'],
  );

  assert.deepEqual(
    inferUpdaterPlatforms({
      file: 'AriaType_1.0.4_x64-setup.exe',
      sourceDir: '/tmp/release',
    }),
    ['windows-x86_64'],
  );
});

test('builds a Tauri updater manifest with signature contents', () => {
  const manifest = buildUpdaterManifest({
    version: '1.0.5',
    pubDate: '2026-07-03T08:00:00.000Z',
    notes: 'Fast and quiet.',
    platforms: {
      'darwin-aarch64': {
        url: 'https://github.com/joe223/AriaType/releases/latest/download/AriaType.app.tar.gz',
        signature: 'mac-signature',
      },
      'windows-x86_64': {
        url: 'https://github.com/joe223/AriaType/releases/latest/download/AriaType_1.0.5_x64-setup.exe',
        signature: 'win-signature',
      },
    },
  });

  assert.deepEqual(manifest, {
    version: '1.0.5',
    pub_date: '2026-07-03T08:00:00.000Z',
    notes: 'Fast and quiet.',
    platforms: {
      'darwin-aarch64': {
        url: 'https://github.com/joe223/AriaType/releases/latest/download/AriaType.app.tar.gz',
        signature: 'mac-signature',
      },
      'windows-x86_64': {
        url: 'https://github.com/joe223/AriaType/releases/latest/download/AriaType_1.0.5_x64-setup.exe',
        signature: 'win-signature',
      },
    },
  });
});

test('drops stale updater platforms when the release version changes', () => {
  assert.deepEqual(
    mergeUpdaterPlatforms({
      existingLatest: {
        version: '1.0.4',
        platforms: {
          'darwin-aarch64': {
            url: 'https://github.com/joe223/AriaType/releases/download/v1.0.4/old.app.tar.gz',
            signature: 'old',
          },
        },
      },
      version: '1.0.5',
      nextPlatforms: {
        'windows-x86_64': {
          url: 'https://github.com/joe223/AriaType/releases/download/v1.0.5/new.nsis.zip',
          signature: 'new',
        },
      },
    }),
    {
      'windows-x86_64': {
        url: 'https://github.com/joe223/AriaType/releases/download/v1.0.5/new.nsis.zip',
        signature: 'new',
      },
    },
  );
});
