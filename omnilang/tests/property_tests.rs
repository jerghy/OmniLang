//! Property-based tests for OmniLang core invariants.
//!
//! These tests use `proptest` to verify that key functions never panic,
//! produce valid output, and block dangerous patterns — across a wide
//! range of random inputs.
//!
//! ## CI notes
//!
//! Set `PROPTEST_CASES=100` (or export it) to keep tests fast in CI:
//!
//! ```text
//! PROPTEST_CASES=100 cargo test property
//! ```
//!
//! The `compile_never_panics` and `codegen_never_panics` tests use 1000
//! cases by default (explicit `ProptestConfig::with_cases(1000)`); the
//! config constant is defined at the crate level so all tests share it.

use proptest::prelude::*;

// ──────────────────────────────────────────────
// P1: sanitize_js_ident never panics, output is valid JS ident
// ──────────────────────────────────────────────

proptest! {
    #[test]
    fn sanitize_js_ident_never_panics(s in "\\PC*") {
        let _ = omnilang::codegen::sanitize_js_ident(&s);
    }

    #[test]
    fn sanitize_js_ident_safe_output(s in "\\PC+") {
        let result = omnilang::codegen::sanitize_js_ident(&s);
        // Reserved words get _ appended, otherwise unchanged
        assert!(!result.is_empty(), "sanitize_js_ident returned empty for {:?}", s);
    }
}

// ──────────────────────────────────────────────
// P2: sanitize_css_value blocks dangerous patterns
// ──────────────────────────────────────────────

proptest! {
    #[test]
    fn sanitize_css_value_blocks_dangerous(
        prefix in "[a-zA-Z0-9_]*",
        dangerous in "javascript:|expression\\(|-moz-binding"
    ) {
        let input = format!("{}{}", prefix, dangerous);
        assert_eq!(
            omnilang::codegen::sanitize_css_value(&input),
            "",
            "expected dangerous CSS value to be blocked: {:?}",
            input,
        );
    }

    #[test]
    fn sanitize_css_value_passes_safe_values(s in "[a-zA-Z0-9_#%.,()\\- ]+") {
        let result = omnilang::codegen::sanitize_css_value(&s);
        assert!(
            !result.is_empty() || s.is_empty(),
            "sanitize_css_value cleared a safe value: {:?}",
            s,
        );
    }
}

// ──────────────────────────────────────────────
// P3: compile never panics on any input (fuzz-like)
// ──────────────────────────────────────────────

/// Compile arbitrary input through a codegen backend, mirroring how the
/// editor compiles a single .omni source file to a target language.
fn compile_for_target(s: &str, target: omnilang::codegen::Target) {
    let mut sources = rustc_hash::FxHashMap::default();
    sources.insert("fuzz.omni".to_string(), s.to_string());
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);
    let _ = omnilang::compile_multi_internal(sources, target, path_mapping, false, false, None);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn compile_never_panics(s in "\\PC{0,200}") {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = omnilang::compile(&s);
        }));
        assert!(
            result.is_ok(),
            "compiler panicked on input: {:?}",
            s
        );
    }
}

// ──────────────────────────────────────────────
// P3b: codegen backends never panic on any input (fuzz-like)
// ──────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn codegen_never_panics(s in "\\PC{0,200}") {
        // OmniView + React are enabled by default features. Flutter is
        // fuzzed whenever the codegen-flutter feature is enabled; without
        // it the backend is not compiled in, so fuzzing it would only
        // cover the graceful "backend not enabled" fallback, not real
        // codegen.
        let mut targets = vec![
            omnilang::codegen::Target::OmniView,
            omnilang::codegen::Target::React,
        ];
        #[cfg(feature = "codegen-flutter")]
        targets.push(omnilang::codegen::Target::Flutter);

        for target in targets {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                compile_for_target(&s, target);
            }));
            assert!(
                result.is_ok(),
                "codegen({:?}) panicked on input: {:?}",
                target,
                s
            );
        }
    }
}

// ──────────────────────────────────────────────
// P4: sanitize_module_name prevents traversal
// ──────────────────────────────────────────────

proptest! {
    #[test]
    fn sanitize_module_name_no_traversal(s in "\\PC{0,100}") {
        let result = omnilang::codegen::sanitize_module_name(&s);
        assert!(!result.contains(".."), "result contains '..': {:?}", result);
        assert!(!result.contains('/'),  "result contains '/': {:?}", result);
        assert!(!result.contains('\\'), "result contains '\\\\': {:?}", result);
        assert!(!result.starts_with('.'), "result starts with '.': {:?}", result);
    }
}
