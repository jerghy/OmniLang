//! Cross-backend regression test baseline.
//!
//! Runs ALL codegen backends on canonical IR fixtures, capturing all outputs
//! via insta snapshots. Provides a "before vs after" comparison baseline for
//! refactoring the codegen layer.
//!
//! See: task-T0-brief.md

use std::collections::HashMap;

use crate::codegen::{
    dart::DartCodegen, dioxus::DioxusCodegen, flutter::FlutterCodegen,
    javascript::JavaScriptCodegen, python::PythonCodegen, react::ReactCodegen, rust::RustCodegen,
    vue::VueCodegen, Codegen, CompileOutput,
};
use crate::ir::*;

// ═════════════════════════════════════════════════════════════════════════════
// Fixture factories
// ═════════════════════════════════════════════════════════════════════════════

/// Empty IRProgram — no components, no stores, no modules.
fn fixture_empty() -> IRProgram {
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

/// A simple component with a state, props, computed, effect, and a view tree.
fn fixture_component() -> IRProgram {
    let mut props = HashMap::new();
    props.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    );
    let state = StateIntent {
        name: "count".into(),
        r#type: TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
        initial_value: ExprIR::LiteralExpr {
            value: serde_json::json!(0),
        },
        mutability: "mutable".into(),
        update_semantics: "setter-call".into(),
        accessor_style: "direct".into(),
        is_copy: false,
    };
    let effect = EffectIntent {
        deps: vec!["count".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::CallExpr {
                    name: "console.log".into(),
                    args: vec![ExprIR::IdentifierExpr {
                        name: "count".into(),
                    }],
                },
            }],
            execution_mode: "client-only".into(),
        },
    };
    let comp = ComponentIR {
        name: "Counter".into(),
        generic_params: vec![],
        props,
        prop_defaults: HashMap::new(),
        states: vec![state],
        computeds: vec![],
        effects: vec![effect],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: Some(ViewTree::Element(ElementIR {
            tag: "div".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: HashMap::new(),
            style_classes: None,
            inline_style: None,
            children: vec![
                ViewTree::Text("Hello".into()),
                ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                })),
            ],
            class_map_bindings: HashMap::new(),
            driven_by: None,
            in_view_threshold: None,
            in_view_once: None,
            in_view_repeat: None,
            scroll_start: None,
            scroll_end: None,
            scroll_scrub: None,
            drag_constraints: None,
            drag_elastic: None,
            drag_threshold: None,
            drag_velocity_max: None,
            drag_scrub: None,
            drag_momentum: None,
            hover_reverse: None,
            paused_override: None,
            hero_tag: None,
            hero_transition: None,
            test_id: None,
            ref_name: None,
            states: None,
            construction_kind: "static".into(),
            source_line: 0,
            source_col: 0,
            motion_triggers: vec![],
        })),
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::from([("Counter".into(), comp)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

/// A store with signals, getters, and an action.
///
/// Uses a single signal to avoid HashMap-ordering non-determinism in snapshots.
/// Multi-signal ordering is covered by each backend's own unit tests.
fn fixture_store() -> IRProgram {
    let store = StoreIR {
        name: "AppStore".into(),
        signals: HashMap::from([(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
        )]),
        signal_defaults: HashMap::from([("count".into(), serde_json::json!(0))]),
        getters: HashMap::from([(
            "doubleCount".into(),
            ExprIR::BinaryOpExpr {
                op: "mul".into(),
                left: Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
            },
        )]),
        actions: vec![FunctionIR {
            name: "increment".into(),
            visibility: "public".into(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: "count".into(),
                        },
                        operation: "increment".into(),
                        amount: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        }),
                        item: None,
                    }),
                }],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        }],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                stores: HashMap::from([("AppStore".into(), store)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Snapshot helper
// ═════════════════════════════════════════════════════════════════════════════

/// Concatenate all output files into a single snapshot string.
fn format_output(output: &CompileOutput) -> String {
    let mut s = String::new();
    for f in &output.files {
        s.push_str(&format!("=== {} ===\n", f.path));
        s.push_str(&f.content);
        if !f.content.ends_with('\n') {
            s.push('\n');
        }
    }
    if s.is_empty() {
        s.push_str("(no output files)\n");
    }
    s
}

// ═════════════════════════════════════════════════════════════════════════════
// Empty program — all 9 backends
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_regression_react_empty() {
    let codegen = ReactCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_react_empty", &format_output(&output));
}

#[test]
fn test_regression_vue_empty() {
    let codegen = VueCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_vue_empty", &format_output(&output));
}

#[test]
fn test_regression_javascript_empty() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_javascript_empty", &format_output(&output));
}

#[test]
fn test_regression_typescript_empty() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_typescript_empty", &format_output(&output));
}

#[test]
fn test_regression_dart_empty() {
    let codegen = DartCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dart_empty", &format_output(&output));
}

#[test]
fn test_regression_rust_empty() {
    let codegen = RustCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_rust_empty", &format_output(&output));
}

#[test]
fn test_regression_python_empty() {
    let codegen = PythonCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_python_empty", &format_output(&output));
}

#[test]
fn test_regression_flutter_empty() {
    let codegen = FlutterCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_flutter_empty", &format_output(&output));
}

#[test]
fn test_regression_dioxus_empty() {
    let codegen = DioxusCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dioxus_empty", &format_output(&output));
}

// ═════════════════════════════════════════════════════════════════════════════
// Component fixture — all 9 backends
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_regression_react_component() {
    let codegen = ReactCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_react_component", &format_output(&output));
}

#[test]
fn test_regression_vue_component() {
    let codegen = VueCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_vue_component", &format_output(&output));
}

#[test]
fn test_regression_javascript_component() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_javascript_component", &format_output(&output));
}

#[test]
fn test_regression_typescript_component() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_typescript_component", &format_output(&output));
}

#[test]
fn test_regression_dart_component() {
    let codegen = DartCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dart_component", &format_output(&output));
}

#[test]
fn test_regression_rust_component() {
    let codegen = RustCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_rust_component", &format_output(&output));
}

#[test]
fn test_regression_python_component() {
    let codegen = PythonCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_python_component", &format_output(&output));
}

#[test]
fn test_regression_flutter_component() {
    let codegen = FlutterCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_flutter_component", &format_output(&output));
}

#[test]
fn test_regression_dioxus_component() {
    let codegen = DioxusCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dioxus_component", &format_output(&output));
}

// ═════════════════════════════════════════════════════════════════════════════
// Store fixture — all 9 backends
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_regression_react_store() {
    let codegen = ReactCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_react_store", &format_output(&output));
}

#[test]
fn test_regression_vue_store() {
    let codegen = VueCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_vue_store", &format_output(&output));
}

#[test]
fn test_regression_javascript_store() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_javascript_store", &format_output(&output));
}

#[test]
fn test_regression_typescript_store() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_typescript_store", &format_output(&output));
}

#[test]
fn test_regression_dart_store() {
    let codegen = DartCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dart_store", &format_output(&output));
}

#[test]
fn test_regression_rust_store() {
    let codegen = RustCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_rust_store", &format_output(&output));
}

#[test]
fn test_regression_python_store() {
    let codegen = PythonCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_python_store", &format_output(&output));
}

#[test]
fn test_regression_flutter_store() {
    let codegen = FlutterCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_flutter_store", &format_output(&output));
}

#[test]
fn test_regression_dioxus_store() {
    let codegen = DioxusCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dioxus_store", &format_output(&output));
}
