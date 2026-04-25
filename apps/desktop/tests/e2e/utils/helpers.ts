import type { Page } from '@playwright/test';

export async function waitForContentLoaded(page: Page, selector: string, timeout = 10000): Promise<void> {
  await page.waitForSelector(selector, { state: 'visible', timeout });
}

export async function dismissOnboardingIfPresent(page: Page): Promise<void> {
  const skipButton = await page.locator('button:has-text("Skip")').isVisible().catch(() => false);
  if (skipButton) {
    await page.locator('button:has-text("Skip")').click();
    await page.waitForTimeout(1000);
  }
  const closeButton = await page.locator('button').filter({ has: page.locator('svg.lucide-x') }).isVisible().catch(() => false);
  if (closeButton) {
    await page.locator('button').filter({ has: page.locator('svg.lucide-x') }).click();
    await page.waitForTimeout(1000);
  }
}

export async function navigateViaSidebar(page: Page, menuItem: string): Promise<void> {
  await page.locator(`nav a:has-text("${menuItem}")`).click();
  await page.waitForTimeout(500);
}

export async function capturePageScreenshot(page: Page, name: string): Promise<void> {
  await page.waitForTimeout(500);
  await page.screenshot({ path: `../baseline/${name}.png`, fullPage: true });
}