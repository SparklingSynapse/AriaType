import { createTauriFixturesFromConfigModule } from '@ariatype/e2e-harness/playwright';

const { test, expect } = await createTauriFixturesFromConfigModule('./e2e.config.mjs', {
  sharedRuntimeKey: 'shared',
}, new URL('.', import.meta.url).pathname);

export { test, expect };
