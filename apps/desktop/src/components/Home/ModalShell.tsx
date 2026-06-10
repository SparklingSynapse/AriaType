import { forwardRef, type ComponentPropsWithoutRef, type ReactNode } from "react";
import { X } from "@phosphor-icons/react";
import { motion, type HTMLMotionProps } from "framer-motion";

import { Button, type ButtonProps } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export const MODAL_NAV_WIDTH_CLASS = "w-[248px]";
export const SETTINGS_MODAL_SIZE_CLASS = "h-[min(720px,calc(100vh-80px))] w-[min(940px,calc(100vw-40px))]";
export const ONBOARDING_MODAL_SIZE_CLASS = "h-[620px] w-[640px]";

export const ModalBackdrop = forwardRef<HTMLDivElement, HTMLMotionProps<"div">>(
  ({ className, ...props }, ref) => (
    <motion.div
      ref={ref}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.14 }}
      className={cn("fixed inset-0 z-50 bg-black/35 backdrop-blur-md", className)}
      {...props}
    />
  ),
);
ModalBackdrop.displayName = "ModalBackdrop";

interface ModalViewportProps extends ComponentPropsWithoutRef<"div"> {
  children: ReactNode;
}

export function ModalViewport({ children, className, ...props }: ModalViewportProps) {
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

export const ModalSurface = forwardRef<HTMLDivElement, ModalSurfaceProps>(
  ({ className, sizeClassName = SETTINGS_MODAL_SIZE_CLASS, ...props }, ref) => (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, y: 8, scale: 0.985 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.16, ease: "easeOut" }}
      className={cn(
        "flex overflow-hidden rounded-[32px] bg-card text-card-foreground shadow-[0_36px_120px_rgba(0,0,0,0.32)] pointer-events-auto",
        sizeClassName,
        className,
      )}
      {...props}
    />
  ),
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
