# Cloud Service Feature Specification

## Feature Name
Cloud Service Tab-based Organization

## Version
v1.0.0

## Overview

Cloud Service provides cloud-based speech-to-text (STT) and text polishing (Polish) functionality. This feature refactors the Cloud Service settings page to use a tab-based UI consistent with the Private AI settings page pattern.

## User Experience

### Navigation
- Route: `/cloud`
- Layout: SettingsPageLayout with title and description
- Pattern: Segmented tab navigation matching Private AI (ModelSettings.tsx)

### Tabs
1. **Cloud STT** - Cloud-based speech recognition configuration
2. **Cloud Polish** - Cloud-based text enhancement configuration

### Tab UI Pattern
```tsx
<div className="inline-flex h-10 items-center justify-center rounded-lg bg-secondary p-1 text-muted-foreground">
  <button className={cn(
    "inline-flex items-center justify-center whitespace-nowrap rounded-md px-4 py-1.5 text-sm font-medium transition-all",
    isActive ? "bg-background text-foreground shadow-sm" : "hover:text-foreground"
  )}>
    Tab Label
  </button>
</div>
```

## Components

### CloudSttSection
- Enable/disable Cloud STT toggle
- Provider selection (volcengine-streaming, volcengine-flash, openai, openai-realtime, deepgram, custom)
- App ID (for Volcengine providers)
- API Key / Access Token
- Base URL
- Model name
- Language

### CloudPolishSection
- Enable/disable Cloud Polish toggle
- Provider selection (anthropic, openai, custom)
- API Key
- Base URL
- Model name
- Enable Thinking toggle
- Runtime behavior: every cloud polish request must use a shared 5-second timeout across providers

## i18n Keys

| Key | Description |
|-----|-------------|
| `cloud.tabs.stt` | Cloud STT tab label |
| `cloud.tabs.polish` | Cloud Polish tab label |

Added to all 10 supported locales: en, zh, de, es, fr, it, ja, ko, pt, ru

## Files Changed

### New Files
- `apps/desktop/src/components/Home/cloud/CloudSttSection.tsx`
- `apps/desktop/src/components/Home/cloud/CloudPolishSection.tsx`
- `context/feat/cloud-service/v1.0.0/prd/erd.md`

### Modified Files
- `apps/desktop/src/components/Home/CloudService.tsx` - Refactored to use tabs
- `apps/desktop/src/i18n/locales/{locale}.json` - Added tab labels to all locales

## Acceptance Criteria

1. **Tab Navigation**: Two tabs (Cloud STT, Cloud Polish) visible on Cloud Service page
2. **Visual Consistency**: Tab UI matches Private AI (ModelSettings.tsx) pattern
3. **Content Switching**: Clicking tab shows corresponding section
4. **i18n**: All tab labels have translations in all 10 locales
5. **Build**: Frontend builds successfully
6. **Tests**: All existing tests pass (312 tests)
7. **Timeout Safety**: Cloud Polish requests fail fast after 5 seconds instead of hanging indefinitely

## Test Coverage

Current test coverage is 30.22% overall. The test infrastructure tests the Rust library code but does not exercise Tauri command handlers. This is a known architectural limitation of the current test setup.

Core modules requiring additional test coverage:
- `commands/` - Tauri IPC handlers (0-5% coverage)
- `text_injector/` - Platform text injection (0% coverage)
- `stt_engine/cloud/` - Cloud STT engines (0-20% coverage)

## Verification

```bash
# Frontend build
pnpm --filter @ariatype/desktop build

# Rust tests
cd apps/desktop/src-tauri && cargo test
```

## Notes

- Feature card (dashed border with Zap, Sparkles, RefreshCw icons) was removed as it's redundant with tabbed navigation
- Routes remain separate (/cloud and /private-ai)
- Settings context via useSettingsContext() pattern maintained
