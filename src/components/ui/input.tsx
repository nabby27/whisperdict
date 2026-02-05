import * as React from "react";

import { cn } from "../../lib/utils";

const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<"input">>(
  ({ className, type, ...props }, ref) => (
    <input
      type={type}
      className={cn(
        "flex h-9 w-full rounded-md border border-border bg-background px-3 text-sm font-medium text-foreground shadow-subtle transition-[border-color,box-shadow,background-color] duration-150 placeholder:text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-foreground/30 focus-visible:border-foreground/30 disabled:cursor-not-allowed disabled:opacity-60",
        className
      )}
      ref={ref}
      {...props}
    />
  )
);
Input.displayName = "Input";

export { Input };
