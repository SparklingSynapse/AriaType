import * as React from "react";
import { cn } from "@/lib/utils";
import { ChevronDown } from "lucide-react";

interface SelectProps {
  value: string;
  onChange: (e: { target: { value: string } }) => void;
  options: { value: string; label: string }[];
  className?: string;
  placeholder?: string;
}

const Select = React.forwardRef<HTMLDivElement, SelectProps>(
  ({ className, options, value, onChange, placeholder }, _ref) => {
    const [open, setOpen] = React.useState(false);
    const [search, setSearch] = React.useState("");
    const containerRef = React.useRef<HTMLDivElement>(null);
    const searchRef = React.useRef<HTMLInputElement>(null);

    const selectedOption = options.find((o) => o.value === value);
    const selectedLabel = selectedOption
      ? selectedOption.label
      : placeholder || value;

    const filtered = search
      ? options.filter(
          (o) =>
            o.label.toLowerCase().includes(search.toLowerCase()) ||
            o.value.toLowerCase().includes(search.toLowerCase()),
        )
      : options;

    React.useEffect(() => {
      const handleOutside = (e: MouseEvent) => {
        if (
          containerRef.current &&
          !containerRef.current.contains(e.target as Node)
        ) {
          setOpen(false);
          setSearch("");
        }
      };
      document.addEventListener("mousedown", handleOutside);
      return () => document.removeEventListener("mousedown", handleOutside);
    }, []);

    React.useEffect(() => {
      if (open) setTimeout(() => searchRef.current?.focus(), 0);
      else setSearch("");
    }, [open]);

    return (
      <div ref={containerRef} className={cn("relative", className)}>
        <button
          type="button"
          data-state={open ? "open" : "closed"}
          onClick={() => setOpen((o) => !o)}
          className="flex h-10 w-full items-center justify-between rounded-xl border border-input bg-background px-3 py-2 text-sm transition-colors hover:bg-secondary data-[state=open]:border-primary focus-visible:border-primary focus-visible:outline-none"
        >
          <span>{selectedLabel}</span>
          <ChevronDown
            className={cn(
              "h-4 w-4 text-muted-foreground transition-transform duration-200",
              open && "rotate-180",
            )}
          />
        </button>

        {open && (
          <div className="absolute z-50 mt-1 w-full rounded-xl border border-border bg-card shadow-lg overflow-hidden flex flex-col">
            {options.length > 8 && (
              <div className="flex-shrink-0 p-2 border-b border-border">
                <input
                  ref={searchRef}
                  type="text"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Search..."
                  className="w-full rounded-md border border-border bg-card px-2 py-1 text-sm outline-none"
                />
              </div>
            )}
            <div className="overflow-y-auto max-h-60">
              {filtered.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => {
                    onChange({ target: { value: option.value } });
                    setOpen(false);
                  }}
                  className={cn(
                    "flex w-full items-center px-3 py-2 text-sm transition-colors hover:bg-secondary outline-none",
                    option.value === value && "bg-background font-medium",
                  )}
                >
                  {option.label}
                </button>
              ))}
              {filtered.length === 0 && (
                <p className="px-3 py-2 text-sm text-muted-foreground">
                  No results
                </p>
              )}
            </div>
          </div>
        )}
      </div>
    );
  },
);
Select.displayName = "Select";

export { Select };
