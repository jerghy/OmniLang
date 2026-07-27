// Delegated from omnilang-core (P35 dedup).
// Sub-modules (`javascript`, `parameterized`) are from this facade directory
// and contain facade-specific test code.

#[cfg(feature = "codegen-javascript")]
pub(crate) mod javascript;
pub(crate) mod parameterized;

// Shared fixture factories from omnilang-core (single source of truth)
include!("../../../../crates/omnilang-core/src/codegen/tests_regression/fixtures_shared.rs");
