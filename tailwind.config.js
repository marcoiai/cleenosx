/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          base: "#f7f8fb",
          panel: "#ffffff",
          muted: "#eef2f7",
        },
        ink: {
          strong: "#172033",
          body: "#39445a",
          muted: "#6a7488",
        },
        signal: {
          safe: "#0f8a5f",
          attention: "#a16207",
          review: "#c2410c",
          danger: "#b42318",
          system: "#5e6678",
        },
      },
      boxShadow: {
        material: "0 1px 2px rgba(21, 31, 51, 0.08), 0 4px 16px rgba(21, 31, 51, 0.08)",
      },
    },
  },
  plugins: [],
};

