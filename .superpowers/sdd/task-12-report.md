# Task 12 Report: Generate __omni_theme JS Runtime

## Status: DELIVERED

## Files Modified

1. **`omnilang/src/codegen/omniview/mod.rs`**
   - Added `emit_theme_runtime()` method to `OmniViewGenerator` that generates the `__omni_theme` JS runtime object with:
     - `_themes`: Array of theme names collected across all design blocks
     - `_current`: Current theme getter
     - `set(name)`: Sets `document.documentElement.dataset.theme`, persists to `localStorage`
     - `toggle()`: Cycles through declared themes
     - `init()`: Restores from `localStorage`, falls back to `prefers-color-scheme` dark detection, defaults to first theme
     - Auto-initialization via `__omni_theme.init()` at the end
   - Called `emit_theme_runtime()` from the `generate()` function inside the `!css_only` block

2. **`omnilang/src/codegen/omniview/tests.rs`**
   - Added `test_theme_runtime_emitted_when_design_has_themes`: Verifies `__omni_theme` is generated with `_themes: ["light", "dark"]`, `_storageKey: "omni-theme"`, `localStorage` persistence, `prefers-color-scheme` detection, and auto-initialization when designs declare themes
   - Added `test_theme_runtime_not_emitted_without_themes`: Verifies `__omni_theme` is NOT generated when no designs declare themes

3. **`crates/omnilang-core/src/parser/ast/design.rs`**
   - Fixed a doc test code block that was being compiled as Rust code (changed ` ``` ` to ` ```text `)

## Test Summary

- 2 new tests pass
- Full workspace: all existing tests pass (no regressions)
- Clippy: clean
- Formatting: clean

## Build Verification

- `cargo build --workspace` passes
- `cargo test --workspace` passes (all test suites)
- `cargo clippy -p omnilang -- -D warnings` passes
- `cargo fmt --all --check` passes

## Concerns

- The `emit_theme_runtime` method reads theme configurations from `self.design_map`, which is populated during design name collection. The method is called inside the `!css_only` block after component generation but before DOM bindings.
- The `prefers-color-scheme` dark detection only triggers for a theme named `"dark"`. Designs using non-standard naming (e.g., "night") won't get auto-detection.
