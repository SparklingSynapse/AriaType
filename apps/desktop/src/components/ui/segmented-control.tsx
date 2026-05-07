import { cn } from "@/lib/utils";

interface SegmentedControlItem {
  value: string;
  label: string;
}

interface SegmentedControlProps {
  items: SegmentedControlItem[];
  value: string;
  onChange: (value: string) => void;
  className?: string;
  size?: "sm" | "md";
  testId?: string;
}

export function SegmentedControl({
  items,
  value,
  onChange,
  className,
  size = "md",
  testId,
}: SegmentedControlProps) {
  const containerSizeClasses = {
    sm: "h-10 p-1",
    md: "h-11 p-1.5",
  };

  const buttonSizeClasses = {
    sm: "px-4 py-1.5",
    md: "px-5",
  };

  return (
    <div
      data-testid={testId}
      className={cn(
        "inline-flex items-center justify-center rounded-full bg-secondary text-muted-foreground gap-1",
        containerSizeClasses[size],
        className
      )}
    >
      {items.map((item) => (
        <button
          key={item.value}
          onClick={() => onChange(item.value)}
          data-testid={testId ? `${testId}-${item.value}` : undefined}
          className={cn(
            "inline-flex h-full items-center justify-center whitespace-nowrap rounded-full text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
            buttonSizeClasses[size],
            value === item.value
              ? "bg-background text-foreground shadow-sm"
              : "hover:text-foreground hover:bg-background/40"
          )}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}