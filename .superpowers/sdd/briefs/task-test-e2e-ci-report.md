# Task Report: Wire Playwright E2E Tests into CI

## Summary

Added a new `e2e` job to `.github/workflows/ci.yml` that runs the 95 Playwright E2E tests in `test/projects/learn-omniview/build/react-test-app/tests/omniview-react.spec.ts` against the React codegen output.

## Changes Made

### 1. `.github/workflows/ci.yml` -- New `e2e` job

Added after the `audit` job. The job:

- **Runs on**: `ubuntu-latest`
- **Builds `omnic`**: `cargo build --release -p omnic` (uses `Swatinem/rust-cache` for speed)
- **Installs Node deps**: `npm ci` in `test/projects/learn-omniview/build/react-test-app/`
- **Regenerates React output**: runs `omnic build --target react -o build/react-test-app` from the project dir, ensuring the test runs against the current compiler output (not stale committed output)
- **Installs Playwright browsers**: `npx playwright install chromium`
- **Runs tests**: `npx playwright test` (Playwright manages the Vite dev server lifecycle via its `webServer` config)
- **Uploads artifacts on failure**:
  - `playwright-test-results` -- JSON results and failure screenshots (on `failure()`)
  - `playwright-report` -- HTML report (on `always()`)

### 2. `test/projects/learn-omniview/build/react-test-app/playwright.config.ts` -- Cross-platform updates

- **Browser**: `channel: 'msedge'` (Windows-only) replaced with `browserName: 'chromium'` (works on all platforms)
- **Artifacts**: Added `video: 'retain-on-failure'` and `trace: 'retain-on-failure'` for CI debugging
- **Reporter**: Added HTML reporter (`playwright-report/`) for rich CI artifacts
- **Vite lifecycle**: Added `webServer` config block so Playwright auto-starts/stops the Vite dev server (with `reuseExistingServer: true` for local dev)

## Test Setup Details

**Pipeline**: `.omni` source files in `test/projects/learn-omniview/src/` are compiled to React `.tsx`/`.module.css` output by `omnic build --target react`. The output lands in `build/react-test-app/src/`. Vite serves these files, and Playwright tests against `http://localhost:5173`.

**95 tests** spanning 10 UI tabs (home, todo, basic interaction, compiler features, motion animation, CSS features, events/forms, language features, settings, supplementary) plus interaction and error-checking test suites.

## Known Considerations

- The `e2e` job is independent (no `needs`), running in parallel with other jobs. On the first run, the Rust cache will be cold and `cargo build --release -p omnic` will take several minutes. Subsequent runs reuse the cache from the `rust` matrix job.
- If the codegen React output is already up-to-date in the repo, the regeneration step is a formality. It ensures the CI catches compiler regressions that affect the generated output.
- The Playwright config's `webServer` feature requires `@playwright/test` v1.39+ (currently `^1.61.1`), which is satisfied.
