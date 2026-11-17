import { defineConfig } from '@playwright/test'

if (!process.env.YO3_RELEASE_DASHBOARD_URL) {
  throw new Error('Run tools/containers/test_dashboard_browser.py with the released server image.')
}

export default defineConfig({
  testDir: './release-tests',
  timeout: 60_000,
  workers: 1,
  forbidOnly: true,
  retries: 0,
  reporter: 'list',
  use: {
    baseURL: process.env.YO3_RELEASE_DASHBOARD_URL,
    browserName: 'chromium',
    // A newly generated identity, limited to this disposable localhost stack.
    ignoreHTTPSErrors: true,
    trace: 'retain-on-failure',
    viewport: { width: 1440, height: 1000 },
  },
})
