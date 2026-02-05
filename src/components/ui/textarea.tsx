import * as React from "react";

import { cn } from "../../lib/utils";

const Textarea = React.forwardRef<HTMLTextAreaElement, React.ComponentProps<"textarea">>(
  ({ className, ...props }, ref) => (
    <textarea
      className={cn(
        "flex min-h-[160px] w-full rounded-md border border-border bg-background px-3 py-3 text-sm font-medium text-foreground shadow-subtle transition-[border-color,box-shadow,background-color] duration-150 placeholder:text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-foreground/30 focus-visible:border-foreground/30 disabled:cursor-not-allowed disabled:opacity-60",
        className
      )}
      ref={ref}
      {...props}
    />
  )
);
Textarea.displayName = "Textarea";

export { Textarea };
