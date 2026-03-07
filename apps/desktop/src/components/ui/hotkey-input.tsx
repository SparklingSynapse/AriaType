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
};

function eventToHotkeyString(e: React.KeyboardEvent): string | null {
  const parts: string[] = [];
  if (e.metaKey) parts.push("cmd");
  if (e.ctrlKey) parts.push("ctrl");
  if (e.altKey) parts.push("alt");
  if (e.shiftKey) parts.push("shift");

  if (!MODIFIER_CODES.has(e.code)) {
    let keyName: string;
    if (SPECIAL_KEYS[e.code]) keyName = SPECIAL_KEYS[e.code];
    else if (e.code.startsWith("Key")) keyName = e.code.slice(3).toLowerCase();
    else if (e.code.startsWith("Digit")) keyName = e.code.slice(5);
    else if (/^F([1-9]|1[0-2])$/.test(e.code)) keyName = e.code.toLowerCase();
    else keyName = e.key.toLowerCase();
    parts.push(keyName);
  }

  return parts.length > 0 ? parts.join("+") : null;
}

function formatHotkey(hotkey: string): string {
  return hotkey.split("+").map(p => p.charAt(0).toUpperCase() + p.slice(1)).join("+");
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
  const stoppingRef = useRef(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const startRecording = async () => {
    if (isRecording) return;
    setRecordedKey(null);
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
      onChange(keyToSave);
    }
    stoppingRef.current = false;
  }, [onChange]);

  const handleKeyDown = async (e: React.KeyboardEvent) => {
    if (!isRecording) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.code === "Escape" && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
      await stopRecording();
      inputRef.current?.blur();
      return;
    }

    const hotkey = eventToHotkeyString(e);
    if (hotkey) setRecordedKey(hotkey);
  };

  const handleKeyUp = async () => {
    if (!isRecording || !recordedKey) return;
    await stopRecording(recordedKey);
    inputRef.current?.blur();
  };

  const handleBlur = async () => {
    if (isRecording) await stopRecording(recordedKey);
  };

  return (
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
        "cursor-pointer font-mono transition-all",
        isRecording && "border-primary ring-2 ring-primary/20",
        className
      )}
    />
  );
}

export { eventToHotkeyString, formatHotkey };