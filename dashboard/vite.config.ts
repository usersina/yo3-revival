import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react-swc'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/panel/api/': { target: process.env.YO3_DASHBOARD_API ?? 'http://127.0.0.1:18080' },
      '/panel/openapi.json': { target: process.env.YO3_DASHBOARD_API ?? 'http://127.0.0.1:18080' },
    },
  },
  test: {
    include: ['src/**/*.test.{ts,tsx}'],
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    clearMocks: true,
    restoreMocks: true,
  },
})
