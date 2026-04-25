import { test, expect } from '../fixtures';

test('Model Settings page renders with model list', async ({ page }) => {
  await page.goto('/private-ai');
  await page.waitForLoadState('networkidle');

  await expect(page.locator('[data-testid="model-page"]')).toBeVisible({ timeout: 10000 });

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('model.png', {
    threshold: 0.1,
    fullPage: true,
  });
});