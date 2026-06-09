import { test, expect } from '../fixtures';
import { openRouteWithOnboarding } from '../utils/helpers';

test('Polish Templates page renders', async ({ tauriPage }) => {
  await openRouteWithOnboarding(tauriPage, '/polish-templates');

  const templatesPage = tauriPage.locator('[data-testid="polish-templates-page"]');

  await expect(templatesPage).toBeVisible({ timeout: 10000 });
  await expect(templatesPage.getByText('Create')).toBeVisible();
  await expect(templatesPage.locator('h3')).toHaveCount(2);
});
