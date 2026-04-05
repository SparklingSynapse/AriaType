import * as React from "react";
import { cn } from "@/lib/utils";
import { ChevronDown } from "lucide-react";
import {
  useFloating,
  offset,
  flip,
  shift,
  size,
  autoUpdate,
  useClick,
  useDismiss,
  useRole,
  useInteractions,
  FloatingPortal,
  FloatingFocusManager,
} from "@floating-ui/react";

interface SelectProps {
  value: string;
  onChange: (e: { target: { value: string } }) => void;
  options: { value: string; label: string }[];
  className?: string;
  placeholder?: string;
}

const Select = React.forwardRef<HTMLButtonElement, SelectProps>(
  ({ className, options, value, onChange, placeholder }, ref) => {
    const [open, setOpen] = React.useState(false);
    const [search, setSearch] = React.useState("");
    const searchRef = React.useRef<HTMLInputElement>(null);

    const { refs, floatingStyles, context } = useFloating({
      open,
      onOpenChange: setOpen,
      placement: "bottom-start",
      whileElementsMounted: autoUpdate,
      middleware: [
        offset(4),
        flip({ padding: 8 }),
        shift({ padding: 8 }),
        size({
          apply({ rects, elements, availableHeight }) {
            Object.assign(elements.floating.style, {
              width: `${rects.reference.width}px`,
              maxHeight: `${Math.min(availableHeight, 280)}px`,
            });
          },
          padding: 8,
        }),
      ],
    });

    const click = useClick(context);
    const dismiss = useDismiss(context);
    const role = useRole(context, { role: "listbox" });

    const { getReferenceProps, getFloatingProps } = useInteractions([
      click,
      dismiss,
      role,
    ]);

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
      if (open) {
        setTimeout(() => searchRef.current?.focus(), 0);
      } else {
        setSearch("");
      }
    }, [open]);

    return (
      <>
        <button
          ref={(node) => {
            refs.setReference(node);
            if (typeof ref === "function") ref(node);
            else if (ref) ref.current = node;
          }}
          type="button"
          data-state={open ? "open" : "closed"}
          className={cn(
            "flex h-10 w-full items-center justify-between rounded-2xl border border-border bg-background px-4 py-2 text-sm transition-colors hover:bg-backgroundHover data-[state=open]:border-primary focus-visible:border-primary focus-visible:outline-none",
            className
          )}
          {...getReferenceProps()}
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
          <FloatingPortal>
            <FloatingFocusManager context={context} initialFocus={searchRef.current ? 0 : -1} returnFocus={true}>
              <div
                ref={refs.setFloating}
                style={floatingStyles}
                className="z-[9999] flex flex-col rounded-2xl border border-border bg-card shadow-lg outline-none overflow-hidden"
                {...getFloatingProps()}
              >
                {options.length > 8 && (
                  <div className="flex-shrink-0 p-2 border-b border-border">
                    <input
                      ref={searchRef}
                      type="text"
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                      placeholder="Search..."
                      className="w-full rounded-2xl border border-border bg-card px-4 py-2 text-sm outline-none"
                    />
                  </div>
                )}
                <div className="overflow-y-auto">
                  {filtered.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => {
                        onChange({ target: { value: option.value } });
                        setOpen(false);
                      }}
                      className={cn(
                        "flex w-full items-center px-3 py-2 text-sm transition-colors hover:bg-backgroundHover outline-none",
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
            </FloatingFocusManager>
          </FloatingPortal>
        )}
      </>
    );
  },
);
Select.displayName = "Select";

export { Select };
