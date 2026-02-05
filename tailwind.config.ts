import type { Config } from "tailwindcss";

export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    container: {
      center: true,
      padding: "1.5rem",
      screens: {
        "2xl": "1200px",
      },
    },
    extend: {
      colors: {
        background: "var(--background)",
        "background-2": "var(--background-2)",
        foreground: "var(--foreground)",
        muted: "var(--muted)",
        border: "var(--border)",
        card: "var(--card)",
        accent: "var(--accent)",
        "accent-foreground": "var(--accent-foreground)",
        success: "var(--success)",
        warning: "var(--warning)",
        danger: "var(--danger)",
      },
      boxShadow: {
        subtle: "0 1px 0 rgba(255, 255, 255, 0.04), 0 0 0 1px var(--border)",
        medium: "0 10px 30px rgba(0, 0, 0, 0.35)",
      },
      borderRadius: {
        xl: "0.75rem",
        "2xl": "1rem",
      },
      fontFamily: {
        sans: ["Geist", "system-ui", "-apple-system", "Segoe UI", "sans-serif"],
        mono: ["Geist Mono", "ui-monospace", "SFMono-Regular", "monospace"],
      },
      keyframes: {
        statusPulse: {
          "0%, 100%": { transform: "scale(1)", opacity: "0.9" },
          "50%": { transform: "scale(1.1)", opacity: "1" },
        },
      },
      animation: {
        "status-pulse": "statusPulse 1.4s ease-in-out infinite",
      },
    },
  },
  plugins: [],
} satisfies Config;
