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
        ink: "#1f2328",
        sage: "#5f7a6b",
        clay: "#b8705c",
        ember: "#d4634c",
        mist: "#eef1ea",
        pine: "#1f3a2e",
      },
      boxShadow: {
        halo: "0 18px 60px rgba(31, 35, 40, 0.12)",
        soft: "0 12px 30px rgba(31, 35, 40, 0.1)",
      },
      borderRadius: {
        xl: "1rem",
        "2xl": "1.5rem",
      },
      fontFamily: {
        display: ["Fraunces", "serif"],
        body: ["Spline Sans", "sans-serif"],
      },
      keyframes: {
        pulseGlow: {
          "0%, 100%": { transform: "scale(1)", opacity: "0.8" },
          "50%": { transform: "scale(1.08)", opacity: "1" },
        },
        drift: {
          "0%": { transform: "translateY(0px)" },
          "50%": { transform: "translateY(8px)" },
          "100%": { transform: "translateY(0px)" },
        },
      },
      animation: {
        "pulse-glow": "pulseGlow 1.6s ease-in-out infinite",
        drift: "drift 12s ease-in-out infinite",
      },
    },
  },
  plugins: [],
} satisfies Config;
