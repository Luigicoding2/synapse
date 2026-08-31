/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx,html}",
  ],
  theme: {
    extend: {
      colors: {
        cyber: {
          bg: '#0a0a0f',
          panel: '#12121e',
          accent: '#5a4eff',
          text: '#e2e8f0',
          muted: '#94a3b8',
          success: '#10b981',
          warn: '#f59e0b',
        }
      }
    },
  },
  plugins: [],
}
