import * as React from "react";

import { cn } from "../../lib/utils";

const Textarea = React.forwardRef<HTMLTextAreaElement, React.ComponentProps<"textarea">>(
  ({ className, ...props }, ref) => (
    <textarea
      className={cn(
        "flex min-h-[140px] w-full rounded-2xl border border-ink/10 bg-white/80 px-4 py-3 text-sm font-medium text-ink shadow-soft transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ink/30",
        className
      )}
      ref={ref}
      {...props}
    />
  )
);
Textarea.displayName = "Textarea";

export { Textarea };
