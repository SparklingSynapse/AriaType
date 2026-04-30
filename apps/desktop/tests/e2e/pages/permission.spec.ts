import { test, expect } from '../fixtures';
import { openRoute } from '../utils/helpers';

test('Permission Settings page renders', async ({ tauriPage }) => {
  await openRoute(tauriPage, '/permission');

  const permissionPage = tauriPage.locator('[data-testid="permission-page"]');

  await expect(permissionPage).toBeVisible({ timeout: 10000 });
  await expect(permissionPage.getByText('Microphone')).toBeVisible();
  await expect(permissionPage.getByText('Accessibility')).toBeVisible();
  await expect(permissionPage.locator('button')).toHaveCount(2);
});
