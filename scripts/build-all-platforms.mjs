#!/usr/bin/env node
/**
 * Build AriaType for all platforms: macOS (ARM + Intel) and Windows.
 *
 * Root release build:
 *   npm run build                     # Desktop all platforms, then website
 *
 * Usage:
 *   pnpm build:all                    # Build desktop for all supported platforms
 *   pnpm build:all --skip-mac-arm     # Skip macOS ARM
 *   pnpm build:all --skip-mac-intel   # Skip macOS Intel
 *   pnpm build:all --skip-win         # Skip Windows
 *   pnpm build:all --unsigned         # Build unsigned (no signing)
 *   pnpm build:all --cross-win        # Explicitly cross-compile Windows from macOS/Linux
 *
 * Cross-compilation notes:
 *   - Windows builds require either:
 *     a) Running on Windows (native)
 *     b) Non-Windows host with cargo-xwin installed (default unless --skip-win)
 *   - Cross-compilation requirements:
 *     brew install ninja llvm nsis
 *     cargo install cargo-xwin
 *     rustup target add x86_64-pc-windows-msvc
 */

import { basename, dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'fs';
import { platform } from 'os';
import { execFileSync, execSync } from 'child_process';
import {
  WINDOWS_NATIVE_BUILD_COMMAND,
  WINDOWS_CROSS_BUILD_COMMAND,
  WINDOWS_CROSS_UNSIGNED_BUILD_COMMAND,
  checkRequiredBuildTools,
  createDmgTraceCommand,
  detachRepoDmgMounts,
  findLastBundledDmgPath,
  runCommand,
  WINDOWS_NATIVE_UNSIGNED_BUILD_COMMAND,
  collectReleaseAssetsFromTarget,
  createBundleArtifactPreserver,
  windowsCrossBuildEnv,
} from './build-all-platforms-lib.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const args = process.argv.slice(2);
const skipMacArm = args.includes('--skip-mac-arm');
const skipMacIntel = args.includes('--skip-mac-intel');
const skipWin = args.includes('--skip-win');
const unsigned = args.includes('--unsigned');

const hostPlatform = platform();
const isMacOS = hostPlatform === 'darwin';
const isWindows = hostPlatform === 'win32';
const crossWin = args.includes('--cross-win') || (!skipWin && !isWindows);

const canCrossCompile = crossWin && !isWindows;
const autoSkipWin = skipWin || (!isWindows && !crossWin);
const autoSkipMacArm = skipMacArm || !isMacOS;
const autoSkipMacIntel = skipMacIntel || !isMacOS;

const desktopDir = resolve(root, 'apps/desktop');
const tauriTargetDir = resolve(desktopDir, 'src-tauri/target');
const githubReleaseDir = resolve(tauriTargetDir, 'release/github-release');
const buildDiagnosticsDir = resolve(desktopDir, '.build-diagnostics');
const runtimeConfig = 'src-tauri/tauri.runtime.generated.conf.json';
const updaterConfig = 'src-tauri/tauri.updater.conf.json';
const signedTauriBuildCommand = 'node ../../scripts/run-tauri-build-with-updater-signing.mjs --';
const tauriBuildCommand = 'npm run tauri -- build';

function ninjaInstallHint() {
  if (isMacOS) {
    return 'brew install ninja';
  }
  if (isWindows) {
    return 'winget install Ninja-build.Ninja';
  }
  return 'Install ninja with your system package manager, for example: sudo apt-get install ninja-build';
}

function requiredBuildTools() {
  if (autoSkipMacArm && autoSkipMacIntel && autoSkipWin) {
    return [];
  }

  const tools = [];

  if (canCrossCompile) {
    tools.push({
      command: 'cargo xwin',
      description: 'cargo-xwin (required by Windows cross builds)',
      installHint: 'cargo install cargo-xwin',
    });
    tools.push({
      command: 'ninja',
      description: 'Ninja (required by Windows cargo-xwin builds)',
      installHint: ninjaInstallHint(),
    });
  }

  return tools;
}

function cleanTarget(targetTriple) {
  console.log(`\n🧹 Cleaning ${targetTriple || 'all'} build artifacts...`);
  
  const pathsToClean = [];
  
  if (targetTriple) {
    // Clean specific target triple directory
    pathsToClean.push(resolve(tauriTargetDir, targetTriple));
    // Also clean build directory for this target
    pathsToClean.push(resolve(tauriTargetDir, 'release/build'));
  } else {
    // Clean entire target directory
    pathsToClean.push(tauriTargetDir);
  }
  
  for (const path of pathsToClean) {
    if (existsSync(path)) {
      try {
        rmSync(path, { recursive: true, force: true });
        console.log(`   Removed: ${path}`);
      } catch (err) {
        console.warn(`   Warning: Could not remove ${path}: ${err.message}`);
      }
    }
  }
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function safeExecText(command, options = {}) {
  try {
    return execSync(command, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      ...options,
    });
  } catch (error) {
    const stdout = typeof error.stdout === 'string' ? error.stdout : '';
    const stderr = typeof error.stderr === 'string' ? error.stderr : '';
    return `${stdout}${stderr}` || error.message;
  }
}

function createDiagnosticDir(targetTriple) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const diagnosticsDir = resolve(buildDiagnosticsDir, `${timestamp}-${targetTriple}`);
  mkdirSync(diagnosticsDir, { recursive: true });
  return diagnosticsDir;
}

function writeDiagnosticFile(diagnosticsDir, name, content) {
  const text = String(content);
  writeFileSync(resolve(diagnosticsDir, name), text.endsWith('\n') ? text : `${text}\n`);
}

function desktopVersion() {
  return JSON.parse(readFileSync(resolve(desktopDir, 'package.json'), 'utf8')).version;
}

function requiredUpdaterPlatforms(results) {
  const required = [];
  for (const result of results) {
    if (!result.success) continue;
    if (result.platform === 'macOS ARM (Apple Silicon)') required.push('darwin-aarch64');
    if (result.platform === 'macOS Intel (x64)') required.push('darwin-x86_64');
    if (result.platform === 'Windows') required.push('windows-x86_64');
  }
  return required;
}

function generateGithubReleaseAssets(results) {
  const version = desktopVersion();
  const copiedAssets = collectReleaseAssetsFromTarget({
    targetDir: tauriTargetDir,
    releaseDir: githubReleaseDir,
    version,
  });
  const requiredPlatforms = requiredUpdaterPlatforms(results);
  const args = [
    'scripts/generate-release-manifests.mjs',
    '--release-dir',
    githubReleaseDir,
    '--version',
    version,
    '--base-url',
    `https://github.com/joe223/AriaType/releases/download/v${version}`,
    '--require-updater',
  ];
  for (const platform of requiredPlatforms) {
    args.push('--require-updater-platform', platform);
  }

  execFileSync(process.execPath, args, { cwd: root, stdio: 'inherit' });
  return copiedAssets;
}

function prepareMacBuildDiagnostics(targetTriple, command) {
  const diagnosticsDir = createDiagnosticDir(targetTriple);
  writeDiagnosticFile(diagnosticsDir, 'command.txt', command);
  console.log(`   Build log: ${resolve(diagnosticsDir, 'build.log')}`);
  return diagnosticsDir;
}

function collectMacDmgDiagnostics(targetTriple, error, diagnosticsDir = createDiagnosticDir(targetTriple)) {
  const bundleDir = resolve(tauriTargetDir, targetTriple, 'release/bundle');
  const dmgDir = resolve(tauriTargetDir, targetTriple, 'release/bundle/dmg');
  const scriptPath = resolve(dmgDir, 'bundle_dmg.sh');
  const buildLogPath = resolve(diagnosticsDir, 'build.log');
  const buildLog = existsSync(buildLogPath) ? readFileSync(buildLogPath, 'utf8') : '';
  const bundledDmgPath = findLastBundledDmgPath(buildLog);
  const sourceDir = resolve(bundleDir, 'macos');
  const traceLogPath = resolve(diagnosticsDir, 'bundle_dmg.trace.log');
  const backgroundPath = resolve(desktopDir, 'assets/background.png');
  const traceDmgPath = bundledDmgPath
    ? resolve(dmgDir, `trace.${process.pid}.${basename(bundledDmgPath)}`)
    : undefined;

  writeDiagnosticFile(
    diagnosticsDir,
    'failure.txt',
    [
      `target=${targetTriple}`,
      `error=${error instanceof Error ? error.message : String(error)}`,
      `bundleDir=${bundleDir}`,
      `dmgDir=${dmgDir}`,
      `bundleScript=${scriptPath}`,
      `bundledDmg=${bundledDmgPath ?? ''}`,
    ].join('\n'),
  );

  writeDiagnosticFile(
    diagnosticsDir,
    'bundle-dir.find.txt',
    existsSync(bundleDir)
      ? safeExecText(`find ${shellQuote(bundleDir)} -maxdepth 5 -print`)
      : `Missing bundle directory: ${bundleDir}`,
  );
  writeDiagnosticFile(
    diagnosticsDir,
    'dmg-dir.find.txt',
    existsSync(dmgDir)
      ? safeExecText(`find ${shellQuote(dmgDir)} -maxdepth 4 -print`)
      : `Missing dmg directory: ${dmgDir}`,
  );
  writeDiagnosticFile(diagnosticsDir, 'hdiutil-info.txt', safeExecText('hdiutil info'));
  writeDiagnosticFile(diagnosticsDir, 'volumes.txt', safeExecText('ls -la /Volumes'));
  writeDiagnosticFile(diagnosticsDir, 'df.txt', safeExecText('df -h'));

  if (existsSync(dmgDir)) {
    try {
      cpSync(dmgDir, resolve(diagnosticsDir, 'dmg'), { recursive: true, force: true });
    } catch (copyError) {
      writeDiagnosticFile(
        diagnosticsDir,
        'copy-error.txt',
        copyError instanceof Error ? copyError.message : String(copyError),
      );
    }
  }

  let rerunCommand = `bundle_dmg.sh was not found at ${scriptPath}`;
  let canTraceDmg = false;

  if (existsSync(scriptPath) && bundledDmgPath && existsSync(sourceDir) && traceDmgPath) {
    canTraceDmg = true;
    rerunCommand = createDmgTraceCommand({
      scriptPath,
      dmgPath: bundledDmgPath,
      sourceDir,
      traceDmgPath,
      traceLogPath,
      backgroundPath: existsSync(backgroundPath) ? backgroundPath : undefined,
      windowSize: { width: 660, height: 400 },
    });
  } else if (!bundledDmgPath) {
    rerunCommand = `Could not infer the DMG path from ${buildLogPath}`;
  } else if (!existsSync(sourceDir)) {
    rerunCommand = `DMG source directory was not found at ${sourceDir}`;
  }

  writeDiagnosticFile(diagnosticsDir, 'rerun-command.sh', `#!/usr/bin/env bash\n${rerunCommand}`);

  if (canTraceDmg) {
    console.error('\n   Capturing bundle_dmg.sh shell trace...');
    const traceOutput = safeExecText(rerunCommand, { shell: '/bin/bash' });
    if (!existsSync(traceLogPath)) {
      writeDiagnosticFile(diagnosticsDir, 'bundle_dmg.trace.log', traceOutput);
    }
    if (traceDmgPath && existsSync(traceDmgPath)) {
      rmSync(traceDmgPath, { force: true });
    }
  }

  const reachedDmgBundling = Boolean(bundledDmgPath);
  const keyFiles = [
    'build.log',
    'failure.txt',
    'bundle-dir.find.txt',
    'dmg-dir.find.txt',
    'hdiutil-info.txt',
    'volumes.txt',
    'df.txt',
  ];
  if (reachedDmgBundling) {
    keyFiles.push('bundle_dmg.trace.log');
  }

  console.error(
    reachedDmgBundling
      ? '\n🔎 macOS DMG failure diagnostics written to:'
      : '\n🔎 macOS build failure diagnostics written to:'
  );
  console.error(`   ${diagnosticsDir}`);
  console.error(`   Key files: ${keyFiles.join(', ')}`);
  if (reachedDmgBundling) {
    console.error('\n   To capture shell tracing before the next clean, run:');
    console.error(`   ${rerunCommand}\n`);
  } else {
    console.error('\n   DMG bundling was not reached; inspect build.log for the compile/package failure.\n');
  }
}

console.log('\n🚀 AriaType Multi-Platform Build\n');
console.log(`   Host platform: ${isMacOS ? 'macOS' : isWindows ? 'Windows' : hostPlatform}\n`);

if (!checkRequiredBuildTools(requiredBuildTools())) {
  process.exit(1);
}

const results = [];
const bundleArtifactPreserver = createBundleArtifactPreserver({ targetDir: tauriTargetDir });

// macOS ARM (Apple Silicon)
if (!autoSkipMacArm) {
  cleanTarget('aarch64-apple-darwin');
  detachRepoDmgMounts({ repoRoot: root });

  const cmd = unsigned
    ? `node ../../scripts/prepare-tauri-runtime-resources.mjs --platform macos --require-runtime && env -u APPLE_SIGNING_IDENTITY -u APPLE_TEAM_ID -u APPLE_ID -u APPLE_PASSWORD ${tauriBuildCommand} --config src-tauri/tauri.dev.conf.json --config src-tauri/tauri.macos.unsigned.conf.json --config ${runtimeConfig} --target aarch64-apple-darwin`
    : `node ../../scripts/prepare-tauri-runtime-resources.mjs --platform macos --require-runtime && node ../../scripts/sign-macos-binaries.mjs && ${signedTauriBuildCommand} ${tauriBuildCommand} --config src-tauri/tauri.macos.conf.json --config ${updaterConfig} --config ${runtimeConfig} --target aarch64-apple-darwin`;
  const diagnosticsDir = prepareMacBuildDiagnostics('aarch64-apple-darwin', cmd);

  const success = runCommand(cmd, 'Building macOS ARM', {
    cwd: desktopDir,
    env: { ...process.env },
    logFile: resolve(diagnosticsDir, 'build.log'),
    maxAttempts: unsigned ? 1 : 2,
    onFailure(error) {
      collectMacDmgDiagnostics('aarch64-apple-darwin', error, diagnosticsDir);
    },
  });
  if (success) {
    bundleArtifactPreserver.preserve('aarch64-apple-darwin');
  }
  results.push({
    platform: 'macOS ARM (Apple Silicon)',
    success
  });
} else {
  const reason = skipMacArm ? '--skip-mac-arm' : 'not on macOS';
  console.log(`⏭️  Skipping macOS ARM (${reason})\n`);
}

// macOS Intel (x64)
if (!autoSkipMacIntel) {
  cleanTarget('x86_64-apple-darwin');
  detachRepoDmgMounts({ repoRoot: root });

  const cmd = unsigned
    ? `node ../../scripts/prepare-tauri-runtime-resources.mjs --platform macos --require-runtime && env -u APPLE_SIGNING_IDENTITY -u APPLE_TEAM_ID -u APPLE_ID -u APPLE_PASSWORD ${tauriBuildCommand} --config src-tauri/tauri.dev.conf.json --config src-tauri/tauri.macos.unsigned.conf.json --config ${runtimeConfig} --target x86_64-apple-darwin`
    : `node ../../scripts/prepare-tauri-runtime-resources.mjs --platform macos --require-runtime && node ../../scripts/sign-macos-binaries.mjs && ${signedTauriBuildCommand} ${tauriBuildCommand} --config src-tauri/tauri.macos.conf.json --config ${updaterConfig} --config ${runtimeConfig} --target x86_64-apple-darwin`;
  const diagnosticsDir = prepareMacBuildDiagnostics('x86_64-apple-darwin', cmd);

  const success = runCommand(cmd, 'Building macOS Intel', {
    cwd: desktopDir,
    env: { ...process.env },
    logFile: resolve(diagnosticsDir, 'build.log'),
    maxAttempts: unsigned ? 1 : 2,
    onFailure(error) {
      collectMacDmgDiagnostics('x86_64-apple-darwin', error, diagnosticsDir);
    },
  });
  if (success) {
    bundleArtifactPreserver.preserve('x86_64-apple-darwin');
  }
  results.push({
    platform: 'macOS Intel (x64)',
    success
  });
} else {
  const reason = skipMacIntel ? '--skip-mac-intel' : 'not on macOS';
  console.log(`⏭️  Skipping macOS Intel (${reason})\n`);
}

// Windows
if (!autoSkipWin) {
  cleanTarget('x86_64-pc-windows-msvc');

  let cmd;
  let skipBuild = false;
  
  if (isWindows) {
    // Native Windows build
    cmd = unsigned ? WINDOWS_NATIVE_UNSIGNED_BUILD_COMMAND : WINDOWS_NATIVE_BUILD_COMMAND;
  } else {
    // Cross-compilation from macOS/Linux using cargo-xwin
    console.log('🔧 Cross-compiling Windows from ' + hostPlatform + '\n');
    
    // Check if cargo-xwin is installed
    try {
      execSync('cargo xwin --version', { stdio: 'ignore' });
      cmd = unsigned ? WINDOWS_CROSS_UNSIGNED_BUILD_COMMAND : WINDOWS_CROSS_BUILD_COMMAND;
    } catch {
      console.error('❌ cargo-xwin not found. Install with:');
      console.error('   cargo install cargo-xwin');
      console.error('   brew install llvm nsis\n');
      results.push({ platform: 'Windows', success: false });
      skipBuild = true;
    }
  }

  if (!skipBuild) {
    const success = runCommand(
      cmd,
      'Building Windows (x64)' + (canCrossCompile ? ' [cross]' : ''),
      {
        cwd: desktopDir,
        env: canCrossCompile ? windowsCrossBuildEnv(process.env) : { ...process.env },
      }
    );
    if (success) {
      bundleArtifactPreserver.preserve('x86_64-pc-windows-msvc');
    }
    results.push({
      platform: 'Windows',
      success
    });
  }
} else {
  const reason = skipWin ? '--skip-win' : 'not on Windows (use --cross-win for cross-compilation)';
  console.log(`⏭️  Skipping Windows (${reason})\n`);
  if (!skipWin && !isWindows && !crossWin) {
    console.log('   💡 Tip: Add --cross-win to enable cross-compilation, or use CI.\n');
    console.log('   Requirements: cargo install cargo-xwin && brew install ninja llvm nsis\n');
  }
}

let artifactRestoreFailed = false;
try {
  const restoredTargets = bundleArtifactPreserver.restore();
  if (restoredTargets.length > 0) {
    console.log(`\n📦 Restored bundle artifacts in src-tauri/target for: ${restoredTargets.join(', ')}\n`);
  }
} catch (error) {
  artifactRestoreFailed = true;
  console.error(`\n❌ Failed to restore preserved bundle artifacts: ${error.message}\n`);
} finally {
  bundleArtifactPreserver.cleanup();
}

if (!artifactRestoreFailed && results.length > 0 && results.every((result) => result.success)) {
  try {
    generateGithubReleaseAssets(results);
    results.push({ platform: 'GitHub release assets', success: true });
  } catch (error) {
    console.error(`\n❌ Failed to generate GitHub release assets: ${error.message}\n`);
    results.push({ platform: 'GitHub release assets', success: false });
  }
}

// Summary
console.log('\n' + '═'.repeat(50));
console.log('📊 Build Summary');
console.log('═'.repeat(50) + '\n');

let allSuccess = true;
for (const result of results) {
  const icon = result.success ? '✅' : '❌';
  console.log(`  ${icon} ${result.platform}`);
  if (!result.success) allSuccess = false;
}
if (artifactRestoreFailed) allSuccess = false;

console.log('\n' + '═'.repeat(50));

if (allSuccess) {
  console.log('\n✅ All builds completed successfully!\n');
  process.exit(0);
} else {
  console.log('\n❌ Some builds failed. Check the output above.\n');
  process.exit(1);
}
