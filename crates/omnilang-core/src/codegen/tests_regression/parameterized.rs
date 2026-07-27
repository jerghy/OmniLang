//! Parameterized regression tests — each fixture × each P0 backend = one test.
//!
//! These tests require backend-specific codegen modules (omniview, react, flutter)
//! which live in individual crates (omnilang-codegen-omniview, etc.) and the
//! omnilang facade crate. The parameterized tests in omnilang-core are disabled —
//! use `omnilang/src/codegen/tests_regression/parameterized.rs` instead.

// (tests disabled in omnilang-core — backends are in separate crates)
