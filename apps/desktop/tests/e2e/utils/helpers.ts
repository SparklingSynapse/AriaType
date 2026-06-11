import type { TauriFixtures } from '@srsholmes/tauri-playwright';
import {
  dismissOnboardingIfPresent,
  waitForAppReady,
  invokeTauri,
} from '@ariatype/e2e-harness/helpers';

export {
  waitForAppReady,
  waitForContentLoaded,
  openRoute,
  remountRoute,
  navigateViaSidebar,
  dismissOnboardingIfPresent,
  expectNativeScreenshot,
  setScreenshotThreshold,
  setScreenshotMaxDiffPixelRatio,
  setScreenshotMaxDiffPixels,
  disableAutoSnapshot,
  sleep,
} from '@ariatype/e2e-harness/helpers';

type E2EPage = TauriFixtures['tauriPage'];
const ONBOARDING_RESET_EVENT = 'ariatype:onboarding-reset';
const ONBOARDING_COMPLETE_EVENT = 'ariatype:onboarding-complete';

type ShortcutProfilePayload = {
  hotkey: string;
  trigger_mode: 'hold' | 'toggle' | 'double_tap';
  action: {
    Record: {
      polish_template_id: string | null;
    };
  };
};

export async function setOnboardingCompleted(page: E2EPage, completed: boolean): Promise<void> {
  await page.evaluate(
    `(function() {
      if (${completed}) {
        localStorage.setItem('onboarding_completed', 'true');
        window.dispatchEvent(new Event(${JSON.stringify(ONBOARDING_COMPLETE_EVENT)}));
      } else {
        localStorage.removeItem('onboarding_completed');
        window.dispatchEvent(new Event(${JSON.stringify(ONBOARDING_RESET_EVENT)}));
      }
      return true;
    })()`,
  );
  if (completed) {
    await dismissOnboardingIfPresent(page);
  }
  await waitForAppReady(page);
}

export async function openRouteWithOnboarding(
  page: E2EPage,
  route: string,
  onboardingCompleted = true,
): Promise<void> {
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
  }
  await setOnboardingCompleted(page, onboardingCompleted);
  await waitForAppReady(page);
}

export async function seedDefaultShortcutProfiles(page: E2EPage): Promise<void> {
  const dictateProfile: ShortcutProfilePayload = {
    hotkey: 'Cmd+Slash',
    trigger_mode: 'hold',
    action: { Record: { polish_template_id: null } },
  };
  const riffProfile: ShortcutProfilePayload = {
    hotkey: 'Opt+Slash',
    trigger_mode: 'toggle',
    action: { Record: { polish_template_id: 'filler' } },
  };

  await invokeTauri(page, 'delete_custom_profile').catch(() => undefined);
  await invokeTauri(page, 'update_shortcut_profile', { key: 'dictate', profile: dictateProfile });
  await invokeTauri(page, 'update_shortcut_profile', { key: 'riff', profile: riffProfile });
}

export async function clearTranscriptionHistory(page: E2EPage): Promise<void> {
  await invokeTauri(page, 'clear_transcription_history');
}
