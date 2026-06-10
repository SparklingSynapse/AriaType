import { test, expect } from '../fixtures';
import { disableAutoSnapshot, expectNativeScreenshot } from '@ariatype/e2e-harness/helpers';
import { openRouteWithOnboarding } from '../utils/helpers';

test('About page renders', async ({ tauriPage }) => {
  disableAutoSnapshot(test.info());
  await openRouteWithOnboarding(tauriPage, '/');
  await tauriPage.keyboard.press('Escape');
  await expect(tauriPage.locator('[data-testid="settings-modal"]')).toHaveCount(0, {
    timeout: 5000,
  });

  const aboutNavItem = tauriPage.locator('[data-testid="nav-about"]');
  const settingsNavItem = tauriPage.locator('[data-testid="open-settings-modal"]');
  const feedbackNavItem = tauriPage.locator('[data-testid="nav-feedback"]');
  await expect(settingsNavItem).toBeVisible({ timeout: 10000 });
  await expect(aboutNavItem).toBeVisible({ timeout: 10000 });
  await expect(feedbackNavItem).toBeVisible({ timeout: 10000 });

  const aboutFollowsSettings = await tauriPage.evaluate<boolean>(
    `(function() {
      const settingsItem = document.querySelector('[data-testid="open-settings-modal"]');
      const aboutItem = document.querySelector('[data-testid="nav-about"]');
      if (!settingsItem || !aboutItem) {
        return false;
      }

      return settingsItem.nextElementSibling === aboutItem;
    })()`,
  );
  expect(aboutFollowsSettings).toBe(true);

  await aboutNavItem.click();
  await expect(tauriPage.locator('[data-testid="nav-github-support"]')).toBeVisible({ timeout: 10000 });

  const aboutPage = tauriPage.locator('[data-testid="about-page"]');
  const supportedPlatformsHeading = aboutPage.getByText('Supported Platforms');

  await expect(aboutPage).toBeVisible({ timeout: 10000 });
  await expect(aboutPage.locator('h1')).toContainText('AriaType');
  await expect(
    aboutPage.getByText('Voice-driven writing, input, and cross-app work for your desktop.'),
  ).toBeVisible();
  await expect(aboutPage.getByText('Software Updates')).toBeVisible();
  await expect(aboutPage.getByText('Features')).toBeVisible();
  await expect(supportedPlatformsHeading).toBeVisible();
  await expect(aboutPage.getByText('View Changelog')).toBeVisible();

  await supportedPlatformsHeading.scrollIntoViewIfNeeded();
  await expectNativeScreenshot(
    tauriPage,
    'About-page-renders.png',
    0.1,
    { captureMode: 'native-with-fallback', stabilizationMs: 1500 },
  );
});
