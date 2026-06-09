import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const {
  WINDOWS_CROSS_BUILD_COMMAND,
  checkRequiredBuildTools,
  detachRepoDmgMounts,
  findRepoDmgMounts,
  parseHdiutilMountedImages,
  runCommand,
  windowsCrossBuildEnv,
} = await import('./build-all-platforms-lib.mjs');

test('retries once when notarization upload times out', () => {
  let attempts = 0;

  const exec = () => {
    attempts += 1;
    if (attempts === 1) {
      throw new Error(
        'failed to notarize app: Error: abortedUpload(error: HTTPClientError.deadlineExceeded)'
      );
    }
  };

  const logs = [];
  const success = runCommand('pnpm tauri build', 'Building macOS Intel', {
    exec,
    log: {
      info(message) {
        logs.push(message);
      },
      error(message) {
        logs.push(message);
      },
      warn(message) {
        logs.push(message);
      },
    },
    maxAttempts: 2,
  });

  assert.equal(success, true);
  assert.equal(attempts, 2);
  assert.ok(logs.some((message) => message.includes('Retrying after notarization upload timeout')));
});

test('does not retry unrelated build failures', () => {
  let attempts = 0;
  const logs = [];

  const exec = () => {
    attempts += 1;
    throw new Error('cargo build failed');
  };

  const success = runCommand('cargo build', 'Building macOS Intel', {
    exec,
    log: {
      info(message) {
        logs.push(message);
      },
      error(message) {
        logs.push(message);
      },
      warn(message) {
        logs.push(message);
      },
    },
    maxAttempts: 2,
  });

  assert.equal(success, false);
  assert.equal(attempts, 1);
});

test('calls failure hook when final attempt fails', () => {
  const failure = new Error('bundle_dmg.sh failed');
  let observedError;

  const success = runCommand('pnpm tauri build', 'Building macOS ARM', {
    exec() {
      throw failure;
    },
    log: {
      info() {},
      error() {},
      warn() {},
    },
    onFailure(error) {
      observedError = error;
    },
  });

  assert.equal(success, false);
  assert.equal(observedError, failure);
});

test('mirrors command output to a build log when requested', () => {
  let observedCommand;
  let observedOptions;

  const success = runCommand('pnpm tauri build', 'Building macOS ARM', {
    exec(command, options) {
      observedCommand = command;
      observedOptions = options;
    },
    log: {
      info() {},
      error() {},
      warn() {},
    },
    logFile: '/tmp/ariatype build.log',
  });

  assert.equal(success, true);
  assert.equal(
    observedCommand,
    "set -o pipefail; (pnpm tauri build) 2>&1 | tee '/tmp/ariatype build.log'",
  );
  assert.equal(observedOptions.shell, '/bin/bash');
  assert.equal(observedOptions.stdio, 'inherit');
});

test('preflight fails fast when a required build tool is missing', () => {
  const logs = [];
  const success = checkRequiredBuildTools(
    [
      {
        command: 'cmake',
        description: 'CMake',
        installHint: 'brew install cmake',
      },
    ],
    {
      exec() {
        throw new Error('not found');
      },
      log: {
        info(message) {
          logs.push(message);
        },
        error(message) {
          logs.push(message);
        },
      },
    },
  );

  assert.equal(success, false);
  assert.ok(logs.some((message) => message.includes('Missing required build tool: CMake')));
  assert.ok(logs.some((message) => message.includes('brew install cmake')));
});

test('preflight passes when all required build tools exist', () => {
  let checks = 0;
  const success = checkRequiredBuildTools(
    [
      {
        command: 'cmake',
        description: 'CMake',
        installHint: 'brew install cmake',
      },
    ],
    {
      exec() {
        checks += 1;
      },
      log: {
        info() {},
        error() {},
      },
    },
  );

  assert.equal(success, true);
  assert.equal(checks, 1);
});

test('windows cross-build preflight documents ninja as a required tool', () => {
  const script = readFileSync(new URL('./build-all-platforms.mjs', import.meta.url), 'utf8');

  assert.match(script, /brew install ninja llvm nsis/);
  assert.match(script, /Ninja \(required by Windows cargo-xwin builds\)/);
  assert.doesNotMatch(script, /llama-cpp-sys-2/);
});

test('windows cross-build command uses the dedicated Windows Tauri config', () => {
  assert.equal(
    WINDOWS_CROSS_BUILD_COMMAND,
    'node ../../scripts/prepare-tauri-runtime-resources.mjs --platform windows --require-runtime && cargo tauri build --config src-tauri/tauri.windows.conf.json --config src-tauri/tauri.runtime.generated.conf.json --runner cargo-xwin --target x86_64-pc-windows-msvc',
  );
});

test('platform build commands merge generated runtime resources config', () => {
  const script = readFileSync(new URL('./build-all-platforms.mjs', import.meta.url), 'utf8');

  assert.match(script, /prepare-tauri-runtime-resources\.mjs --platform macos --require-runtime/);
  assert.match(script, /prepare-tauri-runtime-resources\.mjs --platform windows --require-runtime/);
  assert.match(script, /detachRepoDmgMounts\(\{ repoRoot: root \}\);/);
  assert.match(script, /--config \$\{runtimeConfig\} --target aarch64-apple-darwin/);
  assert.match(script, /--config \$\{runtimeConfig\} --target x86_64-apple-darwin/);
  assert.match(script, /tauri\.windows\.conf\.json --config \$\{runtimeConfig\}/);
});

test('windows cross-build env preserves existing env and enables static CRT flags', () => {
  const env = windowsCrossBuildEnv({
    PATH: '/usr/bin',
    RUSTFLAGS: '-Clink-arg=/DEBUG',
  });

  assert.equal(env.PATH, '/usr/bin');
  assert.equal(env.LLAMA_STATIC_CRT, '1');
  assert.equal(env.STATIC_VCRUNTIME, 'false');
  assert.equal(env.RUSTFLAGS, '-Clink-arg=/DEBUG -Ctarget-feature=+crt-static');
});

test('windows cross-build env does not duplicate crt-static rustflag', () => {
  const env = windowsCrossBuildEnv({
    RUSTFLAGS: '-Ctarget-feature=+crt-static',
  });

  assert.equal(env.RUSTFLAGS, '-Ctarget-feature=+crt-static');
});

test('parses hdiutil mounted images with volume mount points', () => {
  const parsed = parseHdiutilMountedImages(`
================================================
image-path      : /repo/packages/website/public/release/AriaType_0.6.4_aarch64.dmg
/dev/disk8\tGUID_partition_scheme\t
/dev/disk8s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType
================================================
image-path      : /Users/me/Downloads/Other.dmg
/dev/disk9s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/Other
`);

  assert.deepEqual(parsed, [
    {
      imagePath: '/repo/packages/website/public/release/AriaType_0.6.4_aarch64.dmg',
      mountPoints: ['/Volumes/AriaType'],
    },
    {
      imagePath: '/Users/me/Downloads/Other.dmg',
      mountPoints: ['/Volumes/Other'],
    },
  ]);
});

test('finds only stale AriaType dmg mounts inside the repo', () => {
  const info = `
image-path      : /repo/packages/website/public/release/AriaType_0.6.4_aarch64.dmg
/dev/disk8s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType
================================================
image-path      : /repo/packages/website/public/release/AriaType_0.6.4_aarch64.dmg
/dev/disk9s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType 1
================================================
image-path      : /repo/packages/website/public/release/AriaType Inhouse_0.6.5_aarch64.dmg
/dev/disk10s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType Inhouse
================================================
image-path      : /Users/me/Downloads/AriaType_0.6.5_aarch64.dmg
/dev/disk11s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType 2
================================================
image-path      : /repo/packages/website/public/release/Polywise.dmg
/dev/disk12s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/Polywise
`;

  const mounts = findRepoDmgMounts(info, {
    repoRoot: '/repo',
    volumeNames: ['AriaType', 'AriaType Inhouse'],
  });

  assert.deepEqual(
    mounts.map((mount) => mount.mountPoint),
    ['/Volumes/AriaType', '/Volumes/AriaType 1', '/Volumes/AriaType Inhouse']
  );
});

test('detaches stale repo dmg mounts before mac packaging', () => {
  const commands = [];
  const warnings = [];
  const info = `
image-path      : /repo/packages/website/public/release/AriaType_0.6.4_aarch64.dmg
/dev/disk8s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType 1
`;

  const detached = detachRepoDmgMounts({
    repoRoot: '/repo',
    exec(command) {
      commands.push(command);
      if (command === 'hdiutil info') {
        return info;
      }
      return '';
    },
    log: {
      warn(message) {
        warnings.push(message);
      },
    },
  });

  assert.deepEqual(commands, ['hdiutil info', "hdiutil detach '/Volumes/AriaType 1'"]);
  assert.equal(detached.length, 1);
  assert.ok(warnings[0].includes('Detaching stale AriaType DMG mount'));
});
