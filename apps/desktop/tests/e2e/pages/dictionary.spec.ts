import { test, expect } from '../fixtures';
import { navigateViaSidebar, openRouteWithOnboarding } from '../utils/helpers';

test('Dictionary page renders automatic and custom dictionaries', async ({ tauriPage }) => {
  await openRouteWithOnboarding(tauriPage, '/');
  await navigateViaSidebar(tauriPage, 'Dictionary');

  const dictionaryPage = tauriPage.locator('[data-testid="dictionary-page"]');
  await expect(dictionaryPage).toBeVisible({ timeout: 15000 });
  await expect(dictionaryPage.getByText('Dictionary')).toBeVisible();
  await expect(tauriPage.locator('[data-testid="dictionary-tabs-automatic"]')).toBeVisible();
  await expect(tauriPage.locator('[data-testid="dictionary-tabs-manual"]')).toBeVisible();

  await tauriPage.locator('[data-testid="dictionary-tabs-manual"]').click();
  await expect(tauriPage.getByPlaceholder('Heard as')).toBeVisible();
  await expect(tauriPage.getByPlaceholder('Write as')).toBeVisible();
});
