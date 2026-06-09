import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const releaseWorkflow = readFileSync(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');

test('release workflow pins a llama.cpp runtime release', () => {
  assert.match(releaseWorkflow, /LLAMA_CPP_RELEASE_TAG:\s*b\d+/);
});

test('release workflow prepares macOS and Windows local polish runtimes', () => {
  assert.match(releaseWorkflow, /prepare-llama-server-release-assets\.mjs[\s\S]*--platform macos/);
  assert.match(releaseWorkflow, /prepare-llama-server-release-assets\.mjs[\s\S]*--platform windows/);
  assert.match(releaseWorkflow, /\*macos\*arm64\*/);
  assert.match(releaseWorkflow, /\*macos\*x64\*/);
  assert.match(releaseWorkflow, /\*win\*cpu\*x64\*/);
});

test('release workflow requires bundled local polish runtime during packaging', () => {
  const requiredGateCount = releaseWorkflow.match(/ARIATYPE_REQUIRE_LOCAL_POLISH_RUNTIME:\s*"1"/g)
    ?.length ?? 0;

  assert.equal(requiredGateCount, 2);
});

test('release workflow verifies bundled runtime resources before upload', () => {
  assert.match(
    releaseWorkflow,
    /verify-tauri-runtime-resources\.mjs --platform macos --smoke --smoke-timeout-ms 30000/
  );
  assert.match(
    releaseWorkflow,
    /verify-tauri-runtime-resources\.mjs --platform windows --smoke --smoke-timeout-ms 30000/
  );
});
