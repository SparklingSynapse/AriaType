import { useState, useRef, useCallback } from "react";
import { cn } from "@/lib/utils";
import { settingsCommands } from "@/lib/tauri";

const MODIFIER_CODES = new Set([
  "MetaLeft", "MetaRight", "ControlLeft", "ControlRight",
  "AltLeft", "AltRight", "ShiftLeft", "ShiftRight"
]);

const SPECIAL_KEYS: Record<string, string> = {
  Space: "space", Enter: "enter", Backspace: "backspace", Tab: "tab",
  Escape: "escape", ArrowUp: "arrowup", ArrowDown: "arrowdown",
  ArrowLeft: "arrowleft", ArrowRight: "arrowright", Delete: "delete",
  Home: "home", End: "end", PageUp: "pageup", PageDown: "pagedown",
  Insert: "insert", CapsLock: "capslock", PrintScreen: "printscreen",
  ScrollLock: "scrolllock", Pause: "pause", Minus: "minus",
  Equal: "equal", BracketLeft: "bracketleft", BracketRight: "bracketright",
  Backslash: "backslash", Semicolon: "semicolon", Quote: "quote",
  Backquote: "backquote", Comma: "comma", Period: "period",
  Slash: "slash", AudioVolumeDown: "audiovolumedown",
  AudioVolumeUp: "audiovolumeup", AudioVolumeMute: "audiovolumemute",
  MediaPlay: "mediaplay", MediaPause: "mediapause",
  MediaPlayPause: "mediaplaypause", MediaStop: "mediastop",
  MediaTrackNext: "mediatracknext", MediaTrackPrevious: "mediatrackprev",
  NumpadAdd: "numpadadd", NumpadDecimal: "numpaddecimal",
  NumpadDivide: "numpaddivide", NumpadEnter: "numpadenter",
  NumpadEqual: "numpadequal", NumpadMultiply: "numpadmultiply",
  NumpadSubtract: "numpadsubtract", NumLock: "numlock",
};

const MIN_KEYS = 1;
const MAX_KEYS = 5;
const VALID_MODIFIERS = ["ctrl", "alt", "shift", "cmd"];

const MODIFIER_ONLY_KEYS = new Set([
  "ctrl", "alt", "shift", "cmd", "command", "meta", "control",
  "ctrlleft", "ctrlright", "controlleft", "controlright",
  "altleft", "altright", "shiftleft", "shiftright",
  "cmdleft", "cmdright", "metaleft", "metaright",
]);

const HOTKEY_MODIFIER_ONLY_ERROR = "Global shortcuts require a key (e.g., Space, A-Z, F1-F12). Modifier keys alone are not supported by the system.";
const HOTKEY_UNSUPPORTED_KEY_ERROR = "This key is not supported for global shortcuts.";

const HOTKEY_LABELS: Record<string, string> = {
  cmd: "Cmd",
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  space: "Space",
  enter: "Enter",
  backspace: "Backspace",
  tab: "Tab",
  escape: "Escape",
  arrowup: "ArrowUp",
  arrowdown: "ArrowDown",
  arrowleft: "ArrowLeft",
  arrowright: "ArrowRight",
  delete: "Delete",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  insert: "Insert",
  capslock: "CapsLock",
  printscreen: "PrintScreen",
  scrolllock: "ScrollLock",
  pause: "Pause",
  minus: "Minus",
  equal: "Equal",
  bracketleft: "BracketLeft",
  bracketright: "BracketRight",
  backslash: "Backslash",
  semicolon: "Semicolon",
  quote: "Quote",
  backquote: "Backquote",
  comma: "Comma",
  period: "Period",
  slash: "Slash",
  numlock: "NumLock",
  numpadadd: "NumpadAdd",
  numpaddecimal: "NumpadDecimal",
  numpaddivide: "NumpadDivide",
  numpadenter: "NumpadEnter",
  numpadequal: "NumpadEqual",
  numpadmultiply: "NumpadMultiply",
  numpadsubtract: "NumpadSubtract",
  audiovolumedown: "AudioVolumeDown",
  audiovolumeup: "AudioVolumeUp",
  audiovolumemute: "AudioVolumeMute",
  mediaplay: "MediaPlay",
  mediapause: "MediaPause",
  mediaplaypause: "MediaPlayPause",
  mediastop: "MediaStop",
  mediatracknext: "MediaTrackNext",
  mediatrackprev: "MediaTrackPrev",
};

interface ValidationResult {
  valid: boolean;
  error?: string;
}

function validateHotkeyString(hotkey: string | null): ValidationResult {
  if (!hotkey) {
    return { valid: false, error: "No keys pressed" };
  }

  const parts = hotkey.split("+").map((part) => part.trim());

  if (parts.some((part) => part.length === 0)) {
    return { valid: false, error: "Invalid hotkey format" };
  }

  if (parts.length < MIN_KEYS || parts.length > MAX_KEYS) {
    return { valid: false, error: `Must have ${MIN_KEYS}-${MAX_KEYS} keys` };
  }

  const modifiers = parts.slice(0, -1);
  const modifierSet = new Set<string>();

  for (const mod of modifiers) {
    const normalizedModifier = mod.toLowerCase();
    if (modifierSet.has(normalizedModifier)) {
      return { valid: false, error: `Duplicate modifier: ${mod}` };
    }
    if (!VALID_MODIFIERS.includes(normalizedModifier)) {
      return { valid: false, error: `Unknown modifier: ${mod}` };
    }
    modifierSet.add(normalizedModifier);
  }

  const key = parts[parts.length - 1].toLowerCase();
  if (MODIFIER_ONLY_KEYS.has(key)) {
    return { 
      valid: false, 
      error: HOTKEY_MODIFIER_ONLY_ERROR,
    };
  }

  if (!isSupportedHotkeyKey(key)) {
    return {
      valid: false,
      error: HOTKEY_UNSUPPORTED_KEY_ERROR,
    };
  }

  return { valid: true };
}

function eventToHotkeyString(e: React.KeyboardEvent): string | null {
  if (MODIFIER_CODES.has(e.code)) {
    return null;
  }

  const parts: string[] = [];
  if (e.metaKey) parts.push("cmd");
  if (e.ctrlKey) parts.push("ctrl");
  if (e.altKey) parts.push("alt");
  if (e.shiftKey) parts.push("shift");

  const keyName = getKeyNameFromCode(e.code);
  if (!keyName) {
    return null;
  }
  parts.push(keyName);

  return parts.length > 0 ? parts.join("+") : null;
}

function formatHotkey(hotkey: string): string {
  return hotkey
    .split("+")
    .map((part) => {
      const normalizedPart = part.toLowerCase();
      if (HOTKEY_LABELS[normalizedPart]) {
        return HOTKEY_LABELS[normalizedPart];
      }
      if (normalizedPart.length === 1) {
        return normalizedPart.toUpperCase();
      }
      return normalizedPart.charAt(0).toUpperCase() + normalizedPart.slice(1);
    })
    .join("+");
}

function getKeyNameFromCode(code: string): string | null {
  if (SPECIAL_KEYS[code]) {
    return SPECIAL_KEYS[code];
  }

  if (code.startsWith("Key") && code.length === 4) {
    return code.slice(3).toLowerCase();
  }

  if (code.startsWith("Digit") && code.length === 6) {
    return code.slice(5);
  }

  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) {
    return code.toLowerCase();
  }

  if (/^Numpad[0-9]$/.test(code)) {
    return code.toLowerCase();
  }

  return null;
}

function isSupportedHotkeyKey(key: string): boolean {
  return (
    key.length === 1 ||
    /^f([1-9]|1\d|2[0-4])$/.test(key) ||
    /^numpad[0-9]$/.test(key) ||
    Boolean(HOTKEY_LABELS[key])
  );
}

interface HotkeyInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
}

export function HotkeyInput({ value, onChange, placeholder, className }: HotkeyInputProps) {
  const [isRecording, setIsRecording] = useState(false);
  const [recordedKey, setRecordedKey] = useState<string | null>(null);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [modifierPressed, setModifierPressed] = useState(false);
  const stoppingRef = useRef(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const startRecording = async () => {
    if (isRecording) return;
    setRecordedKey(null);
    setValidationError(null);
    setModifierPressed(false);
    await settingsCommands.setHotkeyCaptureMode(true);
    setIsRecording(true);
  };

  const stopRecording = useCallback(async (keyToSave: string | null = null) => {
    if (stoppingRef.current) return;
    stoppingRef.current = true;
    setIsRecording(false);
    setRecordedKey(null);
    await settingsCommands.setHotkeyCaptureMode(false);
    
    if (keyToSave) {
      const validation = validateHotkeyString(keyToSave);
      if (!validation.valid) {
        setValidationError(validation.error || "Invalid hotkey");
        stoppingRef.current = false;
        return;
      }
      setValidationError(null);
      onChange(keyToSave);
    }
    stoppingRef.current = false;
  }, [onChange]);

  const handleKeyDown = async (e: React.KeyboardEvent) => {
    if (!isRecording) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.code === "Escape" && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
      setValidationError(null);
      setModifierPressed(false);
      await stopRecording();
      inputRef.current?.blur();
      return;
    }

    const hotkey = eventToHotkeyString(e);
    if (hotkey) {
      const validation = validateHotkeyString(hotkey);
      if (!validation.valid) {
        setValidationError(validation.error || "Invalid hotkey");
        return;
      }
      setValidationError(null);
      setModifierPressed(false);
      setRecordedKey(hotkey);
    } else if (!MODIFIER_CODES.has(e.code)) {
      setModifierPressed(false);
      setValidationError(HOTKEY_UNSUPPORTED_KEY_ERROR);
    } else {
      setModifierPressed(true);
      setValidationError(null);
    }
  };

  const handleKeyUp = async () => {
    if (!isRecording) return;

    if (recordedKey) {
      await stopRecording(recordedKey);
      inputRef.current?.blur();
    } else if (modifierPressed) {
      setValidationError(HOTKEY_MODIFIER_ONLY_ERROR);
      setModifierPressed(false);
    }
  };

  const handleBlur = async () => {
    if (isRecording) await stopRecording(recordedKey);
  };

  const isError = validationError && isRecording;

  return (
    <div className="space-y-1">
      <input
        ref={inputRef}
        value={isRecording
          ? (recordedKey ? formatHotkey(recordedKey) : (placeholder || "Press keys..."))
          : formatHotkey(value)}
        onMouseDown={startRecording}
        onFocus={startRecording}
        onBlur={handleBlur}
        onKeyDown={handleKeyDown}
        onKeyUp={handleKeyUp}
        readOnly
        className={cn(
          "cursor-pointer font-mono transition-all w-full",
          isRecording && "border-primary ring-2 ring-primary/20",
          isError && "border-destructive focus-visible:ring-destructive",
          className
        )}
      />
      {isError && (
        <p className="text-xs text-destructive">{validationError}</p>
      )}
    </div>
  );
}

export {
  eventToHotkeyString,
  formatHotkey,
  validateHotkeyString,
  HOTKEY_MODIFIER_ONLY_ERROR,
  HOTKEY_UNSUPPORTED_KEY_ERROR,
};
