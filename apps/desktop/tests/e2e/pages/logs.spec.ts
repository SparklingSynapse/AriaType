import { test, expect } from '../fixtures';

test('Log Viewer page renders', async ({ page }) => {
  await page.goto('/logs');
  await page.waitForLoadState('networkidle');

  await expect(page.locator('[data-testid="logs-page"]')).toBeVisible({ timeout: 10000 });

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('logs.png', {
    threshold: 0.2,
    fullPage: true,
  });
});