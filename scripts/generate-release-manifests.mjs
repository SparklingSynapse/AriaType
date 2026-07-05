#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'fs';
import { basename, join, resolve } from 'path';
import {
  buildUpdaterManifest,
  inferUpdaterPlatforms,
  isUpdaterArtifact,
} from './updater-manifest.mjs';

function readArg(name, fallback = '') {
  const index = process.argv.indexOf(name);
  if (index < 0) return fallback;
  return process.argv[index + 1] || fallback;
}

function readArgs(name) {
  const values = [];
  for (let index = 0; index < process.argv.length; index += 1) {
    if (process.argv[index] === name && process.argv[index + 1]) {
      values.push(process.argv[index + 1]);
    }
  }
  return values;
}

function hasFlag(name) {
  return process.argv.includes(name);
}

function stripTrailingSlash(value) {
  return String(value).replace(/\/+$/, '');
}

function artifactUrl(baseUrl, file) {
  return `${stripTrailingSlash(baseUrl)}/${encodeURIComponent(file).replaceAll('%2F', '/')}`;
}

function detectMacArch(file) {
  const name = file.toLowerCase();
  if (name.includes('aarch64') || name.includes('arm64')) return 'aarch64';
  if (name.includes('x86_64') || name.includes('x64') || name.includes('intel')) return 'x86_64';
  if (name.includes('universal')) return 'universal';
  return 'universal';
}

function installerChannel(file) {
  const lower = file.toLowerCase();
  if (lower.endsWith('.dmg')) return `mac-${detectMacArch(file)}`;
  if (lower.endsWith('.exe')) return 'win-exe';
  if (lower.endsWith('.msi')) return 'win-msi';
  return '';
}

function readNotes() {
  const notesFile = readArg('--notes-file');
  if (notesFile) {
    return readFileSync(notesFile, 'utf8').trim();
  }
  return readArg('--notes', process.env.RELEASE_NOTES || '');
}

const releaseDir = resolve(readArg('--release-dir', process.env.RELEASE_DIR || 'dist'));
const version = readArg('--version', process.env.VERSION || '');
const baseUrl = readArg('--base-url', process.env.RELEASE_BASE_URL || `https://github.com/joe223/AriaType/releases/download/v${version}`);
const pubDate = readArg('--pub-date', new Date().toISOString());
const notes = readNotes();
const requireUpdater = hasFlag('--require-updater');
const requiredUpdaterPlatforms = readArgs('--require-updater-platform');

if (!version) {
  throw new Error('Missing required --version or VERSION');
}
if (!existsSync(releaseDir)) {
  throw new Error(`Release directory does not exist: ${releaseDir}`);
}

const files = readdirSync(releaseDir).filter((file) => !file.startsWith('.')).sort();
const releaseFiles = [];
const platforms = {
  mac: { universal: '', aarch64: '', x86_64: '' },
  windows: { exe: '', msi: '' },
};
let defaultUrl = '';

for (const file of files) {
  const channel = installerChannel(file);
  if (!channel) continue;

  const url = artifactUrl(baseUrl, file);
  releaseFiles.push({ file, channel, url });

  if (channel === 'mac-universal') platforms.mac.universal = url;
  if (channel === 'mac-aarch64') platforms.mac.aarch64 = url;
  if (channel === 'mac-x86_64') platforms.mac.x86_64 = url;
  if (channel === 'win-exe') platforms.windows.exe = url;
  if (channel === 'win-msi') platforms.windows.msi = url;

  if (!defaultUrl || channel.startsWith('mac-')) {
    defaultUrl = url;
  }
}

const updaterPlatforms = {};
for (const file of files) {
  const lowerFile = file.toLowerCase();
  const couldBeMacUpdater = lowerFile.endsWith('.app.tar.gz');
  if (!couldBeMacUpdater && !isUpdaterArtifact({ file, sourceDir: releaseDir })) continue;

  const sigPath = join(releaseDir, `${file}.sig`);
  if (!existsSync(sigPath)) {
    console.warn(`Skipping updater artifact without signature: ${file}`);
    continue;
  }

  let inferred = inferUpdaterPlatforms({ file, sourceDir: releaseDir });
  if (couldBeMacUpdater && inferred.length === 0) {
    inferred = ['darwin-aarch64', 'darwin-x86_64'];
  }

  const signature = readFileSync(sigPath, 'utf8').trim();
  for (const platform of inferred) {
    updaterPlatforms[platform] = {
      url: artifactUrl(baseUrl, file),
      signature,
    };
  }
}

const latest = {
  version,
  pub_date: pubDate,
  notes,
  url: defaultUrl,
  platforms,
  files: releaseFiles,
};
writeFileSync(join(releaseDir, 'latest.json'), JSON.stringify(latest, null, 2));
console.log(`Generated ${basename(releaseDir)}/latest.json`);

if (Object.keys(updaterPlatforms).length > 0) {
  const missingUpdaterPlatforms = requiredUpdaterPlatforms.filter((platform) => !updaterPlatforms[platform]);
  if (missingUpdaterPlatforms.length > 0) {
    throw new Error(`Missing required updater platform(s): ${missingUpdaterPlatforms.join(', ')}`);
  }

  const updaterLatest = buildUpdaterManifest({
    version,
    pubDate,
    notes,
    platforms: updaterPlatforms,
  });
  writeFileSync(join(releaseDir, 'latest.updater.json'), JSON.stringify(updaterLatest, null, 2));
  console.log(`Generated ${basename(releaseDir)}/latest.updater.json`);
} else {
  const message = 'No signed updater artifacts found; latest.updater.json was not generated.';
  if (requireUpdater) {
    throw new Error(message);
  }
  console.warn(message);
}
