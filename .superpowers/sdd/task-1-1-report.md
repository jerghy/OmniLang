# Task T1.1 Report — Tauri Security Configuration Fix

## Changes Made

File: `poiesis/src-tauri/tauri.conf.json`

1. **`withGlobalTauri`**: Changed from `true` to `false` (line 13)
   - Prevents exposing `window.__TAURI__` API to all web content, including iframes.
   - The desktop frontend does not need this; it uses Tauri's `@tauri-apps/api` npm package directly.

2. **CSP**: Kept `'unsafe-eval'` as-is (line 22)
   - Required by the OmniView runtime which uses `new Function()` for dynamic expression evaluation.
   - Inline comment was initially attempted but removed because the Tauri JSON parser does not support JSON5 comments — it requires strict JSON.

3. **Plugin scopes** (lines 26-33): Replaced empty plugin configs with restricted scopes:
   - `fs` plugin: Scoped to `$RESOURCE/**` and `$APPDATA/**` — restricts filesystem access to the app's resource and data directories.
   - `shell` plugin: Empty scope array `[]` — denies all shell command execution, preventing arbitrary command injection.

## Verification

- `cargo check` — passed (compilation successful)
- `cargo test` — all 405 tests passed across all crates:
  - 389 omnilang lib tests
  - 4 poiesis unit tests (`src/main.rs`)
  - 4 poiesis integration tests (`tests/integration.rs`)
  - 5 compilation tests (`tests/compilation_test.rs`)
  - 3 smoke tests (`tests/smoke.rs`)

## Concerns

- The `'unsafe-eval'` CSP directive is inherently risky as it allows arbitrary JavaScript execution. It is retained only because it is strictly required by the OmniView runtime's expression evaluation mechanism. If the runtime is ever refactored to avoid `new Function()` (e.g., by precompiling expressions at build time), this directive should be removed.
- The `$RESOURCE/**` and `$APPDATA/**` FS scopes are permissive within those directories. Consider further narrowing if specific subdirectories are sufficient.
- The plugin configuration changes only take full effect when the corresponding Tauri plugins (`tauri-plugin-fs`, `tauri-plugin-shell`) are enabled in `Cargo.toml`. Verify that these plugins are actually registered.
