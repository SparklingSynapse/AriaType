import type { TauriFixtures } from '@srsholmes/tauri-playwright';
import { test, expect } from '../fixtures';
import {
  disableAutoSnapshot,
  expectNativeScreenshot,
  navigateViaSidebar,
  waitForAppReady,
} from '@ariatype/e2e-harness/helpers';
import { openRouteWithOnboarding } from '../utils/helpers';

const journeySnapshotStabilizationMs = 1000;
const modelReadyTimeoutMs = 120000;
const onboardingSteps = ['permissions', 'language', 'hotkey', 'model', 'practice', 'done'] as const;
type E2EPage = TauriFixtures['tauriPage'];

async function assertJourneyStep(
  tauriPage: E2EPage,
  stepId: (typeof onboardingSteps)[number],
): Promise<void> {
  switch (stepId) {
    case 'permissions':
      await expect(tauriPage.locator('h2')).toContainText('Permissions');
      await expect(
        tauriPage.locator('[data-testid="onboarding-permission-microphone"] svg.lucide-check').first(),
      ).toBeVisible({ timeout: 10000 });
      await expect(
        tauriPage.locator('[data-testid="onboarding-permission-accessibility"] svg.lucide-check').first(),
      ).toBeVisible({ timeout: 10000 });
      return;
    case 'language':
      await expect(tauriPage.locator('h2')).toContainText('Language');
      await expect(tauriPage.locator('[data-step-id="language"] button[data-state]')).toBeVisible();
      return;
    case 'hotkey':
      await expect(tauriPage.locator('h2')).toContainText('Hotkey');
      await expect(tauriPage.locator('[data-step-id="hotkey"] [role="button"]')).toBeVisible();
      return;
    case 'model':
      await expect(tauriPage.locator('h2')).toContainText('Model');
      await expect(tauriPage.locator('[data-step-id="model"] .rounded-2xl').first()).toBeVisible({
        timeout: modelReadyTimeoutMs,
      });
      await expect(tauriPage.locator('[data-step-id="model"] svg.lucide-check').first()).toBeVisible({
        timeout: modelReadyTimeoutMs,
      });
      return;
    case 'practice':
      await expect(tauriPage.locator('h2')).toContainText('Try It Out');
      await expect(tauriPage.locator('[tabindex="0"]')).toBeVisible();
      return;
    case 'done':
      await expect(tauriPage.locator('[data-testid="onboarding-modal"] h3')).toBeVisible();
      await expect(tauriPage.locator('[data-testid="onboarding-primary-action"]')).toContainText('Get Started');
      await expect(tauriPage.locator('[data-testid="onboarding-primary-action"]')).not.toBeDisabled();
      return;
  }
}

test('Desktop first-run journey', async ({ tauriPage }) => {
  disableAutoSnapshot(test.info());
  test.setTimeout(180000);

  await waitForAppReady(tauriPage, 15000);
  await expect(tauriPage.locator('[data-testid="dashboard-page"]')).toBeVisible({ timeout: 15000 });

  const modal = tauriPage.locator('[data-testid="onboarding-modal"]');
  const nextButton = tauriPage.locator('[data-testid="onboarding-primary-action"]');

  await expect(modal).toBeVisible({ timeout: 10000 });

  for (const [index, stepId] of onboardingSteps.entries()) {
    await assertJourneyStep(tauriPage, stepId);
    await expectNativeScreenshot(
      tauriPage,
      `Desktop-first-run-journey-step-${index + 1}-${stepId}.png`,
      0.1,
      { captureMode: 'native', stabilizationMs: journeySnapshotStabilizationMs },
    );

    if (stepId !== 'done') {
      await nextButton.click();
    }
  }

  await nextButton.click();
  await expect(modal).not.toBeVisible({ timeout: 10000 });

  const dashboardPage = tauriPage.locator('[data-testid="dashboard-page"]');
  await expect(dashboardPage).toBeVisible({ timeout: 10000 });
  await expect(dashboardPage.locator('[data-testid="dashboard-content"]')).toBeVisible();
  await expectNativeScreenshot(
    tauriPage,
    'Desktop-first-run-journey-step-7-dashboard.png',
    0.1,
    { captureMode: 'native', stabilizationMs: journeySnapshotStabilizationMs },
  );
});

test('Desktop post-onboarding navigation journey', async ({ tauriPage }) => {
  disableAutoSnapshot(test.info());

  await openRouteWithOnboarding(tauriPage, '/');

  const dashboardPage = tauriPage.locator('[data-testid="dashboard-page"]');
  await expect(dashboardPage).toBeVisible({ timeout: 10000 });
  await expect(dashboardPage.locator('[data-testid="dashboard-content"]')).toBeVisible();

  await navigateViaSidebar(tauriPage, 'General');
  const settingsPage = tauriPage.locator('[data-testid="settings-page"]');
  await expect(settingsPage).toBeVisible({ timeout: 10000 });
  await expect(settingsPage.getByText('App Language')).toBeVisible();
  await expectNativeScreenshot(
    tauriPage,
    'Desktop-first-run-journey-step-8-settings.png',
    0.1,
    { captureMode: 'native', stabilizationMs: journeySnapshotStabilizationMs },
  );

  await navigateViaSidebar(tauriPage, 'History');
  const historyPage = tauriPage.locator('[data-testid="history-page"]');
  await expect(historyPage).toBeVisible({ timeout: 10000 });
  const hasEntries = await tauriPage.locator('[data-testid="history-entries"]').isVisible();
  const historyText = await historyPage.innerText();
  expect(hasEntries || /No history yet|No recordings yet/.test(historyText)).toBeTruthy();
  await expectNativeScreenshot(
    tauriPage,
    'Desktop-first-run-journey-step-9-history.png',
    0.1,
    { captureMode: 'native', stabilizationMs: journeySnapshotStabilizationMs },
  );
});
