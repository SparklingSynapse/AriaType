import { test, expect } from '../fixtures';
import { openRouteWithOnboarding } from '../utils/helpers';

test('Changelog page renders', async ({ tauriPage }) => {
  await openRouteWithOnboarding(tauriPage, '/changelog');

  await expect(tauriPage.locator('[data-testid="changelog-page"]')).toBeVisible({ timeout: 10000 });
  await expect(tauriPage.locator('h1')).toContainText('Changelog');

  const versionHeadings = await tauriPage.locator('h2').count();
  expect(versionHeadings).toBeGreaterThan(0);
});
