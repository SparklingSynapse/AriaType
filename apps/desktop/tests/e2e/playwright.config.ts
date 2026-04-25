import { defineConfig } from '@playwright/test';
import { fileURLToPath } from 'url';
import * as path from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const snapshotDir = path.resolve(__dirname, 'snapshots');

export default defineConfig({
  testDir: './pages',
  snapshotDir,

  use: {
    baseURL: 'http://localhost:1422',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },

  expect: {
    toHaveScreenshot: {
      threshold: 0.1,
    },
  },

  timeout: 30000,

  reporter: [['list'], ['html', { open: 'never' }]],

  webServer: {
    command: 'pnpm dev',
    port: 1422,
    reuseExistingServer: true,
    timeout: 120000,
  },

  projects: [
    {
      name: 'with-mock',
      use: {
        baseURL: 'http://localhost:1422',
        viewport: { width: 860, height: 620 },
      },
      testMatch: '**/*.spec.ts',
    },
  ],
});