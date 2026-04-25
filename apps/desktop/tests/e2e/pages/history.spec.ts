import { test, expect } from '../fixtures';
import { navigateViaSidebar } from '../utils/helpers';

test('History page renders with entries or empty state', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  await navigateViaSidebar(page, 'History');

  await page.waitForSelector('[data-testid="history-page"]', { timeout: 15000 });

  const hasEntries = await page.locator('[data-testid="history-entries"]').isVisible().catch(() => false);
  const hasEmptyState = await page.locator('text=/No history yet|No recordings yet/').isVisible().catch(() => false);

  expect(hasEntries || hasEmptyState).toBeTruthy();

  await page.waitForTimeout(1000);

  await expect(page).toHaveScreenshot('history.png', {
    threshold: 0.1,
    fullPage: true,
  });
});