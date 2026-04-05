# Contributing to @ariatype/website

## Overview

**Package**: Next.js 14 marketing website for AriaType.

**URL**: ariatype.com

**Deployment**: Cloudflare Pages (static export)

**Stack**: Next.js 14 + React 18 + TypeScript + Tailwind CSS + i18next

---

## Prerequisites

- Node.js 18+
- pnpm 8+
- Wrangler CLI (for deployment, optional)

---

## Development Setup

```bash
# From repository root
pnpm install

# Start development server
pnpm --filter @ariatype/website dev

# Build (static export)
pnpm --filter @ariatype/website build

# Lint
pnpm --filter @ariatype/website lint
```

---

## Architecture

### Routes (`src/app/[lang]/`)

Localized dynamic routes with `en` and `zh` languages:

| Route | File | Purpose |
|-------|------|---------|
| `/` | `src/app/[lang]/page.tsx` | Homepage |
| `/features` | `src/app/[lang]/features/page.tsx` | Features showcase |
| `/download` | `src/app/[lang]/download/page.tsx` | Download page |
| `/privacy` | `src/app/[lang]/privacy/page.tsx` | Privacy policy |
| `/terms` | `src/app/[lang]/terms/page.tsx` | Terms of service |

**Root Redirect**: `src/app/page.tsx` redirects to default language

### Components (`src/components/`)

| Component | Purpose |
|-----------|---------|
| `Navbar.tsx` | Navigation header |
| `Footer.tsx` | Site footer |
| `HomeDownloadButton.tsx` | Homepage download CTA |
| `AnalyticsProvider.tsx` | Aptabase analytics wrapper |
| `Typewriter.tsx` | Animated text effect |
| `I18nProvider.tsx` | i18next provider wrapper |

### Hooks (`src/hooks/`)

| Hook | Purpose |
|------|---------|
| `useRelease.ts` | Fetch release info from GitHub |
| `useDownload.ts` | Handle download actions |

### Utilities (`src/lib/`)

| File | Purpose |
|------|---------|
| `events.ts` | Event definitions |
| `analytics.ts` | Analytics utilities |

### i18n (`src/i18n/`)

**2 Supported Locales**: `en`, `zh`

| File | Language |
|------|----------|
| `src/i18n/locales/en.json` | English |
| `src/i18n/locales/zh.json` | Chinese (Simplified) |

**Note**: Website has 2 locales (marketing content), while desktop has 10 (full UI).

---

## Code Style

- TypeScript strict mode
- React 18 functional components
- Tailwind CSS for styling
- All identifiers and comments in **English**
- User-facing text via i18n (update both `en.json` and `zh.json`)

**Localization Rules**:
- Add new content to **both** locale files
- Use react-i18next `useTranslation` hook
- Marketing content differs from desktop UI content

---

## Static Export Constraints

From `next.config.mjs`:

```javascript
const nextConfig = {
  output: 'export',  // Production: static export
  trailingSlash: true,
  images: { unoptimized: true },
  // TypeScript/ESLint errors ignored during build
};
```

**Implications**:
- No server-side features (API routes, SSR, ISR)
- No dynamic image optimization
- All pages pre-rendered at build time
- GitHub API handled via **Cloudflare Pages Functions** (not Next.js API routes)

---

## Deployment

### Cloudflare Pages

```bash
# Deploy to production
pnpm --filter @ariatype/website deploy

# Deploy preview branch
pnpm --filter @ariatype/website deploy:preview
```

**Environment Variables** (set in Cloudflare dashboard):
- `GITHUB_REPO` — Repository identifier for release API

**Deployment Flow**:
1. `pnpm build` generates static `out/` directory
2. Wrangler uploads to Cloudflare Pages
3. Cloudflare Functions handle `/api/release` endpoint

---

## Testing

**Build check serves as primary validation**:

```bash
pnpm --filter @ariatype/website build
```

**Note**: TypeScript/ESLint errors are ignored during build (`ignoreBuildErrors: true`, `ignoreDuringBuilds: true`) for flexibility. Manual testing recommended before deployment.

---

## Adding New Pages

1. Create page file in `src/app/[lang]/new-page/page.tsx`
2. Add i18n keys to `en.json` and `zh.json`
3. Update `Navbar.tsx` if page should be in navigation
4. Build and test locally: `pnpm --filter @ariatype/website dev`
5. Deploy: `pnpm --filter @ariatype/website deploy`

---

## See Also

- **Root AGENTS.md** — Monorepo guidelines, TDD workflow
- **packages/shared/CONTRIBUTING.md** — Shared types and constants
- **apps/desktop/CONTRIBUTING.md** — Desktop application (main product)