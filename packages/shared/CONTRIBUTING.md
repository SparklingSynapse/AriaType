# Contributing to @ariatype/shared

## Overview

**Package**: Shared TypeScript utilities for the AriaType monorepo.

**Purpose**: Types and constants used across `@ariatype/desktop` and `@ariatype/website`.

**Entry Point**: `src/index.ts`

---

## Exports

### Types (`src/types.ts`)

| Interface | Description |
|-----------|-------------|
| `Settings` | Application settings configuration |
| `Model` | STT model metadata |
| `PillPosition` | Floating indicator position options |
| `IndicatorMode` | Indicator visibility behavior |
| `UpdateInfo` | Application update information |

### Constants (`src/constants.ts`)

| Constant | Value | Usage |
|----------|-------|-------|
| `APP_VERSION` | `'1.0.0'` | Version display |
| `APP_NAME` | `'AriaType'` | App name display |
| `UPDATE_CHECK_URL` | `ariatype.com/releases/latest.json` | Update API |
| `DOWNLOAD_URL` | `ariatype.com/downloads` | Download page |

---

## Development Setup

```bash
# From repository root
pnpm install

# Type checking (primary validation)
pnpm --filter @ariatype/shared typecheck

# Or from package directory
cd packages/shared && pnpm typecheck
```

---

## Code Style

- TypeScript strict mode
- ES modules (`type: "module"`)
- No runtime dependencies
- All identifiers and comments in **English**
- Export everything via `src/index.ts`

**TypeScript Config**: Extends `tsconfig.json` in package root

---

## Adding New Types/Constants

1. **Add to appropriate source file**:
   - Types → `src/types.ts`
   - Constants → `src/constants.ts`

2. **Export from index**:
   ```typescript
   // src/index.ts
   export * from './types';
   export * from './constants';
   ```

3. **Run typecheck**:
   ```bash
   pnpm --filter @ariatype/shared typecheck
   ```

4. **Validate in dependent packages**:
   ```bash
   pnpm --filter @ariatype/desktop build
   pnpm --filter @ariatype/website build
   ```

---

## Dependencies

**Runtime**: None (zero dependencies)

**DevDependencies**:
- `typescript@^5.7.3`

---

## Usage in Other Packages

```typescript
// In @ariatype/desktop or @ariatype/website
import { Settings, Model, APP_VERSION } from '@ariatype/shared';

// Type usage
const settings: Settings = {
  autoStart: true,
  recordingSound: true,
  pillPosition: 'top-center',
  indicatorMode: 'always-show',
  selectedModel: 'base',
  language: 'en-US',
};
```

---

## Testing

**No test suite currently** — type checking serves as primary validation.

**Validation Approach**:
- `pnpm typecheck` validates TypeScript correctness
- Changes should be validated in dependent packages
- If tests are added, use Vitest (consistent with desktop package)

---

## Package References

This package is consumed by:

| Package | Import Path |
|---------|-------------|
| `@ariatype/desktop` | `workspace:*` (monorepo internal) |
| `@ariatype/website` | `workspace:*` (monorepo internal) |

---

## See Also

- **Root AGENTS.md** — Monorepo guidelines, TDD workflow, coverage gates
- **apps/desktop/CONTRIBUTING.md** — Desktop application development
- **packages/website/CONTRIBUTING.md** — Marketing website development