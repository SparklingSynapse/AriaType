#!/usr/bin/env node

import { fileURLToPath } from 'node:url';
import { existsSync, readFileSync } from 'node:fs';

function readSiblingPublicKey(privateKeyPath) {
  const pubkeyPath = `${privateKeyPath}.pub`;
  if (!existsSync(pubkeyPath)) {
    return null;
  }

  return readFileSync(pubkeyPath, 'utf8').trim();
}

export function normalizeUpdaterSigningEnv(baseEnv = process.env) {
  const privateKey = baseEnv.TAURI_SIGNING_PRIVATE_KEY?.trim();
  const privateKeyPath = baseEnv.TAURI_SIGNING_PRIVATE_KEY_PATH?.trim();
  const pubkey = baseEnv.TAURI_UPDATER_PUBKEY?.trim();
  const siblingPubkey = privateKeyPath ? readSiblingPublicKey(privateKeyPath) : null;
  const resolvedPubkey = siblingPubkey || pubkey;

  const missing = [];
  if (!privateKey && !privateKeyPath) {
    missing.push('TAURI_SIGNING_PRIVATE_KEY or TAURI_SIGNING_PRIVATE_KEY_PATH');
  }
  if (!resolvedPubkey) {
    missing.push('TAURI_UPDATER_PUBKEY');
  }

  const warnings = [];
  if (privateKeyPath && siblingPubkey && pubkey && siblingPubkey !== pubkey) {
    warnings.push(`TAURI_UPDATER_PUBKEY was replaced with ${privateKeyPath}.pub`);
  }

  const env = {
    ...baseEnv,
    TAURI_SIGNING_PRIVATE_KEY: privateKeyPath || privateKey || '',
    TAURI_UPDATER_PUBKEY: resolvedPubkey || '',
  };

  return {
    ok: missing.length === 0,
    missing,
    mismatches: [],
    warnings,
    privateKeyPath,
    env,
  };
}

export function printUpdaterSigningEnvError(missing) {
  if (missing.length > 0) {
    console.error(`Missing updater signing environment variable(s): ${missing.join(', ')}`);
    console.error('Generate a Tauri updater key pair, set TAURI_SIGNING_PRIVATE_KEY to the private key path or contents, or set TAURI_SIGNING_PRIVATE_KEY_PATH to the private key path for local builds. Set TAURI_UPDATER_PUBKEY to the public key embedded in the app.');
  }
}

export function printUpdaterSigningMismatchError(mismatches) {
  if (mismatches.length > 0) {
    console.error(`Invalid updater signing environment: ${mismatches.join(', ')}`);
    console.error('Set TAURI_UPDATER_PUBKEY from the .pub file generated next to TAURI_SIGNING_PRIVATE_KEY_PATH, or point TAURI_SIGNING_PRIVATE_KEY_PATH at the key pair that matches TAURI_UPDATER_PUBKEY.');
  }
}

export function printUpdaterSigningWarnings(warnings) {
  for (const warning of warnings) {
    console.warn(`Updater signing environment warning: ${warning}`);
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const result = normalizeUpdaterSigningEnv(process.env);
  if (!result.ok) {
    printUpdaterSigningEnvError(result.missing);
    printUpdaterSigningMismatchError(result.mismatches);
    process.exit(1);
  }
  printUpdaterSigningWarnings(result.warnings);
}
