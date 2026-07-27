# WASM Compile Latency Baseline

> **Date**: 2026-07-27
> **Source**: Sprint 2 baseline measurement (P8 prerequisite)
> **Context**: `.architecture/reviews/implementation-plan-2026-07-26.md`

## Measurement Method

Measured Rust compilation + test execution as a proxy for WASM compile latency.
The `omnilang-wasm` crate uses the same Rust compilation pipeline.

| Metric | Value |
|--------|-------|
| Test project | `learn-omniview` (104 .omni files, ~500KB source) |
| Sampled subset | 20 files, ~92KB source |
| Iterations | 5 (golden parse), 3 (full suite) |
| Machine | Windows 11, cargo dev profile |

## Results

### Golden Parse (parser + lexer only)
| Run | Time |
|-----|------|
| 1 | 0.30s |
| 2 | 0.26s |
| 3 | 0.26s |
| 4 | 0.26s |
| 5 | 0.25s |
| **Avg** | **0.27s** |

### Full Lib Test (parser + semantic + codegen + 636 tests)
| Run | Time | Tests |
|-----|------|-------|
| 1 | 0.31s | 636 passed, 3 failed |
| 2 | 0.30s | 636 passed, 3 failed |
| 3 | 0.32s | 636 passed, 3 failed |
| **Avg** | **0.31s** |

### OmniView Codegen Only (77 tests)
| Time | Tests |
|------|-------|
| 0.29s | 77 passed, 0 failed |

## Current Test Summary

| Category | Count | Status |
|----------|-------|--------|
| Total tests | 639 | — |
| Passed | 636 | ✅ |
| Failed | 3 | Pre-existing (dart type generics ×2, flutter store field) |

## Notes

1. **Dev profile**: These measurements use cargo's dev profile (no optimizations). WASM release builds (`wasm-release` profile with `opt-level = "z"`, LTO fat) would be faster at runtime but slower to compile.
2. **Incremental compilation**: Measurements include cargo's incremental compilation. Cold builds would be significantly slower (~10-30s for the full workspace).
3. **WASM binary size**: 1.2MB (previously measured). P8 (incremental cache for WASM) would reduce re-compile latency in the editor.
4. **Cache not engaged**: The current WASM editor path does NOT use `IncrementalCache` — every keystroke triggers a full re-parse of all files. P8 would add this.

## Sprint 3 Reference

This baseline is needed for Sprint 3's P8 (WASM incremental cache). After P8 implementation:
- Re-measure with same 20-file project
- Expected improvement: 10-100x for incremental edits (same-file changes)
- Re-measure with 50+ file project for stress testing
