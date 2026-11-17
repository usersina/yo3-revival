import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000, // Real password hashing and cold Vite module transforms on shared/CI machines.
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:5174',
    browserName: 'chromium',
    viewport: { width: 1440, height: 1000 },
    trace: 'retain-on-failure',
  },
  webServer: [
    {
      command:
        'cargo run --quiet --manifest-path ../Cargo.toml -p yo3-server --example dashboard_dev -- 18181',
      url: 'http://127.0.0.1:18181/panel/openapi.json',
      timeout: 120_000,
      gracefulShutdown: { signal: 'SIGINT', timeout: 5_000 },
      reuseExistingServer: false,
    },
    {
      command: 'npx vite --host 127.0.0.1 --port 5174 --strictPort',
      env: { YO3_DASHBOARD_API: 'http://127.0.0.1:18181' },
      url: 'http://127.0.0.1:5174',
      reuseExistingServer: false,
    },
  ],
})
