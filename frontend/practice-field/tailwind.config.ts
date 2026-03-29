import type { Config } from "tailwindcss"

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        bg: "var(--bg)",
        panel: "var(--panel)",
        primary: "var(--primary)",
        danger: "var(--danger)",
        text: "var(--text)",
        muted: "var(--muted)",
      },
    },
  },
};

export default config;