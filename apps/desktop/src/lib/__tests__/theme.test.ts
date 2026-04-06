import { beforeEach, describe, expect, it, vi } from "vitest";

import { applyInitialTheme, applyTheme } from "@/lib/theme";

describe("theme bootstrap", () => {
  beforeEach(() => {
    document.documentElement.className = "";
    localStorage.clear();

    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockImplementation((query: string) => ({
        matches: query === "(prefers-color-scheme: dark)",
        media: query,
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    );
  });

  it("uses the cached resolved theme before settings finish loading", () => {
    localStorage.setItem("ariatype-theme", "dark");

    applyInitialTheme();

    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("falls back to the system theme when no cached theme exists", () => {
    applyInitialTheme();

    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("persists the resolved theme when an explicit mode is applied", () => {
    applyTheme("light");

    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(localStorage.getItem("ariatype-theme")).toBe("light");
  });
});
