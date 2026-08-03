import { defineConfig } from '@playwright/test';

// ════════════════════════════════════════════════════════════════
// OmniView-target browser suite (Task 7.1 terminal validation).
//
// Serves the GENERATED OmniView output (build/index.html + index.js)
// as static files and runs tests/omniview-browser.spec.ts against it.
//
// Why this file lives inside react-test-app/: the suite needs
// @playwright/test on the module-resolution path (the only committed
// node_modules under the showcase project is here). The OmniView
// target output itself is plain static files — no vite involved.
//
// Run:  npx playwright test --config=playwright.omniview.config.ts
// ════════════════════════════════════════════════════════════════

export default defineConfig({
  testDir: './tests',
  testMatch: '**/omniview-browser.spec.ts',
  timeout: 60000,
  expect: {
    timeout: 10000,
  },
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: [
    ['list'],
    ['json', { outputFile: 'test-results-omniview.json' }],
  ],
  use: {
    browserName: 'chromium',
    headless: true,
    viewport: { width: 1280, height: 900 },
    ignoreHTTPSErrors: true,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    // Static server for the OmniView target output. This config lives at
    // build/react-test-app/, so the omniview output (index.html + index.js)
    // is ONE level up: --directory .. (NOT ../build — that resolves to
    // build/build, which does not exist, and every request 404s).
    command: 'python -m http.server 5174 --directory ..',
    port: 5174,
    timeout: 30000,
    reuseExistingServer: true,
  },
});
