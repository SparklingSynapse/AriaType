/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{ts,tsx}",
    "./src/app/**/*.{ts,tsx}"
  ],
  theme: {
    extend: {
      colors: {
        background: "var(--background)",
        foreground: "var(--foreground)",
        accent: "var(--accent)",
        muted: {
          DEFAULT: "var(--muted)",
          foreground: "var(--muted-foreground)",
          light: "var(--muted-light)"
        },
        card: {
          DEFAULT: "var(--card)",
          foreground: "var(--card-foreground)",
          hover: "var(--card-hover)"
        },
        primary: {
          DEFAULT: "var(--primary)",
          foreground: "var(--primary-foreground)"
        },
        secondary: {
          DEFAULT: "var(--secondary)",
          foreground: "var(--secondary-foreground)"
        },
        border: "var(--border)",
        "border-subtle": "var(--border-subtle)",
        input: "var(--input)",
        ring: "var(--ring)",
        black: "var(--black)"
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)"
      },
      fontFamily: {
        sans: ["system-ui", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Roboto", "sans-serif"]
      }
    }
  },
  plugins: []
};
