import { test, expect } from '../fixtures';
import { disableAutoSnapshot, expectNativeScreenshot } from '@ariatype/e2e-harness/helpers';
import { openRouteWithOnboarding } from '../utils/helpers';

test('About page renders', async ({ tauriPage }) => {
  disableAutoSnapshot(test.info());
  await openRouteWithOnboarding(tauriPage, '/about');

  const aboutPage = tauriPage.locator('[data-testid="about-page"]');
  const supportedPlatformsHeading = aboutPage.getByText('Supported Platforms');

  await expect(aboutPage).toBeVisible({ timeout: 10000 });
  await expect(aboutPage.locator('h1')).toContainText('AriaType');
  await expect(aboutPage.getByText('Voice typing, private and effortless')).toBeVisible();
  await expect(aboutPage.getByText('Software Updates')).toBeVisible();
  await expect(aboutPage.getByText('Features')).toBeVisible();
  await expect(supportedPlatformsHeading).toBeVisible();
  await expect(aboutPage.getByText('View Changelog')).toBeVisible();

  await supportedPlatformsHeading.scrollIntoViewIfNeeded();
  await expectNativeScreenshot(
    tauriPage,
    'About-page-renders.png',
    0.1,
    { captureMode: 'native', stabilizationMs: 1500 },
  );
});
