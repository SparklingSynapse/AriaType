import { test, expect } from '../fixtures';
import { openRoute, remountRoute } from '../utils/helpers';

test('Cloud Service page renders with STT and Polish sections', async ({ tauriPage }) => {
  await openRoute(tauriPage, '/cloud');

  await expect(tauriPage.locator('[data-testid="cloud-page"]')).toBeVisible({ timeout: 10000 });
  await expect(tauriPage.getByText('Cloud STT')).toBeVisible();
  await expect(tauriPage.getByText('Cloud Polish')).toBeVisible();
  await expect(tauriPage.locator('#cloud-stt')).toBeVisible();
});

test('Cloud STT enable toggle present', async ({ tauriPage }) => {
  await remountRoute(tauriPage, '/cloud');

  await expect(tauriPage.locator('#cloud-stt')).toBeVisible();
});

test('Cloud Polish enable toggle present', async ({ tauriPage }) => {
  await remountRoute(tauriPage, '/cloud');
  await tauriPage.getByText('Cloud Polish').click();

  await expect(tauriPage.locator('#cloud-polish')).toBeVisible();
});

test('Cloud STT toggle is on with seeded settings', async ({ tauriPage }) => {
  await remountRoute(tauriPage, '/cloud');

  const sttToggle = tauriPage.locator('#cloud-stt');
  await expect(sttToggle).toBeVisible({ timeout: 10000 });
  await expect(sttToggle).toHaveAttribute('aria-checked', 'true');
});

test('Cloud Polish toggle is on with seeded settings', async ({ tauriPage }) => {
  await remountRoute(tauriPage, '/cloud');

  await tauriPage.getByText('Cloud Polish').click();
  const polishToggle = tauriPage.locator('#cloud-polish');
  await expect(polishToggle).toBeVisible({ timeout: 10000 });
  await expect(polishToggle).toHaveAttribute('aria-checked', 'true');
});

test('Cloud STT shows provider and fields when enabled', async ({ tauriPage }) => {
  await remountRoute(tauriPage, '/cloud');

  await expect(tauriPage.locator('#cloud-stt')).toHaveAttribute('aria-checked', 'true');
  await expect(tauriPage.getByText('Provider')).toBeVisible();
  await expect(tauriPage.getByText('How to get API credentials')).toBeVisible();
});

test('Cloud Polish shows provider and fields when enabled', async ({ tauriPage }) => {
  await remountRoute(tauriPage, '/cloud');

  await tauriPage.getByText('Cloud Polish').click();
  await expect(tauriPage.locator('#cloud-polish')).toHaveAttribute('aria-checked', 'true');
  await expect(tauriPage.getByText('API Format')).toBeVisible();
});
