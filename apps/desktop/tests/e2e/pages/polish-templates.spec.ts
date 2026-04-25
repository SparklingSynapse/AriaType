import { test, expect } from '../fixtures';
import { dismissOnboardingIfPresent } from '../utils/helpers';

test('Polish Templates page renders', async ({ page }) => {
  await page.goto('/polish-templates');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await expect(page.locator('[data-testid="polish-templates-page"]')).toBeVisible({ timeout: 10000 });

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('polish-templates.png', {
    threshold: 0.1,
    fullPage: true,
  });
});