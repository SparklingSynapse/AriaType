#!/usr/bin/env node
/**
 * Pre-signs bundled macOS binaries with hardened runtime + secure timestamp.
 * Must run before `tauri build` so notarization doesn't reject them.
 */
import { execSync } from 'child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { existsSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const signingIdentity = process.env.APPLE_SIGNING_IDENTITY;
if (!signingIdentity) {
  console.log('APPLE_SIGNING_IDENTITY not set — skipping binary pre-signing.');
  process.exit(0);
}

const entitlements = resolve(root, 'apps/desktop/src-tauri/entitlements.plist');

const binaries = [
  resolve(root, 'apps/desktop/src-tauri/bin/apple-silicon/sense-voice-main-aarch64-apple-darwin'),
];

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
