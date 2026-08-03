import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 60000,
  expect: {
    timeout: 10000,
  },
  fullyParallel: false,
  retries: 1,
  workers: 1,
  reporter: [
    ['list'],
    ['json', { outputFile: 'test-results.json' }],
    ['html', { outputFolder: 'playwright-report' }],
  ],
  use: {
    browserName: 'chromium',
    headless: true,
    viewport: { width: 1280, height: 900 },
    ignoreHTTPSErrors: true,
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'omniview-react',
      // omniview-browser.spec.ts targets the OmniView static output and
      // is served by playwright.omniview.config.ts instead.
      testIgnore: ['**/omniview-browser.spec.ts'],
    },
  ],
  webServer: {
    command: 'npx vite',
    port: 5173,
    timeout: 30000,
    reuseExistingServer: true,
  },
});
