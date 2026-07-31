// ═══════════════════════════════════════════════════════════
// Unit tests
// ═══════════════════════════════════════════════════════════

use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::fmt::Write;

use super::expr::*;
#[allow(unused_imports)]
use super::utils::*;
use super::ReactCodegen;
use crate::codegen::shared::build_component_map;
use crate::codegen::{
    binary_op_to_str, camel_to_kebab, capitalize_first, format_json_value, CodeWriter, Codegen,
    JSON_REACT,
};
use crate::ir::*;
use crate::ir::{ConstructionKind, ModuleIR};
use crate::path_mapping;

// ── Helper tests ─────────────────────────────────────────

#[test]
fn test_capitalize_first() {
    assert_eq!(capitalize_first("hello"), "Hello");
    assert_eq!(capitalize_first(""), "");
    assert_eq!(capitalize_first("a"), "A");
    assert_eq!(capitalize_first("ABC"), "ABC");
}

#[test]
fn test_format_json_value_tsx() {
    assert_eq!(
        format_json_value(&serde_json::Value::Null, &JSON_REACT),
        "null"
    );
    assert_eq!(
        format_json_value(&serde_json::json!(true), &JSON_REACT),
        "true"
    );
    assert_eq!(
        format_json_value(&serde_json::json!(false), &JSON_REACT),
        "false"
    );
    assert_eq!(format_json_value(&serde_json::json!(42), &JSON_REACT), "42");
    assert_eq!(
        format_json_value(&serde_json::json!(2.71), &JSON_REACT),
        "2.71"
    );
    assert_eq!(
        format_json_value(&serde_json::json!("hello"), &JSON_REACT),
        "\"hello\""
    );
    assert_eq!(
        format_json_value(&serde_json::json!("he\"llo"), &JSON_REACT),
        "\"he\\\"llo\""
    );
}

#[test]
fn test_camel_to_kebab() {
    assert_eq!(camel_to_kebab("backgroundColor"), "background-color");
    assert_eq!(camel_to_kebab("fontSize"), "font-size");
    assert_eq!(camel_to_kebab("display"), "display");
    assert_eq!(camel_to_kebab("borderRadius"), "border-radius");
    // CSS custom properties (--prefixed) should be returned unchanged
    assert_eq!(camel_to_kebab("--myProp"), "--myProp");
}

#[test]
fn test_to_jsx_attr() {
    assert_eq!(to_jsx_attr("class"), "className");
    assert_eq!(to_jsx_attr("for"), "htmlFor");
    assert_eq!(to_jsx_attr("tabindex"), "tabIndex");
    assert_eq!(to_jsx_attr("onclick"), "onclick");
    assert_eq!(to_jsx_attr("autocomplete"), "autoComplete");
}

#[test]
fn test_to_react_event() {
    assert_eq!(to_react_event("click"), "onClick");
    assert_eq!(to_react_event("change"), "onChange");
    assert_eq!(to_react_event("submit"), "onSubmit");
    assert_eq!(to_react_event("focus"), "onFocus");
    assert_eq!(to_react_event("dblclick"), "onDoubleClick");
    assert_eq!(to_react_event("keydown"), "onKeyDown");
    assert_eq!(to_react_event("customEvent"), "onCustomEvent");
}

#[test]
fn test_binary_op_to_str() {
    assert_eq!(binary_op_to_str(&BinaryOpIR::Add, true, false), "+");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Eq, true, false), "===");
    assert_eq!(binary_op_to_str(&BinaryOpIR::And, true, false), "&&");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Or, true, false), "||");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Neq, true, false), "!==");
    // C-loose eq
    assert_eq!(binary_op_to_str(&BinaryOpIR::Eq, false, false), "==");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Neq, false, false), "!=");
    // Python textual bool
    assert_eq!(binary_op_to_str(&BinaryOpIR::And, false, true), "and");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Or, false, true), "or");
}

// ── format_expr_ir_tsx tests ────────────────────────────

#[test]
fn test_format_expr_literal() {
    let expr = ExprIR::LiteralExpr {
        value: serde_json::json!(42),
    };
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "42");
}

#[test]
fn test_format_expr_identifier() {
    let expr = ExprIR::IdentifierExpr {
        name: "count".into(),
        type_ann: None,
    };
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "count");
}

#[test]
fn test_format_expr_binary() {
    let expr = ExprIR::BinaryOpExpr {
        op: BinaryOpIR::Add,
        left: Box::new(ExprIR::IdentifierExpr {
            name: "a".into(),
            type_ann: None,
        }),
        right: Box::new(ExprIR::IdentifierExpr {
            name: "b".into(),
            type_ann: None,
        }),
    };
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "a + b");
}

#[test]
fn test_format_expr_ternary() {
    let expr = ExprIR::TernaryIR {
        cond: Box::new(ExprIR::IdentifierExpr {
            name: "x".into(),
            type_ann: None,
        }),
        then_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(1),
        }),
        else_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(0),
        }),
    };
    assert_eq!(
        format_expr_ir_tsx(&expr, &FxHashSet::default()),
        "x ? 1 : 0"
    );
}

#[test]
fn test_format_expr_field_access() {
    let expr = ExprIR::FieldAccessExpr {
        object: Box::new(ExprIR::IdentifierExpr {
            name: "obj".into(),
            type_ann: None,
        }),
        field: "prop".into(),
    };
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "obj.prop");
}

#[test]
fn test_format_expr_unary_not() {
    let expr = ExprIR::UnaryOpExpr {
        op: UnaryOpIR::Not,
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "flag".into(),
            type_ann: None,
        }),
    };
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "!flag");
}

#[test]
fn test_format_expr_store_field() {
    let expr = ExprIR::StoreFieldAccessExpr {
        store_alias: "store".into(),
        field_name: "getCount".into(),
        call_args: vec![],
    };
    assert_eq!(
        format_expr_ir_tsx(&expr, &FxHashSet::default()),
        "store.getCount"
    );
}

// ── generate_statement_tsx tests ─────────────────────────

#[test]
fn test_tsx_stmt_let() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
    };
    generate_statement_tsx(&mut w, &stmt, &FxHashSet::default(), &FxHashSet::default());
    assert_eq!(w.into_string(), "const x = 42;\n");
}

#[test]
fn test_tsx_stmt_return() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ReturnStmtIR {
        value: Some(ExprIR::IdentifierExpr {
            name: "result".into(),
            type_ann: None,
        }),
    };
    generate_statement_tsx(&mut w, &stmt, &FxHashSet::default(), &FxHashSet::default());
    assert_eq!(w.into_string(), "return result;\n");
}

#[test]
fn test_tsx_stmt_mutation_increment_local() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::Increment {
            target: SignalRefIR {
                name: "count".into(),
            },

            amount: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
        }),
    };
    // owner=None + not in signal_names → direct assignment
    generate_statement_tsx(&mut w, &stmt, &FxHashSet::default(), &FxHashSet::default());
    assert_eq!(w.into_string(), "count = count + 1;\n");
}

#[test]
fn test_tsx_stmt_mutation_increment_signal() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::Increment {
            target: SignalRefIR {
                name: "count".into(),
            },

            amount: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
        }),
    };
    // owner=None but "count" IS in signal_names → setter call
    let mut sig_names = FxHashSet::default();
    sig_names.insert("count".to_string());
    generate_statement_tsx(&mut w, &stmt, &sig_names, &FxHashSet::default());
    assert_eq!(w.into_string(), "setCount(prev => prev + 1);\n");
}

#[test]
fn test_tsx_stmt_mutation_assign_local() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "name".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!("alice"),
            })),
        }),
    };
    // owner=None + not in signal_names → direct assignment
    generate_statement_tsx(&mut w, &stmt, &FxHashSet::default(), &FxHashSet::default());
    assert_eq!(w.into_string(), "name = \"alice\";\n");
}

#[test]
fn test_tsx_stmt_mutation_assign_store() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "name".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!("alice"),
            })),
        }),
    };
    // Note: `owner` field removed from SignalRefIR — store signal
    // detection via target.owner is no longer available, so this
    // falls through to direct assignment.
    generate_statement_tsx(&mut w, &stmt, &FxHashSet::default(), &FxHashSet::default());
    assert_eq!(w.into_string(), "name = \"alice\";\n");
}

#[test]
fn test_tsx_stmt_if() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::IfStmtIR {
        condition: ExprIR::IdentifierExpr {
            name: "cond".into(),
            type_ann: None,
        },
        then_body: BlockIR {
            statements: vec![StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                r#type: None,
            }],
        },
        else_ifs: vec![],
        else_body: None,
    };
    generate_statement_tsx(&mut w, &stmt, &FxHashSet::default(), &FxHashSet::default());
    let out = w.into_string();
    assert!(out.contains("if (cond) {"));
    assert!(out.contains("const x = 1;"));
    assert!(out.contains("}"));
}

// ── gen_component_css tests ──────────────────────────────

#[test]
fn test_css_empty_component() {
    let codegen = ReactCodegen::default();
    let comp = ComponentIR {
        name: "App".into(),
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        events: vec![],
        view: None,
        styles: vec![],
        ..Default::default()
    };
    let css = codegen.gen_component_css("App", &comp);
    assert!(css.contains(".container"));
    assert!(css.contains("/* Styles for App */"));
}

#[test]
fn test_css_with_styles() {
    let codegen = ReactCodegen::default();
    let mut props = FxHashMap::default();
    props.insert(
        "backgroundColor".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("red"),
        },
    );
    props.insert(
        "fontSize".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!(16),
        },
    );
    let style = StyleRuleIR {
        class_name: "header".into(),
        properties: props,
        output_mode: "static".into(),
        is_tag_selector: false,
    };
    let comp = ComponentIR {
        name: "App".into(),
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        events: vec![],
        view: None,
        styles: vec![style],
        ..Default::default()
    };
    let css = codegen.gen_component_css("App", &comp);
    assert!(css.contains(".header"));
    assert!(css.contains("background-color: red;"));
    assert!(css.contains("font-size: 16;"));
}

// ── gen_component_tsx tests ──────────────────────────────

fn make_test_component(name: &str, view: ViewTree) -> ComponentIR {
    ComponentIR {
        name: name.into(),
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        events: vec![],
        view: Some(view),
        styles: vec![],
        ..Default::default()
    }
}

/// Build IRProgram with a single module containing the given components.
fn make_ir(components: FxHashMap<String, ComponentIR>) -> IRProgram {
    IRProgram {
        version: "1.0".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        stdlib_imports: Default::default(),
        source_map: FxHashMap::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

#[test]
fn test_tsx_interpolation_renders_expr() {
    let _codegen = ReactCodegen::default();
    let comp = make_test_component(
        "Test",
        ViewTree::Element(Box::new(ElementIR {
            tag: "div".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: vec![ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
                name: "count".into(),
                type_ann: None,
            }))],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            scroll: None,
            drag: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            construction_kind: ConstructionKind::Static,
            source_span: None,
        })),
    );
    let ir = make_ir(
        vec![("Test".into(), comp)]
            .into_iter()
            .collect::<FxHashMap<_, _>>(),
    );
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("Test")
        .unwrap();
    let component_map = build_component_map(&ir);
    let tsx = ReactCodegen::default().gen_component_tsx("Test", comp_ref, &ir, &component_map);
    assert!(
        !tsx.contains("/* interpolation */"),
        "Interpolation must NOT be a comment placeholder. Got: {}",
        tsx
    );
    assert!(
        tsx.contains("{count}"),
        "Should contain {{count}} JSX expression. Got: {}",
        tsx
    );
}

#[test]
fn test_tsx_empty_component() {
    let codegen = ReactCodegen::default();
    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            name: "App".into(),
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            events: vec![],
            view: None,
            styles: vec![],
            ..Default::default()
        },
    );
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("App")
        .unwrap();
    let tsx = codegen.gen_component_tsx("App", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("export function App() {"));
}

#[test]
fn test_tsx_component_with_state() {
    let codegen = ReactCodegen::default();
    let state = SignalIntent {
        name: "count".into(),
        r#type: TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
        initial_value: ExprIR::LiteralExpr {
            value: serde_json::json!(0),
        },
    };
    let mut components = FxHashMap::default();
    components.insert(
        "Counter".into(),
        ComponentIR {
            name: "Counter".into(),
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![state],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            events: vec![],
            view: None,
            styles: vec![],
            ..Default::default()
        },
    );
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("Counter")
        .unwrap();
    let tsx = codegen.gen_component_tsx("Counter", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("export function Counter() {"));
}

#[test]
fn test_tsx_component_with_props() {
    let codegen = ReactCodegen::default();
    let mut props = FxHashMap::default();
    props.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    props.insert(
        "age".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let mut components = FxHashMap::default();
    components.insert(
        "Greeting".into(),
        ComponentIR {
            name: "Greeting".into(),
            props,
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            events: vec![],
            view: None,
            styles: vec![],
            ..Default::default()
        },
    );
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("Greeting")
        .unwrap();
    let tsx = codegen.gen_component_tsx("Greeting", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("interface GreetingProps {"));
    assert!(tsx.contains("name: string;"));
    assert!(tsx.contains("age: number;"));
    assert!(tsx.contains("export function Greeting("));
    assert!(tsx.contains("}: GreetingProps) {"));
}

#[test]
fn test_tsx_component_with_effect() {
    let codegen = ReactCodegen::default();
    let effect = EffectIntent {
        deps: vec!["count".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::IdentifierExpr {
                    name: "console.log(count)".into(),
                    type_ann: None,
                },
            }],
        },
    };
    let mut components = FxHashMap::default();
    components.insert(
        "Watcher".into(),
        ComponentIR {
            name: "Watcher".into(),
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![effect],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            events: vec![],
            view: None,
            styles: vec![],
            ..Default::default()
        },
    );
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("Watcher")
        .unwrap();
    let tsx = codegen.gen_component_tsx("Watcher", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("useEffect(() => {"));
    assert!(tsx.contains("}, [count]);"));
}

#[test]
fn test_tsx_component_with_lifecycle_mount() {
    let codegen = ReactCodegen::default();
    let lc = LifecycleIntent {
        kind: LifecycleKind::Mount,
        body: BlockIR { statements: vec![] },
    };
    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            name: "App".into(),
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![lc],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            events: vec![],
            view: None,
            styles: vec![],
            ..Default::default()
        },
    );
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("App")
        .unwrap();
    let tsx = codegen.gen_component_tsx("App", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("useEffect(() => {"));
    assert!(tsx.contains("}, []);"));
}

// ── gen_store_hook tests ─────────────────────────────────

#[test]
fn test_store_hook_empty() {
    let codegen = ReactCodegen::default();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: FxHashMap::default(),
        signal_defaults: FxHashMap::default(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let hook = codegen.gen_store_hook("AppStore", &store);
    assert!(hook.contains("export function useAppStore() {"));
    assert!(hook.contains("import { useState, useCallback, useMemo } from 'react';"));
}

#[test]
fn test_store_hook_with_signals() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "count".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    signals.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("count".into(), serde_json::json!(0));
    defaults.insert("name".into(), serde_json::json!("world"));
    let store = StoreIR {
        name: "AppStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let hook = codegen.gen_store_hook("AppStore", &store);
    assert!(hook.contains("const [count, setCount] = useState<number>(0);"));
    assert!(hook.contains("const [name, setName] = useState<string>(\"world\");"));
    assert!(hook.contains("return useMemo("));
    assert!(hook.contains("count"));
    assert!(hook.contains("name"));
}

#[test]
fn test_store_hook_with_getters() {
    let codegen = ReactCodegen::default();
    let mut getters = FxHashMap::default();
    getters.insert(
        "doubleCount".into(),
        ExprIR::BinaryOpExpr {
            op: BinaryOpIR::Mul,
            left: Box::new(ExprIR::IdentifierExpr {
                name: "count".into(),
                type_ann: None,
            }),
            right: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(2),
            }),
        },
    );
    let mut signals = FxHashMap::default();
    signals.insert(
        "count".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("count".into(), serde_json::json!(0));
    let store = StoreIR {
        name: "CalcStore".into(),
        signals,
        signal_defaults: defaults,
        getters,
        actions: vec![],
        ..Default::default()
    };
    let hook = codegen.gen_store_hook("CalcStore", &store);
    assert!(hook.contains("const [count, setCount] = useState<number>(0);"));
    assert!(hook.contains("const doubleCount = useMemo(() => { return count * 2; }, [count]);"));
    assert!(hook.contains("return useMemo(() => ({ count, doubleCount }), [count, doubleCount]);"));
}

// ── Zustand store codegen tests ───────────────────────────

#[test]
fn test_zustand_store_empty() {
    let codegen = ReactCodegen {
        store_mode: super::ReactStoreMode::Zustand,
    };
    let store = StoreIR {
        name: "AppStore".into(),
        signals: FxHashMap::default(),
        signal_defaults: FxHashMap::default(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("AppStore", &store);
    assert!(result.contains("import { create } from 'zustand';"));
    assert!(result.contains("interface AppStoreState {"));
    assert!(result.contains("export const useAppStore"));
}

#[test]
fn test_zustand_store_with_signals_and_getters() {
    let codegen = ReactCodegen {
        store_mode: super::ReactStoreMode::Zustand,
    };
    let mut signals = FxHashMap::default();
    signals.insert(
        "count".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    signals.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("count".into(), serde_json::json!(0));
    defaults.insert("name".into(), serde_json::json!("world"));
    let mut getters = FxHashMap::default();
    getters.insert(
        "doubleCount".into(),
        ExprIR::BinaryOpExpr {
            op: BinaryOpIR::Mul,
            left: Box::new(ExprIR::IdentifierExpr {
                name: "count".into(),
                type_ann: None,
            }),
            right: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(2),
            }),
        },
    );
    let mut getter_types = FxHashMap::default();
    getter_types.insert(
        "doubleCount".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let store = StoreIR {
        name: "CalcStore".into(),
        signals,
        signal_defaults: defaults,
        getters,
        getter_types,
        actions: vec![],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("CalcStore", &store);
    assert!(
        result.contains("count: 0,"),
        "should emit signal default: {}",
        result
    );
    assert!(
        result.contains("name: \"world\","),
        "should emit string default: {}",
        result
    );
    assert!(
        result.contains("get doubleCount()"),
        "should emit getter accessor: {}",
        result
    );
    assert!(
        result.contains("readonly doubleCount: number;"),
        "should emit typed getter return: {}",
        result
    );
}

#[test]
fn test_zustand_store_mutation_multiply_in_action() {
    let codegen = ReactCodegen {
        store_mode: super::ReactStoreMode::Zustand,
    };
    let mut signals = FxHashMap::default();
    signals.insert(
        "count".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("count".into(), serde_json::json!(1));
    let store = StoreIR {
        name: "CalcStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "multiply".into(),
            visibility: Visibility::Public,
            params: vec![ParamIR {
                name: "factor".into(),
                r#type: Some(TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                }),
            }],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent::Multiply {
                        target: SignalRefIR {
                            name: "count".into(),
                        },
                        amount: Some(Box::new(ExprIR::IdentifierExpr {
                            name: "factor".into(),
                            type_ann: None,
                        })),
                    }),
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("CalcStore", &store);
    assert!(
        result.contains("set({ count: state.count * factor })") || result.contains("count *= "),
        "should emit multiply mutation, got: {}",
        result
    );
    assert!(
        !result.contains("TODO"),
        "should NOT contain TODO comment, got: {}",
        result
    );
    assert!(
        result.contains("multiply: (factor: number) => void;"),
        "should emit typed action param, got: {}",
        result
    );
}

// ── Full pipeline through Codegen trait ──────────────────

#[test]
fn test_full_pipeline_empty() {
    let codegen = ReactCodegen::default();
    let ir = IRProgram {
        version: "0.1".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::default(),
                stores: FxHashMap::default(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert!(output.files.is_empty());
}

#[test]
fn test_full_pipeline_component() {
    let codegen = ReactCodegen::default();
    let comp = ComponentIR {
        name: "App".into(),
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![SignalIntent {
            name: "count".into(),
            r#type: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            },
        }],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        events: vec![],
        view: None,
        styles: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: vec![("App".into(), comp)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                stores: FxHashMap::default(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 2); // .tsx + .module.css

    insta::with_settings!({snapshot_path => "snapshots"}, {
        let mut snapshot_content = String::new();
        for f in &output.files {
            let _ = write!(snapshot_content, "=== {} ===\n", f.path);
            snapshot_content.push_str(&f.content);
            snapshot_content.push('\n');
        }
        insta::assert_snapshot!("test_full_pipeline_component", &snapshot_content);
    });
}

#[test]
fn test_full_pipeline_with_store() {
    let codegen = ReactCodegen::default();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: vec![(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        signal_defaults: vec![("count".into(), serde_json::json!(0))]
            .into_iter()
            .collect::<FxHashMap<_, _>>(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::default(),
                stores: vec![("AppStore".into(), store)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 1); // just useAppStore.ts
    let hook = &output.files[0];
    assert_eq!(hook.path, "useAppStore.ts");
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_with_store", &hook.content);
    });
}

#[test]
fn test_full_pipeline_component_with_store_ref() {
    let codegen = ReactCodegen::default();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: vec![(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        signal_defaults: vec![("count".into(), serde_json::json!(0))]
            .into_iter()
            .collect::<FxHashMap<_, _>>(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let comp = ComponentIR {
        name: "App".into(),
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec!["AppStore".into()],
        refs: vec![],
        events: vec![],
        view: None,
        styles: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: vec![("App".into(), comp)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                stores: vec![("AppStore".into(), store)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 3); // .tsx + .module.css + useAppStore.ts
    let tsx = output.files.iter().find(|f| f.path == "App.tsx").unwrap();
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_component_with_store_ref", &tsx.content);
    });
}

// ═══════════════════════════════════════════════════════════
// Integration tests (full pipeline)
// ═══════════════════════════════════════════════════════════

/// Compile a single .omni source string to React, returning the InternalCompileResult.
fn compile_react(source: &str) -> Option<crate::InternalCompileResult> {
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    if result.success {
        Some(result)
    } else {
        None
    }
}

// ── Ref codegen integration tests ────────────────────────

/// Test: ref declaration generates useRef + ref={} JSX attribute
#[test]
fn test_react_ref_basic() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box>content</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // useRef import
    assert!(
        content.contains("useRef"),
        "Import should include useRef. Got:\n{}",
        content
    );
    // useRef declaration
    assert!(
        content.contains("const boxRef = useRef<HTMLElement>(null)"),
        "Should generate useRef declaration. Got:\n{}",
        content
    );
    // ref={} JSX attribute
    assert!(
        content.contains("ref={boxRef}"),
        "JSX should have ref={{boxRef}}. Got:\n{}",
        content
    );
    // NOT data-ref (legacy lowering artifact)
    assert!(
        !content.contains("data-ref"),
        "Should not emit data-ref attribute. Got:\n{}",
        content
    );
}

/// Test: ref declaration + JSX ref binding in event handler component
#[test]
fn test_react_ref_in_handler() {
    let source = r#"
component DragRef {
  signal x: Float = 0
  ref box: Element
  fn onDrag(e) {
    box.style.transform = "translate(100px, 0)"
  }
  view {
    <div ref:box @onPointerMove={onDrag}>drag</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // useRef imported
    assert!(
        content.contains("useRef"),
        "useRef should be imported. Got:\n{}",
        content
    );
    // useRef declaration
    assert!(
        content.contains("const boxRef = useRef<HTMLElement>(null)"),
        "Should generate useRef declaration for 'box'. Got:\n{}",
        content
    );
    // ref={boxRef} in JSX
    assert!(
        content.contains("ref={boxRef}"),
        "JSX should have ref={{boxRef}}. Got:\n{}",
        content
    );
    // Ref names in function bodies are now translated to nameRef.current (ADR-036)
    assert!(
        content.contains("boxRef.current.style.transform"),
        "Handler body should reference 'boxRef.current'. Got:\n{}",
        content
    );
}

/// Test: component without refs does NOT import useRef
#[test]
fn test_react_no_ref_no_use_ref_import() {
    let source = r#"
component NoRef {
  signal count: Float = 0
  view {
    <div>{count}</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("useRef"),
        "useRef should NOT be imported when no refs. Got:\n{}",
        content
    );
}

#[test]
fn warns_on_slot_in_react() {
    let source = r#"
component Card {
    view {
        <div class="card">@slot</div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("WARNING:")
            && content.contains("slot")
            && content.contains("not supported"),
        "Expected slot warning in React output, got: {}",
        content
    );
}
