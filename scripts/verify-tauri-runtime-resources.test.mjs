import test from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import {
  bundleResourceDirs,
  compatibleRuntimeResources,
  expectedRuntimeResources,
  smokeRuntimeExecutable,
  smokeRuntimeResources,
  smokeRuntimeServer,
  verifyRuntimeResources,
} from './verify-tauri-runtime-resources.mjs';

function createFixture() {
  const root = mkdtempSync(join(tmpdir(), 'ariatype-runtime-verify-'));
  mkdirSync(resolve(root, 'apps/desktop/src-tauri'), { recursive: true });
  return root;
}

function writeResource(root, resource) {
  const path = resolve(root, 'apps/desktop/src-tauri', resource);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, 'binary');
  return path;
}

function writeAppResource(root, appName, resource) {
  const path = resolve(
    root,
    'apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos',
    appName,
    'Contents/Resources',
    resource
  );
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, 'binary');
  return path;
}

function writeExecutableResource(root, resource, source) {
  const path = resolve(root, 'apps/desktop/src-tauri', resource);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, source);
  chmodSync(path, 0o755);
  return path;
}

const helpRuntime = `#!/usr/bin/env node
if (process.argv.includes('--help')) {
  console.log('llama-server help');
  process.exit(0);
}
process.exit(2);
`;

const serverRuntime = `#!/usr/bin/env node
const http = require('node:http');
if (process.argv.includes('--help')) {
  console.log('llama-server help');
  process.exit(0);
}
const portIndex = process.argv.indexOf('--port');
const hostIndex = process.argv.indexOf('--host');
const port = Number(process.argv[portIndex + 1]);
const host = hostIndex === -1 ? '127.0.0.1' : process.argv[hostIndex + 1];
const server = http.createServer((request, response) => {
  if (request.url === '/v1/models') {
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end(JSON.stringify({ data: [] }));
    return;
  }
  response.writeHead(404);
  response.end();
});
process.on('SIGTERM', () => server.close(() => process.exit(0)));
server.listen(port, host);
`;

test('expected runtime resources include macOS architecture-specific sidecars', () => {
  assert.deepEqual(expectedRuntimeResources('macos'), [
    'bin/apple-silicon/llama-server',
    'bin/intel/llama-server',
  ]);
});

test('bundle resource dirs include the universal macOS app resources path', () => {
  assert.ok(
    bundleResourceDirs('macos').some((dir) =>
      dir.includes('target/universal-apple-darwin/release/bundle/macos/AriaType.app/Contents/Resources')
    )
  );
});

test('verifies source resources before packaged bundle exists', () => {
  const root = createFixture();
  writeResource(root, 'bin/apple-silicon/llama-server');
  writeResource(root, 'bin/intel/llama-server');

  const result = verifyRuntimeResources({ platform: 'macos', rootDir: root });

  assert.deepEqual(
    result.checked.map((item) => item.resource),
    ['bin/apple-silicon/llama-server', 'bin/intel/llama-server']
  );
});

test('discovers macOS app bundle resources for renamed apps', () => {
  const root = createFixture();
  writeResource(root, 'bin/apple-silicon/llama-server');
  writeResource(root, 'bin/intel/llama-server');
  writeAppResource(root, 'AriaType Inhouse.app', 'bin/apple-silicon/llama-server');
  writeAppResource(root, 'AriaType Inhouse.app', 'bin/intel/llama-server');

  const result = verifyRuntimeResources({ platform: 'macos', rootDir: root });

  assert.ok(result.roots[0].endsWith('AriaType Inhouse.app/Contents/Resources'));
  assert.ok(
    result.checked.every((item) =>
      item.path.includes('AriaType Inhouse.app/Contents/Resources')
    )
  );
});

test('verifies an explicit mounted app resource root', () => {
  const root = createFixture();
  const mountedResourceRoot = resolve(root, 'mounted/AriaType Inhouse.app/Contents/Resources');
  mkdirSync(resolve(mountedResourceRoot, 'bin/apple-silicon'), { recursive: true });
  mkdirSync(resolve(mountedResourceRoot, 'bin/intel'), { recursive: true });
  writeFileSync(resolve(mountedResourceRoot, 'bin/apple-silicon/llama-server'), 'binary');
  writeFileSync(resolve(mountedResourceRoot, 'bin/intel/llama-server'), 'binary');

  const result = verifyRuntimeResources({
    platform: 'macos',
    rootDir: root,
    resourceRoots: [mountedResourceRoot],
  });

  assert.deepEqual(result.roots, [mountedResourceRoot]);
  assert.ok(result.checked.every((item) => item.path.startsWith(mountedResourceRoot)));
});

test('selects the compatible macOS runtime for the current architecture', () => {
  const checked = [
    { resource: 'bin/apple-silicon/llama-server', path: '/arm64/llama-server' },
    { resource: 'bin/intel/llama-server', path: '/x64/llama-server' },
  ];

  assert.deepEqual(
    compatibleRuntimeResources('macos', checked, 'arm64').map((item) => item.resource),
    ['bin/apple-silicon/llama-server']
  );
  assert.deepEqual(
    compatibleRuntimeResources('macos', checked, 'x64').map((item) => item.resource),
    ['bin/intel/llama-server']
  );
});

test(
  'smokes the compatible runtime executable',
  { skip: process.platform === 'win32' ? 'POSIX executable fixture uses a shebang' : false },
  async () => {
    const root = createFixture();
    writeExecutableResource(root, 'bin/apple-silicon/llama-server', helpRuntime);
    writeExecutableResource(root, 'bin/intel/llama-server', helpRuntime);

    const result = await smokeRuntimeResources({
      platform: 'macos',
      rootDir: root,
      arch: 'arm64',
      timeoutMs: 2000,
    });

    assert.deepEqual(
      result.smoked.map((item) => item.resource),
      ['bin/apple-silicon/llama-server']
    );
  }
);

test(
  'reports runtime executable smoke failures',
  { skip: process.platform === 'win32' ? 'POSIX executable fixture uses a shebang' : false },
  async () => {
    const root = createFixture();
    const path = writeExecutableResource(root, 'bin/apple-silicon/llama-server', `#!/usr/bin/env node
process.stderr.write('boom');
process.exit(7);
`);

    await assert.rejects(
      smokeRuntimeExecutable({ path, timeoutMs: 2000 }),
      /Runtime executable smoke failed/
    );
  }
);

test(
  'smokes a runtime server against the OpenAI-compatible models endpoint',
  { skip: process.platform === 'win32' ? 'POSIX executable fixture uses a shebang' : false },
  async () => {
    const root = createFixture();
    const path = writeExecutableResource(root, 'bin/apple-silicon/llama-server', serverRuntime);

    const result = await smokeRuntimeServer({
      path,
      modelPath: resolve(root, 'model.gguf'),
      timeoutMs: 2000,
    });

    assert.match(result.url, /^http:\/\/127\.0\.0\.1:\d+\/v1\/models$/);
  }
);

test('verifies Windows runtime resource', () => {
  const root = createFixture();
  writeResource(root, 'bin/windows/llama-server.exe');

  const result = verifyRuntimeResources({ platform: 'windows', rootDir: root });

  assert.deepEqual(
    result.checked.map((item) => item.resource),
    ['bin/windows/llama-server.exe']
  );
});

test('reports missing runtime resources with checked roots', () => {
  const root = createFixture();

  assert.throws(
    () => verifyRuntimeResources({ platform: 'macos', rootDir: root }),
    /Missing bundled local polish runtime resources for macos/
  );
});
