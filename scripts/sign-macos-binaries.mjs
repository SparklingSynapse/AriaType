#!/usr/bin/env node
/**
 * Pre-signs bundled macOS binaries with hardened runtime + secure timestamp.
 * Must run before `tauri build` so notarization doesn't reject them.
 */
import { execSync } from 'child_process';
import { resolve, dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { existsSync, lstatSync, readdirSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const signingIdentity = process.env.APPLE_SIGNING_IDENTITY;
if (!signingIdentity) {
  console.log('APPLE_SIGNING_IDENTITY not set — skipping binary pre-signing.');
  process.exit(0);
}

const entitlements = resolve(root, 'apps/desktop/src-tauri/entitlements.plist');

const fixedBinaries = [
  resolve(root, 'apps/desktop/src-tauri/bin/apple-silicon/sense-voice-main-aarch64-apple-darwin'),
];

const runtimeDirs = [
  resolve(root, 'apps/desktop/src-tauri/bin/apple-silicon'),
  resolve(root, 'apps/desktop/src-tauri/bin/intel'),
  resolve(root, 'apps/desktop/src-tauri/bin/universal'),
  resolve(root, 'apps/desktop/src-tauri/bin/macos'),
];

function runtimeFilesIn(dir) {
  if (!existsSync(dir)) {
    return [];
  }

  return readdirSync(dir)
    .map((entry) => join(dir, entry))
    .filter((path) => {
      const stats = lstatSync(path);
      if (!stats.isFile()) {
        return false;
      }

      return path.endsWith('/llama-server') || path.endsWith('.dylib');
    });
}

const binaries = Array.from(new Set([
  ...fixedBinaries,
  ...runtimeDirs.flatMap(runtimeFilesIn),
]));

for (const bin of binaries) {
  if (!existsSync(bin)) {
    console.warn(`Warning: binary not found, skipping: ${bin}`);
    continue;
  }
  console.log(`Signing: ${bin}`);
  execSync(
    `codesign --force --options runtime --timestamp --entitlements "${entitlements}" --sign "${signingIdentity}" "${bin}"`,
    { stdio: 'inherit' }
  );
  console.log(`Done: ${bin}`);
}
