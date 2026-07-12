# Task 6: Clean up path mapping duplication - Report

## Summary

Removed duplicated manual path mapping construction from WASM bindings and Poiesis TypeScript code, delegating to the canonical `derive_path_mapping()` function.

## Changes Made

### 1. `omnilang/src/wasm.rs::compile_multi_to_omniview()`
- Replaced manual loop (basename, `./basename`, `./components/Name` mappings) with:
  ```rust
  let path_mapping = crate::path_mapping::derive_path_mapping(&sources_map);
  ```
- The derived function generates all previous mappings plus stem, `.omni`-less, and `../`-prefixed variants.

### 2. `omnilang/src/wasm.rs::resolve_styles_multi()`
- After deserializing the caller-provided `path_map`, merged auto-derived mappings:
  ```rust
  let derived = crate::path_mapping::derive_path_mapping(&sources_map);
  for (k, v) in derived {
      path_map.entry(k).or_insert(v);
  }
  ```
- Caller overrides take priority (`or_insert` keeps the first entry).

### 3. `poiesis/src/commands/rustCompiler.ts`
- Replaced manual pathMapping construction (basename, `./basename`, component mappings) with empty `{}`.

### 4. `poiesis/src/components/design/useWasmCompile.ts`
- Replaced manual pathMapping (design.omni + component mappings) with empty `{}`.

## Build Verification

- **WASM build**: Succeeded (only pre-existing warnings, no new errors)
- **TypeScript check**: Passes — 46 pre-existing errors in 10 files, none in modified files

## Commits

- `6ebf3a1` refactor: use derive_path_mapping() in WASM, remove TS-side path mapping
