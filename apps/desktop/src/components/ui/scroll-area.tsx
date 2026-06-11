import * as React from "react";
import type { PartialOptions } from "overlayscrollbars";
import {
  OverlayScrollbarsComponent,
  type OverlayScrollbarsComponentProps,
  type OverlayScrollbarsComponentRef,
} from "overlayscrollbars-react";
import { cn } from "@/lib/utils";
import "overlayscrollbars/overlayscrollbars.css";

const TRANSPARENT_SCROLLBAR_THEME = "os-theme-transparent";

const defaultScrollAreaOptions: PartialOptions = {
  showNativeOverlaidScrollbars: false,
  scrollbars: {
    theme: TRANSPARENT_SCROLLBAR_THEME,
    visibility: "auto",
    autoHide: "scroll",
    autoHideDelay: 300,
    autoHideSuspend: false,
  },
};

export type ScrollAreaProps = Omit<OverlayScrollbarsComponentProps, "options"> & {
  options?: OverlayScrollbarsComponentProps["options"];
};

function resolveOptions(options: ScrollAreaProps["options"]): ScrollAreaProps["options"] {
  if (options === false) {
    return false;
  }

  return {
    ...defaultScrollAreaOptions,
    ...(options ?? {}),
    showNativeOverlaidScrollbars: false,
    scrollbars: {
      ...defaultScrollAreaOptions.scrollbars,
      ...(options?.scrollbars ?? {}),
      theme: TRANSPARENT_SCROLLBAR_THEME,
    },
  };
}

const ScrollArea = React.forwardRef<OverlayScrollbarsComponentRef, ScrollAreaProps>(
  ({ className, options, ...props }, ref) => (
    <OverlayScrollbarsComponent
      ref={ref}
      className={cn(className)}
      options={resolveOptions(options)}
      {...props}
    />
  ),
);
ScrollArea.displayName = "ScrollArea";

export { ScrollArea };
