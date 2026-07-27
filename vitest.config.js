import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineConfig } from 'vitest/config';

const dirname =
  typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath(import.meta.url));

// Try to load storybook test plugin; gracefully skip if storybook deps are missing
// (e.g., version mismatch with vitest, or CI without full storybook install).
let storybookPlugin = null;
let playwright = null;
try {
  const sb = await import('@storybook/addon-vitest/vitest-plugin');
  storybookPlugin = sb.storybookTest;
  const pw = await import('@vitest/browser-playwright');
  playwright = pw.playwright;
} catch (_) {
  // Storybook dependencies unavailable — browser tests will be skipped.
}

/** @type {import('vitest/config').UserConfig['test']['projects']} */
const projects = [];

// Unit test project — always available
projects.push({
  extends: true,
  test: {
    name: 'unit',
    include: ['poiesis/src/**/*.test.{ts,tsx}'],
    exclude: ['poiesis/src/**/*.stories.{ts,tsx}'],
    environment: 'node',
  },
});

// Storybook browser test project — only when deps are available
if (storybookPlugin && playwright) {
  projects.push({
    extends: true,
    plugins: [
      storybookPlugin({ configDir: path.join(dirname, '.storybook') }),
    ],
    test: {
      name: 'storybook',
      browser: {
        enabled: true,
        headless: true,
        provider: playwright({}),
        instances: [{ browser: 'chromium' }],
      },
    },
  });
}

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig({
  test: {
    projects,
  },
});
