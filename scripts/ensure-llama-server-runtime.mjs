#!/usr/bin/env node
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { prepareLlamaServerReleaseAssets } from './prepare-llama-server-release-assets.mjs';
import {
  existingRuntimeResources,
  normalizePlatform,
  runtimeSidecarSpecs,
} from './prepare-tauri-runtime-resources.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultRoot = resolve(__dirname, '..');

export const DEFAULT_LLAMA_CPP_RELEASE_TAG = 'b9568';

export function configuredRuntimePathEnv(platformValue, env = process.env) {
  const platform = normalizePlatform(platformValue);

  return (
    runtimeSidecarSpecs(platform).find((spec) => {
      const value = env[spec.pathEnv];
      return typeof value === 'string' && value.trim().length > 0;
    })?.pathEnv ?? null
  );
}

export function detectPinnedLlamaCppReleaseTag(rootDir = defaultRoot) {
  const workflowPath = resolve(rootDir, '.github/workflows/release.yml');
  if (!existsSync(workflowPath)) {
    return DEFAULT_LLAMA_CPP_RELEASE_TAG;
  }

  const workflow = readFileSync(workflowPath, 'utf8');
  const match = workflow.match(/LLAMA_CPP_RELEASE_TAG:\s*["']?(b\d+)["']?/);
  return match?.[1] ?? DEFAULT_LLAMA_CPP_RELEASE_TAG;
}

export function officialLlamaServerAssetName(platformValue, releaseTag) {
  const platform = normalizePlatform(platformValue);
  if (platform === 'windows') {
    return `llama-${releaseTag}-bin-win-cpu-x64.zip`;
  }

  throw new Error(`Automatic local polish runtime download is unsupported for ${platform}`);
}

export function officialLlamaServerAssetUrl(platformValue, releaseTag) {
  return `https://github.com/ggml-org/llama.cpp/releases/download/${releaseTag}/${officialLlamaServerAssetName(platformValue, releaseTag)}`;
}

export async function downloadOfficialLlamaServerAsset({
  platform,
  releaseTag,
  assetsDir,
  fetchImpl = globalThis.fetch,
} = {}) {
  if (typeof fetchImpl !== 'function') {
    throw new Error('fetch is unavailable; cannot download local polish runtime asset');
  }

  mkdirSync(assetsDir, { recursive: true });

  const assetName = officialLlamaServerAssetName(platform, releaseTag);
  const assetPath = resolve(assetsDir, assetName);
  const assetUrl = officialLlamaServerAssetUrl(platform, releaseTag);

  if (existsSync(assetPath)) {
    return {
      downloaded: false,
      assetName,
      assetPath,
      assetUrl,
      releaseTag,
    };
  }

  const response = await fetchImpl(assetUrl, {
    headers: {
      'user-agent': 'AriaType build/runtime preparation',
    },
    redirect: 'follow',
  });

  if (!response.ok) {
    throw new Error(
      `Failed to download local polish runtime asset: ${assetUrl} (${response.status} ${response.statusText})`
    );
  }

  const body = Buffer.from(await response.arrayBuffer());
  writeFileSync(assetPath, body);

  return {
    downloaded: true,
    assetName,
    assetPath,
    assetUrl,
    releaseTag,
  };
}

export async function ensureLlamaServerRuntime({
  platform = process.platform,
  rootDir = defaultRoot,
  env = process.env,
  assetsDir = resolve(rootDir, '.tmp/llama-server-assets'),
  releaseTag = detectPinnedLlamaCppReleaseTag(rootDir),
  fetchImpl = globalThis.fetch,
  prepareAssets = prepareLlamaServerReleaseAssets,
} = {}) {
  const normalizedPlatform = normalizePlatform(platform);
  const runtimeResources = existingRuntimeResources(normalizedPlatform, rootDir);
  if (runtimeResources.length > 0) {
    return {
      status: 'already_present',
      platform: normalizedPlatform,
      resources: runtimeResources,
    };
  }

  const configuredPathEnv = configuredRuntimePathEnv(normalizedPlatform, env);
  if (configuredPathEnv) {
    return {
      status: 'env_configured',
      platform: normalizedPlatform,
      pathEnv: configuredPathEnv,
    };
  }

  if (normalizedPlatform !== 'windows') {
    return {
      status: 'skipped',
      platform: normalizedPlatform,
      reason: 'auto_download_not_supported',
    };
  }

  const download = await downloadOfficialLlamaServerAsset({
    platform: normalizedPlatform,
    releaseTag,
    assetsDir,
    fetchImpl,
  });
  const prepared = prepareAssets({
    platform: normalizedPlatform,
    assetsDir,
    rootDir,
  });

  return {
    status: 'prepared',
    platform: normalizedPlatform,
    releaseTag,
    assetName: download.assetName,
    assetPath: download.assetPath,
    assetUrl: download.assetUrl,
    downloaded: download.downloaded,
    prepared,
  };
}

function parseArgs(args) {
  const getValue = (name) => {
    const index = args.indexOf(name);
    return index === -1 ? undefined : args[index + 1];
  };

  return {
    platform: getValue('--platform') ?? process.platform,
    assetsDir: getValue('--assets-dir'),
    releaseTag: getValue('--release-tag'),
  };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const result = await ensureLlamaServerRuntime({
    platform: options.platform,
    assetsDir: options.assetsDir,
    releaseTag: options.releaseTag,
  });

  if (result.status === 'already_present') {
    console.log(`Local polish runtime already present for ${result.platform}.`);
    return;
  }

  if (result.status === 'env_configured') {
    console.log(
      `Local polish runtime for ${result.platform} will be prepared from ${result.pathEnv}.`
    );
    return;
  }

  if (result.status === 'skipped') {
    console.log(`Automatic local polish runtime download is disabled for ${result.platform}.`);
    return;
  }

  const downloadState = result.downloaded ? 'downloaded' : 'reused cached';
  console.log(
    `Prepared ${result.platform} local polish runtime from ${result.assetName} (${downloadState}).`
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
