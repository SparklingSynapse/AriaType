import { useState, useRef, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { hotkeyCommands, events } from "@/lib/tauri";
import { showErrorToast } from "@/lib/toast";

const HOTKEY_LABELS: Record<string, string> = {
  cmd: "⌘",
  cmdleft: "L⌘",
  cmdright: "R⌘",
  ctrl: "Ctrl",
  ctrlleft: "LCtrl",
  ctrlright: "RCtrl",
  alt: "⌥",
  altleft: "L⌥",
  altright: "R⌥",
  opt: "⌥",
  optleft: "L⌥",
  optright: "R⌥",
  shift: "⇧",
  shiftleft: "L⇧",
  shiftright: "R⇧",
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

function getKeyLabel(part: string): string {
  const normalizedPart = part.toLowerCase();
  if (HOTKEY_LABELS[normalizedPart]) {
    return HOTKEY_LABELS[normalizedPart];
  }
  if (normalizedPart.length === 1) {
    return normalizedPart.toUpperCase();
  }
  return normalizedPart.charAt(0).toUpperCase() + normalizedPart.slice(1);
}

function formatHotkey(hotkey: string): string {
  return hotkey
    .split("+")
    .map((part) => getKeyLabel(part))
    .join("+");
}

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

function HotkeyTags({ hotkey }: { hotkey: string }) {
  if (!hotkey) {
    return null;
  }
  
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
  profileKey: string;
  placeholder?: string;
  className?: string;
}

export function HotkeyInput({ value, onChange, profileKey, placeholder, className }: HotkeyInputProps) {
  const { t } = useTranslation();
  const [isCapturing, setIsCapturing] = useState(false);
  const [registrationError, setRegistrationError] = useState<string | null>(null);
  const stoppingRef = useRef(false);
  const containerRef = useRef<HTMLDivElement>(null);

  function getHotkeyConflictMessage(errorMsg: string): string | null {
    if (!errorMsg.startsWith("hotkey_conflict:")) {
      return null;
    }
    return errorMsg.split(":")[1];
  }

  useEffect(() => {
    if (!isCapturing) return;

    const unlistenPromise = events.onHotkeyCaptured((_hotkey: string) => {
      if (stoppingRef.current) return;

      stoppingRef.current = true;
      setIsCapturing(false);
      setRegistrationError(null);

      hotkeyCommands.stopCapture(profileKey)
        .then((registeredHotkey) => {
          onChange(registeredHotkey);
          containerRef.current?.blur();
          stoppingRef.current = false;
        })
        .catch((error) => {
          const errorMsg = String(error);
          const profileId = getHotkeyConflictMessage(errorMsg);
          if (profileId) {
            switch (profileId) {
              case "dictate":
                showErrorToast(t("hotkey.conflictDictate"));
                break;
              case "chat":
                showErrorToast(t("hotkey.conflictChat"));
                break;
              case "custom":
                showErrorToast(t("hotkey.conflictCustom"));
                break;
              default:
                showErrorToast(t("hotkey.registrationFailed"));
            }
          }
          stoppingRef.current = false;
        });
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [isCapturing, onChange, profileKey, t]);

  useEffect(() => {
    const unlistenPromise = events.onShortcutRegistrationFailed((payload: { error: string; profile_id: string }) => {
      if (payload.profile_id === profileKey) {
        const profileId = getHotkeyConflictMessage(payload.error);
        let message: string;
        if (profileId) {
          switch (profileId) {
            case "dictate":
              message = t("hotkey.conflictDictate");
              break;
            case "chat":
              message = t("hotkey.conflictChat");
              break;
            case "custom":
              message = t("hotkey.conflictCustom");
              break;
            default:
              message = t("hotkey.registrationFailed");
          }
        } else {
          message = payload.error;
        }
        setRegistrationError(message);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [profileKey, t]);

  useEffect(() => {
    setRegistrationError(null);
  }, [value]);

  const startCapture = async () => {
    if (isCapturing) return;
    setRegistrationError(null);

    try {
      await hotkeyCommands.startCapture(profileKey);
      setIsCapturing(true);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (errorMsg.includes("already in progress")) {
        await hotkeyCommands.cancelCapture();
        try {
          await hotkeyCommands.startCapture(profileKey);
          setIsCapturing(true);
        } catch {
        }
      }
    }
  };

  const handleBlur = async () => {
    if (isCapturing && !stoppingRef.current) {
      await hotkeyCommands.cancelCapture();
      setIsCapturing(false);
      stoppingRef.current = false;
    }
  };

  const handleKeyDown = async (e: React.KeyboardEvent) => {
    if (!isCapturing) return;

    if (e.code === "Escape") {
      e.preventDefault();
      await hotkeyCommands.cancelCapture();
      setIsCapturing(false);
      stoppingRef.current = false;
      containerRef.current?.blur();
    }
  };

  const hasRegistrationError = registrationError && !isCapturing;

  return (
    <div className="space-y-1">
      <div
        ref={containerRef}
        tabIndex={0}
        onMouseDown={startCapture}
        onFocus={startCapture}
        onBlur={handleBlur}
        onKeyDown={handleKeyDown}
        className={cn(
          "cursor-pointer w-full h-10 flex items-center px-4 rounded-2xl",
          "border border-border bg-background transition-all",
          "focus-visible:outline-none",
          isCapturing && "border-primary ring-2 ring-primary/20",
          hasRegistrationError && "border-destructive",
          className
        )}
        aria-label={value ? `Hotkey: ${formatHotkey(value)}. Click to change.` : t("hotkey.clickToSet")}
        role="button"
      >
        {isCapturing ? (
          <span className="text-sm text-muted-foreground">
            {placeholder || t("hotkey.recording.pressKeys")}
          </span>
        ) : value ? (
          <HotkeyTags hotkey={value} />
        ) : (
          <span className="text-sm text-muted-foreground">
            {placeholder || t("hotkey.clickToSet")}
          </span>
        )}
      </div>
      {hasRegistrationError && (
        <p className="text-xs text-destructive">{registrationError}</p>
      )}
      {isCapturing && !hasRegistrationError && (
        <p className="text-xs text-muted-foreground">Press a key combination... (ESC to cancel)</p>
      )}
    </div>
  );
}

export { formatHotkey, HotkeyTags };