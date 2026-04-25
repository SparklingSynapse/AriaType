import { execSync } from 'child_process';

export default async function globalTeardown() {
  console.log('Cleaning up Tauri processes...');

  try {
    execSync('pkill -f "AriaType"', { stdio: 'ignore' });
  } catch {}

  try {
    execSync('pkill -f "tauri-driver"', { stdio: 'ignore' });
  } catch {}

  console.log('Teardown complete');
}