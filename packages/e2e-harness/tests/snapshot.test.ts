import { describe, expect, it } from 'vitest';

import { captureStableScreenshot, waitForTransientToastsToSettle } from '../src/snapshot';

describe('waitForTransientToastsToSettle', () => {
  it('waits for visible Sonner toasts before snapshot capture', async () => {
    const evaluatedScripts: string[] = [];
    const page = {
      async evaluate<T>(expression: string): Promise<T> {
        evaluatedScripts.push(expression);
        return true as T;
      },
    };

    await expect(waitForTransientToastsToSettle(page as never, 1234, 56)).resolves.toBeUndefined();

    expect(evaluatedScripts).toHaveLength(1);
    expect(evaluatedScripts[0]).toContain('[data-sonner-toast]');
    expect(evaluatedScripts[0]).toContain('data-visible');
    expect(evaluatedScripts[0]).toContain('1234');
    expect(evaluatedScripts[0]).toContain('56');
  });
});

describe('captureStableScreenshot', () => {
  it('uses e2e main-window fallback before command screenshot fallback', async () => {
    const fallbackImage = Buffer.from('e2e fallback image');
    const evaluatedScripts: string[] = [];
    const page = {
      mouse: {
        async move(): Promise<void> {},
      },
      async evaluate<T>(expression: string): Promise<T> {
        evaluatedScripts.push(expression);
        if (expression.includes('window.innerWidth')) {
          return { width: 860, height: 620 } as T;
        }

        if (expression.includes('capture_main_window_snapshot')) {
          return { base64: fallbackImage.toString('base64') } as T;
        }

        return true as T;
      },
      async screenshot(): Promise<Buffer> {
        throw new Error('native_screenshot failed: no on-screen window found');
      },
      async command(): Promise<{ data: { base64: string } }> {
        throw new Error('canvas tainted');
      },
    };

    await expect(
      captureStableScreenshot(page, {
        stabilizationMs: 0,
        nativeRetryCount: 1,
        captureMode: 'native-with-fallback',
      }),
    ).resolves.toEqual(fallbackImage);

    expect(evaluatedScripts.some((script) => script.includes('capture_main_window_snapshot'))).toBe(true);
  });

  it('falls back to command screenshot when native capture cannot find a window', async () => {
    const fallbackImage = Buffer.from('fallback image');
    const page = {
      mouse: {
        async move(): Promise<void> {},
      },
      async evaluate<T>(expression: string): Promise<T> {
        if (expression.includes('window.innerWidth')) {
          return { width: 860, height: 620 } as T;
        }

        return true as T;
      },
      async screenshot(): Promise<Buffer> {
        throw new Error('native_screenshot failed: no on-screen window found');
      },
      async command(): Promise<{ data: { base64: string } }> {
        return { data: { base64: fallbackImage.toString('base64') } };
      },
    };

    await expect(
      captureStableScreenshot(page, {
        stabilizationMs: 0,
        nativeRetryCount: 1,
        captureMode: 'native-with-fallback',
      }),
    ).resolves.toEqual(fallbackImage);
  });

  it('preserves the native screenshot error when command fallback also fails', async () => {
    const page = {
      mouse: {
        async move(): Promise<void> {},
      },
      async evaluate<T>(expression: string): Promise<T> {
        if (expression.includes('window.innerWidth')) {
          return { width: 860, height: 620 } as T;
        }

        return true as T;
      },
      async screenshot(): Promise<Buffer> {
        throw new Error('native_screenshot failed: no on-screen window found');
      },
      async command(): Promise<{ data: { base64: string } }> {
        throw new Error('canvas tainted');
      },
    };

    await expect(
      captureStableScreenshot(page, {
        stabilizationMs: 0,
        nativeRetryCount: 1,
        captureMode: 'native-with-fallback',
      }),
    ).rejects.toThrow(
      'Native screenshot failed before fallback: native_screenshot failed: no on-screen window found; e2e main-window fallback failed: E2E main-window fallback returned no image data; command fallback failed: canvas tainted',
    );
  });
});
