#!/usr/bin/env node
/**
 * Build AriaType for all platforms: macOS (ARM + Intel) and Windows.
 * 
 * Usage:
 *   pnpm build:all                    # Build all platforms
 *   pnpm build:all --skip-mac-arm     # Skip macOS ARM
 *   pnpm build:all --skip-mac-intel   # Skip macOS Intel
 *   pnpm build:all --skip-win         # Skip Windows
 *   pnpm build:all --unsigned         # Build unsigned (no signing)
 *   pnpm build:all --cross-win        # Cross-compile Windows from macOS/Linux
 * 
 * Cross-compilation notes:
 *   - Windows builds require either:
 *     a) Running on Windows (native)
 *     b) --cross-win flag with cargo-xwin installed
 *   - Cross-compilation requirements:
 *     brew install llvm nsis
 *     cargo install cargo-xwin
 *     rustup target add x86_64-pc-windows-msvc
 */

import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'fs';
import { platform } from 'os';
import { execSync } from 'child_process';
import { runCommand } from './build-all-platforms-lib.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const args = process.argv.slice(2);
const skipMacArm = args.includes('--skip-mac-arm');
const skipMacIntel = args.includes('--skip-mac-intel');
const skipWin = args.includes('--skip-win');
const unsigned = args.includes('--unsigned');
const crossWin = args.includes('--cross-win');

const hostPlatform = platform();
const isMacOS = hostPlatform === 'darwin';
const isWindows = hostPlatform === 'win32';

// Cross-compilation requires --cross-win flag on non-Windows hosts
const canCrossCompile = crossWin && !isWindows;
const autoSkipWin = skipWin || (!isWindows && !crossWin);
const autoSkipMacArm = skipMacArm || !isMacOS;
const autoSkipMacIntel = skipMacIntel || !isMacOS;

const desktopDir = resolve(root, 'apps/desktop');
const tauriTargetDir = resolve(desktopDir, 'src-tauri/target');
const buildDiagnosticsDir = resolve(desktopDir, '.build-diagnostics');

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

  writeDiagnosticFile(
    diagnosticsDir,
    'failure.txt',
    [
      `target=${targetTriple}`,
      `error=${error instanceof Error ? error.message : String(error)}`,
      `bundleDir=${bundleDir}`,
      `dmgDir=${dmgDir}`,
      `bundleScript=${scriptPath}`,
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

  const rerunCommand = existsSync(scriptPath)
    ? `cd ${shellQuote(dmgDir)} && bash -x ./bundle_dmg.sh 2>&1 | tee ${shellQuote(resolve(diagnosticsDir, 'bundle_dmg.trace.log'))}`
    : `bundle_dmg.sh was not found at ${scriptPath}`;

  writeDiagnosticFile(diagnosticsDir, 'rerun-command.sh', `#!/usr/bin/env bash\n${rerunCommand}`);

  console.error('\n🔎 macOS DMG failure diagnostics written to:');
  console.error(`   ${diagnosticsDir}`);
  console.error('   Key files: build.log, failure.txt, bundle-dir.find.txt, dmg-dir.find.txt, hdiutil-info.txt, volumes.txt, df.txt');
  console.error('\n   To capture shell tracing before the next clean, run:');
  console.error(`   ${rerunCommand}\n`);
}

console.log('\n🚀 AriaType Multi-Platform Build\n');
console.log(`   Host platform: ${isMacOS ? 'macOS' : isWindows ? 'Windows' : hostPlatform}\n`);

const results = [];

// macOS ARM (Apple Silicon)
if (!autoSkipMacArm) {
  cleanTarget('aarch64-apple-darwin');

  const cmd = unsigned
    ? 'env -u APPLE_SIGNING_IDENTITY -u APPLE_TEAM_ID -u APPLE_ID -u APPLE_PASSWORD pnpm tauri build --config src-tauri/tauri.dev.conf.json --config src-tauri/tauri.macos.unsigned.conf.json --target aarch64-apple-darwin'
    : 'node ../../scripts/sign-macos-binaries.mjs && pnpm tauri build --config src-tauri/tauri.macos.conf.json --target aarch64-apple-darwin';
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
    runCommand('pnpm copy-installer', 'Copying macOS ARM installer', {
      cwd: desktopDir,
      env: { ...process.env },
    });
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

  const cmd = unsigned
    ? 'env -u APPLE_SIGNING_IDENTITY -u APPLE_TEAM_ID -u APPLE_ID -u APPLE_PASSWORD pnpm tauri build --config src-tauri/tauri.dev.conf.json --config src-tauri/tauri.macos.unsigned.conf.json --target x86_64-apple-darwin'
    : 'node ../../scripts/sign-macos-binaries.mjs && pnpm tauri build --config src-tauri/tauri.macos.conf.json --target x86_64-apple-darwin';
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
    runCommand('pnpm copy-installer', 'Copying macOS Intel installer', {
      cwd: desktopDir,
      env: { ...process.env },
    });
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
    cmd = 'pnpm tauri build --config src-tauri/tauri.windows.conf.json --target x86_64-pc-windows-msvc';
  } else {
    // Cross-compilation from macOS/Linux using cargo-xwin
    console.log('🔧 Cross-compiling Windows from ' + hostPlatform + '\n');
    
    // Check if cargo-xwin is installed
    try {
      execSync('cargo xwin --version', { stdio: 'ignore' });
      cmd = 'cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc';
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
        env: { ...process.env },
      }
    );
    if (success) {
      runCommand('pnpm copy-installer', 'Copying Windows installer', {
        cwd: desktopDir,
        env: { ...process.env },
      });
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
    console.log('   Requirements: cargo install cargo-xwin && brew install llvm nsis\n');
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

console.log('\n' + '═'.repeat(50));

if (allSuccess) {
  console.log('\n✅ All builds completed successfully!\n');
  process.exit(0);
} else {
  console.log('\n❌ Some builds failed. Check the output above.\n');
  process.exit(1);
}
