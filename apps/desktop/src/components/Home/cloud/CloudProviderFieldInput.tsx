import { useState } from "react";
import { Eye, EyeSlash } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

interface CloudProviderFieldInputProps {
  id: string;
  secret: boolean;
  invalid?: boolean;
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
}

export function CloudProviderFieldInput({
  id,
  secret,
  invalid = false,
  value,
  placeholder,
  onChange,
}: CloudProviderFieldInputProps) {
  const { t } = useTranslation();
  const [revealed, setRevealed] = useState(false);
  const inputType = secret && !revealed ? "password" : "text";
  const toggleLabel = revealed ? t("cloud.secret.hide") : t("cloud.secret.show");
  const ToggleIcon = revealed ? EyeSlash : Eye;

  return (
    <div className="relative">
      <input
        id={id}
        type={inputType}
        className={cn(
          "flex h-10 w-full rounded-2xl border bg-background py-2 text-sm transition-all ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50",
          secret ? "pl-4 pr-11" : "px-4",
          invalid
            ? "border-destructive focus-visible:ring-1 focus-visible:ring-destructive"
            : "border-border focus-visible:border-primary",
        )}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        autoComplete={secret ? "off" : undefined}
        spellCheck={secret ? false : undefined}
      />
      {secret && (
        <button
          type="button"
          className="absolute right-1 top-1 inline-flex h-8 w-8 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          aria-label={toggleLabel}
          aria-pressed={revealed}
          title={toggleLabel}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => setRevealed((current) => !current)}
        >
          <ToggleIcon className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}
