//! Unit tests for OmniView codegen.
//!
//! Tests cover expression formatting, statement generation, component/store emission,
//! HTML generation, and full pipeline integration. The OmniView codegen produces
//! HTML, CSS, and JS from an IRProgram.

use super::*;
use std::collections::HashMap;

// ── Helpers ─────────────────────────────────────────

fn make_element(tag: &str) -> ElementIR {
    ElementIR {
        tag: tag.into(),
        props: HashMap::new(),
        bindings: HashMap::new(),
        events: HashMap::new(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        test_id: None,
        ref_name: None,
        construction_kind: ConstructionKind::Static,
        class_map_bindings: HashMap::new(),
        source_line: 0,
        source_col: 0,
        motion_triggers: vec![],
        states: None,
        paused_override: None,
        driven_by: None,
        hover_reverse: None,
        in_view_once: None,
        in_view_repeat: None,
        in_view_threshold: None,
        scroll_start: None,
        scroll_end: None,
        scroll_scrub: None,
        drag_constraints: None,
        drag_elastic: None,
        drag_scrub: None,
        drag_momentum: None,
        drag_threshold: None,
        drag_velocity_max: None,
        hero_tag: None,
        hero_transition: None,
    }
}

fn make_module(name: &str) -> ModuleIR {
    ModuleIR {
        name: name.into(),
        functions: HashMap::new(),
        structs: HashMap::new(),
        enums: HashMap::new(),
        type_aliases: HashMap::new(),
        components: HashMap::new(),
        stores: HashMap::new(),
        themes: vec![],
        imports: vec![],
        exports: vec![],
        motions: vec![],
        timelines: vec![],
    }
}

fn make_component(name: &str) -> ComponentIR {
    ComponentIR {
        name: name.into(),
        source_file: None,
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
        if_mode_default: None,
        hero_defs: vec![],
        motions: vec![],
        timelines: vec![],
    }
}

fn make_program(modules: HashMap<String, ModuleIR>) -> IRProgram {
    IRProgram {
        version: "0.1".into(),
        modules,
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

// ── wrap_debug_html ──────────────────────────────────

#[test]
fn test_wrap_debug_html() {
    let html = wrap_debug_html(
        "/// OmniView Native Runtime v2 — reactive signal system\nconsole.log('hi');",
        "body { color: red; }",
        "<div>Hello</div>",
    );
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("OmniView Debug"));
    assert!(html.contains("console.log('hi');"));
    assert!(html.contains("body { color: red; }"));
    assert!(html.contains("<div>Hello</div>"));
    assert!(html.contains("OmniView Native Runtime v2"));
}

// ── Store Generation ────────────────────────────────

#[test]
fn test_store_signals_with_defaults() {
    let mut module = make_module("main");
    module.stores.insert(
        "AppStore".to_string(),
        StoreIR {
            name: "AppStore".to_string(),
            signals: HashMap::from([
                ("count".into(), TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false }),
                ("name".into(), TypeIR { kind: "string".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false }),
            ]),
            signal_defaults: HashMap::from([
                ("count".into(), serde_json::json!(0)),
                ("name".into(), serde_json::json!("world")),
            ]),
            getters: HashMap::new(),
            actions: vec![],
            ..Default::default()
        },
    );
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("AppStore_count"));
    assert!(output.js.contains("__omni_signal"));
    assert!(output.js.contains("0"));
    assert!(output.js.contains("world"));
}

#[test]
fn test_store_getters() {
    let mut module = make_module("main");
    module.stores.insert(
        "CalcStore".to_string(),
        StoreIR {
            name: "CalcStore".to_string(),
            signals: HashMap::from([("x".into(), TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false })]),
            signal_defaults: HashMap::from([("x".into(), serde_json::json!(10))]),
            getters: HashMap::from([(
                "doubled".into(),
                ExprIR::BinaryOpExpr {
                    op: "mul".into(),
                    left: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
                    right: Box::new(ExprIR::LiteralExpr { value: serde_json::json!(2) }),
                },
            )]),
            actions: vec![],
            ..Default::default()
        },
    );
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("CalcStore_doubled"));
    assert!(output.js.contains("__omni_computed"));
    assert!(output.js.contains("CalcStore_x.value * 2"));
}

#[test]
fn test_store_actions() {
    let mut module = make_module("main");
    module.stores.insert(
        "TodoStore".to_string(),
        StoreIR {
            name: "TodoStore".to_string(),
            signals: HashMap::from([("items".into(), TypeIR { kind: "Vec".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false })]),
            signal_defaults: HashMap::from([("items".into(), serde_json::json!([]))]),
            getters: HashMap::new(),
            actions: vec![FunctionIR {
                name: "addItem".into(),
                visibility: Visibility::Public,
                params: vec![ParamIR { name: "item".into(), r#type: None }],
                return_type: None,
                body: BlockIR { statements: vec![StatementIR::ExprStmtIR { expr: ExprIR::LiteralExpr { value: serde_json::json!("ok") } }] },
                is_async: false,
            }],
            ..Default::default()
        },
    );
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("function TodoStore_addItem"));
    assert!(output.js.contains("item"));
}

// ── Expression Formatting ────────────────────────────

#[test]
fn test_expr_literal() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::LiteralExpr { value: serde_json::json!(42) }), "42");
}

#[test]
fn test_expr_literal_string() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::LiteralExpr { value: serde_json::json!("hello") }), "\"hello\"");
    assert_eq!(gen.expr_to_js(&ExprIR::LiteralExpr { value: serde_json::Value::Null }), "null");
    assert_eq!(gen.expr_to_js(&ExprIR::LiteralExpr { value: serde_json::json!(true) }), "true");
}

#[test]
fn test_expr_identifier() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::IdentifierExpr { name: "count".into() }), "count");
}

#[test]
fn test_expr_binary() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::BinaryOpExpr {
        op: "add".into(), left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }), right: Box::new(ExprIR::IdentifierExpr { name: "b".into() })
    }), "a + b");
    assert_eq!(gen.expr_to_js(&ExprIR::BinaryOpExpr {
        op: "eq".into(), left: Box::new(ExprIR::IdentifierExpr { name: "x".into() }), right: Box::new(ExprIR::IdentifierExpr { name: "y".into() })
    }), "x === y");
    assert_eq!(gen.expr_to_js(&ExprIR::BinaryOpExpr {
        op: "and".into(), left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }), right: Box::new(ExprIR::IdentifierExpr { name: "b".into() })
    }), "a && b");
}

#[test]
fn test_expr_unary() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::UnaryOpExpr { op: "!".into(), operand: Box::new(ExprIR::IdentifierExpr { name: "flag".into() }) }), "!flag");
    assert_eq!(gen.expr_to_js(&ExprIR::UnaryOpExpr { op: "-".into(), operand: Box::new(ExprIR::IdentifierExpr { name: "x".into() }) }), "-x");
}

#[test]
fn test_expr_ternary() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::TernaryIR {
        cond: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
        then_expr: Box::new(ExprIR::LiteralExpr { value: serde_json::json!(1) }),
        else_expr: Box::new(ExprIR::LiteralExpr { value: serde_json::json!(0) }),
    }), "x ? 1 : 0");
}

#[test]
fn test_expr_field_access() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::FieldAccessExpr {
        object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }), field: "prop".into()
    }), "obj.prop");
}

#[test]
fn test_expr_method_call() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::MethodCallExpr {
        object: Box::new(ExprIR::IdentifierExpr { name: "arr".into() }), method: "len".into(), args: vec![], is_optional: false
    }), "(arr || []).length");
    assert_eq!(gen.expr_to_js(&ExprIR::MethodCallExpr {
        object: Box::new(ExprIR::IdentifierExpr { name: "val".into() }), method: "to_string".into(), args: vec![], is_optional: false
    }), "String(val)");
}

#[test]
fn test_expr_closure() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::ClosureExpr {
        params: vec!["x".into()],
        body: Box::new(ExprIR::BinaryOpExpr {
            op: "mul".into(), left: Box::new(ExprIR::IdentifierExpr { name: "x".into() }), right: Box::new(ExprIR::LiteralExpr { value: serde_json::json!(2) }),
        }),
    }), "((x) => x * 2)");
}

#[test]
fn test_expr_array() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(gen.expr_to_js(&ExprIR::ArrayExpr {
        elements: vec![
            ExprIR::LiteralExpr { value: serde_json::json!(1) },
            ExprIR::LiteralExpr { value: serde_json::json!(2) },
            ExprIR::LiteralExpr { value: serde_json::json!(3) },
        ],
    }), "[1, 2, 3]");
}

#[test]
fn test_expr_store_field_access() {
    let mut gen = OmniViewGenerator::test_new();
    gen.store_names.insert("AppStore".into());
    let js = gen.expr_to_js(&ExprIR::StoreFieldAccessExpr {
        store_alias: "AppStore".into(), field_name: "count".into(), call_args: vec![],
    });
    assert!(js.contains("AppStore_count"));
}

#[test]
fn test_expr_block() {
    let gen = OmniViewGenerator::test_new();
    let js = gen.expr_to_js(&ExprIR::BlockExpr {
        statements: vec![StatementIR::LetStmtIR { name: "x".into(), value: Some(ExprIR::LiteralExpr { value: serde_json::json!(42) }), r#type: None }],
        tail: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
    });
    assert!(js.contains("function"));
    assert!(js.contains("let x = 42"));
    assert!(js.contains("return x"));
}

#[test]
fn test_expr_variant_literal() {
    let gen = OmniViewGenerator::test_new();
    let js = gen.expr_to_js(&ExprIR::VariantLiteral {
        union_type: "Result".into(), variant: "ok".into(),
        fields: vec![("value".into(), ExprIR::LiteralExpr { value: serde_json::json!(42) })],
    });
    assert!(js.contains("\"_variant\": \"ok\""));
    assert!(js.contains("\"value\": 42"));
}

// ── Statement Generation ────────────────────────────

#[test]
fn test_stmt_let() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::LetStmtIR { name: "x".into(), value: Some(ExprIR::LiteralExpr { value: serde_json::json!(42) }), r#type: None });
    assert!(gen.js.contains("let x = 42"));
}

#[test]
fn test_stmt_return() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ReturnStmtIR { value: Some(ExprIR::IdentifierExpr { name: "result".into() }) });
    assert!(gen.js.contains("return result"));
}

#[test]
fn test_stmt_if_else() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::IfStmtIR {
        condition: ExprIR::IdentifierExpr { name: "cond".into() },
        then_body: BlockIR { statements: vec![StatementIR::LetStmtIR { name: "x".into(), value: Some(ExprIR::LiteralExpr { value: serde_json::json!(1) }), r#type: None }] },
        else_ifs: vec![],
        else_body: Some(BlockIR { statements: vec![StatementIR::LetStmtIR { name: "y".into(), value: Some(ExprIR::LiteralExpr { value: serde_json::json!(2) }), r#type: None }] }),
    });
    assert!(gen.js.contains("if (cond)"));
    assert!(gen.js.contains("let x = 1"));
    assert!(gen.js.contains("} else {"));
    assert!(gen.js.contains("let y = 2"));
}

#[test]
fn test_stmt_while() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::WhileStmtIR {
        condition: ExprIR::IdentifierExpr { name: "running".into() },
        body: BlockIR { statements: vec![StatementIR::BreakStmtIR] },
    });
    assert!(gen.js.contains("while (running)"));
    assert!(gen.js.contains("break"));
}

#[test]
fn test_stmt_for_in() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ForInStmtIR {
        var_name: "item".into(), iterable: ExprIR::IdentifierExpr { name: "items".into() },
        body: BlockIR { statements: vec![StatementIR::ExprStmtIR { expr: ExprIR::CallExpr { name: "println".into(), args: vec![ExprIR::IdentifierExpr { name: "item".into() }] } }] },
    });
    assert!(gen.js.contains("for (const item of items)"));
    assert!(gen.js.contains("console.log(item)"));
}

#[test]
fn test_stmt_mutation_assign() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR { owner: None, name: "count".into() },
            operation: MutationOp::DirectAssign,
            amount: Some(ExprIR::LiteralExpr { value: serde_json::json!(42) }),
            item: None,
        }),
    });
    assert!(gen.js.contains("count = 42"));
}

#[test]
fn test_stmt_mutation_increment() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR { owner: None, name: "count".into() },
            operation: MutationOp::Increment,
            amount: Some(ExprIR::LiteralExpr { value: serde_json::json!(1) }),
            item: None,
        }),
    });
    assert!(gen.js.contains("count += 1"));
}

#[test]
fn test_stmt_mutation_signal_set() {
    let mut gen = OmniViewGenerator::test_new();
    gen.signal_names.insert("App_count".into());
    gen.name_renames.insert("count".into(), "App_count".into());
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR { owner: None, name: "count".into() },
            operation: MutationOp::DirectAssign,
            amount: Some(ExprIR::LiteralExpr { value: serde_json::json!(99) }),
            item: None,
        }),
    });
    assert!(gen.js.contains(".set(99)"));
}

#[test]
fn test_stmt_try_catch() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::TryCatchStmtIR {
        try_body: BlockIR { statements: vec![StatementIR::ExprStmtIR { expr: ExprIR::CallExpr { name: "riskyOp".into(), args: vec![] } }] },
        catch_var: Some("err".into()),
        catch_body: Some(BlockIR { statements: vec![StatementIR::ExprStmtIR { expr: ExprIR::CallExpr { name: "println".into(), args: vec![ExprIR::IdentifierExpr { name: "err".into() }] } }] }),
        finally_body: None,
    });
    assert!(gen.js.contains("try {"));
    assert!(gen.js.contains("riskyOp()"));
    assert!(gen.js.contains("catch (err)"));
    assert!(gen.js.contains("console.log(err)"));
}

#[test]
fn test_stmt_throw() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ThrowStmtIR { expr: ExprIR::LiteralExpr { value: serde_json::json!("error!") } });
    assert!(gen.js.contains("throw"));
    assert!(gen.js.contains("error!"));
}

#[test]
fn test_stmt_continue() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ContinueStmtIR);
    assert!(gen.js.contains("continue"));
}

// ── Component Generation ────────────────────────────

#[test]
fn test_component_with_states() {
    let mut module = make_module("main");
    module.components.insert("Counter".to_string(), ComponentIR {
        states: vec![StateIntent {
            name: "count".into(),
            r#type: TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false },
            initial_value: ExprIR::LiteralExpr { value: serde_json::json!(0) },
            is_copy: false,
        }],
        ..make_component("Counter")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("Counter_count"));
    assert!(output.js.contains("__omni_signal"));
}

#[test]
fn test_component_with_computed() {
    let mut module = make_module("main");
    module.components.insert("Calc".to_string(), ComponentIR {
        states: vec![StateIntent {
            name: "x".into(), r#type: TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false },
            initial_value: ExprIR::LiteralExpr { value: serde_json::json!(10) },
            is_copy: false,
        }],
        computeds: vec![ComputedIntent {
            name: "doubled".into(), r#type: TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false },
            body: ExprIR::BinaryOpExpr {
                op: "mul".into(), left: Box::new(ExprIR::IdentifierExpr { name: "x".into() }), right: Box::new(ExprIR::LiteralExpr { value: serde_json::json!(2) }),
            },
            deps: vec!["x".into()],
        }],
        ..make_component("Calc")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("Calc_doubled"));
    assert!(output.js.contains("__omni_computed"));
}

#[test]
fn test_component_with_functions() {
    let mut module = make_module("main");
    module.components.insert("App".to_string(), ComponentIR {
        fns: vec![FunctionIR {
            name: "greet".into(), visibility: Visibility::Public, params: vec![ParamIR { name: "name".into(), r#type: None }], return_type: None,
            body: BlockIR { statements: vec![StatementIR::ExprStmtIR { expr: ExprIR::CallExpr {
                name: "println".into(),
                args: vec![ExprIR::BinaryOpExpr {
                    op: "add".into(), left: Box::new(ExprIR::LiteralExpr { value: serde_json::json!("Hello ") }), right: Box::new(ExprIR::IdentifierExpr { name: "name".into() }),
                }],
            } }] },
            is_async: false,
        }],
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("function App_greet"));
    assert!(output.js.contains("name"));
    assert!(output.js.contains("console.log"));
}

#[test]
fn test_component_with_lifecycle_mount() {
    let mut module = make_module("main");
    module.components.insert("App".to_string(), ComponentIR {
        states: vec![StateIntent {
            name: "count".into(), r#type: TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false },
            initial_value: ExprIR::LiteralExpr { value: serde_json::json!(0) }, is_copy: false,
        }],
        lifecycles: vec![LifecycleIntent {
            kind: LifecycleKind::Mount,
            body: BlockIR { statements: vec![StatementIR::ExprStmtIR { expr: ExprIR::CallExpr { name: "println".into(), args: vec![ExprIR::LiteralExpr { value: serde_json::json!("mounted") }] } }] },
        }],
        view: Some(ViewTree::Element(make_element("div"))),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("@mount App"));
    assert!(output.js.contains("console.log"));
}

#[test]
fn test_component_with_store_ref() {
    let mut module = make_module("main");
    module.stores.insert("AppStore".to_string(), StoreIR {
        name: "AppStore".to_string(),
        signals: HashMap::from([("count".into(), TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false })]),
        signal_defaults: HashMap::from([("count".into(), serde_json::json!(0))]),
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    });
    module.components.insert("App".to_string(), ComponentIR {
        store_refs: vec!["AppStore".into()],
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("AppStore_count"));
}

#[test]
fn test_component_with_effect() {
    let mut module = make_module("main");
    module.components.insert("App".to_string(), ComponentIR {
        states: vec![StateIntent {
            name: "count".into(), r#type: TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false },
            initial_value: ExprIR::LiteralExpr { value: serde_json::json!(0) }, is_copy: false,
        }],
        effects: vec![EffectIntent {
            deps: vec!["count".into()],
            body: BlockIR { statements: vec![StatementIR::ExprStmtIR { expr: ExprIR::CallExpr { name: "println".into(), args: vec![ExprIR::IdentifierExpr { name: "count".into() }] } }] },
        }],
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("__omni_effect"));
}

// ── HTML Generation ─────────────────────────────────

#[test]
fn test_html_element_with_attributes() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.props.insert("class".into(), ExprIR::LiteralExpr { value: serde_json::json!("container") });
    let mut child = make_element("span");
    child.props.insert("id".into(), ExprIR::LiteralExpr { value: serde_json::json!("label") });
    el.children = vec![ViewTree::Element(child)];
    module.components.insert("App".to_string(), ComponentIR {
        view: Some(ViewTree::Element(el)),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.html.contains("class=\"container\""));
    assert!(output.html.contains("id=\"label\""));
    assert!(output.html.contains("data-nid="));
}

#[test]
fn test_html_data_nid_present() {
    let mut module = make_module("main");
    module.components.insert("App".to_string(), ComponentIR {
        view: Some(ViewTree::Element(make_element("button"))),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.html.contains("data-nid=\"el-button-"));
}

#[test]
fn test_html_interpolation() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.children = vec![ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr { name: "name".into() }))];
    module.components.insert("App".to_string(), ComponentIR {
        states: vec![StateIntent {
            name: "name".into(), r#type: TypeIR { kind: "string".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false },
            initial_value: ExprIR::LiteralExpr { value: serde_json::json!("World") }, is_copy: false,
        }],
        view: Some(ViewTree::Element(el)),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.html.contains("<span"));
    assert!(output.html.contains("data-nid="));
    assert!(output.js.contains("textContent"));
}

#[test]
fn test_html_conditional() {
    let mut module = make_module("main");
    let mut visible_el = make_element("div");
    visible_el.children = vec![ViewTree::Text("Visible".into())];
    let mut hidden_el = make_element("div");
    hidden_el.children = vec![ViewTree::Text("Hidden".into())];
    module.components.insert("App".to_string(), ComponentIR {
        view: Some(ViewTree::Conditional(Box::new(ConditionalIntent {
            branches: vec![ConditionalBranch {
                condition: ExprIR::IdentifierExpr { name: "show".into() },
                body: Some(ViewTree::Element(visible_el)),
            }],
            fallback: Some(ViewTree::Element(hidden_el)),
            transition_mode: None,
        }))),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.html.contains("Visible"));
    assert!(output.html.contains("Hidden"));
}

// ── Full Pipeline ───────────────────────────────────

#[test]
fn test_full_pipeline_empty() {
    let ir = IRProgram::default();
    let output = generate(&ir, false);
    assert!(output.js.contains("__omni_signal"));
}

#[test]
fn test_full_pipeline_component_only() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.children = vec![ViewTree::Text("Hello".into())];
    module.components.insert("App".to_string(), ComponentIR {
        states: vec![StateIntent {
            name: "count".into(), r#type: TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false },
            initial_value: ExprIR::LiteralExpr { value: serde_json::json!(0) }, is_copy: false,
        }],
        view: Some(ViewTree::Element(el)),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.js.contains("App_count"));
    assert!(output.js.contains("__omni_signal"));
    assert!(output.html.contains("Hello"));
}

#[test]
fn test_full_pipeline_store_only() {
    let mut module = make_module("main");
    module.stores.insert("DataStore".to_string(), StoreIR {
        name: "DataStore".to_string(),
        signals: HashMap::from([("value".into(), TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false })]),
        signal_defaults: HashMap::from([("value".into(), serde_json::json!(5))]),
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, false);
    assert!(output.js.contains("DataStore_value"));
    assert!(output.js.contains("__omni_signal(\"DataStore_value\", 5)"));
}

#[test]
fn test_full_pipeline_component_with_store() {
    let mut module = make_module("main");
    module.stores.insert("AppStore".to_string(), StoreIR {
        name: "AppStore".to_string(),
        signals: HashMap::from([("count".into(), TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false })]),
        signal_defaults: HashMap::from([("count".into(), serde_json::json!(0))]),
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    });
    module.components.insert("App".to_string(), ComponentIR {
        store_refs: vec!["AppStore".into()],
        view: Some(ViewTree::Element(make_element("div"))),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.js.contains("AppStore_count"));
}

// ── Event Bindings ──────────────────────────────────

#[test]
fn test_event_binding_click() {
    let mut module = make_module("main");
    let mut el = make_element("button");
    el.events.insert("onClick".into(), EventHandlerIR {
        kind: EventHandlerKind::FnRef, body: None, r#ref: Some("handleClick".into()), params: vec![],
        event_data: None, used_fields: None, needs_prevent_default: false, needs_stop_propagation: false,
    });
    module.components.insert("App".to_string(), ComponentIR {
        fns: vec![FunctionIR {
            name: "handleClick".into(), visibility: Visibility::Public, params: vec![], return_type: None,
            body: BlockIR { statements: vec![] }, is_async: false,
        }],
        view: Some(ViewTree::Element(el)),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.js.contains("click"));
    assert!(output.js.contains("App_handleClick"));
    assert!(output.js.contains("addEventListener"));
}

// ── Styles ──────────────────────────────────────────

#[test]
fn test_style_emission() {
    let mut module = make_module("main");
    module.components.insert("App".to_string(), ComponentIR {
        view: Some(ViewTree::Element(make_element("div"))),
        styles: vec![StyleRuleIR {
            class_name: "myClass".into(),
            properties: HashMap::from([("color".into(), ExprIR::LiteralExpr { value: serde_json::json!("red") })]),
            output_mode: "css".into(), is_tag_selector: false,
        }],
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(output.css.contains("myClass"));
    assert!(output.css.contains("color: red"));
    assert!(output.css.contains("data-component"));
}

// ── Source Map ──────────────────────────────────────

#[test]
fn test_source_map_entries() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.source_line = 5;
    el.source_col = 3;
    module.components.insert("App".to_string(), ComponentIR {
        view: Some(ViewTree::Element(el)),
        ..make_component("App")
    });
    let ir = make_program(HashMap::from([("main".into(), module)]));
    let output = generate(&ir, true);
    assert!(!output.source_map.is_empty(), "should have source map entries");
    assert!(output.source_map[0].node_id.starts_with("el-div-"));
    assert_eq!(output.source_map[0].omni_span.line, 5);
    assert_eq!(output.source_map[0].omni_span.col, 3);
}

// ── Proptest: backtick escaping for innerHTML ───────

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    /// The escape function from view.rs:49-51 that protects innerHTML
    /// template literal insertion: backslash -> \\, backtick -> \`, dollar -> \$.
    fn escape_for_inner_html(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("$", "\\$")
    }

    proptest! {
        /// After escaping, no raw backtick must remain — an unescaped backtick
        /// would close the template literal and allow JS injection.
        #[test]
        fn no_unescaped_backtick(s in "\\PC*") {
            let escaped = escape_for_inner_html(&s);
            // Find all backtick positions in the escaped string
            for (i, ch) in escaped.char_indices() {
                if ch == '`' {
                    // Every backtick must be preceded by a backslash
                    assert!(i > 0, "backtick at position 0 is unescaped: {escaped:?}");
                    assert_eq!(
                        &escaped[i-1..i], "\\",
                        "backtick at position {i} is not escaped: {escaped:?}"
                    );
                }
            }
        }

        /// After escaping, `${` must not appear — that would be a JS template
        /// literal interpolation that could execute arbitrary code.
        #[test]
        fn no_interpolation_breakout(s in "\\PC*") {
            let escaped = escape_for_inner_html(&s);
            // The only `$` in the output must be preceded by `\`
            for (i, ch) in escaped.char_indices() {
                if ch == '$' {
                    assert!(i > 0, "$ at position 0 is unescaped: {escaped:?}");
                    assert_eq!(
                        &escaped[i-1..i], "\\",
                        "$ at position {i} is not escaped: {escaped:?}"
                    );
                }
            }
        }

        /// Verify that the three escape replacements are applied correctly:
        /// `\` → `\\`, `` ` `` → `` \` ``, `$` → `\$`.
        #[test]
        fn specific_chars_are_escaped(s in "[`\\\\$]{1,10}") {
            let escaped = escape_for_inner_html(&s);
            // All three dangerous chars are doubled/escaped in the output
            for ch in s.chars() {
                match ch {
                    '\\' => prop_assert!(escaped.contains("\\\\"), "backslash not doubled"),
                    '`' => prop_assert!(escaped.contains("\\`"), "backtick not escaped"),
                    '$' => prop_assert!(escaped.contains("\\$"), "dollar not escaped"),
                    _ => {}
                }
            }
        }
    }
}
