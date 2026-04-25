import { spawn, ChildProcess } from 'child_process';
import path from 'path';
import http from 'http';

const projectRoot = path.resolve(__dirname, '..');
const tauriAppPath = path.join(
  projectRoot,
  'src-tauri',
  'target',
  'release',
  'bundle',
  'macos',
  'AriaType.app',
  'Contents',
  'MacOS',
  'AriaType'
);

let tauriDriver: ChildProcess | null = null;
let tauriApp: ChildProcess | null = null;

async function waitForDriver(url: string, maxAttempts = 30): Promise<void> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      await new Promise<void>((resolve, reject) => {
        const req = http.request(url, { method: 'GET', timeout: 1000 }, (res) => {
          if (res.statusCode === 200) resolve();
          else reject(new Error(`Status ${res.statusCode}`));
        });
        req.on('error', reject);
        req.on('timeout', () => reject(new Error('timeout')));
        req.end();
      });
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 500));
    }
  }
  throw new Error('tauri-driver not ready after 15s');
}

export default async function globalSetup() {
  console.log('Starting tauri-driver on port 4444...');
  tauriDriver = spawn('tauri-driver', [], {
    stdio: ['ignore', 'inherit', 'inherit'],
    detached: true,
  });

  await waitForDriver('http://127.0.0.1:4444/status');

  console.log('Launching Tauri app...');
  tauriApp = spawn(tauriAppPath, [], {
    stdio: ['ignore', 'inherit', 'inherit'],
    detached: true,
  });

  await new Promise((r) => setTimeout(r, 3000));

  console.log('Setup complete');

  return async () => {
    console.log('Teardown from setup...');
    if (tauriApp) {
      try {
        process.kill(-tauriApp.pid!, 'SIGTERM');
      } catch {}
    }
    if (tauriDriver) {
      try {
        process.kill(-tauriDriver.pid!, 'SIGTERM');
      } catch {}
    }
  };
}