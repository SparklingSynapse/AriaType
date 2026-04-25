import { test, expect } from '../fixtures';
import { dismissOnboardingIfPresent } from '../utils/helpers';

test.describe('Onboarding Modal', () => {
  test('shows onboarding on first visit', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await page.evaluate(() => localStorage.removeItem('onboarding_completed'));
    await page.reload();
    await page.waitForLoadState('networkidle');

    await page.waitForTimeout(2000);

    await expect(page.locator('h2')).toContainText('Permissions');
    
    await expect(page).toHaveScreenshot('onboarding-first-step.png');
  });

  test('skip button dismisses onboarding', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await page.evaluate(() => localStorage.removeItem('onboarding_completed'));
    await page.reload();
    await page.waitForLoadState('networkidle');

    await page.waitForTimeout(2000);
    await expect(page.locator('h2')).toContainText('Permissions');

    const skipButton = page.locator('button:has-text("Skip")');
    await skipButton.click();
    await page.waitForTimeout(1000);

    const stored = await page.evaluate(() => localStorage.getItem('onboarding_completed'));
    expect(stored).toBe('true');
    
    await expect(page.locator('[data-testid="dashboard-page"]')).toBeVisible({ timeout: 5000 });
  });

  test('close button (X) dismisses onboarding', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await page.evaluate(() => localStorage.removeItem('onboarding_completed'));
    await page.reload();
    await page.waitForLoadState('networkidle');

    await page.waitForTimeout(2000);
    await expect(page.locator('h2')).toContainText('Permissions');

    const closeButton = page.locator('button').filter({ has: page.locator('svg.lucide-x') });
    await closeButton.click();
    await page.waitForTimeout(1000);

    const stored = await page.evaluate(() => localStorage.getItem('onboarding_completed'));
    expect(stored).toBe('true');
    
    await expect(page.locator('[data-testid="dashboard-page"]')).toBeVisible({ timeout: 5000 });
  });

  test('onboarding not shown on subsequent visits', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    
    await page.evaluate(() => localStorage.setItem('onboarding_completed', 'true'));
    await page.reload();
    await page.waitForLoadState('networkidle');

    await page.waitForTimeout(2000);
    
    await expect(page.locator('[data-testid="dashboard-page"]')).toBeVisible({ timeout: 5000 });
  });

  test('can navigate through first three steps', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await page.evaluate(() => localStorage.removeItem('onboarding_completed'));
    await page.reload();
    await page.waitForLoadState('networkidle');

    await page.waitForTimeout(2000);

    await expect(page.locator('h2')).toContainText('Permissions');

    const nextButton = page.locator('button:has-text("Next")');
    
    await nextButton.click();
    await page.waitForTimeout(500);
    await expect(page.locator('h2')).toContainText('Language');
    
    await nextButton.click();
    await page.waitForTimeout(500);
    await expect(page.locator('h2')).toContainText('Hotkey');
    
    await nextButton.click();
    await page.waitForTimeout(500);
    await expect(page.locator('h2')).toContainText('Model');
  });

  test('prev button works after first step', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await page.evaluate(() => localStorage.removeItem('onboarding_completed'));
    await page.reload();
    await page.waitForLoadState('networkidle');

    await page.waitForTimeout(2000);
    await expect(page.locator('h2')).toContainText('Permissions');

    const nextButton = page.locator('button:has-text("Next")');
    await nextButton.click();
    await page.waitForTimeout(500);
    
    await expect(page.locator('h2')).toContainText('Language');
    
    const prevButton = page.locator('button:has-text("Back")');
    await expect(prevButton).toBeVisible();
    
    await prevButton.click();
    await page.waitForTimeout(500);
    
    await expect(page.locator('h2')).toContainText('Permissions');
    
    const skipButton = page.locator('button:has-text("Skip")');
    await expect(skipButton).toBeVisible();
  });

  test('dismiss onboarding and dashboard is visible', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    
    await dismissOnboardingIfPresent(page);
    
    await page.waitForTimeout(1000);
    
    await expect(page.locator('[data-testid="dashboard-page"]')).toBeVisible({ timeout: 5000 });
  });
});