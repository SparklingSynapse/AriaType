import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import {
  prepareLlamaServerReleaseAssets,
  selectArchiveForTarget,
  selectBinaryMember,
  selectRuntimeDependencyMembers,
  sidecarTargets,
} from './prepare-llama-server-release-assets.mjs';

function createFixture() {
  const root = mkdtempSync(join(tmpdir(), 'ariatype-llama-assets-'));
  const assetsDir = resolve(root, 'assets');
  mkdirSync(assetsDir, { recursive: true });
  mkdirSync(resolve(root, 'apps/desktop/src-tauri'), { recursive: true });
  return { root, assetsDir };
}

function createTarArchive({ assetsDir, archiveName, binaryName, content, dependencies = {} }) {
  const stageDir = mkdtempSync(join(tmpdir(), 'ariatype-llama-archive-'));
  const binaryPath = resolve(stageDir, 'llama-build/bin', binaryName);
  mkdirSync(dirname(binaryPath), { recursive: true });
  writeFileSync(binaryPath, content);
  for (const [name, dependencyContent] of Object.entries(dependencies)) {
    writeFileSync(resolve(stageDir, 'llama-build/bin', name), dependencyContent);
  }

  const archivePath = resolve(assetsDir, archiveName);
  execFileSync('tar', ['-czf', archivePath, '-C', stageDir, '.']);
  return archivePath;
}

function createZipArchive({ assetsDir, archiveName, binaryName, content, dependencies = {} }) {
  const stageDir = mkdtempSync(join(tmpdir(), 'ariatype-llama-archive-'));
  const binaryPath = resolve(stageDir, 'llama-build/bin', binaryName);
  mkdirSync(dirname(binaryPath), { recursive: true });
  writeFileSync(binaryPath, content);
  for (const [name, dependencyContent] of Object.entries(dependencies)) {
    writeFileSync(resolve(stageDir, 'llama-build/bin', name), dependencyContent);
  }

  const archivePath = resolve(assetsDir, archiveName);
  execFileSync('zip', ['-qr', archivePath, '.'], { cwd: stageDir });
  return archivePath;
}

test('selects release archives by target matcher', () => {
  const target = sidecarTargets('windows')[0];
  const archives = [
    '/tmp/llama-b9568-bin-win-cuda-x64.zip',
    '/tmp/llama-b9568-bin-win-cpu-x64.zip',
  ];

  assert.equal(
    selectArchiveForTarget(archives, target),
    '/tmp/llama-b9568-bin-win-cpu-x64.zip'
  );
});

test('selects nested llama-server binary member', () => {
  assert.equal(
    selectBinaryMember(['llama-b9568/bin/llama-cli', 'llama-b9568/bin/llama-server'], 'llama-server'),
    'llama-b9568/bin/llama-server'
  );
});

test('selects same-directory runtime dependency members', () => {
  assert.deepEqual(
    selectRuntimeDependencyMembers(
      [
        'llama-b9568/llama-server',
        'llama-b9568/libllama.0.dylib',
        'llama-b9568/libggml-base.dylib',
        'llama-b9568/bin/ignored.dylib',
        'llama-b9568/llama-cli',
      ],
      'llama-b9568/llama-server',
      'macos'
    ),
    ['llama-b9568/libggml-base.dylib', 'llama-b9568/libllama.0.dylib']
  );
});

test('prepares macOS arm64 and x64 release archives', () => {
  const { root, assetsDir } = createFixture();
  createTarArchive({
    assetsDir,
    archiveName: 'llama-b9568-bin-macos-arm64.tar.gz',
    binaryName: 'llama-server',
    content: 'arm64 server',
    dependencies: {
      'libllama.0.dylib': 'arm64 libllama',
      'libggml-base.dylib': 'arm64 libggml',
    },
  });
  createTarArchive({
    assetsDir,
    archiveName: 'llama-b9568-bin-macos-x64.tar.gz',
    binaryName: 'llama-server',
    content: 'x64 server',
    dependencies: {
      'libllama.0.dylib': 'x64 libllama',
    },
  });

  const prepared = prepareLlamaServerReleaseAssets({
    platform: 'macos',
    assetsDir,
    rootDir: root,
  });

  assert.deepEqual(prepared.map((item) => item.key), ['macos-arm64', 'macos-x64']);
  assert.equal(
    readFileSync(
      resolve(root, 'apps/desktop/src-tauri/bin/apple-silicon/llama-server'),
      'utf8'
    ),
    'arm64 server'
  );
  assert.equal(
    readFileSync(resolve(root, 'apps/desktop/src-tauri/bin/intel/llama-server'), 'utf8'),
    'x64 server'
  );
  assert.equal(
    readFileSync(resolve(root, 'apps/desktop/src-tauri/bin/apple-silicon/libllama.0.dylib'), 'utf8'),
    'arm64 libllama'
  );
  assert.deepEqual(prepared[0].dependencyResources, [
    'bin/apple-silicon/libggml-base.dylib',
    'bin/apple-silicon/libllama.0.dylib',
  ]);
  assert.notEqual(
    statSync(resolve(root, 'apps/desktop/src-tauri/bin/apple-silicon/llama-server')).mode & 0o111,
    0
  );
});

test('prepares Windows CPU x64 release archive', () => {
  const { root, assetsDir } = createFixture();
  createTarArchive({
    assetsDir,
    archiveName: 'llama-b9568-bin-win-cpu-x64.tar.gz',
    binaryName: 'llama-server.exe',
    content: 'windows server',
  });

  const prepared = prepareLlamaServerReleaseAssets({
    platform: 'windows',
    assetsDir,
    rootDir: root,
  });

  assert.deepEqual(prepared.map((item) => item.key), ['windows-x64-cpu']);
  assert.equal(
    readFileSync(resolve(root, 'apps/desktop/src-tauri/bin/windows/llama-server.exe'), 'utf8'),
    'windows server'
  );
});

test('prepares official Windows CPU x64 zip release archive', () => {
  const { root, assetsDir } = createFixture();
  createZipArchive({
    assetsDir,
    archiveName: 'llama-b9568-bin-win-cpu-x64.zip',
    binaryName: 'llama-server.exe',
    content: 'windows zip server',
    dependencies: {
      'llama.dll': 'windows llama dll',
      'ggml.dll': 'windows ggml dll',
    },
  });

  const prepared = prepareLlamaServerReleaseAssets({
    platform: 'windows',
    assetsDir,
    rootDir: root,
  });

  assert.deepEqual(prepared.map((item) => item.key), ['windows-x64-cpu']);
  assert.equal(
    readFileSync(resolve(root, 'apps/desktop/src-tauri/bin/windows/llama-server.exe'), 'utf8'),
    'windows zip server'
  );
  assert.equal(
    readFileSync(resolve(root, 'apps/desktop/src-tauri/bin/windows/llama.dll'), 'utf8'),
    'windows llama dll'
  );
  assert.deepEqual(prepared[0].dependencyResources, [
    'bin/windows/ggml.dll',
    'bin/windows/llama.dll',
  ]);
});

test('fails when a required target archive is missing', () => {
  const { root, assetsDir } = createFixture();

  assert.throws(
    () =>
      prepareLlamaServerReleaseAssets({
        platform: 'macos',
        assetsDir,
        rootDir: root,
      }),
    /Missing llama.cpp release archive for macos-arm64/
  );
});

test('optional mode skips missing target archives', () => {
  const { root, assetsDir } = createFixture();

  assert.deepEqual(
    prepareLlamaServerReleaseAssets({
      platform: 'macos',
      assetsDir,
      rootDir: root,
      required: false,
    }),
    []
  );
});
