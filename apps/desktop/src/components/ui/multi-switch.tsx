import * as React from "react";
import { cn } from "@/lib/utils";
import { motion } from "framer-motion";

export interface MultiSwitchOption {
  label: React.ReactNode;
  value: string;
}

export interface MultiSwitchProps {
  options: MultiSwitchOption[];
  value: string;
  onChange: (value: string) => void;
  className?: string;
  fullWidth?: boolean;
}

export function MultiSwitch({
  options,
  value,
  onChange,
  className,
  fullWidth,
}: MultiSwitchProps) {
  const layoutId = React.useId();

  return (
    <div
      role="radiogroup"
      className={cn(
        "inline-flex h-10 items-center justify-center rounded-full bg-background p-1.5 text-muted-foreground border border-border/40 gap-1",
        fullWidth && "flex w-full",
        className,
      )}
    >
      {options.map((option) => {
        const isActive = value === option.value;
        return (
          <button
            key={option.value}
            role="radio"
            aria-checked={isActive}
            onClick={(e) => {
              e.preventDefault();
              onChange(option.value);
            }}
            type="button"
            className={cn(
              "cursor-pointer relative inline-flex h-full items-center justify-center whitespace-nowrap rounded-full px-4 py-1 text-xs font-medium transition-colors duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
              fullWidth && "flex-1",
              isActive
                ? "text-foreground"
                : "hover:text-foreground hover:bg-background/40",
            )}
          >
            {isActive && (
              <motion.div
                layoutId={`active-pill-${layoutId}`}
                className="absolute inset-0 rounded-full bg-card ring-1 ring-border/50 pointer-events-none"
                transition={{ type: "spring", stiffness: 400, damping: 30 }}
              />
            )}
            <span className="relative z-10 pointer-events-none">{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}
