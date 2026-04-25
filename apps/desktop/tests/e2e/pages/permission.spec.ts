import { test, expect } from '../fixtures';

test('Permission Settings page renders', async ({ page }) => {
  await page.goto('/permission');
  await page.waitForLoadState('networkidle');

  await expect(page.locator('[data-testid="permission-page"]')).toBeVisible({ timeout: 10000 });

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('permission.png', {
    threshold: 0.1,
    fullPage: true,
  });
});