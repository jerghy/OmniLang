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
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
    );
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
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
    );
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
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
    );
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
    generate_statement_tsx(
        &mut w,
        &stmt,
        &sig_names,
        &FxHashMap::default(),
        &FxHashSet::default(),
    );
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
            field_object: None,
        }),
    };
    // owner=None + not in signal_names → direct assignment
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
    );
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
            field_object: None,
        }),
    };
    // Note: `owner` field removed from SignalRefIR — store signal
    // detection via target.owner is no longer available, so this
    // falls through to direct assignment.
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
    );
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
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
    );
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

// ── Zustand store codegen tests ───────────────────────────

#[test]
fn test_zustand_store_empty() {
    let codegen = ReactCodegen::default();
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
    assert!(
        result.contains("export function useCount() {"),
        "should emit standalone signal selector hook: {}",
        result
    );
    assert!(
        result.contains("export function useDoubleCount() {"),
        "should emit standalone getter selector hook: {}",
        result
    );
    assert!(
        result.contains("return useCalcStore((s) => s.count);"),
        "selector hook should subscribe via the store hook: {}",
        result
    );
}

#[test]
fn test_zustand_store_mutation_multiply_in_action() {
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
    assert!(
        result.contains("export function useMultiply() {"),
        "should emit standalone action selector hook, got: {}",
        result
    );
}

/// Regression (CRITICAL): `items = [...items, {...}]` must emit the
/// `const state = get();` prologue and rename `items` to `state.items`.
/// The prologue walker must recurse into list/object literals.
#[test]
fn test_zustand_store_array_literal_mutation_emits_prologue() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "items".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("items".into(), serde_json::json!([]));
    let store = StoreIR {
        name: "TodoStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "add".into(),
            visibility: Visibility::Public,
            params: vec![ParamIR {
                name: "text".into(),
                r#type: Some(TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                }),
            }],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent::DirectAssign {
                        target: SignalRefIR {
                            name: "items".into(),
                        },
                        value: Some(Box::new(ExprIR::ArrayExpr {
                            elements: vec![
                                ExprIR::SpreadExpr {
                                    operand: Box::new(ExprIR::IdentifierExpr {
                                        name: "items".into(),
                                        type_ann: None,
                                    }),
                                },
                                ExprIR::MapExpr {
                                    entries: vec![
                                        (
                                            ExprIR::LiteralExpr {
                                                value: serde_json::json!("text"),
                                            },
                                            ExprIR::IdentifierExpr {
                                                name: "text".into(),
                                                type_ann: None,
                                            },
                                        ),
                                        (
                                            ExprIR::LiteralExpr {
                                                value: serde_json::json!("done"),
                                            },
                                            ExprIR::LiteralExpr {
                                                value: serde_json::json!(false),
                                            },
                                        ),
                                    ],
                                },
                            ],
                        })),
                        field_object: None,
                    }),
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("TodoStore", &store);
    assert!(
        result.contains("const state = get();"),
        "array-literal mutation must emit the prologue, got: {}",
        result
    );
    assert!(
        result.contains("set({ items: [...state.items,"),
        "items must be renamed to state.items inside the array literal, got: {}",
        result
    );
    assert!(
        result.contains("\"text\": text"),
        "object literal values must keep local identifiers, got: {}",
        result
    );
}

/// Regression (HIGH #1): getter references inside action bodies must be
/// renamed to `state.<getter>` and trigger the prologue.
#[test]
fn test_zustand_store_getter_ref_renamed_in_action() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "notifications".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("notifications".into(), serde_json::json!([]));
    let mut getters = FxHashMap::default();
    getters.insert(
        "unreadCount".into(),
        ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "notifications".into(),
                type_ann: None,
            }),
            method: "len".into(),
            args: vec![],
            is_optional: false,
        },
    );
    let mut getter_types = FxHashMap::default();
    getter_types.insert(
        "unreadCount".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let store = StoreIR {
        name: "NotificationStore".into(),
        signals,
        signal_defaults: defaults,
        getters,
        getter_types,
        actions: vec![FunctionIR {
            name: "logUnread".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::LetStmtIR {
                    name: "summary".into(),
                    value: Some(ExprIR::MethodCallExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "unreadCount".into(),
                            type_ann: None,
                        }),
                        method: "to_string".into(),
                        args: vec![],
                        is_optional: false,
                    }),
                    r#type: None,
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("NotificationStore", &store);
    assert!(
        result.contains("const state = get();"),
        "getter read in action must emit the prologue, got: {}",
        result
    );
    assert!(
        result.contains("state.unreadCount.toString()"),
        "getter reference must be renamed to state.unreadCount, got: {}",
        result
    );
}

/// Regression (HIGH #2): string literals containing signal names must NOT be
/// renamed — the rename happens at the AST identifier level only.
#[test]
fn test_zustand_store_string_literal_not_renamed() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "items".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("items".into(), serde_json::json!([]));
    let store = StoreIR {
        name: "CartStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "logCart".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: ExprIR::CallExpr {
                        name: "println".into(),
                        args: vec![ExprIR::LiteralExpr {
                            value: serde_json::json!("items in cart"),
                        }],
                    },
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("CartStore", &store);
    assert!(
        result.contains("console.log(\"items in cart\")"),
        "string literal containing a signal name must stay intact, got: {}",
        result
    );
    assert!(
        !result.contains("state.items in cart"),
        "string literal must NOT be renamed, got: {}",
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

// ── Design codegen integration tests ─────────────────────

/// Test: a design block compiles to design.css (unscoped, plain HTML) plus
/// useThemeStore.ts when @themes is declared.
#[test]
fn react_design_emits_css_and_theme_store() {
    let source = r##"
design AppDesign {
    @themes light, dark;
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token surface: color = "#ffffff" dark "#111827";

    class Card {
        background: token(surface);
        border-radius: "8px";
    }

    variant Button {
        @dim color: primary | danger;
        @default { color: primary; }
        base {
            display: inline-flex;
        }
        color.primary { background: token(primary); }
    }
}
"##;
    let output = compile_react(source).unwrap();
    // design.css + useThemeStore.ts
    let css = output
        .files
        .get("design.css")
        .expect("design.css should be emitted");
    assert!(
        css.contains(":root {"),
        "tokens must be declared on :root. Got:\n{}",
        css
    );
    assert!(
        css.contains("--primary: #6366f1;"),
        "static token value must be emitted. Got:\n{}",
        css
    );
    assert!(
        css.contains("[data-theme=\"dark\"] {"),
        "theme overrides must be emitted. Got:\n{}",
        css
    );
    assert!(
        css.contains("--primary: #a5b4fc;"),
        "dark theme token override must be emitted. Got:\n{}",
        css
    );
    assert!(
        css.contains("\n.Card {") && css.contains("background: var(--surface);"),
        "class rules must be emitted unscoped. Got:\n{}",
        css
    );
    assert!(
        css.contains("\n.Button {") && css.contains("\n.Button.primary {"),
        "variant base + branch rules must be emitted. Got:\n{}",
        css
    );
    assert!(
        !css.contains("[data-design~="),
        "React design CSS must be unscoped (plain HTML, no data-design wrapper)"
    );

    let theme_store = output
        .files
        .get("useThemeStore.ts")
        .expect("useThemeStore.ts should be emitted when @themes is declared");
    assert!(
        theme_store.contains("theme: 'light',"),
        "default theme must come from @default-theme. Got:\n{}",
        theme_store
    );
    assert!(
        theme_store.contains("setTheme: (theme) => set({ theme }),"),
        "setTheme action must be emitted. Got:\n{}",
        theme_store
    );
    assert!(
        theme_store.contains(
            "document.documentElement.setAttribute('data-theme', state.theme);"
        ),
        "data-theme subscription must be emitted. Got:\n{}",
        theme_store
    );

    // No dynamic tokens → no useDesignTokens.ts
    assert!(
        !output.files.contains_key("useDesignTokens.ts"),
        "no hook without dynamic tokens"
    );
}

/// Test: dynamic tokens referencing store signals emit a useDesignTokens.ts
/// hook with per-token useEffect setProperty calls and the store hook import.
#[test]
fn react_design_dynamic_tokens_emit_hook() {
    let source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
    signal liveBg: String = "#eef2ff";
}

design AppDesign {
    token dynColor: color = DynamicTokenStore.liveColor;
    token dynBg: color = DynamicTokenStore.liveBg;
    token staticColor: color = "#ffffff";
}
"##;
    let output = compile_react(source).unwrap();
    let hook = output
        .files
        .get("useDesignTokens.ts")
        .expect("useDesignTokens.ts should be emitted for dynamic tokens");
    assert!(
        hook.contains("import { useEffect } from 'react';"),
        "useEffect must be imported. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("const DynamicTokenStore = useDynamicTokenStore();"),
        "store hook must be instantiated. Got:\n{}",
        hook
    );
    assert!(
        hook.contains(
            "document.documentElement.style.setProperty('--dynColor', sanitizeCss(DynamicTokenStore.liveColor));"
        ),
        "dynamic token must set the CSS custom property at runtime. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("}, [DynamicTokenStore.liveColor]);"),
        "effect deps must be the exact store signal expression. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("}, [DynamicTokenStore.liveBg]);"),
        "one effect per dynamic token. Got:\n{}",
        hook
    );
    // The store hook file itself must also be generated
    assert!(
        output.files.contains_key("useDynamicTokenStore.ts"),
        "referenced store hook must be generated"
    );
}

/// Test: design outputs land in the directory mirroring the design's source
/// file, and multiple design blocks in one file share one set of outputs.
#[test]
fn react_design_files_follow_source_directory() {
    let store_source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}
"##;
    let design_source = r##"
design ADesign {
    token a: color = "#111111";
}
design BDesign {
    token b: color = "#222222";
}
"##;
    let comp_source = r#"
component App {
    view {
        <div>hello</div>
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert("src/stores/DynamicTokenStore.omni".to_string(), store_source.to_string());
    sources.insert("src/design.omni".to_string(), design_source.to_string());
    sources.insert("src/main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "design compilation must succeed. Errors: {:?}",
        result.errors
    );
    // design.css at the design file's level (src/ → output root)
    let css = result
        .files
        .get("design.css")
        .expect("design.css should be emitted at output root");
    assert!(
        css.contains("Design: ADesign") && css.contains("Design: BDesign"),
        "both design blocks must be concatenated into one design.css. Got:\n{}",
        css
    );
    assert!(css.contains("--a: #111111;") && css.contains("--b: #222222;"));
    // Store hook generated in stores/
    assert!(result.files.contains_key("stores/useDynamicTokenStore.ts"));
    // Component at root (src/main.omni → output root)
    assert!(result.files.contains_key("App.tsx"));
}

/// Test (HIGH #1): design classes and variant combination classes are
/// emitted as literal class names so the global rules in design.css match;
/// component-local style classes keep CSS-module lookups.
#[test]
fn react_design_classes_emit_literal_class_names() {
    let source = r##"
design LearnDesign {
    @themes light, dark;
    token primary: color = "#6366f1";

    class Page {
        background: token(primary);
    }
    class Card {
        border-radius: "8px";
    }

    variant Button {
        @dim color: primary | danger;
        @default { color: primary; }
        base { display: inline-flex; }
        color.primary { background: token(primary); }
    }
}

component App {
    signal isActive: bool = true
    style {
        .box { padding: 8px; }
    }
    view {
        <div>
            <div class: LearnDesign.Page>page</div>
            <button variant: LearnDesign.Button(color=primary)>go</button>
            <div class: { Card: true, box: isActive }>card</div>
        </div>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");

    // `class: LearnDesign.Page` → literal class name (global .Page rule)
    assert!(
        app.contains("className={\"Page\"}"),
        "design class must be a literal class name. Got:\n{}",
        app
    );
    assert!(
        !app.contains("styles['Page']"),
        "design class must NOT go through the CSS module. Got:\n{}",
        app
    );
    // `variant: LearnDesign.Button(color=primary)` → "Button primary"
    assert!(
        app.contains("className={\"Button primary\"}"),
        "variant binding must emit combination classes as literals. Got:\n{}",
        app
    );
    assert!(
        !app.contains("styles['Button']"),
        "variant classes must NOT go through the CSS module. Got:\n{}",
        app
    );
    // class map: design class literal + local class CSS-module lookup
    assert!(
        app.contains("\"Card\"") && app.contains("styles['box']"),
        "class map must mix design literals and module lookups. Got:\n{}",
        app
    );
    // design.css still emits the global rules the literals match
    let css = output
        .files
        .get("design.css")
        .expect("design.css should be emitted");
    assert!(css.contains("\n.Page {"), "global .Page rule missing");
    assert!(
        css.contains("\n.Button.primary {"),
        "global .Button.primary rule missing"
    );
}

/// Test (HIGH #1): a component using ONLY design classes does not import a
/// CSS module (nothing local to look up).
#[test]
fn react_design_only_classes_skip_css_module_import() {
    let source = r##"
design LearnDesign {
    token primary: color = "#6366f1";
    class Page {
        background: token(primary);
    }
}

component App {
    view {
        <div class: LearnDesign.Page>page</div>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        !app.contains("import styles from './App.module.css';"),
        "no CSS module import when only design classes are used. Got:\n{}",
        app
    );
    assert!(
        app.contains("className={\"Page\"}"),
        "design class must still be a literal. Got:\n{}",
        app
    );
}

/// Test (HIGH #2): the root component imports and calls `useDesignTokens`
/// and side-effect-imports `useThemeStore`, so the emitted design runtime
/// actually activates.
#[test]
fn react_root_component_wires_design_runtime() {
    let source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}

design AppDesign {
    @themes light, dark;
    @default-theme light;
    token dynColor: color = DynamicTokenStore.liveColor;
    token staticColor: color = "#ffffff";
}

component App {
    view {
        <div>hello</div>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        app.contains("import { useDesignTokens } from './useDesignTokens';"),
        "root component must import the design tokens hook. Got:\n{}",
        app
    );
    assert!(
        app.contains("useDesignTokens();"),
        "root component must call the design tokens hook. Got:\n{}",
        app
    );
    assert!(
        app.contains("import './useThemeStore';"),
        "root component must import the theme store (side effect). Got:\n{}",
        app
    );
    // Non-root components must not import the wiring
    let non_root = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}

design AppDesign {
    @themes light, dark;
    token dynColor: color = DynamicTokenStore.liveColor;
}

component App {
    view {
        <div>app</div>
    }
}

component Sidebar {
    view {
        <div>side</div>
    }
}
"##;
    let output = compile_react(non_root).unwrap();
    let sidebar = output
        .files
        .get("Sidebar.tsx")
        .expect("Sidebar.tsx should be emitted");
    assert!(
        !sidebar.contains("useDesignTokens"),
        "non-root components must not import design hooks. Got:\n{}",
        sidebar
    );
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        app.contains("useDesignTokens();"),
        "App must still be wired as root. Got:\n{}",
        app
    );
}

/// Test (HIGH #2): the root component wires hooks from a design file in a
/// different source directory via a correct relative import path.
#[test]
fn react_root_component_wires_design_across_directories() {
    let store_source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}
"##;
    let design_source = r##"
import { DynamicTokenStore } from "./stores/DynamicTokenStore";

design AppDesign {
    @themes light, dark;
    token dynColor: color = DynamicTokenStore.liveColor;
}
"##;
    let comp_source = r#"
component App {
    view {
        <div>app</div>
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert("src/stores/DynamicTokenStore.omni".to_string(), store_source.to_string());
    sources.insert("src/designs/theme.omni".to_string(), design_source.to_string());
    sources.insert("src/main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "design compilation must succeed. Errors: {:?}",
        result.errors
    );
    let app = result
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        app.contains("import { useDesignTokens } from './designs/useDesignTokens';"),
        "hook import must resolve across directories. Got:\n{}",
        app
    );
    assert!(
        app.contains("useDesignTokens();"),
        "hook must be called. Got:\n{}",
        app
    );
    assert!(
        result.files.contains_key("designs/useDesignTokens.ts"),
        "hook file must be emitted next to the design source"
    );
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
    // useRef declaration — typed by the bound element's tag (div → HTMLDivElement)
    assert!(
        content.contains("const boxRef = useRef<HTMLDivElement>(null)"),
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
    box.focus()
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
    // useRef declaration — typed by the bound element's tag (div → HTMLDivElement)
    assert!(
        content.contains("const boxRef = useRef<HTMLDivElement>(null)"),
        "Should generate useRef declaration for 'box'. Got:\n{}",
        content
    );
    // ref={boxRef} in JSX
    assert!(
        content.contains("ref={boxRef}"),
        "JSX should have ref={{boxRef}}. Got:\n{}",
        content
    );
    // Legal ref ops are rewritten to ExprIR::RefOpExpr by validate_ref_ops
    // and codegen emits the native DOM call on the ref's .current (ADR-036).
    // (ref.style.* was removed by the semantic DSL — ADR-052/053 — and now
    // produces a compile error instead.)
    assert!(
        content.contains("boxRef.current?.focus()"),
        "Handler body should emit the native focus() call. Got:\n{}",
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

// ── Ref semantic binding + RefOpExpr codegen tests ────────

/// Test: useRef is typed by the bound element's tag — input → HTMLInputElement,
/// textarea → HTMLTextAreaElement, canvas → HTMLCanvasElement, div → HTMLDivElement.
#[test]
fn test_react_ref_typed_by_element_tag() {
    let source = r#"
component TypedRefs {
  ref field: Element
  ref area: Element
  ref canvasEl: Element
  ref box: Element
  view {
    <div>
      <input ref:field />
      <textarea ref:area></textarea>
      <canvas ref:canvasEl></canvas>
      <div ref:box />
    </div>
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
        content.contains("const fieldRef = useRef<HTMLInputElement>(null)"),
        "input should type useRef as HTMLInputElement. Got:\n{}",
        content
    );
    assert!(
        content.contains("const areaRef = useRef<HTMLTextAreaElement>(null)"),
        "textarea should type useRef as HTMLTextAreaElement. Got:\n{}",
        content
    );
    assert!(
        content.contains("const canvasElRef = useRef<HTMLCanvasElement>(null)"),
        "canvas should type useRef as HTMLCanvasElement. Got:\n{}",
        content
    );
    assert!(
        content.contains("const boxRef = useRef<HTMLDivElement>(null)"),
        "div should type useRef as HTMLDivElement. Got:\n{}",
        content
    );
}

/// Test: semantic bindings (focus, scroll-y, value) on a ref-bound element
/// generate useEffect blocks driven by the bound signal.
#[test]
fn test_react_semantic_bindings_generate_effects() {
    let source = r#"
component SearchBox {
  signal active: Bool = false
  signal pos: Float = 0
  signal text: String = ""
  ref field: Element
  view {
    <input ref:field focus={active} scroll-y={pos} value={text} />
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
    // useEffect must be imported for the binding effects
    assert!(
        content.contains("useEffect"),
        "useEffect should be imported for semantic bindings. Got:\n{}",
        content
    );
    // focus binding → focus()/blur() on the boolean signal
    assert!(
        content.contains("if (active) {")
            && content.contains("fieldRef.current?.focus();")
            && content.contains("fieldRef.current?.blur();"),
        "focus binding should emit focus/blur effect. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [active]);"),
        "focus effect deps should track the bound signal. Got:\n{}",
        content
    );
    // scroll-y binding → scrollTo({ top: pos })
    assert!(
        content.contains("fieldRef.current?.scrollTo({ top: pos });"),
        "scroll-y binding should emit scrollTo top. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [pos]);"),
        "scroll-y effect deps should track the bound signal. Got:\n{}",
        content
    );
    // value binding → guarded imperative value sync
    assert!(
        content.contains(
            "if (fieldRef.current && fieldRef.current.value !== text) {\n            fieldRef.current.value = text;"
        ),
        "value binding should emit guarded value sync. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [text]);"),
        "value effect deps should track the bound signal. Got:\n{}",
        content
    );
}

/// Test: scroll-x semantic binding emits scrollTo({ left: sig }).
#[test]
fn test_react_semantic_binding_scroll_x() {
    let source = r#"
component Carousel {
  signal pos: Float = 0
  ref track: Element
  view {
    <div ref:track scroll-x={pos}>items</div>
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
        content.contains("trackRef.current?.scrollTo({ left: pos });"),
        "scroll-x binding should emit scrollTo left. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [pos]);"),
        "scroll-x effect deps should track the bound signal. Got:\n{}",
        content
    );
}

/// Test: store signals work in semantic binding position — `value={Store.sig}`
/// is a semantic binding (not a raw JSX prop), generates the store hook, and
/// the effect depends on `Store.sig` (not the store hook object).
#[test]
fn test_react_semantic_binding_store_signal() {
    let store_source = r#"
store CartStore {
    signal text: String = "";
}
"#;
    let comp_source = r#"
import { CartStore } from "./store";

component StoreBinding {
    @store CartStore;
    ref field: Element;
    view {
        <input ref:field value={CartStore.text} />
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert("store.omni".to_string(), store_source.to_string());
    sources.insert("main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "store-backed semantic binding must compile. Errors: {:?}",
        result.errors
    );
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // Store hook must be generated — binding signal feeds used_store_refs
    assert!(
        content.contains("const CartStore = useCartStore();"),
        "store hook must be generated for a store-backed binding. Got:\n{}",
        content
    );
    // value={Store.sig} must NOT leak as a raw JSX prop (silent one-way break)
    assert!(
        !content.contains("value={CartStore.text}"),
        "store binding must not emit a raw value prop. Got:\n{}",
        content
    );
    // The generated effect reads the qualified signal and depends on the
    // exact expression, so unrelated store updates don't re-fire it.
    assert!(
        content.contains("fieldRef.current.value !== CartStore.text"),
        "value binding should read the qualified store signal. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [CartStore.text]);"),
        "value effect deps should be Store.sig, not the store hook. Got:\n{}",
        content
    );
}

/// Test: every RefOpIR variant formats to a native DOM call on `nameRef.current`.
#[test]
fn test_react_ref_ops_format_native_dom_calls() {
    let source = r#"
component RefOps {
  signal x: Float = 0
  signal y: Float = 0
  signal text: String = ""
  signal ptr: Float = 0
  ref box: Element
  fn runAll() {
    box.focus()
    box.blur()
    box.scrollTo(x, y)
    box.scrollBy(1, 2)
    box.measure()
    box.width
    box.height
    box.position
    box.select()
    box.setSelection(0, 3)
    box.value
    box.value = text
    box.capturePointer(ptr)
    box.releasePointer(ptr)
  }
  view {
    <div ref:box @onClick={runAll}>click</div>
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
    let expectations = [
        "boxRef.current?.focus();",
        "boxRef.current?.blur();",
        "boxRef.current?.scrollTo({ left: x, top: y });",
        "boxRef.current?.scrollBy({ left: 1, top: 2 });",
        "boxRef.current?.getBoundingClientRect();",
        "boxRef.current?.getBoundingClientRect().width;",
        "boxRef.current?.getBoundingClientRect().height;",
        "boxRef.current?.getBoundingClientRect();",
        "boxRef.current?.select();",
        "boxRef.current?.setSelectionRange(0, 3);",
        "boxRef.current?.value;",
        "if (boxRef.current) { boxRef.current.value = text; };",
        "boxRef.current?.setPointerCapture(ptr);",
        "boxRef.current?.releasePointerCapture(ptr);",
    ];
    for expected in expectations {
        assert!(
            content.contains(expected),
            "expected RefOp output to contain {:?}. Got:\n{}",
            expected,
            content
        );
    }
}

/// Test: illegal DOM APIs on refs still produce compile errors with migration hints.
#[test]
fn test_react_ref_illegal_ops_compile_error() {
    let source = r#"
component BadRefOps {
  ref box: Element
  fn bad() {
    box.innerHTML = "x"
    box.getBoundingClientRect()
  }
  view {
    <div ref:box>content</div>
  }
}
"#;
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
    assert!(!result.success, "illegal ref ops must fail compilation");
    let messages: Vec<&str> = result.errors.iter().map(|e| e.message.as_str()).collect();
    assert!(
        messages.iter().any(|m| m.contains("ref.innerHTML is not allowed")
            || m.contains("unknown ref field: ref.innerHTML")),
        "expected innerHTML migration error, got: {:?}",
        messages
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("ref.getBoundingClientRect() is not allowed")),
        "expected getBoundingClientRect migration error, got: {:?}",
        messages
    );
}

/// Test: unnamed slot without fallback renders `{children}` and the Props
/// interface declares `children?: ReactNode`.
#[test]
fn slot_unnamed_renders_children_prop() {
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
        content.contains("children?: ReactNode"),
        "Expected children slot prop in Props interface, got:\n{}",
        content
    );
    assert!(
        content.contains("export function Card({ children }: CardProps)"),
        "Expected destructured children prop, got:\n{}",
        content
    );
    assert!(
        content.contains("{children}"),
        "Expected {{children}} render, got:\n{}",
        content
    );
}

/// Test: named slot with fallback renders `{header ?? (fallback)}` and the
/// Props interface declares `header?: ReactNode`.
#[test]
fn slot_named_renders_prop_with_fallback() {
    let source = r#"
component Card {
    view {
        <div class="card">
            @slot header {
                <h4>Default Header</h4>
            }
        </div>
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
        content.contains("header?: ReactNode"),
        "Expected header slot prop in Props interface, got:\n{}",
        content
    );
    assert!(
        content.contains("export function Card({ header }: CardProps)"),
        "Expected destructured header prop, got:\n{}",
        content
    );
    assert!(
        content.contains("{header ?? ("),
        "Expected fallback-wrapped header render, got:\n{}",
        content
    );
    assert!(
        content.contains("Default Header"),
        "Expected fallback content in output, got:\n{}",
        content
    );
    assert!(
        !content.contains("WARNING:"),
        "Slot warning should be gone, got:\n{}",
        content
    );
}

/// Test: unnamed slot with fallback renders `{children ?? (fallback)}`.
#[test]
fn slot_unnamed_renders_children_with_fallback() {
    let source = r#"
component Panel {
    view {
        <div>
            @slot {
                <p>Default Body</p>
            }
        </div>
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
        content.contains("{children ?? ("),
        "Expected fallback-wrapped children render, got:\n{}",
        content
    );
    assert!(
        content.contains("children?: ReactNode"),
        "Expected children slot prop in Props interface, got:\n{}",
        content
    );
}

/// Test: `@slot header: String { ... }` type constraint types the prop as
/// `string` (fallback braces required — bare `@slot name: Type` is ambiguous
/// with the following `</tag>` at parse time, see parser view/mod.rs).
#[test]
fn slot_type_constraint_types_prop() {
    let source = r#"
component Badge {
    view {
        <div>
            @slot header: String { <span>Fallback</span> }
        </div>
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
        content.contains("header?: string"),
        "Expected string-typed header prop from type constraint, got:\n{}",
        content
    );
}

/// Test: a slot inside an `@if` body with sibling nodes is wrapped in a
/// React fragment so the emitted TSX stays valid (`{cond && (<>...)}`).
#[test]
fn slot_inside_conditional_wraps_in_fragment() {
    let source = r#"
component Panel {
    signal show: bool = true
    view {
        @if show {
            @slot
            <p>No data</p>
        }
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
        content.contains("&& (") && content.contains("<>") && content.contains("</>"),
        "Expected fragment-wrapped conditional body, got:\n{}",
        content
    );
    assert!(
        content.contains("{children}"),
        "Expected {{children}} slot render, got:\n{}",
        content
    );
}

/// Test: a multi-node slot fallback is wrapped in a React fragment inside
/// `{prop ?? ( ... )}` so the emitted TSX stays valid.
#[test]
fn slot_multi_node_fallback_wraps_in_fragment() {
    let source = r#"
component Card {
    view {
        <div class="card">
            @slot header {
                <h4>Title</h4>
                <p>Subtitle</p>
            }
        </div>
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
        content.contains("{header ?? (") && content.contains("<>") && content.contains("</>"),
        "Expected fragment-wrapped multi-node fallback, got:\n{}",
        content
    );
    assert!(
        content.contains("Title") && content.contains("Subtitle"),
        "Expected both fallback nodes, got:\n{}",
        content
    );
}

/// Test: consumer-position `@slot name { ... }` inside a component's
/// children projects into the child component's named prop — it is neither
/// rendered as a bare identifier nor added to the consumer's Props interface.
#[test]
fn slot_projection_directive_at_call_site() {
    let source = r#"
component SlotBox {
    view {
        <div class: box>
            <div>@slot header</div>
            <div>@slot body</div>
        </div>
    }
}

component Consumer {
    view {
        <SlotBox>
            @slot header {
                <h4>Custom Header</h4>
            }
            @slot {
                <p>Body content</p>
            }
            <span>Default child</span>
        </SlotBox>
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
    let consumer: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("Consumer.tsx"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("header={<h4>") || content.contains("header={<h4"),
        "Expected @slot directive projected to header prop, got:\n{}",
        content
    );
    assert!(
        content.contains("Custom Header"),
        "Expected projected header content, got:\n{}",
        content
    );
    assert!(
        content.contains("Body content"),
        "Expected unnamed @slot directive content as children, got:\n{}",
        content
    );
    assert!(
        !consumer.contains("header?: ReactNode"),
        "Consumer Props interface must not be polluted by projection slots, got:\n{}",
        consumer
    );
    assert!(
        !consumer.contains("interface ConsumerProps"),
        "Consumer without own props/declarations should have no Props interface, got:\n{}",
        consumer
    );
}

/// Test: consumer-side `slot="name"` children project into named props at
/// the call site (with the `slot` attribute stripped).
#[test]
fn slot_projection_at_call_site() {
    let source = r#"
component SlotBox {
    view {
        <div class: box>
            <div>@slot header</div>
            <div>@slot body</div>
        </div>
    }
}

component Consumer {
    signal title: String = "T"
    view {
        <SlotBox>
            <h4 slot="header">Custom Header</h4>
            <div slot="body">
                <p>Body content</p>
            </div>
            <span>Default child</span>
        </SlotBox>
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
        content.contains("header={"),
        "Expected named slot projected to header prop, got:\n{}",
        content
    );
    assert!(
        content.contains("body={"),
        "Expected named slot projected to body prop, got:\n{}",
        content
    );
    assert!(
        content.contains("Custom Header"),
        "Expected projected header content, got:\n{}",
        content
    );
    assert!(
        content.contains("Default child"),
        "Expected un-slotted child to remain in children, got:\n{}",
        content
    );
    assert!(
        !content.contains("slot=\"header\""),
        "slot attribute must be stripped after projection, got:\n{}",
        content
    );
}
