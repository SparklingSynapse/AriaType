import { describe, expect, it } from 'vitest';

import { dismissOnboardingIfPresent, navigateViaSidebar } from '../src/helpers';

function createPageForNavigation(pathnameAfterClick: string | null) {
  let pathname = '/dashboard';

  const linkLocator = {
    async click() {
      if (pathnameAfterClick) {
        pathname = pathnameAfterClick;
      }
    },
    filter() {
      return linkLocator;
    },
    async isVisible() {
      return true;
    },
  };

  return {
    async evaluate<T>(expression: string): Promise<T> {
      if (expression === 'document.readyState') {
        return 'complete' as T;
      }

      if (expression === 'window.location.pathname') {
        return pathname as T;
      }

      throw new Error(`Unexpected expression: ${expression}`);
    },
    async waitForSelector() {
      return undefined;
    },
    locator(selector: string) {
      if (selector === 'a') {
        return linkLocator;
      }

      throw new Error(`Unexpected selector: ${selector}`);
    },
  };
}

function createPageForDismiss(skipInitiallyVisible: boolean) {
  let skipVisible = skipInitiallyVisible;
  let closeVisible = !skipInitiallyVisible;

  const skipLocator = {
    async click() {
      skipVisible = false;
    },
    filter() {
      return skipLocator;
    },
    async isVisible() {
      return skipVisible;
    },
  };

  const closeLocator = {
    async click() {
      closeVisible = false;
    },
    filter() {
      return closeLocator;
    },
    async isVisible() {
      return closeVisible;
    },
  };

  return {
    async evaluate<T>(expression: string): Promise<T> {
      if (expression === 'document.readyState') {
        return 'complete' as T;
      }

      throw new Error(`Unexpected expression: ${expression}`);
    },
    async waitForSelector() {
      return undefined;
    },
    locator(selector: string) {
      if (selector === 'button') {
        return {
          async click() {},
          filter(options: { hasText: string | RegExp }) {
            return options.hasText === 'Skip' ? skipLocator : skipLocator;
          },
          async isVisible() {
            return false;
          },
        };
      }

      if (selector === 'button:has(svg.lucide-x)') {
        return closeLocator;
      }

      throw new Error(`Unexpected selector: ${selector}`);
    },
  };
}

describe('navigateViaSidebar', () => {
  it('waits for the route to change after clicking a sidebar link', async () => {
    const page = createPageForNavigation('/settings');

    await expect(navigateViaSidebar(page, 'General')).resolves.toBeUndefined();
  });

  it('fails when the route never changes after clicking a sidebar link', async () => {
    const page = createPageForNavigation(null);

    await expect(navigateViaSidebar(page, 'General', 100)).rejects.toThrow(
      'Timed out waiting for sidebar navigation after 100ms',
    );
  });
});

describe('dismissOnboardingIfPresent', () => {
  it('waits for the skip button to disappear after dismissing onboarding', async () => {
    const page = createPageForDismiss(true);

    await expect(dismissOnboardingIfPresent(page)).resolves.toBeUndefined();
  });

  it('waits for the close button to disappear when skip is not visible', async () => {
    const page = createPageForDismiss(false);

    await expect(dismissOnboardingIfPresent(page)).resolves.toBeUndefined();
  });
});
