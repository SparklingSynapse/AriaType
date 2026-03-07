import * as React from "react";
import { cn } from "@/lib/utils";

interface SliderProps extends React.InputHTMLAttributes<HTMLInputElement> {
  max?: number;
  min?: number;
  step?: number;
}

const Slider = React.forwardRef<HTMLInputElement, SliderProps>(
  ({ className, max = 100, min = 0, step = 1, ...props }, ref) => {
    return (
      <input
        type="range"
        max={max}
        min={min}
        step={step}
        className={cn(
          "flex h-2 w-full cursor-pointer appearance-none rounded-full bg-secondary",
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);
Slider.displayName = "Slider";

export { Slider };
