type ScreenshotPage = {
  evaluate<T>(expression: string): Promise<T>;
  screenshot(): Promise<Buffer>;
  mouse: {
    move(x: number, y: number): Promise<void>;
  };
};

type InternalScreenshotPage = ScreenshotPage & {
  command: (
    type: string,
    payload: Record<string, unknown>,
  ) => Promise<{ data?: { base64?: string } }>;
};

type E2eMainWindowSnapshot = {
  base64?: string;
};

export type StableScreenshotOptions = {
  stabilizationMs?: number;
  captureMode?: 'native' | 'command' | 'native-with-fallback';
  nativeRetryCount?: number;
  nativeRetryDelayMs?: number;
};

const TRANSIENT_TOAST_TIMEOUT_MS = 4000;
const TRANSIENT_TOAST_POLL_MS = 50;

export function getSnapshotStabilizationMs(): number {
  const configured = Number(process.env.E2E_HARNESS_SNAPSHOT_STABILIZATION_MS ?? '1000');
  return Number.isFinite(configured) ? configured : 1000;
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function moveMouseToNeutralZone(page: ScreenshotPage): Promise<void> {
  const viewport = (await page
    .evaluate<{ width: number; height: number } | null>(
      '(function() { return { width: window.innerWidth, height: window.innerHeight }; })()',
    )
    .catch(() => null)) ?? { width: 860, height: 620 };

  const targetX = Math.max(8, Math.min(viewport.width - 12, viewport.width / 2));
  const targetY = Math.max(8, Math.min(20, viewport.height - 12));

  await page.mouse.move(targetX, targetY).catch(() => undefined);
}

export async function waitForTransientToastsToSettle(
  page: Pick<ScreenshotPage, 'evaluate'>,
  timeoutMs = TRANSIENT_TOAST_TIMEOUT_MS,
  pollMs = TRANSIENT_TOAST_POLL_MS,
): Promise<void> {
  await page
    .evaluate(
      `(async function() {
        const deadline = Date.now() + ${JSON.stringify(timeoutMs)};
        const pollMs = ${JSON.stringify(pollMs)};
        const selector = '[data-sonner-toast]';

        const isVisibleToast = (element) => {
          if (!(element instanceof HTMLElement)) {
            return false;
          }

          if (element.getAttribute('data-removed') === 'true') {
            return false;
          }

          if (element.getAttribute('data-visible') === 'false') {
            return false;
          }

          const rect = element.getBoundingClientRect();
          if (rect.width <= 0 || rect.height <= 0) {
            return false;
          }

          const style = window.getComputedStyle(element);
          const opacity = Number(style.opacity || '1');
          return style.display !== 'none'
            && style.visibility !== 'hidden'
            && Number.isFinite(opacity)
            && opacity > 0.01;
        };

        while (Date.now() < deadline) {
          const hasVisibleToast = Array.from(document.querySelectorAll(selector)).some(isVisibleToast);
          if (!hasVisibleToast) {
            return true;
          }

          await new Promise((resolve) => setTimeout(resolve, pollMs));
        }

        return false;
      })()`,
    )
    .catch(() => undefined);
}

async function showMainWindowForSnapshot(page: Pick<ScreenshotPage, 'evaluate'>): Promise<void> {
  await page
    .evaluate(
      "(async function() { await window.__TAURI_INTERNALS__.invoke('show_main_window'); return true; })()",
    )
    .catch((error) => {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`show_main_window failed before screenshot: ${message}`);
    });
}

export async function preparePageForSnapshot(
  page: ScreenshotPage,
  stabilizationMs = getSnapshotStabilizationMs(),
): Promise<void> {
  await page.evaluate(
    `(async function() {
      if (document.fonts?.ready) {
        try {
          await document.fonts.ready;
        } catch (_) {}
      }

      const images = Array.from(document.images ?? []);
      await Promise.all(
        images.map(async (image) => {
          if (!image.complete) {
            await new Promise((resolve) => {
              image.addEventListener('load', resolve, { once: true });
              image.addEventListener('error', resolve, { once: true });
            });
          }

          if (typeof image.decode === 'function') {
            try {
              await image.decode();
            } catch (_) {}
          }
        }),
      );

      return true;
    })()`,
  ).catch(() => undefined);

  await page.evaluate(
    `(function() {
      const active = document.activeElement;
      if (active && active instanceof HTMLElement) {
        active.blur();
      }
      return true;
    })()`,
  ).catch(() => undefined);

  await showMainWindowForSnapshot(page);
  await sleep(200);

  await waitForTransientToastsToSettle(page);
  await moveMouseToNeutralZone(page);
  await sleep(stabilizationMs);
}

async function takeNativeScreenshotWithRetry(
  page: ScreenshotPage,
  retryCount: number,
  retryDelayMs: number,
): Promise<Buffer> {
  let lastError: unknown;

  for (let attempt = 0; attempt < retryCount; attempt += 1) {
    try {
      await showMainWindowForSnapshot(page);
      await sleep(120);

      if (attempt > 0) {
        await sleep(retryDelayMs);
      }

      return await page.screenshot();
    } catch (error) {
      lastError = error;

      if (attempt < retryCount - 1) {
        await sleep(retryDelayMs);
      }
    }
  }

  throw lastError instanceof Error ? lastError : new Error('Failed to capture native screenshot');
}

async function takeCommandScreenshot(page: ScreenshotPage): Promise<Buffer> {
  const response = await (page as InternalScreenshotPage).command('screenshot', {});
  const base64 = response.data?.base64;

  if (!base64) {
    throw new Error('Screenshot fallback returned no image data');
  }

  return Buffer.from(base64, 'base64');
}

async function takeE2eMainWindowScreenshot(page: ScreenshotPage): Promise<Buffer> {
  const response = await page.evaluate<E2eMainWindowSnapshot>(
    "(async function() { return await window.__TAURI_INTERNALS__.invoke('capture_main_window_snapshot'); })()",
  );
  const base64 = response.base64;

  if (!base64) {
    throw new Error('E2E main-window fallback returned no image data');
  }

  return Buffer.from(base64, 'base64');
}

export async function captureStableScreenshot(
  page: ScreenshotPage,
  options: StableScreenshotOptions = {},
): Promise<Buffer> {
  const stabilizationMs = options.stabilizationMs ?? getSnapshotStabilizationMs();
  const captureMode = options.captureMode ?? 'native-with-fallback';
  const nativeRetryCount = Math.max(1, options.nativeRetryCount ?? 3);
  const nativeRetryDelayMs = Math.max(0, options.nativeRetryDelayMs ?? 250);

  await preparePageForSnapshot(page, stabilizationMs);

  if (captureMode === 'command') {
    return takeCommandScreenshot(page);
  }

  try {
    return await takeNativeScreenshotWithRetry(page, nativeRetryCount, nativeRetryDelayMs);
  } catch (nativeError) {
    if (captureMode !== 'native-with-fallback') {
      throw nativeError;
    }

    let e2eFallbackError: unknown;
    try {
      return await takeE2eMainWindowScreenshot(page);
    } catch (error) {
      e2eFallbackError = error;
    }

    try {
      return await takeCommandScreenshot(page);
    } catch (commandError) {
      const nativeMessage = nativeError instanceof Error ? nativeError.message : String(nativeError);
      const e2eFallbackMessage = e2eFallbackError instanceof Error
        ? e2eFallbackError.message
        : String(e2eFallbackError);
      const commandMessage = commandError instanceof Error ? commandError.message : String(commandError);
      throw new Error(
        `Native screenshot failed before fallback: ${nativeMessage}; e2e main-window fallback failed: ${e2eFallbackMessage}; command fallback failed: ${commandMessage}`,
      );
    }
  }
}
