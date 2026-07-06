// ═══════════════════════════════════════════════════════════
// Unit tests
// ═══════════════════════════════════════════════════════════

use std::collections::{HashMap, HashSet};

use super::expr::*;
#[allow(unused_imports)]
use super::utils::*;
use super::ReactCodegen;
use crate::codegen::{
    binary_op_to_str, camel_to_kebab, capitalize_first, format_json_value, CodeWriter, Codegen,
    JSON_REACT,
};
use crate::ir::ModuleIR;
use crate::ir::*;

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
    assert_eq!(binary_op_to_str("add", true, false), "+");
    assert_eq!(binary_op_to_str("eq", true, false), "===");
    assert_eq!(binary_op_to_str("and", true, false), "&&");
    assert_eq!(binary_op_to_str("or", true, false), "||");
    assert_eq!(binary_op_to_str("neq", true, false), "!==");
    // C-loose eq
    assert_eq!(binary_op_to_str("eq", false, false), "==");
    assert_eq!(binary_op_to_str("neq", false, false), "!=");
    // Python textual bool
    assert_eq!(binary_op_to_str("and", false, true), "and");
    assert_eq!(binary_op_to_str("or", false, true), "or");
}

// ── format_expr_ir_tsx tests ────────────────────────────

#[test]
fn test_format_expr_literal() {
    let expr = ExprIR::LiteralExpr {
        value: serde_json::json!(42),
    };
    assert_eq!(format_expr_ir_tsx(&expr), "42");
}

#[test]
fn test_format_expr_identifier() {
    let expr = ExprIR::IdentifierExpr {
        name: "count".into(),
    };
    assert_eq!(format_expr_ir_tsx(&expr), "count");
}

#[test]
fn test_format_expr_binary() {
    let expr = ExprIR::BinaryOpExpr {
        op: "add".into(),
        left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }),
        right: Box::new(ExprIR::IdentifierExpr { name: "b".into() }),
    };
    assert_eq!(format_expr_ir_tsx(&expr), "a + b");
}

#[test]
fn test_format_expr_ternary() {
    let expr = ExprIR::TernaryIR {
        cond: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
        then_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(1),
        }),
        else_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(0),
        }),
    };
    assert_eq!(format_expr_ir_tsx(&expr), "x ? 1 : 0");
}

#[test]
fn test_format_expr_field_access() {
    let expr = ExprIR::FieldAccessExpr {
        object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }),
        field: "prop".into(),
    };
    assert_eq!(format_expr_ir_tsx(&expr), "obj.prop");
}

#[test]
fn test_format_expr_unary_not() {
    let expr = ExprIR::UnaryOpExpr {
        op: "not".into(),
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "flag".into(),
        }),
    };
    assert_eq!(format_expr_ir_tsx(&expr), "!flag");
}

#[test]
fn test_format_expr_store_field() {
    let expr = ExprIR::StoreFieldAccessExpr {
        store_alias: "store".into(),
        field_name: "getCount".into(),
        call_args: vec![],
    };
    assert_eq!(format_expr_ir_tsx(&expr), "store.getCount");
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
    generate_statement_tsx(&mut w, &stmt, &HashSet::new());
    assert_eq!(w.into_string(), "let x = 42;\n");
}

#[test]
fn test_tsx_stmt_return() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ReturnStmtIR {
        value: Some(ExprIR::IdentifierExpr {
            name: "result".into(),
        }),
    };
    generate_statement_tsx(&mut w, &stmt, &HashSet::new());
    assert_eq!(w.into_string(), "return result;\n");
}

#[test]
fn test_tsx_stmt_mutation_increment_local() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },
            operation: MutationOp::Increment,
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            item: None,
        }),
    };
    // owner=None + not in signal_names → direct assignment
    generate_statement_tsx(&mut w, &stmt, &HashSet::new());
    assert_eq!(w.into_string(), "count = count + 1;\n");
}

#[test]
fn test_tsx_stmt_mutation_increment_signal() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },
            operation: MutationOp::Increment,
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            item: None,
        }),
    };
    // owner=None but "count" IS in signal_names → setter call
    let mut sig_names = HashSet::new();
    sig_names.insert("count".to_string());
    generate_statement_tsx(&mut w, &stmt, &sig_names);
    assert_eq!(w.into_string(), "setCount(prev => prev + 1);\n");
}

#[test]
fn test_tsx_stmt_mutation_assign_local() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "name".into(),
            },
            operation: MutationOp::DirectAssign,
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("alice"),
            }),
            item: None,
        }),
    };
    // owner=None + not in signal_names → direct assignment
    generate_statement_tsx(&mut w, &stmt, &HashSet::new());
    assert_eq!(w.into_string(), "name = \"alice\";\n");
}

#[test]
fn test_tsx_stmt_mutation_assign_store() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: Some("Store".to_string()),
                name: "name".into(),
            },
            operation: MutationOp::DirectAssign,
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("alice"),
            }),
            item: None,
        }),
    };
    // owner=Some → always setter call
    generate_statement_tsx(&mut w, &stmt, &HashSet::new());
    assert_eq!(w.into_string(), "setName(\"alice\");\n");
}

#[test]
fn test_tsx_stmt_if() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::IfStmtIR {
        condition: ExprIR::IdentifierExpr {
            name: "cond".into(),
        },
        then_body: BlockIR {
            statements: vec![StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                r#type: None,
            }],
            execution_mode: "client-only".into(),
        },
        else_ifs: vec![],
        else_body: None,
    };
    generate_statement_tsx(&mut w, &stmt, &HashSet::new());
    let out = w.into_string();
    assert!(out.contains("if (cond) {"));
    assert!(out.contains("let x = 1;"));
    assert!(out.contains("}"));
}

// ── gen_component_css tests ──────────────────────────────

#[test]
fn test_css_empty_component() {
    let codegen = ReactCodegen;
    let comp = ComponentIR {
        name: "App".into(),
        generic_params: vec![],
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    let css = codegen.gen_component_css("App", &comp);
    assert!(css.contains(".container"));
    assert!(css.contains("display: flex;"));
}

#[test]
fn test_css_with_styles() {
    let codegen = ReactCodegen;
    let mut props = HashMap::new();
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
        generic_params: vec![],
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![style],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    let css = codegen.gen_component_css("App", &comp);
    assert!(css.contains(".header"));
    assert!(css.contains("background-color: red;"));
    assert!(css.contains("font-size: 16px;"));
}

// ── gen_component_tsx tests ──────────────────────────────

#[allow(dead_code)]
fn make_test_component(name: &str, view: ViewTree) -> ComponentIR {
    ComponentIR {
        name: name.into(),
        generic_params: vec![],
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: Some(view),
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    }
}

/// Build IRProgram with a single module containing the given components.
#[allow(dead_code)]
fn make_ir(components: HashMap<String, ComponentIR>) -> IRProgram {
    IRProgram {
        version: "1.0".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                ..Default::default()
            },
        )]),
        stdlib_imports: Vec::new(),
        source_map: HashMap::new(),
    }
}

#[test]
fn test_tsx_interpolation_renders_expr() {
    let _codegen = ReactCodegen;
    let comp = make_test_component(
        "Test",
        ViewTree::Element(ElementIR {
            tag: "div".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: HashMap::new(),
            style_classes: None,
            inline_style: None,
            children: vec![ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
                name: "count".into(),
            }))],
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
        }),
    );
    let ir = make_ir(HashMap::from([("Test".into(), comp)]));
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("Test")
        .unwrap();
    let tsx = ReactCodegen.gen_component_tsx("Test", comp_ref, &ir);
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
    let codegen = ReactCodegen;
    let mut components = HashMap::new();
    components.insert(
        "App".into(),
        ComponentIR {
            name: "App".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            if_mode_default: None,
            hero_defs: vec![],
            timelines: vec![],
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
    let tsx = codegen.gen_component_tsx("App", comp_ref, &ir);
    assert!(tsx.contains("export function App() {"));
    assert!(tsx.contains("import styles from './App.module.css';"));
    assert!(tsx.contains(
        "import React, { useState, useEffect, useMemo, useCallback, useRef } from 'react';"
    ));
}

#[test]
fn test_tsx_component_with_state() {
    let codegen = ReactCodegen;
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
    let mut components = HashMap::new();
    components.insert(
        "Counter".into(),
        ComponentIR {
            name: "Counter".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![state],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            if_mode_default: None,
            hero_defs: vec![],
            timelines: vec![],
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
    let tsx = codegen.gen_component_tsx("Counter", comp_ref, &ir);
    assert!(tsx.contains("export function Counter() {"));
    assert!(tsx.contains("const [count, setCount] = useState(0);"));
}

#[test]
fn test_tsx_component_with_props() {
    let codegen = ReactCodegen;
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
    props.insert(
        "age".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    );
    let mut components = HashMap::new();
    components.insert(
        "Greeting".into(),
        ComponentIR {
            name: "Greeting".into(),
            generic_params: vec![],
            props,
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            if_mode_default: None,
            hero_defs: vec![],
            timelines: vec![],
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
    let tsx = codegen.gen_component_tsx("Greeting", comp_ref, &ir);
    assert!(tsx.contains("interface GreetingProps {"));
    assert!(tsx.contains("name: string;"));
    assert!(tsx.contains("age: number;"));
    assert!(tsx.contains("export function Greeting("));
    assert!(tsx.contains("}: GreetingProps) {"));
}

#[test]
fn test_tsx_component_with_effect() {
    let codegen = ReactCodegen;
    let effect = EffectIntent {
        deps: vec!["count".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::IdentifierExpr {
                    name: "console.log(count)".into(),
                },
            }],
            execution_mode: "client-only".into(),
        },
    };
    let mut components = HashMap::new();
    components.insert(
        "Watcher".into(),
        ComponentIR {
            name: "Watcher".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![effect],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            if_mode_default: None,
            hero_defs: vec![],
            timelines: vec![],
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
    let tsx = codegen.gen_component_tsx("Watcher", comp_ref, &ir);
    assert!(tsx.contains("useEffect(() => {"));
    assert!(tsx.contains("}, [count]);"));
}

#[test]
fn test_tsx_component_with_lifecycle_mount() {
    let codegen = ReactCodegen;
    let lc = LifecycleIntent {
        kind: LifecycleKind::Mount,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
    };
    let mut components = HashMap::new();
    components.insert(
        "App".into(),
        ComponentIR {
            name: "App".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![lc],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            if_mode_default: None,
            hero_defs: vec![],
            timelines: vec![],
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
    let tsx = codegen.gen_component_tsx("App", comp_ref, &ir);
    assert!(tsx.contains("useEffect(() => {"));
    assert!(tsx.contains("}, []);"));
}

// ── gen_store_hook tests ─────────────────────────────────

#[test]
fn test_store_hook_empty() {
    let codegen = ReactCodegen;
    let store = StoreIR {
        name: "AppStore".into(),
        signals: HashMap::new(),
        signal_defaults: HashMap::new(),
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    };
    let hook = codegen.gen_store_hook("AppStore", &store);
    assert!(hook.contains("export function useAppStore() {"));
    assert!(hook.contains("import { useState, useCallback, useMemo } from 'react';"));
}

#[test]
fn test_store_hook_with_signals() {
    let codegen = ReactCodegen;
    let mut signals = HashMap::new();
    signals.insert(
        "count".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    );
    signals.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    );
    let mut defaults = HashMap::new();
    defaults.insert("count".into(), serde_json::json!(0));
    defaults.insert("name".into(), serde_json::json!("world"));
    let store = StoreIR {
        name: "AppStore".into(),
        signals,
        signal_defaults: defaults,
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    };
    let hook = codegen.gen_store_hook("AppStore", &store);
    assert!(hook.contains("const [count, setCount] = useState<number>(0);"));
    assert!(hook.contains("const [name, setName] = useState<string>(\"world\");"));
    assert!(hook.contains("return {"));
    assert!(hook.contains("count"));
    assert!(hook.contains("name"));
}

#[test]
fn test_store_hook_with_getters() {
    let codegen = ReactCodegen;
    let mut getters = HashMap::new();
    getters.insert(
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
    );
    let mut signals = HashMap::new();
    signals.insert(
        "count".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    );
    let mut defaults = HashMap::new();
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
    assert!(hook.contains("return { count, doubleCount };"));
}

// ── Full pipeline through Codegen trait ──────────────────

#[test]
fn test_full_pipeline_empty() {
    let codegen = ReactCodegen;
    let ir = IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::new(),
                stores: HashMap::new(),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert!(output.files.is_empty());
}

#[test]
fn test_full_pipeline_component() {
    let codegen = ReactCodegen;
    let comp = ComponentIR {
        name: "App".into(),
        generic_params: vec![],
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![StateIntent {
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
        }],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::from([("App".into(), comp)]),
                stores: HashMap::new(),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 2); // .tsx + .module.css

    insta::with_settings!({snapshot_path => "snapshots"}, {
        let mut snapshot_content = String::new();
        for f in &output.files {
            snapshot_content.push_str(&format!("=== {} ===\n", f.path));
            snapshot_content.push_str(&f.content);
            snapshot_content.push('\n');
        }
        insta::assert_snapshot!("test_full_pipeline_component", &snapshot_content);
    });
}

#[test]
fn test_full_pipeline_with_store() {
    let codegen = ReactCodegen;
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
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::new(),
                stores: HashMap::from([("AppStore".into(), store)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };
    let sources = HashMap::from([("main.omni".into(), "".into())]);
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
    let codegen = ReactCodegen;
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
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    };
    let comp = ComponentIR {
        name: "App".into(),
        generic_params: vec![],
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec!["AppStore".into()],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::from([("App".into(), comp)]),
                stores: HashMap::from([("AppStore".into(), store)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 3); // .tsx + .module.css + useAppStore.ts
    let tsx = output.files.iter().find(|f| f.path == "App.tsx").unwrap();
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_component_with_store_ref", &tsx.content);
    });
}
