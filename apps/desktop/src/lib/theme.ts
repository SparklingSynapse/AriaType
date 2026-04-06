export type ThemeMode = "system" | "light" | "dark";

const RESOLVED_THEME_STORAGE_KEY = "ariatype-theme";

function getStoredResolvedTheme(): "light" | "dark" | null {
  try {
    const value = localStorage.getItem(RESOLVED_THEME_STORAGE_KEY);
    return value === "light" || value === "dark" ? value : null;
  } catch {
    return null;
  }
}

export function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function resolveThemeMode(mode: ThemeMode): "light" | "dark" {
  return mode === "system" ? getSystemTheme() : mode;
}

export function applyResolvedTheme(theme: "light" | "dark") {
  document.documentElement.classList.toggle("dark", theme === "dark");
  try {
    localStorage.setItem(RESOLVED_THEME_STORAGE_KEY, theme);
  } catch {
    // Ignore storage failures and still apply the DOM class.
  }
}

export function applyTheme(mode: ThemeMode) {
  applyResolvedTheme(resolveThemeMode(mode));
}

export function applyInitialTheme() {
  const storedTheme = getStoredResolvedTheme();
  applyResolvedTheme(storedTheme ?? getSystemTheme());
}
