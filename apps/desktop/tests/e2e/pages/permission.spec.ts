import { test, expect } from '../fixtures';
import { openRouteWithOnboarding } from '../utils/helpers';

test('Permission Settings page renders', async ({ tauriPage }) => {
  await openRouteWithOnboarding(tauriPage, '/permission');

  const permissionPage = tauriPage.locator('[data-testid="permission-page"]');

  await expect(permissionPage).toBeVisible({ timeout: 10000 });
  await expect(permissionPage.getByText('Microphone')).toBeVisible();
  await expect(permissionPage.getByText('Accessibility')).toBeVisible();
  await expect(permissionPage.getByText('Screen Recording')).toBeVisible();
  await expect(permissionPage.locator('button')).toHaveCount(3);
});
