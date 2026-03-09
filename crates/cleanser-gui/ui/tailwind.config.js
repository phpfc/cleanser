/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        orange: {
          primary: "#FF6B35",
          light: "#FF8C5A",
          dark: "#E55A2B",
        },
        coral: "#FF8566",
        sand: "#FFF4E6",
        dark: {
          primary: "#0f0f1a",
          secondary: "#1a1a2e",
          tertiary: "#252542",
        },
        safe: "#27ca40",
        moderate: "#ffbd2e",
        risky: "#ff5f56",
      },
      fontFamily: {
        sans: ["Space Grotesk", "sans-serif"],
        mono: ["JetBrains Mono", "monospace"],
      },
      boxShadow: {
        orange: "0 4px 15px rgba(255, 107, 53, 0.3)",
        "orange-lg": "0 6px 20px rgba(255, 107, 53, 0.4)",
      },
    },
  },
  plugins: [],
};
