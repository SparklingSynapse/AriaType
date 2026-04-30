import { test, expect } from '../fixtures';
import { clearTranscriptionHistory, openRoute, remountRoute } from '../utils/helpers';

test('Dashboard renders with backend data', async ({ tauriPage }) => {
  await openRoute(tauriPage, '/');
  await clearTranscriptionHistory(tauriPage);
  await remountRoute(tauriPage, '/');

  const dashboardPage = tauriPage.locator('[data-testid="dashboard-page"]');

  await expect(dashboardPage).toBeVisible({ timeout: 15000 });
  await expect(dashboardPage.getByText('Usage pattern')).toBeVisible();
  await expect(dashboardPage.getByText('Summary')).toBeVisible();
  await expect(dashboardPage.getByText('Active days')).toBeVisible();
  await expect(dashboardPage.getByText('Engines')).toBeVisible();
  await expect(dashboardPage.locator('[data-testid="dashboard-content"]')).toBeVisible();
});
