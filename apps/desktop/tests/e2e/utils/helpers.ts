import type { TauriFixtures } from '@srsholmes/tauri-playwright';
import {
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
  setScreenshotMaxDiffPixels,
  disableAutoSnapshot,
  sleep,
} from '@ariatype/e2e-harness/helpers';

type E2EPage = TauriFixtures['tauriPage'];
type ShortcutProfilePayload = {
  hotkey: string;
  trigger_mode: 'hold' | 'toggle';
  action: {
    Record: {
      polish_template_id: string | null;
    };
  };
};

export async function setOnboardingCompleted(page: E2EPage, completed: boolean): Promise<void> {
  const current = await page.evaluate<string | null>("localStorage.getItem('onboarding_completed')");
  const expected = completed ? 'true' : null;

  if (current === expected) {
    await page.evaluate(
      `(function() {
        window.dispatchEvent(new Event(${JSON.stringify(
          completed ? 'ariatype:onboarding-complete' : 'ariatype:onboarding-reset',
        )}));
        return true;
      })()`,
    );
    return;
  }

  await page.evaluate(
    `(function() {
      if (${completed}) {
        localStorage.setItem('onboarding_completed', 'true');
      } else {
        localStorage.removeItem('onboarding_completed');
      }
      window.dispatchEvent(new Event(${JSON.stringify(
        completed ? 'ariatype:onboarding-complete' : 'ariatype:onboarding-reset',
      )}));
      return true;
    })()`,
  );
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
