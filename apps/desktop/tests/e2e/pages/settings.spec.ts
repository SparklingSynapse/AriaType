import { test, expect } from '../fixtures';
import { navigateViaSidebar } from '../utils/helpers';

test('General Settings page renders', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  await navigateViaSidebar(page, 'General');

  await page.waitForTimeout(2000);

  await expect(page.locator('nav a:has-text("General")')).toHaveClass(/bg-primary/);

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('settings.png', {
    threshold: 0.2,
    fullPage: true,
  });
});