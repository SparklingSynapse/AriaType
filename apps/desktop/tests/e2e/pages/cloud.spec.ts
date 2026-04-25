import { test, expect } from '../fixtures';
import { dismissOnboardingIfPresent } from '../utils/helpers';

test('Cloud Service page renders with STT and Polish sections', async ({ page }) => {
  await page.goto('/cloud');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await expect(page.locator('[data-testid="cloud-page"]')).toBeVisible({ timeout: 10000 });

  await expect(page.locator('text=Cloud STT')).toBeVisible();
  await expect(page.locator('text=Cloud Polish')).toBeVisible();

  await page.waitForTimeout(500);

  await expect(page).toHaveScreenshot('cloud-with-mock.png', {
    threshold: 0.1,
    fullPage: true,
  });
});

test('Cloud STT enable toggle present', async ({ page }) => {
  await page.goto('/cloud');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  const sttSwitch = page.locator('#cloud-stt');
  await expect(sttSwitch).toBeVisible();
});

test('Cloud Polish enable toggle present', async ({ page }) => {
  await page.goto('/cloud');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  const polishSwitch = page.locator('#cloud-polish');
  await expect(polishSwitch).toBeVisible();
});