import { test, expect } from '../fixtures';

test('Changelog page renders', async ({ page }) => {
  await page.goto('/changelog');
  await page.waitForLoadState('networkidle');

  await expect(page.locator('[data-testid="changelog-page"]')).toBeVisible({ timeout: 10000 });

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('changelog.png', {
    threshold: 0.2,
    fullPage: true,
  });
});