import { test, expect } from '../fixtures';
import { navigateViaSidebar, openRoute } from '../utils/helpers';

test('General Settings page renders', async ({ tauriPage }) => {
  await openRoute(tauriPage, '/');
  await navigateViaSidebar(tauriPage, 'General');

  const settingsPage = tauriPage.locator('[data-testid="settings-page"]');
  const generalLink = tauriPage.locator('a').filter({ hasText: 'General' });

  await expect(settingsPage).toBeVisible({ timeout: 10000 });
  await expect(generalLink).toHaveClass(/bg-primary/);
  await expect(settingsPage.getByText('General')).toBeVisible();
  await expect(settingsPage.getByText('App Language')).toBeVisible();
  await expect(settingsPage.getByText('Auto-start on login')).toBeVisible();
  await expect(settingsPage.getByText('Theme')).toBeVisible();
  await expect(settingsPage.getByText('Transcription')).toBeVisible();
});
