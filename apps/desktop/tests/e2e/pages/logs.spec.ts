import { test, expect } from '../fixtures';
import {
  disableAutoSnapshot,
  expectNativeScreenshot,
  openRoute,
  setScreenshotThreshold,
} from '../utils/helpers';

test('Log Viewer page renders', async ({ tauriPage }) => {
  disableAutoSnapshot(test.info());
  setScreenshotThreshold(test.info(), 0.5);
  await openRoute(tauriPage, '/logs');

  const logsPage = tauriPage.locator('[data-testid="logs-page"]');
  const filterInput = logsPage.locator('input[type="text"]');

  await expect(logsPage).toBeVisible({ timeout: 10000 });
  await expect(filterInput).toBeVisible();
  await expect(logsPage.locator('button')).toHaveCount(2);
  await filterInput.fill('__e2e-no-log-match__');
  await expect(filterInput).toHaveValue('__e2e-no-log-match__');
  await expect(logsPage.getByText('No logs found.')).toBeVisible();
  await expectNativeScreenshot(
    tauriPage,
    'Log-Viewer-page-renders.png',
    0.5,
    { captureMode: 'native', stabilizationMs: 1500 },
  );
});
