// Cross-backend regression test baseline.
//
// Provides shared fixture factories and helpers for per-backend test modules.
// Each backend sub-module contains its own snapshot tests.
//
// Shared fixture functions are in `fixtures_shared.rs` (included by both
// omnilang-core and the omnilang facade crate via P35 dedup).

pub(crate) mod parameterized;

// Shared fixture factories and helpers (single source of truth). Most
// factories are consumed by the omnilang facade crate's test build (which
// includes this same file), so they look dead inside this crate's own test
// build — the `#[allow(dead_code)]`s live on the items in fixtures_shared.rs
// (outer attributes are permitted there; `include!` rejects inner ones).
include!("fixtures_shared.rs");
