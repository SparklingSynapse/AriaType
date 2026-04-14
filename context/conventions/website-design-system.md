# AriaType Website — UI Design System

> Source of truth for visual and interaction patterns across ariatype.com.
> When in doubt, match what exists. When adding new, follow these tokens.

---

## 1. Color

### Semantic Palette (CSS Custom Properties)

All colors are Tailwind-native via `tailwind.config.js` → `globals.css`.

#### Light Mode

| Token | Hex | Swatch | Role |
|-------|-----|--------|------|
| `--background` | `#faf9f7` | ■ | Page surface |
| `--background-hover` | `#f0eeeb` | ■ | Interactive hover fill |
| `--foreground` | `#1c1917` | ■ | Primary text, primary button fill |
| `--card` | `#ffffff` | ■ | Card / popover surface |
| `--primary` | `#1c1917` | ■ | CTA buttons, strong emphasis |
| `--primary-foreground` | `#fafaf9` | ■ | Text on primary |
| `--secondary` | `#e7e5e4` | ■ | Secondary fills, tags |
| `--muted-foreground` | `#78716c` | ■ | Body copy, descriptions |
| `--border` | `#e7e5e4` | ■ | Lines, outlines |
| `--input` | `#d6d3d1` | ■ | Input borders |

#### Dark Mode

| Token | Hex | Role |
|-------|-----|------|
| `--background` | `#1c1917` | Page surface |
| `--background-hover` | `#292524` | Interactive hover fill |
| `--foreground` | `#e7e5e4` | Primary text |
| `--card` | `#292524` | Card / popover surface |
| `--primary` | `#e7e5e4` | CTA buttons |
| `--secondary` | `#44403c` | Secondary fills |
| `--border` | `#44403c` | Borders |

#### Accent Colors (same in both modes)

| Token | Hex | Usage |
|-------|-----|-------|
| `--accent-green` | `#4ade80` | Success indicator (download checkmark) |
| `--accent-blue` | `#60a5fa` | Feature card icon tint |
| `--accent-amber` | `#fbbf24` | Feature card icon tint |
| `--accent-purple` | `#c084fc` | Reserved (currently unused) |

**Palette origin**: Tailwind Stone (warm neutral). No cool grays.

### Opacity Convention

Use `bg-foreground/10`, `text-foreground/60`, `border-border/50` for layered effects. All colors support `/opacity` via `color-mix()` in `tailwind.config.js`.

### Inconsistencies to Fix

| Location | Current | Should Be |
|----------|---------|-----------|
| `features/page.tsx` | `bg-blue-500/10`, `bg-rose-500/10`, etc. (hardcoded Tailwind colors) | Use `accent-*` tokens or define per-feature brand colors |
| `globals.css` scrollbar | `rgba(0,0,0,0.18)` hardcoded | Use `foreground/18` |

---

## 2. Typography

### Font Stack

```css
font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
```

Single family. No custom fonts. Platform-native rendering.

### Type Scale

| Level | Classes | Size | Weight | Tracking | Line Height | Usage |
|-------|---------|------|--------|----------|-------------|-------|
| **Display** | `text-[clamp(2.25rem,5.5vw,4.25rem)] font-semibold tracking-[-0.04em] leading-[1.08]` | 36–68px | 600 | −0.04em | 1.08 | Home hero title |
| **H1** | `text-[clamp(2.5rem,5vw,4.5rem)] font-bold tracking-tight leading-[1.05]` | 40–72px | 700 | tight | 1.05 | Download page title |
| **H1 (inner)** | `text-4xl md:text-5xl font-bold` | 36/45px | 700 | — | — | Features page title |
| **H2** | `text-3xl md:text-4xl font-semibold tracking-[-0.04em]` | 30/36px | 600 | −0.04em | — | Section headings |
| **H2 (features)** | `text-3xl font-bold` | 30px | 700 | — | — | Features page section headings |
| **H3** | `text-xl font-semibold tracking-[-0.03em]` | 20px | 600 | −0.03em | — | Step titles, card headings |
| **H3 (card)** | `text-base font-semibold` | 16px | 600 | — | — | Feature card titles, download card titles |
| **Eyebrow** | `text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground` | 12px | 500 | +0.2em | — | Section labels |
| **Body** | `text-base leading-8 text-muted-foreground` | 16px | 400 | — | 2rem | Descriptions |
| **Body small** | `text-sm leading-7 text-muted-foreground` | 14px | 400 | — | 1.75rem | Feature descriptions |
| **Caption** | `text-sm text-muted-foreground` | 14px | 400 | — | — | Footnotes, metadata |
| **Nav link** | `text-sm tracking-wide` | 14px | 400 | wide | — | Navbar links |
| **Lang label** | `text-xs uppercase tracking-wider` | 12px | 400 | wider | — | Language indicator |

### Weight Rules

| Weight | When to Use |
|--------|-------------|
| 400 (normal) | Body text, descriptions, nav links |
| 500 (medium) | Eyebrow labels, secondary CTAs, selected states |
| 600 (semibold) | Headlines (H1–H3 on home page), step titles |
| 700 (bold) | Inner page titles (features, download), legal headings |

---

## 3. Spacing

### Section Rhythm

| Context | Mobile | Desktop (`md:`) |
|---------|--------|------------------|
| Hero top | `pt-32` (8rem) | `md:pt-44` (11rem) |
| Hero bottom | `pb-16` (4rem) | `md:pb-24` (6rem) |
| Content sections | `py-20` (5rem) | `md:py-28` (7rem) |
| Video showcase | `pb-20` (5rem) | `md:pb-28` (7rem) |
| Inner pages | `py-24` (6rem) | — |

### Container Widths

| Tier | Max-Width | Usage |
|------|-----------|-------|
| Narrow | `max-w-2xl` (672px) | Features CTA, prose body |
| Standard | `max-w-3xl` (768px) | Closing CTA section |
| Content | `max-w-4xl` (896px) | Hero text, workflow, legal pages, download |
| Wide | `max-w-5xl` (1024px) | Navbar, footer, video showcase |
| Full | `max-w-6xl` (1152px) | Feature sections (two-column) |

All containers: `mx-auto px-6` (24px side padding).

### Internal Spacing

| Context | Value | Tailwind |
|---------|-------|----------|
| Heading → eyebrow label | 1rem | `mt-4` |
| Heading → description | 1rem | `mt-4` |
| Description → feature list | 2.5rem | `mt-10` |
| Feature item gap | 2rem | `space-y-8` |
| Button group gap | 0.75rem | `gap-3` |
| Grid → two-column gap | 4rem | `gap-16` |
| Grid → three-column gap | 2rem | `md:gap-8` |
| Card padding | 2rem | `p-8` |
| Section label → heading | 1rem | `mt-4` |

---

## 4. Layout

### Page Shell

```
┌──────────────────────────────────────┐
│ Navbar (fixed, h-14, z-50)          │ max-w-5xl mx-auto px-6
├──────────────────────────────────────┤
│                                      │
│ main (flex-1 pt-16)                  │
│                                      │
│   ┌─── Section ──────────────────┐   │
│   │ max-w-{tier} mx-auto px-6    │   │
│   │                              │   │
│   │ py-20 md:py-28               │   │
│   └──────────────────────────────┘   │
│                                      │
├──────────────────────────────────────┤
│ Footer (border-t border-border/50)   │ max-w-5xl mx-auto px-6 py-6
└──────────────────────────────────────┘
```

### Grid Patterns

| Pattern | Classes | Usage |
|---------|---------|-------|
| Three-column | `grid gap-12 md:grid-cols-3 md:gap-8` | Workflow steps |
| Two-column (alternating) | `grid items-center gap-16 lg:grid-cols-2` | Feature image + text |
| Card grid | `grid md:grid-cols-2 lg:grid-cols-3 gap-6` | Feature cards |
| Download grid | `grid md:grid-cols-2 gap-6` | Platform cards |

### Responsive Breakpoints

Standard Tailwind defaults. No custom `screens`.

| Breakpoint | Width | Role |
|------------|-------|------|
| `sm` | 640px | Button row (`sm:flex-row`) |
| `md` | 768px | Grid columns activate, navbar links show |
| `lg` | 1024px | Two-column feature layouts |

**Strategy**: Mobile-first. Layout shifts at `md` (grids, nav) and `lg` (alternating sections).

---

## 5. Components

### Button — Primary CTA

```tsx
className="inline-flex h-11 items-center justify-center rounded-full
  bg-primary px-6 text-sm font-medium text-primary-foreground
  transition-all hover:bg-primary/90"
```

- Height: `h-11` (44px)
- Shape: `rounded-full` (pill)
- Touch target: ≥ 44px
- Used by: `HomeDownloadButton`, download page CTA

### Button — Secondary / Outline

```tsx
className="inline-flex h-11 items-center justify-center rounded-full
  border border-border bg-card px-6 text-sm font-medium text-foreground
  transition-colors hover:bg-secondary"
```

- Same dimensions as primary
- Border instead of fill
- Used by: "View source on GitHub" link

### Button — Ghost (Language Trigger)

```tsx
className="flex items-center gap-1.5 rounded-md px-2 py-1 text-sm
  text-foreground/60 hover:bg-secondary hover:text-foreground
  transition-colors"
```

### Section Label (Eyebrow)

```tsx
<p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
  {children}
</p>
```

Always paired with `<h2>` below via `mt-4`.

### Feature List Item (with Dot)

```tsx
<div className="flex gap-3">
  <span className="mt-2 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground/40" />
  <div>
    <h3 className="text-base font-medium text-foreground">{title}</h3>
    <p className="mt-1.5 text-sm leading-7 text-muted-foreground">{description}</p>
  </div>
</div>
```

### Popover / Dropdown

Matches desktop app `Select` component pattern:

```tsx
// Container
className="rounded-2xl border border-border bg-card shadow-lg
  transition-all duration-200 origin-top-right"
// Open: scale-100 opacity-100
// Closed: pointer-events-none scale-95 opacity-0

// Items
className="w-full rounded-lg px-3 py-2 text-left text-sm transition-colors"
// Selected: bg-background font-medium text-foreground
// Unselected: text-muted-foreground hover:bg-background-hover hover:text-foreground
```

### Card (Feature)

```tsx
className="p-8 rounded-xl bg-card border border-border
  hover:border-border/80 hover:shadow-md transition-all duration-200"
```

### Card (Download Platform)

```tsx
className="p-8 rounded-[1.5rem] bg-card border border-border"
```

> ⚠️ `rounded-[1.5rem]` should be `rounded-3xl` (same value, uses token).

### Step Number Circle

```tsx
<div className="w-10 h-10 rounded-full bg-primary text-primary-foreground
  flex items-center justify-center font-bold text-sm shrink-0">
  {index + 1}
</div>
```

### Image Block

```tsx
<img className="w-full rounded-3xl object-cover" style={{ aspectRatio: "4 / 3" }} />
```

---

## 6. Motion

### Reveal Animation (Framer Motion)

```tsx
const reveal = {
  hidden: { opacity: 0, y: 16 },
  visible: { opacity: 1, y: 0 },
};

const transition = {
  duration: 0.6,
  ease: [0.16, 1, 0.3, 1], // custom ease-out
};
```

**Usage variants:**

| Context | Props |
|---------|-------|
| Hero (page load) | `initial="hidden" animate="visible" transition={{ duration: 0.7 }}` |
| Secondary hero elements | Same + `delay: 0.12` |
| Scroll-triggered sections | `initial="hidden" whileInView="visible" viewport={{ once: true, margin: "-60px" }}` |
| Staggered children | Same + `delay: index * 0.08` |
| Inner pages (features, download) | `initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }}` |

### CSS Animations

```css
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to   { opacity: 1; transform: translateY(0); }
}

@keyframes cursor-blink {
  0%, 100% { opacity: 1; }
  50%      { opacity: 0; }
}
```

### Transition Durations

| Duration | Usage |
|----------|-------|
| 200ms | Micro-interactions (dropdown, chevron rotation) |
| 300ms | Navbar scroll state, theme-level transitions |
| 600ms | Section reveal (default) |
| 700ms | Hero title reveal (slightly longer) |

---

## 7. Icon System

### Lucide Icons

Primary icon set: [Lucide](https://lucide.dev/).

| Icon | Size | Usage |
|------|------|-------|
| Nav links | `w-3.5 h-3.5` (14px) | Globe, ChevronDown |
| Feature cards | `w-5 h-5` (20px) | Lock, Mic, Keyboard, etc. |
| Download options | `w-4 h-4` (16px) | Download, Check |
| GitHub link | `w-4.5 h-4.5` (18px) | Custom SVG path |

### Custom SVG Icons

File: `src/components/Icons.tsx`

7 brand icons (Waveform, Shield, Sparkles, Lock, Hold, Speak, Type). All use `stroke="currentColor"` for theme integration.

### Icon Container Pattern (Feature Cards)

```tsx
<div className="w-11 h-11 rounded-lg flex items-center justify-center mb-4
  bg-{color}-500/10">
  <Icon className="w-5 h-5 text-{color}-500" />
</div>
```

> ⚠️ Uses hardcoded Tailwind colors. Consider migrating to `accent-*` tokens.

---

## 8. Dark Mode

Dark mode enabled via Tailwind `class` strategy (`.dark` selector on `<html>`).

All semantic tokens swap in `globals.css` → components inherit automatically via `bg-*`, `text-*`, `border-*`.

**No component-level dark mode overrides needed** when using semantic tokens.

---

## 9. Known Inconsistencies

| Issue | Location | Recommendation |
|-------|----------|----------------|
| Hardcoded Tailwind colors for icons | `features/page.tsx` | Migrate to `accent-*` or define `--icon-{name}` tokens |
| `rounded-[1.5rem]` instead of `rounded-3xl` | `DownloadClient.tsx` | Use token class |
| Inline `style={{ aspectRatio: "4 / 3" }}` | `page.tsx` | Use Tailwind `aspect-[4/3]` class |
| Scrollbar uses hardcoded RGBA | `globals.css` | Use `foreground/18` opacity syntax |
| `--accent-purple` defined but unused | `globals.css` | Remove or assign usage |
| Features page uses `font-bold` (700) for H2 | `features/page.tsx` | Home uses `font-semibold` (600) — align |
| Features page H1 is `text-4xl md:text-5xl` | `features/page.tsx` | Home uses `clamp()` — consider alignment |
| Download page has its own `clamp()` range | `DownloadClient.tsx` | Consolidate with hero display size |

---

## 10. File Reference

| File | Purpose |
|------|---------|
| `src/app/globals.css` | CSS custom properties, scrollbar styles, keyframes |
| `tailwind.config.js` | Theme extensions (colors, radius, font) |
| `src/app/[lang]/layout.tsx` | Page shell (navbar + main + footer) |
| `src/app/[lang]/page.tsx` | Homepage layout, `SectionLabel`, animation constants |
| `src/components/Navbar.tsx` | Fixed nav, language switcher |
| `src/components/Footer.tsx` | Footer |
| `src/components/HomeDownloadButton.tsx` | Primary CTA |
| `src/components/Icons.tsx` | Custom SVG icons |
| `src/app/[lang]/features/page.tsx` | Feature cards, step cards |
| `src/app/[lang]/download/DownloadClient.tsx` | Download cards, platform icons |
