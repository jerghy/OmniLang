import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  timeout: 30000,
  expect: { timeout: 10000 },
  use: {
    baseURL: "http://localhost:5199",
    headless: true,
    screenshot: "only-on-failure",
  },
  webServer: {
    command: "python editor/server/main.py --dir build/smoketest --port 5199",
    url: "http://localhost:5199/api/project",
    reuseExistingServer: true,
    timeout: 15000,
  },
});
