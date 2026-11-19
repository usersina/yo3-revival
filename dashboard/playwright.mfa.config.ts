import { defineConfig } from '@playwright/test'

// The staff second factor under the real default policy (Required), on its own ports so it can run beside the main
// browser checks (whose dev server keeps password-only staff). `npm run test:e2e:mfa`.
export default defineConfig({
  testDir: './e2e-mfa',
  timeout: 60_000,
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:17843',
    browserName: 'chromium',
    viewport: { width: 1440, height: 1000 },
    trace: 'retain-on-failure',
  },
  webServer: [
    {
      env: { YO3_DEV_FAST_HASH: '1' },
      command:
        'cargo run --quiet --manifest-path ../Cargo.toml -p yo3-server --example dashboard_dev -- 18381 required',
      url: 'http://127.0.0.1:18381/panel/openapi.json',
      timeout: 120_000,
      gracefulShutdown: { signal: 'SIGINT', timeout: 5_000 },
      reuseExistingServer: false,
    },
    {
      command: 'npx vite --host 127.0.0.1 --port 17843 --strictPort',
      env: { YO3_DASHBOARD_API: 'http://127.0.0.1:18381' },
      url: 'http://127.0.0.1:17843',
      reuseExistingServer: false,
    },
  ],
})
