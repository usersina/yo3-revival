import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react-swc'
import tailwindcss from '@tailwindcss/vite'

const commit = process.env.VITE_DASHBOARD_COMMIT ?? 'development'
if (commit !== 'development' && !/^[0-9a-f]{40}$/.test(commit))
  throw new Error('VITE_DASHBOARD_COMMIT must be a full Git commit')

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    {
      name: 'dashboard-build-identity',
      generateBundle() {
        this.emitFile({
          type: 'asset',
          fileName: 'assets/dashboard-version.json',
          source: JSON.stringify({ commit }) + '\n',
        })
      },
    },
  ],
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
