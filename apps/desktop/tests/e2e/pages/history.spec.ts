import { test, expect } from '../fixtures';
import { clearTranscriptionHistory, navigateViaSidebar, openRoute } from '../utils/helpers';

test('History page renders with entries or empty state', async ({ tauriPage }) => {
  await openRoute(tauriPage, '/');
  await clearTranscriptionHistory(tauriPage);
  await navigateViaSidebar(tauriPage, 'History');

  const historyPage = tauriPage.locator('[data-testid="history-page"]');
  await expect(historyPage).toBeVisible({ timeout: 15000 });

  const hasEntries = await tauriPage.locator('[data-testid="history-entries"]').isVisible();
  const historyText = await historyPage.innerText();
  expect(hasEntries || /No history yet|No recordings yet/.test(historyText)).toBeTruthy();
  await expect(tauriPage.locator('input[type="text"]')).toBeVisible();
});
