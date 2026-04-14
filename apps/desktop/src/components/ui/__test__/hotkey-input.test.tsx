import { describe, expect, it } from "vitest";
import { formatHotkey, HotkeyTags } from "../hotkey-input";
import { render, screen } from "@testing-library/react";

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
    expect(formatHotkey(input)).toBe(expected);
  });

  it("handles case insensitive input", () => {
    expect(formatHotkey("CMD+A")).toBe("⌘+A");
    expect(formatHotkey("ctrl+SHIFT+Space")).toBe("Ctrl+⇧+Space");
    expect(formatHotkey("CMDRIGHT+SLASH")).toBe("R⌘+/");
  });

  it("capitalizes unknown keys", () => {
    expect(formatHotkey("unknownkey")).toBe("Unknownkey");
  });
});

describe("HotkeyTags", () => {
  it("renders multiple key tags", () => {
    render(<HotkeyTags hotkey="cmdleft+shift+a" />);
    // Should render 3 key tags: L⌘, ⇧, A
    const tags = screen.getAllByText(/L⌘|⇧|A/);
    expect(tags.length).toBe(3);
  });

  it("renders single key", () => {
    render(<HotkeyTags hotkey="fn" />);
    expect(screen.getByText("Fn")).toBeInTheDocument();
  });

  it("renders special key symbols", () => {
    render(<HotkeyTags hotkey="cmd+enter" />);
    expect(screen.getByText("⌘")).toBeInTheDocument();
    expect(screen.getByText("↵")).toBeInTheDocument();
  });
});
