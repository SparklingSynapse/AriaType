import { test, expect } from '../fixtures';
import { openRouteWithOnboarding } from '../utils/helpers';

test('Model Settings page renders with model list', async ({ tauriPage }) => {
  await openRouteWithOnboarding(tauriPage, '/private-ai');

  await expect(tauriPage.locator('[data-testid="model-page"]')).toBeVisible({ timeout: 10000 });
  await expect(tauriPage.getByText('Voice Input')).toBeVisible();
  await expect(tauriPage.getByText('Polish')).toBeVisible();
  await expect(tauriPage.getByText('Performance')).toBeVisible();
});
