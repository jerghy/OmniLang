// Delegated from omnilang-core (P35 dedup).
// Sub-modules (`javascript`, `parameterized`) are from this facade directory
// and contain facade-specific test code.

#[cfg(feature = "codegen-javascript")]
pub(crate) mod javascript;
pub(crate) mod parameterized;

// Cross-backend injection-payload corpus — runs when all five UI backends
// are enabled (all-backends feature).
#[cfg(all(
    feature = "codegen-react",
    feature = "codegen-omniview",
    feature = "codegen-dioxus",
    feature = "codegen-vue",
    feature = "codegen-flutter"
))]
pub(crate) mod injection;

// Shared fixture factories from omnilang-core (single source of truth)
include!("../../../../crates/omnilang-core/src/codegen/tests_regression/fixtures_shared.rs");
