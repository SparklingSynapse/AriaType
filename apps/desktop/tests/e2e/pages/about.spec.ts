import { test, expect } from '../fixtures';

test('About page renders', async ({ page }) => {
  await page.goto('/about');
  await page.waitForLoadState('networkidle');

  await expect(page.locator('[data-testid="about-page"]')).toBeVisible({ timeout: 10000 });

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('about.png', {
    threshold: 0.1,
    fullPage: true,
  });
});