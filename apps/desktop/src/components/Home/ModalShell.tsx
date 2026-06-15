import {
  forwardRef,
  useCallback,
  useEffect,
  useState,
  type ComponentPropsWithoutRef,
  type ReactNode,
} from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X } from "@phosphor-icons/react";
import {
  motion,
  useIsPresent,
  type AnimationDefinition,
  type HTMLMotionProps,
} from "framer-motion";

import { Button, type ButtonProps } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export const MODAL_NAV_WIDTH_CLASS = "w-[248px]";
export const SETTINGS_MODAL_SIZE_CLASS = "h-[min(720px,calc(100vh-80px))] w-[min(940px,calc(100vw-40px))]";
export const ONBOARDING_MODAL_SIZE_CLASS = "h-[620px] w-[640px]";

const MODAL_BACKDROP_ANIMATION = {
  open: { opacity: 1 },
  closed: { opacity: 0 },
};

const MODAL_SURFACE_ANIMATION = {
  open: { opacity: 1, y: 0, scale: 1 },
  closed: { opacity: 0, y: 10, scale: 0.98 },
};

type ModalSurfaceAnimationTarget = typeof MODAL_SURFACE_ANIMATION.closed;

type RadixPresenceState = {
  "data-state"?: string;
};

type ModalSurfaceDomProps = Omit<
  ModalSurfaceProps,
  "animate" | "children" | "className" | "onAnimationComplete" | "sizeClassName"
> & {
  [key: `data-${string}`]: string | number | boolean | undefined;
};

interface ModalFrameProps {
  children: ReactNode;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  modal?: boolean;
  sizeClassName?: string;
  surfaceProps?: ModalSurfaceDomProps;
  surfaceClassName?: string;
  testId?: string;
  viewportClassName?: string;
}

export function ModalFrame({
  children,
  modal = true,
  onOpenChange,
  open,
  sizeClassName,
  surfaceProps,
  surfaceClassName,
  testId,
  viewportClassName,
}: ModalFrameProps) {
  const [shouldRender, setShouldRender] = useState(open);

  useEffect(() => {
    if (open) {
      setShouldRender(true);
    }
  }, [open]);

  const handleSurfaceAnimationComplete = useCallback(
    (definition: AnimationDefinition) => {
      if (!open && isClosedSurfaceAnimation(definition)) {
        setShouldRender(false);
      }
    },
    [open],
  );

  return (
    <Dialog.Root modal={modal && open} open={open} onOpenChange={onOpenChange}>
      {(open || shouldRender) && (
        <Dialog.Portal forceMount>
          <Dialog.Overlay asChild forceMount>
            <ModalBackdrop
              animate={
                open
                  ? MODAL_BACKDROP_ANIMATION.open
                  : MODAL_BACKDROP_ANIMATION.closed
              }
            />
          </Dialog.Overlay>
          <ModalViewport className={viewportClassName}>
            <Dialog.Content asChild forceMount>
              <ModalSurface
                {...surfaceProps}
                animate={
                  open
                    ? MODAL_SURFACE_ANIMATION.open
                    : MODAL_SURFACE_ANIMATION.closed
                }
                className={surfaceClassName}
                data-testid={testId}
                onAnimationComplete={handleSurfaceAnimationComplete}
                sizeClassName={sizeClassName}
              >
                {children}
              </ModalSurface>
            </Dialog.Content>
          </ModalViewport>
        </Dialog.Portal>
      )}
    </Dialog.Root>
  );
}

function isClosedSurfaceAnimation(definition: AnimationDefinition): boolean {
  if (
    typeof definition !== "object" ||
    definition === null ||
    Array.isArray(definition)
  ) {
    return false;
  }

  const target = definition as Partial<
    Record<keyof ModalSurfaceAnimationTarget, unknown>
  >;

  return (
    target.opacity === MODAL_SURFACE_ANIMATION.closed.opacity &&
    target.y === MODAL_SURFACE_ANIMATION.closed.y &&
    target.scale === MODAL_SURFACE_ANIMATION.closed.scale
  );
}

const ModalBackdrop = forwardRef<HTMLDivElement, HTMLMotionProps<"div">>(
  ({ className, ...props }, ref) => {
    const isPresent = useIsPresent();
    const isClosed = (props as RadixPresenceState)["data-state"] === "closed";

    return (
      <motion.div
        ref={ref}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: 0.14 }}
        className={cn(
          "fixed inset-0 z-50 bg-black/35 backdrop-blur-md",
          className,
          (!isPresent || isClosed) && "pointer-events-none",
        )}
        {...props}
      />
    );
  },
);
ModalBackdrop.displayName = "ModalBackdrop";

interface ModalViewportProps extends ComponentPropsWithoutRef<"div"> {
  children: ReactNode;
}

function ModalViewport({ children, className, ...props }: ModalViewportProps) {
  return (
    <div
      className={cn(
        "fixed inset-x-0 bottom-4 top-14 z-50 flex items-center justify-center px-5 pointer-events-none",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}

interface ModalSurfaceProps extends HTMLMotionProps<"div"> {
  sizeClassName?: string;
}

const ModalSurface = forwardRef<HTMLDivElement, ModalSurfaceProps>(
  ({ className, sizeClassName = SETTINGS_MODAL_SIZE_CLASS, ...props }, ref) => {
    const isPresent = useIsPresent();
    const isClosed = (props as RadixPresenceState)["data-state"] === "closed";

    return (
      <motion.div
        ref={ref}
        initial={{ opacity: 0, y: 14, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 10, scale: 0.98 }}
        transition={{ duration: 0.16, ease: "easeOut" }}
        className={cn(
          "flex overflow-hidden rounded-[32px] bg-card text-card-foreground shadow-[0_36px_120px_rgba(0,0,0,0.32)] pointer-events-auto",
          sizeClassName,
          className,
          (!isPresent || isClosed) && "pointer-events-none",
        )}
        {...props}
      />
    );
  },
);
ModalSurface.displayName = "ModalSurface";

interface ModalCloseButtonProps extends Omit<ButtonProps, "variant" | "size"> {
  "aria-label": string;
}

export const ModalCloseButton = forwardRef<HTMLButtonElement, ModalCloseButtonProps>(
  ({ className, children, ...props }, ref) => (
    <Button
      ref={ref}
      type="button"
      variant="ghost"
      size="icon"
      className={cn(
        "h-10 w-10 shrink-0 rounded-full hover:bg-foreground/10 active:bg-foreground/15",
        className,
      )}
      {...props}
    >
      {children ?? <X className="h-4 w-4" />}
    </Button>
  ),
);
ModalCloseButton.displayName = "ModalCloseButton";
