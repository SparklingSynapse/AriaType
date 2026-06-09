#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultRoot = resolve(__dirname, '..');

const EXPECTED_RESOURCES = {
  macos: [
    'bin/apple-silicon/llama-server',
    'bin/intel/llama-server',
  ],
  windows: [
    'bin/windows/llama-server.exe',
  ],
  linux: [
    'bin/linux/llama-server',
  ],
};

const BUNDLE_RESOURCE_DIRS = {
  macos: [
    'apps/desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/AriaType.app/Contents/Resources',
    'apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/AriaType.app/Contents/Resources',
    'apps/desktop/src-tauri/target/x86_64-apple-darwin/release/bundle/macos/AriaType.app/Contents/Resources',
  ],
  windows: [
    'apps/desktop/src-tauri/target/release/resources',
    'apps/desktop/src-tauri/target/x86_64-pc-windows-msvc/release/resources',
    'apps/desktop/src-tauri/target/release/bundle/nsis/resources',
    'apps/desktop/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/resources',
  ],
  linux: [
    'apps/desktop/src-tauri/target/release/resources',
  ],
};

const MACOS_BUNDLE_PARENT_DIRS = [
  'apps/desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos',
  'apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos',
  'apps/desktop/src-tauri/target/x86_64-apple-darwin/release/bundle/macos',
];

export function normalizePlatform(value = process.platform) {
  if (value === 'darwin') return 'macos';
  if (value === 'win32') return 'windows';
  if (value === 'linux') return 'linux';
  if (Object.hasOwn(EXPECTED_RESOURCES, value)) return value;
  throw new Error(`Unsupported platform for runtime resource verification: ${value}`);
}

export function expectedRuntimeResources(platformValue) {
  return EXPECTED_RESOURCES[normalizePlatform(platformValue)];
}

export function bundleResourceDirs(platformValue) {
  return BUNDLE_RESOURCE_DIRS[normalizePlatform(platformValue)];
}

function sourceResourceDir(rootDir) {
  return resolve(rootDir, 'apps/desktop/src-tauri');
}

function discoverMacosBundleResourceDirs(rootDir) {
  const resourceDirs = [];
  for (const parent of MACOS_BUNDLE_PARENT_DIRS) {
    const absoluteParent = resolve(rootDir, parent);
    if (!existsSync(absoluteParent)) {
      continue;
    }

    for (const entry of readdirSync(absoluteParent, { withFileTypes: true })) {
      if (!entry.isDirectory() || !entry.name.endsWith('.app')) {
        continue;
      }

      const resourceDir = resolve(absoluteParent, entry.name, 'Contents/Resources');
      if (existsSync(resourceDir)) {
        resourceDirs.push(resourceDir);
      }
    }
  }

  return resourceDirs;
}

function unique(values) {
  return Array.from(new Set(values));
}

function existingResourceDirs(platform, rootDir, resourceRoots = []) {
  const explicitRoots = resourceRoots
    .map((dir) => resolve(rootDir, dir))
    .filter((dir) => existsSync(dir));
  if (explicitRoots.length > 0) {
    return unique(explicitRoots);
  }

  const bundleDirs = unique([
    ...bundleResourceDirs(platform).map((dir) => resolve(rootDir, dir)),
    ...(platform === 'macos' ? discoverMacosBundleResourceDirs(rootDir) : []),
  ])
    .filter((dir) => existsSync(dir));

  return bundleDirs.length > 0 ? bundleDirs : [sourceResourceDir(rootDir)];
}

export function verifyRuntimeResources({
  platform = process.platform,
  rootDir = defaultRoot,
  resourceRoots = [],
} = {}) {
  const normalizedPlatform = normalizePlatform(platform);
  const roots = existingResourceDirs(normalizedPlatform, rootDir, resourceRoots);
  const checked = [];
  const missing = [];

  for (const resource of expectedRuntimeResources(normalizedPlatform)) {
    const foundAt = roots
      .map((root) => resolve(root, resource))
      .find((path) => existsSync(path));

    if (foundAt) {
      checked.push({ resource, path: foundAt });
    } else {
      missing.push(resource);
    }
  }

  if (missing.length > 0) {
    throw new Error(
      [
        `Missing bundled local polish runtime resources for ${normalizedPlatform}: ${missing.join(', ')}`,
        `Checked roots: ${roots.join(', ')}`,
      ].join('\n')
    );
  }

  return { platform: normalizedPlatform, roots, checked };
}

export function compatibleRuntimeResources(platformValue, checked, arch = process.arch) {
  const platform = normalizePlatform(platformValue);
  if (platform !== 'macos') {
    return checked;
  }

  if (arch === 'arm64') {
    return checked.filter((item) => item.resource.includes('bin/apple-silicon/'));
  }
  if (arch === 'x64') {
    return checked.filter((item) => item.resource.includes('bin/intel/'));
  }

  return checked;
}

function formatProcessOutput(stdout, stderr) {
  const output = [stdout, stderr]
    .map((value) => value.trim())
    .filter(Boolean)
    .join('\n');
  return output.length > 1200 ? `${output.slice(0, 1200)}...` : output;
}

function stopChild(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve();
  }

  return new Promise((resolve) => {
    const timeout = setTimeout(() => {
      child.kill('SIGKILL');
      resolve();
    }, 2000);

    child.once('exit', () => {
      clearTimeout(timeout);
      resolve();
    });
    child.kill('SIGTERM');
  });
}

export function smokeRuntimeExecutable({
  path,
  args = ['--help'],
  timeoutMs = 5000,
} = {}) {
  if (!path) {
    throw new Error('Runtime executable path is required');
  }

  return new Promise((resolvePromise, rejectPromise) => {
    let stdout = '';
    let stderr = '';
    let settled = false;
    let timedOut = false;
    let timeout;
    const child = spawn(path, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });

    const finish = (fn, value) => {
      if (settled) return;
      settled = true;
      if (timeout) clearTimeout(timeout);
      fn(value);
    };

    timeout = setTimeout(() => {
      timedOut = true;
      void stopChild(child).finally(() => {
        finish(
          rejectPromise,
          new Error(`Runtime executable smoke timed out after ${timeoutMs}ms: ${path}`)
        );
      });
    }, timeoutMs);

    child.stdout?.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr?.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('error', (error) => {
      finish(rejectPromise, error);
    });
    child.on('exit', (code, signal) => {
      const output = formatProcessOutput(stdout, stderr);
      if (timedOut) {
        finish(
          rejectPromise,
          new Error(
            [
              `Runtime executable smoke timed out after ${timeoutMs}ms: ${path}`,
              output ? `Output:\n${output}` : '',
            ].filter(Boolean).join('\n')
          )
        );
        return;
      }

      if (code === 0) {
        finish(resolvePromise, { path, code, signal, stdout, stderr });
        return;
      }

      finish(
        rejectPromise,
        new Error(
          [
            `Runtime executable smoke failed: ${path}`,
            `Exit: ${code ?? signal ?? 'unknown'}`,
            output ? `Output:\n${output}` : '',
          ].filter(Boolean).join('\n')
        )
      );
    });
  });
}

export async function smokeRuntimeResources({
  platform = process.platform,
  rootDir = defaultRoot,
  resourceRoots = [],
  arch = process.arch,
  all = false,
  args = ['--help'],
  timeoutMs = 30000,
} = {}) {
  const verified = verifyRuntimeResources({ platform, rootDir, resourceRoots });
  const selected = all
    ? verified.checked
    : compatibleRuntimeResources(verified.platform, verified.checked, arch);

  if (selected.length === 0) {
    throw new Error(
      `No compatible runtime resource found for ${verified.platform}/${arch}`
    );
  }

  const smoked = [];
  for (const item of selected) {
    await smokeRuntimeExecutable({ path: item.path, args, timeoutMs });
    smoked.push(item);
  }

  return { ...verified, smoked };
}

function parseBaseUrl(host, port) {
  return `http://${host}:${port}/v1/models`;
}

function getFreePort(host) {
  return new Promise((resolvePromise, rejectPromise) => {
    const server = createServer();
    server.on('error', rejectPromise);
    server.listen(0, host, () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === 'object') {
          resolvePromise(address.port);
        } else {
          rejectPromise(new Error('Unable to allocate a local smoke-test port'));
        }
      });
    });
  });
}

async function waitForModelsEndpoint({ url, timeoutMs }) {
  const deadline = Date.now() + timeoutMs;
  let lastError = '';

  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = String(error);
    }

    await delay(250);
  }

  throw new Error(`Timed out waiting for ${url}: ${lastError}`);
}

export async function smokeRuntimeServer({
  path,
  modelPath,
  host = '127.0.0.1',
  port,
  modelAlias = 'ariatype-smoke',
  timeoutMs = 30000,
} = {}) {
  if (!path) {
    throw new Error('Runtime executable path is required');
  }
  if (!modelPath) {
    throw new Error('A GGUF model path is required for server smoke verification');
  }

  const selectedPort = port ?? await getFreePort(host);
  const url = parseBaseUrl(host, selectedPort);
  const child = spawn(
    path,
    [
      '--model',
      modelPath,
      '--alias',
      modelAlias,
      '--host',
      host,
      '--port',
      String(selectedPort),
    ],
    {
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    }
  );

  let stdout = '';
  let stderr = '';
  child.stdout?.on('data', (chunk) => {
    stdout += chunk.toString();
  });
  child.stderr?.on('data', (chunk) => {
    stderr += chunk.toString();
  });
  const exitedBeforeReady = new Promise((_, reject) => {
    child.on('error', reject);
    child.on('exit', (code, signal) => {
      reject(
        new Error(
          [
            'Runtime server exited before readiness',
            `Exit: ${code ?? signal ?? 'unknown'}`,
            formatProcessOutput(stdout, stderr),
          ].filter(Boolean).join('\n')
        )
      );
    });
  });

  try {
    await Promise.race([
      waitForModelsEndpoint({ url, timeoutMs }),
      exitedBeforeReady,
    ]);
    return { path, modelPath, url };
  } catch (error) {
    throw new Error(
      [
        `Runtime server smoke failed: ${path}`,
        String(error),
        stderr.trim() ? `stderr:\n${formatProcessOutput('', stderr)}` : '',
      ].filter(Boolean).join('\n')
    );
  } finally {
    await stopChild(child);
  }
}

function parseArgs(args) {
  const getValue = (name) => {
    const index = args.indexOf(name);
    return index === -1 ? undefined : args[index + 1];
  };
  const getValues = (name) => {
    const values = [];
    for (let index = 0; index < args.length; index += 1) {
      if (args[index] === name && args[index + 1]) {
        values.push(args[index + 1]);
        index += 1;
      }
    }
    return values;
  };

  return {
    platform: getValue('--platform') ?? process.platform,
    rootDir: getValue('--root') ?? defaultRoot,
    resourceRoots: getValues('--resource-root'),
    smoke: args.includes('--smoke'),
    smokeAll: args.includes('--smoke-all'),
    smokeTimeoutMs: Number(getValue('--smoke-timeout-ms') ?? 30000),
    serverModel: getValue('--server-model'),
    serverTimeoutMs: Number(getValue('--server-timeout-ms') ?? 30000),
  };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const result = verifyRuntimeResources(options);
  for (const item of result.checked) {
    console.log(`Verified ${result.platform} runtime resource: ${item.resource} at ${item.path}`);
  }

  if (options.smoke) {
    const smoked = await smokeRuntimeResources({
      ...options,
      all: options.smokeAll,
      timeoutMs: options.smokeTimeoutMs,
    });
    for (const item of smoked.smoked) {
      console.log(`Smoked ${smoked.platform} runtime executable: ${item.resource} at ${item.path}`);
    }
  }

  if (options.serverModel) {
    const selected = compatibleRuntimeResources(result.platform, result.checked);
    if (selected.length === 0) {
      throw new Error(`No compatible runtime resource found for ${result.platform}/${process.arch}`);
    }
    const server = await smokeRuntimeServer({
      path: selected[0].path,
      modelPath: resolve(options.serverModel),
      timeoutMs: options.serverTimeoutMs,
    });
    console.log(`Smoked ${result.platform} runtime server: ${server.url}`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
