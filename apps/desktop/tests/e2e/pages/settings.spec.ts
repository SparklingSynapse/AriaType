import type { TauriFixtures } from '@srsholmes/tauri-playwright';
import { test, expect } from '../fixtures';
import {
  disableAutoSnapshot,
  openRouteWithOnboarding,
  seedDefaultShortcutProfiles,
} from '../utils/helpers';

type E2EPage = TauriFixtures['tauriPage'];

async function openSettingsModal(tauriPage: E2EPage) {
  await openRouteWithOnboarding(tauriPage, '/');

  await expect(tauriPage.locator('[data-testid="dashboard-page"]')).toBeVisible({
    timeout: 15000,
  });
  const settingsButton = tauriPage.locator('[data-testid="open-settings-modal"]');
  await expect(settingsButton).toBeVisible({ timeout: 10000 });
  await expect(settingsButton).toBeEnabled();
  await settingsButton.click();

  const settingsModal = tauriPage.locator('[data-testid="settings-modal"]');
  await expect(settingsModal).toBeVisible({ timeout: 10000 });
  return settingsModal;
}

async function getTopHitHrefAtCenter(tauriPage: E2EPage, selector: string) {
  return tauriPage.locator(selector).evaluate((element) => {
    const rect = element.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    const hit = document.elementFromPoint(x, y);
    return hit?.closest('a')?.getAttribute('href') ?? null;
  });
}

async function getSettingsModalState(tauriPage: E2EPage) {
  const modal = tauriPage.locator('[data-testid="settings-modal"]');
  if ((await modal.count()) === 0) {
    return 'unmounted';
  }

  return modal.evaluate((element) => element.getAttribute('data-state') ?? 'mounted');
}

test('Settings modal navigation matches current sections', async ({ tauriPage }, testInfo) => {
  disableAutoSnapshot(testInfo);
  const settingsModal = await openSettingsModal(tauriPage);
  await seedDefaultShortcutProfiles(tauriPage);

  await expect(tauriPage.locator('[data-testid="home-sidebar"]')).toHaveCSS('width', '248px');
  await expect(settingsModal.locator('[data-testid="settings-modal-nav"]')).toHaveCSS('width', '248px');

  for (const section of ['basics', 'recording', 'transcription', 'models', 'cloud', 'permissions']) {
    await expect(settingsModal.locator(`[data-testid="settings-modal-section-${section}"]`)).toBeVisible();
  }
  await expect(settingsModal.locator('[data-testid="settings-modal-section-advanced"]')).toHaveCount(0);

  const basicsPage = settingsModal.locator('[data-testid="settings-page"]');
  await expect(basicsPage).toBeVisible();
  await expect(basicsPage.getByText('App Language')).toBeVisible();
  await expect(basicsPage.getByText('Auto-start on login')).toBeVisible();
  await expect(basicsPage.getByText('Theme')).toBeVisible();

  await settingsModal.locator('[data-testid="settings-modal-section-recording"]').click();
  const hotkeyPage = settingsModal.locator('[data-testid="hotkey-page"]');
  await expect(hotkeyPage).toBeVisible();
  await expect(hotkeyPage.locator('[data-testid="profile-dictate"]')).toBeVisible();
  await expect(hotkeyPage.locator('[data-testid="profile-riff"]')).toBeVisible();
  await expect(hotkeyPage.locator('[data-testid="create-custom-profile"]')).toBeVisible();

  await settingsModal.locator('[data-testid="settings-modal-section-transcription"]').click();
  const transcriptionPage = settingsModal.locator('[data-testid="settings-page"]');
  await expect(transcriptionPage.getByText('Audio Input')).toBeVisible();
  await expect(transcriptionPage.getByText('Output Language')).toBeVisible();
  await expect(transcriptionPage.getByText('Glossary')).toBeVisible();

  await settingsModal.locator('[data-testid="settings-modal-section-models"]').click();
  const modelPage = settingsModal.locator('[data-testid="model-page"]');
  await expect(modelPage).toBeVisible();
  await expect(modelPage.getByText('Voice Input')).toBeVisible();
  await expect(modelPage.getByText('Polish')).toBeVisible();
  await expect(modelPage.getByText('Performance')).toBeVisible();

  await settingsModal.locator('[data-testid="settings-modal-section-cloud"]').click();
  const cloudPage = settingsModal.locator('[data-testid="cloud-page"]');
  await expect(cloudPage).toBeVisible();
  await expect(cloudPage.getByText('Cloud STT')).toBeVisible();
  await cloudPage.getByText('Cloud Polish').click();
  await expect(cloudPage.locator('#cloud-polish')).toBeVisible();

  await settingsModal.locator('[data-testid="settings-modal-section-permissions"]').click();
  const permissionPage = settingsModal.locator('[data-testid="permission-page"]');
  await expect(permissionPage).toBeVisible();
  await expect(permissionPage.getByText('Microphone')).toBeVisible();
  await expect(permissionPage.getByText('Accessibility')).toBeVisible();
  await expect(permissionPage.getByText('Screen Recording')).toBeVisible();
});

test('Settings modal unmounts after close and releases the page', async ({ tauriPage }, testInfo) => {
  disableAutoSnapshot(testInfo);
  const settingsModal = await openSettingsModal(tauriPage);

  await tauriPage.keyboard.press('Escape');
  await expect.poll(() => getSettingsModalState(tauriPage), { timeout: 1000 }).toMatch(/^(closed|unmounted)$/);
  const historyHitHref = await getTopHitHrefAtCenter(tauriPage, 'a[href="/history"]');
  expect(historyHitHref).toBe('/history');
  await expect(settingsModal).toHaveCount(0, { timeout: 5000 });

  await tauriPage.click('a[href="/history"]');
  await expect(tauriPage.locator('[data-testid="history-page"]')).toBeVisible({
    timeout: 10000,
  });
});
