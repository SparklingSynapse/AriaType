import { useState, useRef, useEffect } from "react";
import { cn } from "@/lib/utils";
import { hotkeyCommands, events } from "@/lib/tauri";

// Hotkey display labels - for UI formatting only
const HOTKEY_LABELS: Record<string, string> = {
  cmd: "⌘",
  cmdleft: "⌘",
  cmdright: "⌘",
  ctrl: "Ctrl",
  ctrlleft: "Ctrl",
  ctrlright: "Ctrl",
  alt: "⌥",
  altleft: "⌥",
  altright: "⌥",
  opt: "⌥",
  optleft: "⌥",
  optright: "⌥",
  shift: "⇧",
  shiftleft: "⇧",
  shiftright: "⇧",
  fn: "Fn",
  space: "Space",
  enter: "↵",
  backspace: "⌫",
  tab: "⇥",
  escape: "Esc",
  arrowup: "↑",
  arrowdown: "↓",
  arrowleft: "←",
  arrowright: "→",
  delete: "Del",
  home: "Home",
  end: "End",
  pageup: "PgUp",
  pagedown: "PgDn",
  insert: "Ins",
  capslock: "⇪",
  printscreen: "PrtSc",
  scrolllock: "ScrLk",
  pause: "Pause",
  minus: "-",
  equal: "=",
  bracketleft: "[",
  bracketright: "]",
  backslash: "\\",
  semicolon: ";",
  quote: "'",
  backquote: "`",
  comma: ",",
  period: ".",
  slash: "/",
  numlock: "Num",
  numpadadd: "+",
  numpaddecimal: ".",
  numpaddivide: "/",
  numpadenter: "↵",
  numpadequal: "=",
  numpadmultiply: "*",
  numpadsubtract: "-",
  audiovolumedown: "🔉",
  audiovolumeup: "🔊",
  audiovolumemute: "🔇",
  mediaplay: "▶",
  mediapause: "⏸",
  mediaplaypause: "⏯",
  mediastop: "⏹",
  mediatracknext: "⏭",
  mediatrackprev: "⏮",
};

/**
 * Get display label for a single key part.
 */
function getKeyLabel(part: string): string {
  const normalizedPart = part.toLowerCase();
  if (HOTKEY_LABELS[normalizedPart]) {
    return HOTKEY_LABELS[normalizedPart];
  }
  if (normalizedPart.length === 1) {
    return normalizedPart.toUpperCase();
  }
  // Capitalize first letter for unknown keys
  return normalizedPart.charAt(0).toUpperCase() + normalizedPart.slice(1);
}

/**
 * Format hotkey string for UI display (text-only version).
 * Used for accessibility and legacy display contexts.
 */
function formatHotkey(hotkey: string): string {
  return hotkey
    .split("+")
    .map((part) => getKeyLabel(part))
    .join("+");
}

/**
 * Single hotkey key tag component.
 * Displays one key with keyboard-like styling.
 */
function HotkeyTag({ keyLabel }: { keyLabel: string }) {
  return (
    <span
      className="inline-flex items-center justify-center min-w-[1.5rem] px-1.5 py-0.5 
                 rounded-md bg-secondary/40 border border-border/50 
                 text-xs font-medium shadow-sm
                 text-foreground/80"
    >
      {keyLabel}
    </span>
  );
}

/**
 * Render hotkey as a series of styled key tags.
 */
function HotkeyTags({ hotkey }: { hotkey: string }) {
  const keys = hotkey.split("+").map((part) => getKeyLabel(part));

  return (
    <div className="flex items-center gap-1">
      {keys.map((key, index) => (
        <HotkeyTag key={`${key}-${index}`} keyLabel={key} />
      ))}
    </div>
  );
}

interface HotkeyInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
}

/**
 * Hotkey input component.
 *
 * Flow:
 * 1. User clicks input → enter capture mode (backend unregisters old hotkey)
 * 2. User presses keys → backend captures and emits hotkey-captured event
 * 3. Frontend receives event → calls stop_hotkey_recording to register new hotkey
 * 4. Backend registers new hotkey, saves to settings
 *
 * Frontend only handles UI display, no validation.
 */
export function HotkeyInput({ value, onChange, placeholder, className }: HotkeyInputProps) {
  const [isRecording, setIsRecording] = useState(false);
  const [registrationError, setRegistrationError] = useState<string | null>(null);
  const stoppingRef = useRef(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Listen for hotkey-captured event from backend
  // Backend has already validated - we just need to call stop_hotkey_recording
  // to complete registration
  useEffect(() => {
    if (!isRecording) return;

    const unlistenPromise = events.onHotkeyCaptured((_hotkey: string) => {
      if (stoppingRef.current) return;

      // Backend captured and validated - now call stop to register
      stoppingRef.current = true;
      setIsRecording(false);
      setRegistrationError(null);

      // Call stop_hotkey_recording to register the new hotkey
      hotkeyCommands.stopRecording().then((registeredHotkey) => {
        if (registeredHotkey) {
          onChange(registeredHotkey);
        }
        containerRef.current?.blur();
        stoppingRef.current = false;
      });
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [isRecording, onChange]);

  // Listen for registration failure event
  useEffect(() => {
    const unlistenPromise = events.onShortcutRegistrationFailed((error: string) => {
      setRegistrationError(error);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Clear error when value changes
  useEffect(() => {
    setRegistrationError(null);
  }, [value]);

  const startRecording = async () => {
    if (isRecording) return;
    setRegistrationError(null);

    try {
      await hotkeyCommands.startRecording();
      setIsRecording(true);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (errorMsg.includes("already in progress")) {
        await hotkeyCommands.cancelRecording();
        try {
          await hotkeyCommands.startRecording();
          setIsRecording(true);
        } catch {
          // Silently fail
        }
      }
    }
  };

  const handleBlur = async () => {
    if (isRecording && !stoppingRef.current) {
      // Cancel recording on blur - backend will re-register old hotkey
      await hotkeyCommands.cancelRecording();
      setIsRecording(false);
      stoppingRef.current = false;
    }
  };

  const handleKeyDown = async (e: React.KeyboardEvent) => {
    if (!isRecording) return;

    if (e.code === "Escape") {
      e.preventDefault();
      // Cancel recording - backend will re-register old hotkey
      await hotkeyCommands.cancelRecording();
      setIsRecording(false);
      stoppingRef.current = false;
      containerRef.current?.blur();
    }
  };

  const hasRegistrationError = registrationError && !isRecording;

  return (
    <div className="space-y-1">
      <div
        ref={containerRef}
        tabIndex={0}
        onMouseDown={startRecording}
        onFocus={startRecording}
        onBlur={handleBlur}
        onKeyDown={handleKeyDown}
        className={cn(
          "cursor-pointer w-full h-10 flex items-center px-4 rounded-2xl",
          "border border-border bg-background transition-all",
          "focus-visible:outline-none",
          isRecording && "border-primary ring-2 ring-primary/20",
          hasRegistrationError && "border-destructive",
          className
        )}
        aria-label={`Hotkey: ${formatHotkey(value)}. Click to change.`}
        role="button"
      >
        {isRecording ? (
          <span className="text-sm text-muted-foreground">
            {placeholder || "Press keys..."}
          </span>
        ) : (
          <HotkeyTags hotkey={value} />
        )}
      </div>
      {hasRegistrationError && (
        <p className="text-xs text-destructive">{registrationError}</p>
      )}
      {isRecording && !hasRegistrationError && (
        <p className="text-xs text-muted-foreground">Press a key combination... (ESC to cancel)</p>
      )}
    </div>
  );
}

export { formatHotkey, HotkeyTags };