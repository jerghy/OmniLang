//! Parameterized regression tests — each fixture × each P0 backend = one test.
//!
//! Uses a declarative macro to generate individual `#[test]` functions with
//! compile-time snapshot names, avoiding the need for `rstest` or runtime loops.

use rustc_hash::FxHashMap;

use crate::codegen::{CompileOutput, CompiledFile};
use crate::codegen::omniview;
use crate::ir::IRProgram;

// ═════════════════════════════════════════════════════════════════════════════
// OmniView adapter — converts OmniViewOutput → CompileOutput
// ═════════════════════════════════════════════════════════════════════════════

fn omni_to_compile_output(ir: &IRProgram) -> CompileOutput {
    let sources = FxHashMap::from_iter([("main.omni".to_string(), String::new())]);
    let output = omniview::generate(ir, false, false, false);
    CompileOutput {
        files: vec![
            CompiledFile {
                path: "index.html".into(),
                content: output.html,
            },
            CompiledFile {
                path: "style.css".into(),
                content: output.css,
            },
            CompiledFile {
                path: "app.js".into(),
                content: output.js,
            },
        ],
        errors: vec![],
        source_map: output.source_map,
        warnings: vec![],
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Test generation macro — one #[test] fn per (backend, fixture) pair
// ═════════════════════════════════════════════════════════════════════════════

macro_rules! gen_tests_for_fixture {
    ($fixture_fn:ident) => {
        gen_tests_for_fixture!(@omniview, $fixture_fn);
        gen_tests_for_fixture!(@react, $fixture_fn);
        gen_tests_for_fixture!(@flutter, $fixture_fn);
    };
    // Omniview
    (@omniview, $fixture_fn:ident) => {
        #[test]
        fn omniview() {
            let ir = super::super::$fixture_fn();
            let output = super::omni_to_compile_output(&ir);
            assert!(output.errors.is_empty(),
                "omniview {} errors: {:?}", stringify!($fixture_fn), output.errors);
            insta::assert_snapshot!(
                concat!("regression_omniview_", stringify!($fixture_fn)),
                &super::super::format_output(&output)
            );
        }
    };
    // React
    (@react, $fixture_fn:ident) => {
        #[test]
        fn react() {
            use crate::codegen::Codegen;
            let ir = super::super::$fixture_fn();
            let sources = rustc_hash::FxHashMap::from_iter([("main.omni".to_string(), String::new())]);
            let output = crate::codegen::react::ReactCodegen::default().generate(&ir, &sources);
            assert!(output.errors.is_empty(),
                "react {} errors: {:?}", stringify!($fixture_fn), output.errors);
            insta::assert_snapshot!(
                concat!("regression_react_", stringify!($fixture_fn)),
                &super::super::format_output(&output)
            );
        }
    };
    // Flutter
    (@flutter, $fixture_fn:ident) => {
        #[test]
        fn flutter() {
            use crate::codegen::Codegen;
            let ir = super::super::$fixture_fn();
            let sources = rustc_hash::FxHashMap::from_iter([("main.omni".to_string(), String::new())]);
            let output = crate::codegen::flutter::FlutterCodegen.generate(&ir, &sources);
            assert!(output.errors.is_empty(),
                "flutter {} errors: {:?}", stringify!($fixture_fn), output.errors);
            insta::assert_snapshot!(
                concat!("regression_flutter_", stringify!($fixture_fn)),
                &super::super::format_output(&output)
            );
        }
    };
}

// ═════════════════════════════════════════════════════════════════════════════
// Per-fixture modules (each gets its own 3-backend test functions)
// ═════════════════════════════════════════════════════════════════════════════

mod empty {
    //! Regression tests for the empty IR program fixture.
    gen_tests_for_fixture!(fixture_empty);
}

mod component {
    //! Regression tests for the component fixture.
    gen_tests_for_fixture!(fixture_component);
}

mod store {
    //! Regression tests for the store fixture.
    gen_tests_for_fixture!(fixture_store);
}

mod component_with_store {
    //! Regression tests for the component-with-store fixture.
    gen_tests_for_fixture!(fixture_component_with_store);
}

mod expressions {
    //! Regression tests for the expressions fixture.
    gen_tests_for_fixture!(fixture_expressions);
}
