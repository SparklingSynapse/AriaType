import test from 'node:test';
import assert from 'node:assert/strict';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const {
  WINDOWS_CROSS_BUILD_COMMAND,
  WINDOWS_CROSS_UNSIGNED_BUILD_COMMAND,
  WINDOWS_NATIVE_BUILD_COMMAND,
  WINDOWS_NATIVE_UNSIGNED_BUILD_COMMAND,
  checkRequiredBuildTools,
  collectReleaseAssetsFromTarget,
  createBundleArtifactPreserver,
  createDmgTraceCommand,
  detachRepoDmgMounts,
  findLastBundledDmgPath,
  findRepoDmgMounts,
  inferDmgVolumeName,
  parseHdiutilMountedImages,
  runCommand,
  windowsCrossBuildEnv,
} = await import('./build-all-platforms-lib.mjs');

test('preserves and restores bundle artifacts after later target cleanups', () => {
  const tempDir = mkdtempSync(join(tmpdir(), 'ariatype-target-preserve-'));
  try {
    const targetDir = join(tempDir, 'target');
    const bundleDmgDir = join(
      targetDir,
      'aarch64-apple-darwin',
      'release',
      'bundle',
      'dmg',
    );
    const bundleMacosDir = join(
      targetDir,
      'aarch64-apple-darwin',
      'release',
      'bundle',
      'macos',
    );
    mkdirSync(bundleDmgDir, { recursive: true });
    mkdirSync(bundleMacosDir, { recursive: true });
    writeFileSync(join(bundleDmgDir, 'AriaType_1.0.4_aarch64.dmg'), 'dmg');
    writeFileSync(join(bundleMacosDir, 'AriaType.app.tar.gz'), 'archive');
    writeFileSync(join(bundleMacosDir, 'AriaType.app.tar.gz.sig'), 'signature');

    const preserver = createBundleArtifactPreserver({
      targetDir,
      cacheDir: join(tempDir, 'cache'),
      log: {
        info() {},
        warn() {},
      },
    });

    assert.equal(preserver.preserve('aarch64-apple-darwin'), true);
    rmSync(join(targetDir, 'aarch64-apple-darwin'), { recursive: true, force: true });

    assert.deepEqual(preserver.restore(), ['aarch64-apple-darwin']);
    assert.equal(
      readFileSync(join(bundleDmgDir, 'AriaType_1.0.4_aarch64.dmg'), 'utf8'),
      'dmg',
    );
    assert.equal(readFileSync(join(bundleMacosDir, 'AriaType.app.tar.gz.sig'), 'utf8'), 'signature');

    preserver.cleanup();
    assert.equal(existsSync(join(tempDir, 'cache')), false);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('collects release assets from restored target bundles with unique updater names', () => {
  const tempDir = mkdtempSync(join(tmpdir(), 'ariatype-release-assets-'));
  try {
    const targetDir = join(tempDir, 'target');
    const releaseDir = join(targetDir, 'release', 'github-release');

    const armBundle = join(targetDir, 'aarch64-apple-darwin', 'release', 'bundle');
    mkdirSync(join(armBundle, 'dmg'), { recursive: true });
    mkdirSync(join(armBundle, 'macos'), { recursive: true });
    writeFileSync(join(armBundle, 'dmg', 'AriaType_1.0.4_aarch64.dmg'), 'arm dmg');
    writeFileSync(join(armBundle, 'macos', 'AriaType.app.tar.gz'), 'arm archive');
    writeFileSync(join(armBundle, 'macos', 'AriaType.app.tar.gz.sig'), 'arm signature');

    const intelBundle = join(targetDir, 'x86_64-apple-darwin', 'release', 'bundle');
    mkdirSync(join(intelBundle, 'dmg'), { recursive: true });
    mkdirSync(join(intelBundle, 'macos'), { recursive: true });
    writeFileSync(join(intelBundle, 'dmg', 'AriaType_1.0.4_x64.dmg'), 'intel dmg');
    writeFileSync(join(intelBundle, 'macos', 'AriaType.app.tar.gz'), 'intel archive');
    writeFileSync(join(intelBundle, 'macos', 'AriaType.app.tar.gz.sig'), 'intel signature');

    const windowsBundle = join(targetDir, 'x86_64-pc-windows-msvc', 'release', 'bundle', 'nsis');
    mkdirSync(windowsBundle, { recursive: true });
    writeFileSync(join(windowsBundle, 'AriaType_1.0.4_x64-setup.exe'), 'setup');
    writeFileSync(join(windowsBundle, 'AriaType_1.0.4_x64-setup.exe.sig'), 'setup signature');

    const copied = collectReleaseAssetsFromTarget({
      targetDir,
      releaseDir,
      version: '1.0.4',
      log: {
        info() {},
        warn() {},
      },
    });

    assert.deepEqual(copied.sort(), [
      'AriaType_1.0.4_aarch64.app.tar.gz',
      'AriaType_1.0.4_aarch64.app.tar.gz.sig',
      'AriaType_1.0.4_aarch64.dmg',
      'AriaType_1.0.4_x64-setup.exe',
      'AriaType_1.0.4_x64-setup.exe.sig',
      'AriaType_1.0.4_x64.app.tar.gz',
      'AriaType_1.0.4_x64.app.tar.gz.sig',
      'AriaType_1.0.4_x64.dmg',
    ]);
    assert.equal(readFileSync(join(releaseDir, 'AriaType_1.0.4_x64.app.tar.gz'), 'utf8'), 'intel archive');
    assert.equal(readFileSync(join(releaseDir, 'AriaType_1.0.4_x64-setup.exe.sig'), 'utf8'), 'setup signature');
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('extracts the last bundled DMG path from a Tauri build log', () => {
  const buildLog = `
    Bundling AriaType_1.0.0_aarch64.dmg (/repo/target/aarch64/release/bundle/dmg/AriaType_1.0.0_aarch64.dmg)
     Running bundle_dmg.sh
    Bundling AriaType_1.0.0_x64.dmg (/repo/target/x86/release/bundle/dmg/AriaType_1.0.0_x64.dmg)
     Running bundle_dmg.sh
`;

  assert.equal(
    findLastBundledDmgPath(buildLog),
    '/repo/target/x86/release/bundle/dmg/AriaType_1.0.0_x64.dmg',
  );
});

test('builds a replayable bundle_dmg trace command with required arguments', () => {
  const command = createDmgTraceCommand({
    scriptPath: '/repo/target/x86/release/bundle/dmg/bundle_dmg.sh',
    dmgPath: '/repo/target/x86/release/bundle/dmg/AriaType_1.0.0_x64.dmg',
    sourceDir: '/repo/target/x86/release/bundle/macos',
    traceDmgPath: '/repo/target/x86/release/bundle/dmg/trace.AriaType_1.0.0_x64.dmg',
    traceLogPath: '/repo/apps/desktop/.build-diagnostics/run/bundle_dmg.trace.log',
    backgroundPath: '/repo/apps/desktop/assets/background.png',
    windowSize: { width: 660, height: 400 },
  });

  assert.match(command, /^set -o pipefail; cd '\/repo\/target\/x86\/release\/bundle\/dmg' && bash -x \.\/bundle_dmg\.sh /);
  assert.match(command, /'--volname' 'AriaType'/);
  assert.match(command, /'--background' '\/repo\/apps\/desktop\/assets\/background\.png'/);
  assert.match(command, /'--window-size' '660' '400'/);
  assert.match(command, /'\/repo\/target\/x86\/release\/bundle\/dmg\/trace\.AriaType_1\.0\.0_x64\.dmg' '\/repo\/target\/x86\/release\/bundle\/macos'/);
  assert.match(command, /2>&1 \| tee '\/repo\/apps\/desktop\/\.build-diagnostics\/run\/bundle_dmg\.trace\.log'$/);
});

test('infers DMG volume names for release artifacts with spaces', () => {
  assert.equal(
    inferDmgVolumeName('/repo/dmg/AriaType Inhouse_0.5.2_x64.dmg'),
    'AriaType Inhouse',
  );
});

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
  const success = runCommand('npm run tauri -- build', 'Building macOS Intel', {
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

  const success = runCommand('npm run tauri -- build', 'Building macOS ARM', {
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

  const success = runCommand('npm run tauri -- build', 'Building macOS ARM', {
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
    "set -o pipefail; (npm run tauri -- build) 2>&1 | tee '/tmp/ariatype build.log'",
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

  assert.match(script, /cargo install cargo-xwin/);
  assert.match(script, /cargo-xwin \(required by Windows cross builds\)/);
  assert.match(script, /brew install ninja llvm nsis/);
  assert.match(script, /Ninja \(required by Windows cargo-xwin builds\)/);
  assert.doesNotMatch(script, /llama-cpp-sys-2/);
});

test('default platform build cross-compiles Windows without an extra npm flag', () => {
  const script = readFileSync(new URL('./build-all-platforms.mjs', import.meta.url), 'utf8');

  assert.match(
    script,
    /const crossWin = args\.includes\('--cross-win'\) \|\| \(!skipWin && !isWindows\);/,
  );
  assert.match(script, /const canCrossCompile = crossWin && !isWindows;/);
});

test('root npm build runs full signed release pipeline then website build', () => {
  const rootPackage = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
  const orchestrator = readFileSync(new URL('./build-all.mjs', import.meta.url), 'utf8');

  assert.equal(rootPackage.scripts.build, 'node scripts/build-all.mjs');
  assert.equal(rootPackage.scripts['build:all'], 'node scripts/build-all.mjs');

  const desktopBuildIndex = orchestrator.indexOf("run('node', ['scripts/build-all-platforms.mjs', ...args]");
  const websiteBuildIndex = orchestrator.indexOf("run('npm', ['run', 'build:website']");
  assert.ok(desktopBuildIndex >= 0);
  assert.ok(websiteBuildIndex > desktopBuildIndex);
});

test('windows cross-build command uses the dedicated Windows Tauri config', () => {
  assert.equal(
    WINDOWS_NATIVE_BUILD_COMMAND,
    'node ../../scripts/ensure-llama-server-runtime.mjs --platform windows && node ../../scripts/prepare-tauri-runtime-resources.mjs --platform windows --require-runtime && node ../../scripts/run-tauri-build-with-updater-signing.mjs -- npm run tauri -- build --config src-tauri/tauri.windows.conf.json --config src-tauri/tauri.updater.conf.json --config src-tauri/tauri.runtime.generated.conf.json --target x86_64-pc-windows-msvc',
  );
  assert.equal(
    WINDOWS_CROSS_BUILD_COMMAND,
    'node ../../scripts/ensure-llama-server-runtime.mjs --platform windows && node ../../scripts/prepare-tauri-runtime-resources.mjs --platform windows --require-runtime && node ../../scripts/run-tauri-build-with-updater-signing.mjs -- cargo tauri build --config src-tauri/tauri.windows.conf.json --config src-tauri/tauri.updater.conf.json --config src-tauri/tauri.runtime.generated.conf.json --runner cargo-xwin --target x86_64-pc-windows-msvc',
  );
  assert.equal(
    WINDOWS_NATIVE_UNSIGNED_BUILD_COMMAND,
    'node ../../scripts/ensure-llama-server-runtime.mjs --platform windows && node ../../scripts/prepare-tauri-runtime-resources.mjs --platform windows --require-runtime && npm run tauri -- build --config src-tauri/tauri.windows.conf.json --config src-tauri/tauri.runtime.generated.conf.json --target x86_64-pc-windows-msvc',
  );
  assert.equal(
    WINDOWS_CROSS_UNSIGNED_BUILD_COMMAND,
    'node ../../scripts/ensure-llama-server-runtime.mjs --platform windows && node ../../scripts/prepare-tauri-runtime-resources.mjs --platform windows --require-runtime && cargo tauri build --config src-tauri/tauri.windows.conf.json --config src-tauri/tauri.runtime.generated.conf.json --runner cargo-xwin --target x86_64-pc-windows-msvc',
  );
});

test('platform build commands merge generated runtime resources config', () => {
  const script = readFileSync(new URL('./build-all-platforms.mjs', import.meta.url), 'utf8');
  const sharedScript = readFileSync(new URL('./build-all-platforms-lib.mjs', import.meta.url), 'utf8');

  assert.match(script, /prepare-tauri-runtime-resources\.mjs --platform macos --require-runtime/);
  assert.match(sharedScript, /ensure-llama-server-runtime\.mjs --platform windows/);
  assert.match(sharedScript, /prepare-tauri-runtime-resources\.mjs --platform windows --require-runtime/);
  assert.match(script, /detachRepoDmgMounts\(\{ repoRoot: root \}\);/);
  assert.match(script, /node \.\.\/\.\.\/scripts\/sign-macos-binaries\.mjs && \$\{signedTauriBuildCommand\} \$\{tauriBuildCommand\} --config src-tauri\/tauri\.macos\.conf\.json --config \$\{updaterConfig\} --config \$\{runtimeConfig\} --target aarch64-apple-darwin/);
  assert.match(script, /node \.\.\/\.\.\/scripts\/sign-macos-binaries\.mjs && \$\{signedTauriBuildCommand\} \$\{tauriBuildCommand\} --config src-tauri\/tauri\.macos\.conf\.json --config \$\{updaterConfig\} --config \$\{runtimeConfig\} --target x86_64-apple-darwin/);
  assert.match(sharedScript, /tauri\.windows\.conf\.json --config \$\{UPDATER_CONFIG\} --config src-tauri\/tauri\.runtime\.generated\.conf\.json/);
  assert.match(sharedScript, /tauri\.windows\.conf\.json --config src-tauri\/tauri\.runtime\.generated\.conf\.json/);
  assert.doesNotMatch(script, /pnpm tauri build/);
  assert.doesNotMatch(sharedScript, /pnpm tauri build/);
});

test('multi-platform build does not copy installers into the website public release folder', () => {
  const script = readFileSync(new URL('./build-all-platforms.mjs', import.meta.url), 'utf8');
  const packageJson = JSON.parse(
    readFileSync(new URL('../apps/desktop/package.json', import.meta.url), 'utf8')
  );

  assert.doesNotMatch(script, /copy-installer/);
  assert.equal(packageJson.scripts['copy-installer'], undefined);
  for (const [name, command] of Object.entries(packageJson.scripts)) {
    if (name.startsWith('tauri:build:')) {
      assert.doesNotMatch(command, /copy-installer/);
    }
  }
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
image-path      : /repo/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/AriaType_0.6.4_aarch64.dmg
/dev/disk8\tGUID_partition_scheme\t
/dev/disk8s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType
================================================
image-path      : /Users/me/Downloads/Other.dmg
/dev/disk9s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/Other
`);

  assert.deepEqual(parsed, [
    {
      imagePath: '/repo/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/AriaType_0.6.4_aarch64.dmg',
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
image-path      : /repo/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/AriaType_0.6.4_aarch64.dmg
/dev/disk8s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType
================================================
image-path      : /repo/apps/desktop/src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/AriaType_0.6.4_x64.dmg
/dev/disk9s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType 1
================================================
image-path      : /repo/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/AriaType Inhouse_0.6.5_aarch64.dmg
/dev/disk10s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType Inhouse
================================================
image-path      : /Users/me/Downloads/AriaType_0.6.5_aarch64.dmg
/dev/disk11s1\t48465300-0000-11AA-AA11-00306543ECAC\t/Volumes/AriaType 2
================================================
image-path      : /repo/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/Polywise.dmg
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
image-path      : /repo/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/AriaType_0.6.4_aarch64.dmg
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
