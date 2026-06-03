import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { hotkeyCommands } from "@/lib/tauri";
import { showErrorToast } from "@/lib/toast";

import { formatHotkeyForPlatform, HotkeyInput, HotkeyTags } from "../hotkey-input";

vi.mock("@/lib/tauri", () => ({
  hotkeyCommands: {
    startCapture: vi.fn(),
    stopCapture: vi.fn(),
    cancelCapture: vi.fn(),
  },
  events: {
    onHotkeyCaptured: vi.fn(() => Promise.resolve(() => {})),
    onShortcutRegistrationFailed: vi.fn(() => Promise.resolve(() => {})),
  },
}));

vi.mock("@/lib/toast", () => ({
  showErrorToast: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("formatHotkey", () => {
  // Format tests - frontend only handles display formatting
  // Now uses symbols for better visual representation
  it.each([
    ["ctrl+a", "Ctrl+A"],
    ["ctrl+shift+a", "Ctrl+⇧+A"],
    ["cmd+shift+space", "⌘+⇧+Space"],
    ["fn", "Fn"],
    ["fn+space", "Fn+Space"],
    ["cmd+fn+a", "⌘+Fn+A"],
    ["f1", "F1"],
    ["f12", "F12"],
    ["f20", "F20"],
    // Side-specific modifiers stay visible in the UI
    ["cmdright+slash", "R⌘+/"],
    ["cmdleft+a", "L⌘+A"],
    ["ctrlright+space", "RCtrl+Space"],
    ["shiftleft+a", "L⇧+A"],
    ["optright+b", "R⌥+B"],
    // Combinations
    ["shiftright+cmdright+space", "R⇧+R⌘+Space"],
    ["ctrlleft+optleft+a", "LCtrl+L⌥+A"],
    // Special keys - using symbols
    ["ctrl+enter", "Ctrl+↵"],
    ["ctrl+escape", "Ctrl+Esc"],
    ["ctrl+arrowup", "Ctrl+↑"],
    // Slash - direct symbol
    ["slash", "/"],
    ["ctrl+slash", "Ctrl+/"],
    ["ctrl+/", "Ctrl+/"],
    ["/", "/"],
    // Additional special keys
    ["cmd+backspace", "⌘+⌫"],
    ["cmd+tab", "⌘+⇥"],
    ["cmd+capslock", "⌘+⇪"],
  ])("formats %s as %s", (input, expected) => {
    expect(formatHotkeyForPlatform(input, "macos")).toBe(expected);
  });

  it("handles case insensitive input", () => {
    expect(formatHotkeyForPlatform("CMD+A", "macos")).toBe("⌘+A");
    expect(formatHotkeyForPlatform("ctrl+SHIFT+Space", "macos")).toBe("Ctrl+⇧+Space");
    expect(formatHotkeyForPlatform("CMDRIGHT+SLASH", "macos")).toBe("R⌘+/");
  });

  it("capitalizes unknown keys", () => {
    expect(formatHotkeyForPlatform("unknownkey", "macos")).toBe("Unknownkey");
  });

  it.each([
    ["cmd+slash", "Win+/"],
    ["cmdleft+slash", "LWin+/"],
    ["cmdright+slash", "RWin+/"],
    ["opt+slash", "Alt+/"],
    ["alt+slash", "Alt+/"],
    ["optright+b", "RAlt+B"],
    ["ctrl+slash", "Ctrl+/"],
  ])("formats Windows hotkey %s as %s", (input, expected) => {
    expect(formatHotkeyForPlatform(input, "windows")).toBe(expected);
  });
});

describe("HotkeyTags", () => {
  it("renders multiple key tags", () => {
    render(<HotkeyTags hotkey="cmdleft+shift+a" platform="macos" />);
    // Should render 3 key tags: L⌘, ⇧, A
    const tags = screen.getAllByText(/L⌘|⇧|A/);
    expect(tags.length).toBe(3);
  });

  it("renders single key", () => {
    render(<HotkeyTags hotkey="fn" platform="macos" />);
    expect(screen.getByText("Fn")).toBeInTheDocument();
  });

  it("renders special key symbols", () => {
    render(<HotkeyTags hotkey="cmd+enter" platform="macos" />);
    expect(screen.getByText("⌘")).toBeInTheDocument();
    expect(screen.getByText("↵")).toBeInTheDocument();
  });

  it("renders platform-native modifier labels on Windows", () => {
    render(<HotkeyTags hotkey="cmd+opt+slash" platform="windows" />);
    expect(screen.getByText("Win")).toBeInTheDocument();
    expect(screen.getByText("Alt")).toBeInTheDocument();
    expect(screen.getByText("/")).toBeInTheDocument();
  });
});

describe("HotkeyInput", () => {
  it("shows a visible error when capture startup fails", async () => {
    vi.mocked(hotkeyCommands.startCapture).mockRejectedValue(
      new Error("windows shortcut hook install failed"),
    );

    render(<HotkeyInput value="" onChange={vi.fn()} profileKey="dictate" />);

    fireEvent.mouseDown(screen.getByRole("button", { name: "hotkey.clickToSet" }));

    await waitFor(() => {
      expect(screen.getByText("windows shortcut hook install failed")).toBeInTheDocument();
    });
    expect(showErrorToast).toHaveBeenCalledWith("Unable to capture hotkey");
  });
});
