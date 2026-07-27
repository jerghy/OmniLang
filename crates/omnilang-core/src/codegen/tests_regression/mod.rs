// Cross-backend regression test baseline.
//
// Provides shared fixture factories and helpers for per-backend test modules.
// Each backend sub-module contains its own snapshot tests.
//
// Shared fixture functions are in `fixtures_shared.rs` (included by both
// omnilang-core and the omnilang facade crate via P35 dedup).

#[cfg(feature = "codegen-javascript")]
pub(crate) mod javascript;
pub(crate) mod parameterized;

// Shared fixture factories and helpers (single source of truth)
include!("fixtures_shared.rs");
