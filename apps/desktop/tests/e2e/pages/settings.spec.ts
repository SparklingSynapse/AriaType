import { test, expect } from '../fixtures';
import { openRouteWithOnboarding } from '../utils/helpers';
import { disableAutoSnapshot, expectNativeScreenshot } from '@ariatype/e2e-harness/helpers';

test('Settings modal groups core settings', async ({ tauriPage }) => {
  disableAutoSnapshot(test.info());

  await openRouteWithOnboarding(tauriPage, '/');

  await expect(tauriPage.locator('[data-testid="dashboard-page"]')).toBeVisible({
    timeout: 15000,
  });
  const settingsButton = tauriPage.locator('[data-testid="open-settings-modal"]');
  await expect(settingsButton).toBeVisible({ timeout: 10000 });
  await expect(settingsButton).toBeEnabled();
  await settingsButton.click();

  const settingsModal = tauriPage.locator('[data-testid="settings-modal"]');
  const settingsPage = settingsModal.locator('[data-testid="settings-page"]');

  await expect(settingsModal).toBeVisible({ timeout: 10000 });
  await expect(settingsModal).toHaveCSS('height', '720px');
  await expect(settingsModal).toHaveCSS('width', '940px');
  await expect(settingsModal).toHaveCSS('border-top-width', '0px');
  await expect(settingsModal).not.toHaveCSS('box-shadow', 'none');
  await expect(settingsPage).toBeVisible({ timeout: 10000 });
  await expect(
    settingsModal.locator('[data-testid="settings-modal-section-basics"]'),
  ).not.toContainText('Language, startup');
  await expect(
    settingsModal.locator('[data-testid="settings-modal-section-transcription"]'),
  ).not.toContainText('Microphone, output language');
  await expect(
    settingsModal.locator('[data-testid="settings-modal-section-advanced"]'),
  ).toHaveCount(0);
  await expect(tauriPage.locator('[data-testid="home-sidebar"]')).toHaveCSS('width', '248px');
  await expect(settingsModal.locator('[data-testid="settings-modal-nav"]')).toHaveCSS('width', '248px');
  await expect(settingsPage.getByText('App Language')).toBeVisible();
  await expect(settingsPage.getByText('Auto-start on login')).toBeVisible();
  await expect(settingsPage.getByText('Theme')).toBeVisible();

  await settingsModal.locator('[data-testid="settings-modal-section-transcription"]').click();
  await expect(settingsPage.getByText('Audio Input')).toBeVisible();
  await expect(settingsPage.getByText('Output Language')).toBeVisible();
  await expectNativeScreenshot(
    tauriPage,
    'Settings-modal-groups-core-settings-1.png',
    0.1,
    { captureMode: 'native-with-fallback', stabilizationMs: 1000 },
  );

  await tauriPage.keyboard.press('Escape');
  await expect(settingsModal).toHaveCount(0, { timeout: 5000 });
});

test('Settings modal unmounts after close and releases the page', async ({ tauriPage }) => {
  disableAutoSnapshot(test.info());

  await openRouteWithOnboarding(tauriPage, '/');
  await expect(tauriPage.locator('[data-testid="dashboard-page"]')).toBeVisible({
    timeout: 15000,
  });

  await tauriPage.locator('[data-testid="open-settings-modal"]').click();
  const settingsModal = tauriPage.locator('[data-testid="settings-modal"]');
  await expect(settingsModal).toBeVisible({ timeout: 10000 });

  await tauriPage.keyboard.press('Escape');
  await expect(settingsModal).toHaveCount(0, { timeout: 5000 });

  await tauriPage.click('a[href="/history"]');
  await expect(tauriPage.locator('[data-testid="history-page"]')).toBeVisible({
    timeout: 10000,
  });
});
