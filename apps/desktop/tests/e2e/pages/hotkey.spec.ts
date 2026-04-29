import { test, expect } from '../fixtures';
import { dismissOnboardingIfPresent } from '../utils/helpers';

test('Hotkey Settings page displays profiles', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await page.locator('nav a:has-text("Hotkey")').click();
  await page.waitForTimeout(1000);

  await expect(page.locator('[data-testid="hotkey-page"]')).toBeVisible();

  await expect(page).toHaveScreenshot('hotkey-with-mock.png', {
    threshold: 0.2,
    fullPage: true,
  });
});

test('Hotkey profiles show Dictate and Riff sections', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await expect(page.locator('[data-testid="hotkey-page"]')).toBeVisible();

  await expect(page.locator('[data-testid="profile-dictate"]')).toBeVisible();
  await expect(page.locator('[data-testid="profile-riff"]')).toBeVisible();
});

test('Dictate profile shows Cmd+Slash hotkey', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await expect(page.locator('[data-testid="hotkey-page"]')).toBeVisible();

  const dictateSection = page.locator('[data-testid="profile-dictate"]');
  await expect(dictateSection.locator('text=⌘')).toBeVisible();
  await expect(dictateSection.locator('text=/').or(dictateSection.locator('text=Slash'))).toBeVisible();
});

test('Riff profile shows Opt+Slash hotkey', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await expect(page.locator('[data-testid="hotkey-page"]')).toBeVisible();

  const riffSection = page.locator('[data-testid="profile-riff"]');
  await expect(riffSection.locator('text=⌥')).toBeVisible();
});

test('Create custom profile button visible when no custom', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await expect(page.locator('[data-testid="create-custom-profile"]')).toBeVisible();
});

test('Recording mode section visible', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  const dictateSection = page.locator('[data-testid="profile-dictate"]');
  const riffSection = page.locator('[data-testid="profile-riff"]');

  await expect(dictateSection.getByRole('radiogroup')).toBeVisible();
  await expect(riffSection.getByRole('radiogroup')).toBeVisible();
  await expect(dictateSection.getByRole('radio', { name: 'Hold' })).toBeVisible();
  await expect(dictateSection.getByRole('radio', { name: 'Toggle' })).toBeVisible();
  await expect(riffSection.getByRole('radio', { name: 'Hold' })).toBeVisible();
  await expect(riffSection.getByRole('radio', { name: 'Toggle' })).toBeVisible();
});

test('Riff profile has polish template selector', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  const riffSection = page.locator('[data-testid="profile-riff"]');
  await expect(riffSection.getByText(/Template/i)).toBeVisible();
});

test('Dictate profile has no polish template selector', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  const dictateSection = page.locator('[data-testid="profile-dictate"]');
  await expect(dictateSection.locator('select')).not.toBeVisible();
});

test('Custom profile not visible by default', async ({ page }) => {
  await page.goto('/hotkey');
  await page.waitForLoadState('networkidle');

  await dismissOnboardingIfPresent(page);

  await expect(page.locator('[data-testid="profile-custom"]')).not.toBeVisible();
});
