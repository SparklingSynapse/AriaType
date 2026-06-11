import { test, expect } from '../fixtures';
import {
  disableAutoSnapshot,
  openRouteWithOnboarding,
} from '../utils/helpers';

test('Dictionary page follows current automatic and custom tabs', async ({ tauriPage }, testInfo) => {
  disableAutoSnapshot(testInfo);
  await openRouteWithOnboarding(tauriPage, '/dictionary');

  const dictionaryPage = tauriPage.locator('[data-testid="dictionary-page"]');
  await expect(dictionaryPage).toBeVisible({ timeout: 15000 });
  await expect(dictionaryPage.getByText('Dictionary')).toBeVisible();
  await expect(tauriPage.locator('[data-testid="dictionary-tabs-automatic"]')).toBeVisible();
  await expect(tauriPage.locator('[data-testid="dictionary-tabs-manual"]')).toBeVisible();
  await expect(dictionaryPage.getByPlaceholder('Search dictionary...')).toBeVisible();

  await tauriPage.locator('[data-testid="dictionary-tabs-manual"]').click();
  await expect(dictionaryPage.getByPlaceholder('Write as')).toBeVisible();
  await expect(dictionaryPage.getByText('Import CSV')).toBeVisible();
  await expect(dictionaryPage.getByText('Add word')).toBeVisible();
});
