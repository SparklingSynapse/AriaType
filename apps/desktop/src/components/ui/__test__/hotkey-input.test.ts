import { describe, expect, it } from "vitest";
import {
  eventToHotkeyString,
  HOTKEY_MODIFIER_ONLY_ERROR,
  HOTKEY_UNSUPPORTED_KEY_ERROR,
  validateHotkeyString,
} from "../hotkey-input";

describe("validateHotkeyString", () => {
  it.each([
    "a",
    "ctrl+a",
    "ctrl+shift+a",
    "ctrl+shift+alt+x",
    "ctrl+shift+alt+cmd+x",
    "space",
    "f1",
    "enter",
    "comma",
    "ctrl+slash",
    "cmd+backquote",
  ])("accepts %s", (hotkey) => {
    expect(validateHotkeyString(hotkey).valid).toBe(true);
  });

  it.each(["cmd", "ctrl", "shift", "alt"])(
    "rejects modifier-only hotkey %s",
    (hotkey) => {
      const result = validateHotkeyString(hotkey);
      expect(result.valid).toBe(false);
      expect(result.error).toBe(HOTKEY_MODIFIER_ONLY_ERROR);
    },
  );

  it.each([
    "ctrlleft",
    "ctrlright",
    "altleft",
    "altright",
    "shiftleft",
    "shiftright",
    "cmdleft",
    "cmdright",
  ])("rejects sided modifier %s", (hotkey) => {
    const result = validateHotkeyString(hotkey);
    expect(result.valid).toBe(false);
    expect(result.error).toBe(HOTKEY_MODIFIER_ONLY_ERROR);
  });

  it.each(["", null])("rejects invalid empty input %s", (hotkey) => {
    expect(validateHotkeyString(hotkey).valid).toBe(false);
  });

  it("rejects more than 5 keys", () => {
    expect(validateHotkeyString("ctrl+shift+alt+cmd+x+v").valid).toBe(false);
  });

  it("rejects duplicate modifiers", () => {
    const result = validateHotkeyString("ctrl+ctrl+a");
    expect(result.valid).toBe(false);
    expect(result.error).toContain("Duplicate");
  });

  it("rejects unknown modifiers", () => {
    const result = validateHotkeyString("unknown+a");
    expect(result.valid).toBe(false);
    expect(result.error).toContain("Unknown");
  });

  it("rejects unsupported keys", () => {
    const result = validateHotkeyString("ctrl+unknownkey123");
    expect(result.valid).toBe(false);
    expect(result.error).toBe(HOTKEY_UNSUPPORTED_KEY_ERROR);
  });
});

describe("eventToHotkeyString", () => {
  it.each([
    [
      { metaKey: false, ctrlKey: true, altKey: false, shiftKey: true, code: "KeyA", key: "a" },
      "ctrl+shift+a",
    ],
    [
      { metaKey: false, ctrlKey: false, altKey: false, shiftKey: true, code: "Space", key: " " },
      "shift+space",
    ],
    [
      { metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, code: "ArrowUp", key: "ArrowUp" },
      "arrowup",
    ],
    [
      { metaKey: false, ctrlKey: false, altKey: true, shiftKey: false, code: "KeyA", key: "a" },
      "alt+a",
    ],
    [
      { metaKey: false, ctrlKey: true, altKey: false, shiftKey: false, code: "Comma", key: "," },
      "ctrl+comma",
    ],
    [
      { metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, code: "NumpadEnter", key: "Enter" },
      "numpadenter",
    ],
  ])("normalizes keyboard event to %s", (event, expected) => {
    expect(eventToHotkeyString(event as React.KeyboardEvent<HTMLInputElement>)).toBe(expected);
  });

  it.each([
    { metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, code: "MetaLeft", key: "Meta" },
    { metaKey: false, ctrlKey: false, altKey: true, shiftKey: false, code: "AltLeft", key: "Alt" },
    { metaKey: false, ctrlKey: true, altKey: false, shiftKey: false, code: "ControlRight", key: "Control" },
    { metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, code: "IntlRo", key: "ろ" },
  ])("returns null for unsupported event %j", (event) => {
    expect(eventToHotkeyString(event as React.KeyboardEvent<HTMLInputElement>)).toBe(null);
  });
});
