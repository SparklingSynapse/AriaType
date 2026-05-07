import { expect as playwrightExpect, type TestInfo } from '@playwright/test';
import { captureStableScreenshot, sleep } from './snapshot';

type Locator = {
  isVisible(): Promise<boolean>;
  click(): Promise<void>;
  filter(options: { hasText: string | RegExp }): Locator;
};

type Page = {
  evaluate<T>(expression: string): Promise<T>;
  waitForSelector(selector: string, timeout?: number): Promise<unknown>;
  locator(selector: string): Locator;
};

export type ScreenshotOptions = {
  captureMode?: 'native' | 'command' | 'native-with-fallback';
  stabilizationMs?: number;
};

export { sleep } from './snapshot';

async function waitForCondition(
  condition: () => Promise<boolean>,
  timeout: number,
  errorMessage: string,
): Promise<void> {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await condition()) {
      return;
    }

    await sleep(50);
  }

  throw new Error(errorMessage);
}

async function waitForPathname(page: Page, expectedPath: string, timeout: number): Promise<void> {
  await waitForCondition(
    async () => (await page.evaluate<string>('window.location.pathname')) === expectedPath,
    timeout,
    `Timed out waiting for route ${expectedPath} after ${timeout}ms`,
  );
}

async function waitForPathnameChange(page: Page, previousPath: string, timeout: number): Promise<void> {
  await waitForCondition(
    async () => (await page.evaluate<string>('window.location.pathname')) !== previousPath,
    timeout,
    `Timed out waiting for sidebar navigation after ${timeout}ms`,
  );
}

async function waitForLocatorHidden(locator: Locator, timeout: number, description: string): Promise<void> {
  await waitForCondition(
    async () => !(await locator.isVisible()),
    timeout,
    `Timed out waiting for ${description} to disappear after ${timeout}ms`,
  );
}

/**
 * Wait for the page document.readyState to become 'complete'.
 * Polls every 100ms until ready or timeout.
 */
export async function waitForAppReady(page: Page, timeout = 10000): Promise<void> {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    try {
      const readyState = await page.evaluate<string>('document.readyState');
      if (readyState === 'complete') {
        return;
      }
    } catch {
      // Ignore transient evaluation failures while the webview is still booting.
    }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for app ready after ${timeout}ms`);
}

/**
 * Wait for app ready, then wait for a specific selector to appear.
 */
export async function waitForContentLoaded(page: Page, selector: string, timeout = 10000): Promise<void> {
  await waitForAppReady(page, timeout);
  await page.waitForSelector(selector, timeout);
}

/**
 * Navigate to a route via the History API and dispatch popstate.
 * Does NOT set onboarding state — use the app-specific overload for that.
 */
export async function openRoute(page: Page, route: string): Promise<void> {
  await waitForAppReady(page);
  const currentPath = await page.evaluate<string>('window.location.pathname');
  if (currentPath !== route) {
    await page.evaluate(
      `(function() {
        window.history.pushState({}, '', ${JSON.stringify(route)});
        window.dispatchEvent(new PopStateEvent('popstate'));
        return window.location.pathname;
      })()`,
    );
    await waitForPathname(page, route, 1000);
  }
  await waitForAppReady(page);
}

/**
 * Navigate away from a route and back to force a remount.
 */
export async function remountRoute(page: Page, route: string): Promise<void> {
  const detourRoute = route === '/' ? '/about' : '/';
  await openRoute(page, detourRoute);
  await openRoute(page, route);
}

/**
 * Click a sidebar link and wait for navigation to settle.
 */
export async function navigateViaSidebar(page: Page, menuItem: string, timeout = 10000): Promise<void> {
  const currentPath = await page.evaluate<string>('window.location.pathname');
  await page.locator('a').filter({ hasText: menuItem }).click();
  await waitForPathnameChange(page, currentPath, timeout);
}

/**
 * Dismiss the onboarding modal if it is currently visible.
 */
export async function dismissOnboardingIfPresent(page: Page, timeout = 2000): Promise<void> {
  const skipButton = page.locator('button').filter({ hasText: 'Skip' });
  if (await skipButton.isVisible()) {
    await skipButton.click();
    await waitForLocatorHidden(skipButton, timeout, 'skip button');
    return;
  }

  const closeButton = page.locator('button:has(svg.lucide-x)');
  if (await closeButton.isVisible()) {
    await closeButton.click();
    await waitForLocatorHidden(closeButton, timeout, 'close button');
  }
}

/**
 * Invoke a Tauri IPC command from the page context.
 */
export async function invokeTauri<T>(
  page: Page,
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return page.evaluate<T>(
    `(async function() {
      return await window.__TAURI_INTERNALS__.invoke(
        ${JSON.stringify(command)},
        ${JSON.stringify(args ?? {})}
      );
    })()`,
  );
}

/**
 * Take a stable screenshot and compare against a named snapshot.
 */
export async function expectNativeScreenshot(
  page: Page,
  name: string,
  threshold = 0.1,
  options?: ScreenshotOptions,
): Promise<void> {
  const image = await captureStableScreenshot(page as never, {
    captureMode: options?.captureMode ?? 'native',
    stabilizationMs: options?.stabilizationMs,
  });

  await playwrightExpect(image).toMatchSnapshot(name, { threshold });
}

/**
 * Set the screenshot threshold for the current test via annotation.
 */
export function setScreenshotThreshold(testInfo: TestInfo, threshold: number): void {
  testInfo.annotations.push({
    type: 'screenshot-threshold',
    description: String(threshold),
  });
}

/**
 * Set the max diff pixels for the current test via annotation.
 */
export function setScreenshotMaxDiffPixels(testInfo: TestInfo, maxDiffPixels: number): void {
  testInfo.annotations.push({
    type: 'screenshot-max-diff-pixels',
    description: String(maxDiffPixels),
  });
}

/**
 * Disable the automatic end-of-test snapshot assertion.
 */
export function disableAutoSnapshot(testInfo: TestInfo): void {
  testInfo.annotations.push({
    type: 'disable-auto-snapshot',
    description: 'true',
  });
}
