import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
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
  existingRuntimeResources,
  prepareRuntimeSidecarArtifact,
  prepareRuntimeSidecarArtifacts,
  runtimeResourceConfig,
  runtimeResourceDestination,
  runtimeResourceDirs,
  runtimeSidecarSpecs,
  writeRuntimeResourceConfig,
} from './prepare-tauri-runtime-resources.mjs';

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function createFixture() {
  const root = mkdtempSync(join(tmpdir(), 'ariatype-runtime-resources-'));
  const tauriDir = resolve(root, 'apps/desktop/src-tauri');
  mkdirSync(tauriDir, { recursive: true });

  writeJson(resolve(tauriDir, 'tauri.conf.json'), {
    bundle: {
      resources: [
        'bin/apple-silicon/sense-voice-main-aarch64-apple-darwin',
        'assets/start_beep.wav',
      ],
    },
  });
  writeJson(resolve(tauriDir, 'tauri.windows.conf.json'), {
    bundle: {
      resources: [
        'bin/windows/sense-voice-main-x86_64-pc-windows.exe',
        'assets/start_beep.wav',
      ],
    },
  });

  return { root, tauriDir };
}

function sha256(text) {
  return createHash('sha256').update(text).digest('hex');
}

test('mac runtime config preserves base resources and adds existing sidecar resources', () => {
  const { root, tauriDir } = createFixture();
  const sidecarPath = resolve(tauriDir, 'bin/universal/llama-server');
  mkdirSync(resolve(sidecarPath, '..'), { recursive: true });
  writeFileSync(sidecarPath, '');
  writeFileSync(resolve(tauriDir, 'bin/universal/libllama.0.dylib'), '');
  writeFileSync(resolve(tauriDir, 'bin/universal/ggml-metal.metal'), '');
  writeFileSync(resolve(tauriDir, 'bin/universal/ggml-common.h'), '');
  writeFileSync(resolve(tauriDir, 'bin/universal/sense-voice-main'), '');

  assert.deepEqual(existingRuntimeResources('macos', root), [
    'bin/universal/llama-server',
    'bin/universal/ggml-common.h',
    'bin/universal/ggml-metal.metal',
    'bin/universal/libllama.0.dylib',
  ]);

  const config = runtimeResourceConfig('macos', root);
  assert.deepEqual(config.bundle.resources, [
    'bin/apple-silicon/sense-voice-main-aarch64-apple-darwin',
    'assets/start_beep.wav',
    'bin/universal/llama-server',
    'bin/universal/ggml-common.h',
    'bin/universal/ggml-metal.metal',
    'bin/universal/libllama.0.dylib',
  ]);
});

test('windows runtime config uses windows resources and skips missing sidecars', () => {
  const { root } = createFixture();

  const config = runtimeResourceConfig('windows', root);
  assert.deepEqual(config.bundle.resources, [
    'bin/windows/sense-voice-main-x86_64-pc-windows.exe',
    'assets/start_beep.wav',
  ]);
});

test('sidecar preparation is a no-op when no source is configured', () => {
  const { root } = createFixture();

  const result = prepareRuntimeSidecarArtifact({
    platform: 'windows',
    rootDir: root,
    sourcePath: '',
  });

  assert.equal(result.copied, false);
  assert.equal(result.reason, 'source_not_configured');
  assert.equal(result.destinationResource, runtimeResourceDestination('windows'));
});

test('sidecar preparation can require a configured source', () => {
  const { root } = createFixture();

  assert.throws(
    () =>
      prepareRuntimeSidecarArtifact({
        platform: 'macos',
        rootDir: root,
        sourcePath: '',
        required: true,
      }),
    /Missing local polish runtime artifact for macos/
  );
});

test('sidecar specs expose arch-specific macOS artifact slots', () => {
  assert.deepEqual(
    runtimeSidecarSpecs('macos').map((spec) => [
      spec.pathEnv,
      spec.destinationResource,
    ]),
    [
      ['ARIATYPE_LLAMA_SERVER_MACOS_ARM64_PATH', 'bin/apple-silicon/llama-server'],
      ['ARIATYPE_LLAMA_SERVER_MACOS_X64_PATH', 'bin/intel/llama-server'],
      ['ARIATYPE_LLAMA_SERVER_MACOS_PATH', 'bin/universal/llama-server'],
    ]
  );
});

test('runtime resource dirs expose dependency packaging locations', () => {
  assert.deepEqual(runtimeResourceDirs('windows'), ['bin/windows']);
  assert.ok(runtimeResourceDirs('macos').includes('bin/apple-silicon'));
});

test('sidecar preparation copies arch-specific macOS artifacts', () => {
  const { root, tauriDir } = createFixture();
  const arm64Source = resolve(root, 'runtime/llama-server-arm64');
  const x64Source = resolve(root, 'runtime/llama-server-x64');
  mkdirSync(dirname(arm64Source), { recursive: true });
  writeFileSync(arm64Source, 'arm64');
  writeFileSync(x64Source, 'x64');

  const results = prepareRuntimeSidecarArtifacts({
    platform: 'macos',
    rootDir: root,
    env: {
      ARIATYPE_LLAMA_SERVER_MACOS_ARM64_PATH: arm64Source,
      ARIATYPE_LLAMA_SERVER_MACOS_ARM64_SHA256: sha256('arm64'),
      ARIATYPE_LLAMA_SERVER_MACOS_X64_PATH: x64Source,
      ARIATYPE_LLAMA_SERVER_MACOS_X64_SHA256: sha256('x64'),
    },
  });

  assert.equal(results.length, 2);
  assert.deepEqual(results.map((result) => result.destinationResource), [
    'bin/apple-silicon/llama-server',
    'bin/intel/llama-server',
  ]);
  assert.equal(
    readFileSync(resolve(tauriDir, 'bin/apple-silicon/llama-server'), 'utf8'),
    'arm64'
  );
  assert.equal(readFileSync(resolve(tauriDir, 'bin/intel/llama-server'), 'utf8'), 'x64');
});

test('sidecar preparation prefers Windows x64 artifact over generic Windows artifact', () => {
  const { root, tauriDir } = createFixture();
  const x64Source = resolve(root, 'runtime/llama-server-x64.exe');
  const genericSource = resolve(root, 'runtime/llama-server.exe');
  mkdirSync(dirname(x64Source), { recursive: true });
  writeFileSync(x64Source, 'x64');
  writeFileSync(genericSource, 'generic');

  const results = prepareRuntimeSidecarArtifacts({
    platform: 'windows',
    rootDir: root,
    env: {
      ARIATYPE_LLAMA_SERVER_WINDOWS_X64_PATH: x64Source,
      ARIATYPE_LLAMA_SERVER_WINDOWS_PATH: genericSource,
    },
  });

  assert.equal(results.length, 1);
  assert.equal(results[0].pathEnv, 'ARIATYPE_LLAMA_SERVER_WINDOWS_X64_PATH');
  assert.equal(
    readFileSync(resolve(tauriDir, 'bin/windows/llama-server.exe'), 'utf8'),
    'x64'
  );
});

test('required sidecar preparation names accepted environment variables', () => {
  const { root } = createFixture();

  assert.throws(
    () =>
      prepareRuntimeSidecarArtifacts({
        platform: 'windows',
        rootDir: root,
        env: {},
        required: true,
      }),
    /ARIATYPE_LLAMA_SERVER_WINDOWS_X64_PATH/
  );
});

test('required sidecar preparation accepts already-present runtime resources', () => {
  const { root, tauriDir } = createFixture();
  const sidecarPath = resolve(tauriDir, 'bin/windows/llama-server.exe');
  mkdirSync(dirname(sidecarPath), { recursive: true });
  writeFileSync(sidecarPath, 'existing');

  const results = prepareRuntimeSidecarArtifacts({
    platform: 'windows',
    rootDir: root,
    env: {},
    required: true,
  });

  assert.deepEqual(results, [
    {
      copied: false,
      reason: 'already_present',
      destinationResource: 'bin/windows/llama-server.exe',
      destinationPath: sidecarPath,
    },
  ]);
});

test('sidecar preparation copies and verifies a provided artifact', () => {
  const { root, tauriDir } = createFixture();
  const sourcePath = resolve(root, 'runtime/llama-server');
  mkdirSync(dirname(sourcePath), { recursive: true });
  writeFileSync(sourcePath, 'fake llama server');

  const result = prepareRuntimeSidecarArtifact({
    platform: 'macos',
    rootDir: root,
    sourcePath,
    expectedSha256: sha256('fake llama server'),
  });
  const destinationPath = resolve(tauriDir, runtimeResourceDestination('macos'));

  assert.equal(result.copied, true);
  assert.equal(result.destinationPath, destinationPath);
  assert.equal(readFileSync(destinationPath, 'utf8'), 'fake llama server');
  assert.notEqual(statSync(destinationPath).mode & 0o111, 0);
  assert.deepEqual(existingRuntimeResources('macos', root), [
    'bin/universal/llama-server',
  ]);
});

test('sidecar preparation rejects checksum mismatches', () => {
  const { root } = createFixture();
  const sourcePath = resolve(root, 'runtime/llama-server');
  mkdirSync(dirname(sourcePath), { recursive: true });
  writeFileSync(sourcePath, 'fake llama server');

  assert.throws(
    () =>
      prepareRuntimeSidecarArtifact({
        platform: 'macos',
        rootDir: root,
        sourcePath,
        expectedSha256: '0'.repeat(64),
      }),
    /sha256 mismatch/
  );
});

test('writes generated runtime config to the requested path', () => {
  const { root } = createFixture();
  const outputPath = resolve(root, 'apps/desktop/src-tauri/tauri.runtime.generated.conf.json');

  const { outputPath: writtenPath } = writeRuntimeResourceConfig({
    platform: 'windows',
    rootDir: root,
    outputPath,
    sidecarSourcePath: '',
  });

  assert.equal(writtenPath, outputPath);
  const written = JSON.parse(readFileSync(outputPath, 'utf8'));
  assert.deepEqual(written.bundle.resources, [
    'bin/windows/sense-voice-main-x86_64-pc-windows.exe',
    'assets/start_beep.wav',
  ]);
});
