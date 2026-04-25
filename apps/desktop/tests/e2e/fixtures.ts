import { test as base } from '@playwright/test';
import { generateMockIPCScript } from './utils/mock-ipc.js';

export const test = base.extend({
  page: async ({ page }, use) => {
    await page.addInitScript(generateMockIPCScript());
    await page.addInitScript(() => {
      localStorage.setItem('onboarding_completed', 'true');
    });
    await use(page);
  },
});

export { expect } from '@playwright/test';