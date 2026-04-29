# TypeScript/React Style Guide

TypeScript and React conventions for the AriaType desktop application.

---

## TypeScript Configuration

Strict mode enforced:

```json
{
  "strict": true,
  "noUnusedLocals": true,
  "noUnusedParameters": true,
  "noFallthroughCasesInSwitch": true,
  "isolatedModules": true,
  "lib": ["ES2020", "DOM", "DOM.Iterable"],
  "jsx": "react-jsx"
}
```

All code must pass strict type checking. No implicit `any` without justification.

---

## Imports

Use `@/` alias. **Never** relative paths with `..`.

```typescript
// ✓ Correct
import { settingsCommands } from "@/lib/tauri";

// ✗ Forbidden
import { settingsCommands } from "../lib/tauri";
```

Group imports: React core → third-party → app internals.

---

## React 19 Components

Functional components only.

```typescript
export function Dashboard() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<DashboardStats | null>(null);
  
  useEffect(() => {
    historyCommands.getDashboardStats().then(setStats);
  }, []);
  
  return <div>{/* JSX */}</div>;
}
```

Use `React.forwardRef` for reusable UI components. Set `displayName`.

---

## Custom Hooks

Encapsulate stateful logic. Always clean up event listeners.

```typescript
export function useSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    settingsCommands.getSettings().then(setSettings);
    events.onSettingsChanged(setSettings).then(fn => unlisten = fn);
    return () => unlisten?.();
  }, []);

  const updateSetting = useCallback(async (key: string, value: unknown) => {
    await settingsCommands.updateSettings(key, value);
  }, []);

  return { settings, updateSetting };
}
```

---

## State Management

React state + contexts only. No external libraries.

```typescript
const SettingsContext = createContext<SettingsContextType | undefined>(undefined);

export function SettingsProvider({ children }: { children: ReactNode }) {
  return <SettingsContext.Provider value={useSettings()}>{children}</SettingsContext.Provider>;
}

export function useSettingsContext() {
  const context = useContext(SettingsContext);
  if (!context) throw new Error("useSettingsContext must be within SettingsProvider");
  return context;
}
```

---

## IPC Boundary (Critical)

**Never call raw `invoke()`.** All IPC through `src/lib/tauri.ts`.

```typescript
// ✓ Correct
import { settingsCommands } from "@/lib/tauri";
const settings = await settingsCommands.getSettings();

// ✗ Forbidden
import { invoke } from "@tauri-apps/api/core";
await invoke("get_settings");
```

Commands organized into typed groups with logging:

```typescript
export const settingsCommands = {
  getSettings: () => invokeWithLogging<AppSettings>("get_settings"),
};

export const events = {
  onSettingsChanged: (cb: (s: AppSettings) => void) =>
    listen<AppSettings>("settings-changed", e => cb(e.payload)),
};
```

---

## i18n

All user-facing text uses i18n keys. 10 locales: en, zh, ja, ko, fr, de, es, pt, ru, it.

**Key organization:** `category.subcategory.item` (e.g., `dashboard.stats.today`)

**Usage:**

```typescript
const { t } = useTranslation();
return <h1>{t("dashboard.title")}</h1>;
```

**Adding keys:** Add to `en.json` first, then all 10 locale files.

**Forbidden patterns:**

```typescript
// ❌ WRONG - String concatenation cannot be scanned
t(`model.polish.template${template.id}`)
t("model.domain." + domain)

// ✅ RIGHT - Static mapping table
const TEMPLATE_KEY_MAP: Record<string, string> = {
  filler: "model.polish.templateFiller",
  formal: "model.polish.templateFormal",
};
t(TEMPLATE_KEY_MAP[template.id] ?? template.id);
```

**Reason:** Static analysis (`pnpm check:i18n`) scans for literal `t("key")` patterns. Concatenated keys appear unused, causing false positives and potential deletion.

---

## Tailwind CSS

Utility-first in JSX. Theme colors via CSS variables in `index.css`.

```typescript
<div className="rounded-3xl border border-border bg-card px-5">
  <h3 className="text-lg font-semibold text-foreground">{t("title")}</h3>
</div>
```

Use `cn()` for conditional classes:

```typescript
import { cn } from "@/lib/utils";
<div className={cn("rounded-full", isActive && "bg-primary")} />
```

---

## Error Handling

Structured logging. **Never** swallow errors.

```typescript
try {
  await settingsCommands.getSettings();
} catch (err) {
  logger.error("failed_to_load_settings", { error: String(err) });
  throw err;
}
```

Empty catch blocks forbidden.

---

## Testing

Vitest + React Testing Library. Mock IPC.

```typescript
vi.mock("@/lib/tauri", () => ({
  historyCommands: { getDashboardStats: vi.fn() },
}));

it("renders data", async () => {
  vi.mocked(historyCommands.getDashboardStats).mockResolvedValue({ total_count: 14 });
  render(<Dashboard />);
  await waitFor(() => expect(screen.getByText("14")).toBeInTheDocument());
});
```

---

## Types

**Shared types:** `packages/shared/src/types.ts`

**App types:** `src/types/index.ts`

**IPC types:** `src/lib/tauri.ts`

```typescript
export type RecordingStatus = "idle" | "recording" | "transcribing" | "error";

export interface AppSettings {
  hotkey: string;
  model: string;
  theme_mode: "system" | "light" | "dark";
}
```

---

## File Structure

```
src/
├── components/
│   ├── Home/Dashboard.tsx
│   └── ui/button.tsx
├── hooks/
│   ├── useRecording.ts
│   └── useSettings.ts
├── lib/
│   ├── tauri.ts      # IPC boundary (CRITICAL)
│   ├── logger.ts
│   └── utils.ts      # cn(), formatDate()
├── types/index.ts
└── i18n/
    ├── index.ts
    └── locales/en.json
```

---

## Forbidden Patterns

```typescript
// ✗ Never use
as any
@ts-ignore
invoke("command_name")        // raw IPC
catch {}                      // empty catch
import { x } from "../lib/x"; // relative imports
```

If breaking rules: document justification, keep scope minimal.

---

## Enforcement

1. TypeScript strict mode
2. oxlint
3. CI build/test checks
4. Code review

Conventions not mechanically enforced are suggestions. Encode rules in linters or tests.