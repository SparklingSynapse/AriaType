import type { TauriFixtures } from '@srsholmes/tauri-playwright';
import { test, expect } from '../fixtures';
import {
  navigateViaSidebar,
  openRoute,
  seedDefaultShortcutProfiles,
  setOnboardingCompleted,
} from '../utils/helpers';

async function openSeededHotkeyPage(tauriPage: TauriFixtures['tauriPage']): Promise<void> {
  await openRoute(tauriPage, '/');
  await setOnboardingCompleted(tauriPage, true);
  await seedDefaultShortcutProfiles(tauriPage);
  await openRoute(tauriPage, '/hotkey');
}

test('Hotkey Settings page displays profiles', async ({ tauriPage }) => {
  await openRoute(tauriPage, '/');
  await setOnboardingCompleted(tauriPage, true);
  await seedDefaultShortcutProfiles(tauriPage);
  await navigateViaSidebar(tauriPage, 'Hotkey');

  const hotkeyPage = tauriPage.locator('[data-testid="hotkey-page"]');

  await expect(hotkeyPage).toBeVisible();
  await expect(hotkeyPage.locator('[data-testid="profile-dictate"]')).toBeVisible();
  await expect(hotkeyPage.locator('[data-testid="profile-riff"]')).toBeVisible();
  await expect(hotkeyPage.locator('[data-testid="create-custom-profile"]')).toBeVisible();
});

test('Hotkey profiles show Dictate and Riff sections', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  await expect(tauriPage.locator('[data-testid="hotkey-page"]')).toBeVisible();
  await expect(tauriPage.locator('[data-testid="profile-dictate"]')).toBeVisible();
  await expect(tauriPage.locator('[data-testid="profile-riff"]')).toBeVisible();
});

test('Dictate profile shows Cmd+Slash hotkey', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const dictateSection = tauriPage.locator('[data-testid="profile-dictate"]');
  await expect(dictateSection).toBeVisible();

  const dictateText = await dictateSection.innerText();
  expect(dictateText).toContain('⌘');
  expect(/\/|Slash/.test(dictateText)).toBeTruthy();
});

test('Riff profile shows Opt+Slash hotkey', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const riffSection = tauriPage.locator('[data-testid="profile-riff"]');
  await expect(riffSection).toBeVisible();

  const riffText = await riffSection.innerText();
  expect(riffText).toContain('⌥');
});

test('Create custom profile button visible when no custom', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  await expect(tauriPage.locator('[data-testid="create-custom-profile"]')).toBeVisible();
});

test('Recording mode section visible', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const dictateSection = tauriPage.locator('[data-testid="profile-dictate"]');
  const riffSection = tauriPage.locator('[data-testid="profile-riff"]');

  await expect(dictateSection.locator('[role="radiogroup"]')).toBeVisible();
  await expect(riffSection.locator('[role="radiogroup"]')).toBeVisible();
  await expect(dictateSection.locator('[role="radio"]').filter({ hasText: 'Hold' })).toBeVisible();
  await expect(dictateSection.locator('[role="radio"]').filter({ hasText: 'Toggle' })).toBeVisible();
  await expect(riffSection.locator('[role="radio"]').filter({ hasText: 'Hold' })).toBeVisible();
  await expect(riffSection.locator('[role="radio"]').filter({ hasText: 'Toggle' })).toBeVisible();
});

test('Riff profile has polish template selector', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const riffSection = tauriPage.locator('[data-testid="profile-riff"]');
  await expect(riffSection.getByText('Template')).toBeVisible();
});

test('Dictate profile has no polish template selector', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const dictateSection = tauriPage.locator('[data-testid="profile-dictate"]');
  await expect(dictateSection.locator('select')).not.toBeVisible();
});

test('Custom profile not visible by default', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  await expect(tauriPage.locator('[data-testid="profile-custom"]')).not.toBeVisible();
});

test('Riff section visible after scroll', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const riffSection = tauriPage.locator('[data-testid="profile-riff"]');
  await riffSection.scrollIntoViewIfNeeded();

  await expect(riffSection).toBeVisible();
  await expect(riffSection.locator('[role="radiogroup"]')).toBeVisible();
  await expect(riffSection.getByText('Template')).toBeVisible();
});

test('Create custom profile button visible after scroll', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const createBtn = tauriPage.locator('[data-testid="create-custom-profile"]');
  await createBtn.scrollIntoViewIfNeeded();

  await expect(createBtn).toBeVisible();
});

test('Riff recording mode can be toggled after scroll', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const riffSection = tauriPage.locator('[data-testid="profile-riff"]');
  await riffSection.scrollIntoViewIfNeeded();

  const holdRadio = riffSection.locator('[role="radio"]').filter({ hasText: 'Hold' });
  await holdRadio.scrollIntoViewIfNeeded();
  await holdRadio.click();

  await expect(holdRadio).toHaveAttribute('aria-checked', 'true');
});

test('Custom profile can be created after scroll', async ({ tauriPage }) => {
  await openSeededHotkeyPage(tauriPage);

  const createBtn = tauriPage.locator('[data-testid="create-custom-profile"]');
  await createBtn.scrollIntoViewIfNeeded();
  await createBtn.click();

  const customSection = tauriPage.locator('[data-testid="profile-custom"]');
  await expect(customSection).toBeVisible({ timeout: 5000 });
});
