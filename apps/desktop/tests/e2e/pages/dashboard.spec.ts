import { test, expect } from '../fixtures';

test('Dashboard renders with backend data', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('domcontentloaded');
  
  await page.waitForTimeout(2000);

  await expect(page.locator('[data-testid="dashboard-page"]')).toBeVisible({ timeout: 15000 });

  await page.waitForTimeout(1000);

  await expect(page).toHaveScreenshot('dashboard.png', {
    threshold: 0.2,
    fullPage: true,
  });
});