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
// Port: OMNI_TEST_PORT=<port> npx playwright test ... — overrides the
// default 5174 (CI behavior unchanged). 5174 is the user's Vite dev
// server port; with reuseExistingServer the suite would attach to it,
// so local runs MUST use a different port (e.g. OMNI_TEST_PORT=5199).
// ════════════════════════════════════════════════════════════════

const OMNI_TEST_PORT = process.env.OMNI_TEST_PORT ?? '5174';

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
    command: `python -m http.server ${OMNI_TEST_PORT} --directory ..`,
    port: Number(OMNI_TEST_PORT),
    timeout: 30000,
    reuseExistingServer: true,
  },
});
