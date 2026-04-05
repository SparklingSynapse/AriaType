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

import { execSync } from 'child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { existsSync, rmSync } from 'fs';
import { platform } from 'os';

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

function runCommand(command, description) {
  console.log(`\n${'═'.repeat(50)}`);
  console.log(`📦 ${description}`);
  console.log(`${'═'.repeat(50)}\n`);
  
  const startTime = Date.now();
  
  try {
    execSync(command, {
      cwd: desktopDir,
      stdio: 'inherit',
      env: { ...process.env }
    });
    
    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
    console.log(`\n✅ ${description} completed in ${elapsed}s\n`);
    return true;
  } catch (error) {
    console.error(`\n❌ ${description} failed\n`);
    return false;
  }
}

console.log('\n🚀 AriaType Multi-Platform Build\n');
console.log(`   Host platform: ${isMacOS ? 'macOS' : isWindows ? 'Windows' : hostPlatform}\n`);

const results = [];

// macOS ARM (Apple Silicon)
if (!autoSkipMacArm) {
  cleanTarget('aarch64-apple-darwin');

  const cmd = unsigned
    ? 'env -u APPLE_SIGNING_IDENTITY -u APPLE_TEAM_ID -u APPLE_ID -u APPLE_PASSWORD pnpm tauri build --config src-tauri/tauri.macos.unsigned.conf.json --target aarch64-apple-darwin'
    : 'node ../../scripts/sign-macos-binaries.mjs && pnpm tauri build --config src-tauri/tauri.macos.conf.json --target aarch64-apple-darwin';

  const success = runCommand(cmd, 'Building macOS ARM');
  if (success) {
    runCommand('pnpm copy-installer', 'Copying macOS ARM installer');
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
    ? 'env -u APPLE_SIGNING_IDENTITY -u APPLE_TEAM_ID -u APPLE_ID -u APPLE_PASSWORD pnpm tauri build --config src-tauri/tauri.macos.unsigned.conf.json --target x86_64-apple-darwin'
    : 'node ../../scripts/sign-macos-binaries.mjs && pnpm tauri build --config src-tauri/tauri.macos.conf.json --target x86_64-apple-darwin';

  const success = runCommand(cmd, 'Building macOS Intel');
  if (success) {
    runCommand('pnpm copy-installer', 'Copying macOS Intel installer');
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
    const success = runCommand(cmd, 'Building Windows (x64)' + (canCrossCompile ? ' [cross]' : ''));
    if (success) {
      runCommand('pnpm copy-installer', 'Copying Windows installer');
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
