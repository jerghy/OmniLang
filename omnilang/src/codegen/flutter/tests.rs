//! Unit tests for flutter codegen.

use super::*;
use crate::codegen::{format_json_value, JSON_FLUTTER};
use std::collections::HashMap;

// ── Helper tests ──────────────────────────────────────────

#[test]
fn test_capitalize_first() {
    assert_eq!(capitalize_first("hello"), "Hello");
    assert_eq!(capitalize_first(""), "");
    assert_eq!(capitalize_first("count"), "Count");
}

#[test]
fn test_tag_to_flutter_widget() {
    assert_eq!(tag_to_flutter_widget("div"), "Container");
    assert_eq!(tag_to_flutter_widget("button"), "ElevatedButton");
    assert_eq!(tag_to_flutter_widget("input"), "TextField");
    assert_eq!(tag_to_flutter_widget("text"), "Text");
    assert_eq!(tag_to_flutter_widget("row"), "Row");
    assert_eq!(tag_to_flutter_widget("column"), "Column");
    assert_eq!(tag_to_flutter_widget("span"), "Text");
    assert_eq!(tag_to_flutter_widget("h1"), "Text");
    assert_eq!(tag_to_flutter_widget("img"), "Image.network");
    assert_eq!(tag_to_flutter_widget("unknown"), "Container");
}

#[test]
fn test_event_to_flutter_callback() {
    assert_eq!(event_to_flutter_callback("click"), "onPressed");
    assert_eq!(event_to_flutter_callback("change"), "onChanged");
    assert_eq!(event_to_flutter_callback("submit"), "onSubmitted");
    assert_eq!(event_to_flutter_callback("focus"), "onFocus");
    assert_eq!(event_to_flutter_callback("blur"), "onBlur");
    assert_eq!(event_to_flutter_callback("hover"), "onHover");
    assert_eq!(event_to_flutter_callback("dblclick"), "onDoubleTap");
}

#[test]
fn test_prop_to_flutter_param_name() {
    assert_eq!(prop_to_flutter_param_name("src"), "image");
    assert_eq!(prop_to_flutter_param_name("alt"), "semanticLabel");
    assert_eq!(prop_to_flutter_param_name("placeholder"), "hintText");
    assert_eq!(prop_to_flutter_param_name("width"), "width");
    assert_eq!(prop_to_flutter_param_name("color"), "color");
    assert_eq!(prop_to_flutter_param_name("fontSize"), "fontSize");
    assert_eq!(prop_to_flutter_param_name("class"), "");
    assert_eq!(prop_to_flutter_param_name("style"), "");
}

#[test]
fn test_binary_op_to_flutter() {
    assert_eq!(crate::codegen::binary_op_to_str("add", false, false), "+");
    assert_eq!(crate::codegen::binary_op_to_str("eq", false, false), "==");
    assert_eq!(crate::codegen::binary_op_to_str("and", false, false), "&&");
    assert_eq!(crate::codegen::binary_op_to_str("or", false, false), "||");
    assert_eq!(crate::codegen::binary_op_to_str("neq", false, false), "!=");
}

#[test]
fn test_format_json_value_flutter() {
    assert_eq!(
        format_json_value(&serde_json::Value::Null, &JSON_FLUTTER),
        "null"
    );
    assert_eq!(
        format_json_value(&serde_json::Value::Bool(true), &JSON_FLUTTER),
        "true"
    );
    assert_eq!(
        format_json_value(&serde_json::Value::Bool(false), &JSON_FLUTTER),
        "false"
    );
    assert_eq!(
        format_json_value(&serde_json::json!(42), &JSON_FLUTTER),
        "42"
    );
    assert_eq!(
        format_json_value(&serde_json::json!("hello"), &JSON_FLUTTER),
        "'hello'"
    );
    assert_eq!(
        format_json_value(&serde_json::json!("he'llo"), &JSON_FLUTTER),
        "'he\\'llo'"
    );
}

// ── format_expr_ir_flutter tests ──────────────────────────

#[test]
fn test_format_expr_literal() {
    let expr = ExprIR::LiteralExpr {
        value: serde_json::json!(42),
    };
    assert_eq!(format_expr_ir_flutter(&expr), "42");
}

#[test]
fn test_format_expr_identifier() {
    let expr = ExprIR::IdentifierExpr {
        name: "count".into(),
        type_ann: None,
    };
    assert_eq!(format_expr_ir_flutter(&expr), "count");
}

#[test]
fn test_format_expr_binary() {
    let expr = ExprIR::BinaryOpExpr {
        op: "add".into(),
        left: Box::new(ExprIR::IdentifierExpr { name: "a".into(), type_ann: None }),
        right: Box::new(ExprIR::IdentifierExpr { name: "b".into(), type_ann: None }),
    };
    assert_eq!(format_expr_ir_flutter(&expr), "a + b");
}

#[test]
fn test_format_expr_ternary() {
    let expr = ExprIR::TernaryIR {
        cond: Box::new(ExprIR::IdentifierExpr { name: "x".into(), type_ann: None }),
        then_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(1),
        }),
        else_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(0),
        }),
    };
    assert_eq!(format_expr_ir_flutter(&expr), "x ? 1 : 0");
}

#[test]
fn test_format_expr_field_access() {
    let expr = ExprIR::FieldAccessExpr {
        object: Box::new(ExprIR::IdentifierExpr { name: "obj".into(), type_ann: None }),
        field: "prop".into(),
    };
    assert_eq!(format_expr_ir_flutter(&expr), "obj.prop");
}

#[test]
fn test_format_expr_unary_not() {
    let expr = ExprIR::UnaryOpExpr {
        op: "not".into(),
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "flag".into(),
            type_ann: None,
        }),
    };
    assert_eq!(format_expr_ir_flutter(&expr), "!flag");
}

#[test]
fn test_format_expr_store_field() {
    let expr = ExprIR::StoreFieldAccessExpr {
        store_alias: "store".into(),
        field_name: "getCount".into(),
        call_args: vec![],
    };
    assert_eq!(format_expr_ir_flutter(&expr), "store.getCount");
}

// ── generate_statement_flutter tests ──────────────────────

#[test]
fn test_stmt_let() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
    };
    generate_statement_flutter(&mut w, &stmt);
    assert_eq!(w.into_string(), "var x = 42;\n");
}

#[test]
fn test_stmt_return() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ReturnStmtIR {
        value: Some(ExprIR::IdentifierExpr {
            name: "result".into(),
            type_ann: None,
        }),
    };
    generate_statement_flutter(&mut w, &stmt);
    assert_eq!(w.into_string(), "return result;\n");
}

#[test]
fn test_stmt_mutation_increment() {
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
    generate_statement_flutter(&mut w, &stmt);
    let out = w.into_string();
    assert!(out.contains("setState(() {"));
    assert!(out.contains("count += 1;"));
    assert!(out.contains("});"));
}

#[test]
fn test_stmt_mutation_assign() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "name".into(),
            },
            operation: MutationOp::DirectAssign,
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            }),
            item: None,
        }),
    };
    generate_statement_flutter(&mut w, &stmt);
    let out = w.into_string();
    assert!(out.contains("setState(() {"));
    assert!(out.contains("name = 'hello';"));
    assert!(out.contains("});"));
}

#[test]
fn test_stmt_mutation_decrement() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },
            operation: MutationOp::Decrement,
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            item: None,
        }),
    };
    generate_statement_flutter(&mut w, &stmt);
    let out = w.into_string();
    assert!(out.contains("setState(() {"));
    assert!(out.contains("count -= 1;"));
    assert!(out.contains("});"));
}

#[test]
fn test_stmt_if() {
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
        else_body: Some(BlockIR {
            statements: vec![StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
                r#type: None,
            }],
        }),
    };
    generate_statement_flutter(&mut w, &stmt);
    let out = w.into_string();
    assert!(out.contains("if (cond) {"));
    assert!(out.contains("var x = 1;"));
    assert!(out.contains("} else {"));
    assert!(out.contains("var x = 2;"));
    assert!(out.contains("}"));
}

// ── format_view_tree / format_element tests ───────────────

#[test]
fn test_format_text_node() {
    let node = ViewTree::Text("Hello".into());
    assert_eq!(
        format_view_tree(&node, &[], &HashMap::new()),
        "Text('Hello')"
    );
}

#[test]
fn test_format_text_with_interpolation() {
    let node = ViewTree::Text("{name}".into());
    assert_eq!(
        format_view_tree(&node, &[], &HashMap::new()),
        "Text('$name')"
    );
}

#[test]
fn test_format_element_empty() {
    let el = ElementIR {
        class_map_bindings: HashMap::new(),
        tag: "div".into(),
        props: HashMap::new(),
        bindings: HashMap::new(),
        events: HashMap::new(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        test_id: None,
        ref_name: None,
        construction_kind: ConstructionKind::TagMapping,
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
    };
    assert_eq!(
        format_view_tree(&ViewTree::Element(el), &[], &HashMap::new()),
        "Container()"
    );
}

#[test]
fn test_format_element_with_text_child() {
    let el = ElementIR {
        class_map_bindings: HashMap::new(),
        tag: "div".into(),
        props: HashMap::new(),
        bindings: HashMap::new(),
        events: HashMap::new(),
        style_classes: None,
        inline_style: None,
        children: vec![ViewTree::Text("Hello".into())],
        test_id: None,
        ref_name: None,
        construction_kind: ConstructionKind::TagMapping,
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
    };
    let expr = format_view_tree(&ViewTree::Element(el), &[], &HashMap::new());
    assert_eq!(expr, "Container(child: Text('Hello'))");
}

#[test]
fn test_format_element_with_multiple_children() {
    let el = ElementIR {
        class_map_bindings: HashMap::new(),
        tag: "column".into(),
        props: HashMap::new(),
        bindings: HashMap::new(),
        events: HashMap::new(),
        style_classes: None,
        inline_style: None,
        children: vec![
            ViewTree::Text("First".into()),
            ViewTree::Text("Second".into()),
        ],
        test_id: None,
        ref_name: None,
        construction_kind: ConstructionKind::TagMapping,
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
    };
    let expr = format_view_tree(&ViewTree::Element(el), &[], &HashMap::new());
    assert_eq!(expr, "Column(children: [Text('First'), Text('Second')])");
}

#[test]
fn test_format_element_with_event() {
    let mut events = HashMap::new();
    events.insert(
        "click".into(),
        EventHandlerIR {
            kind: EventHandlerKind::FnRef,
            body: None,
            r#ref: Some("handleClick".into()),
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    let el = ElementIR {
        class_map_bindings: HashMap::new(),
        tag: "button".into(),
        props: HashMap::new(),
        bindings: HashMap::new(),
        events,
        style_classes: None,
        inline_style: None,
        children: vec![],
        test_id: None,
        ref_name: None,
        construction_kind: ConstructionKind::TagMapping,
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
    };
    let expr = format_view_tree(&ViewTree::Element(el), &[], &HashMap::new());
    assert!(expr.contains("ElevatedButton("));
    assert!(expr.contains("onPressed: () { handleClick(); }"));
}

// ── generate_function_flutter tests ───────────────────────

#[test]
fn test_generate_function_empty() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "doSomething".into(),
        visibility: Visibility::Private,
        params: vec![],
        return_type: None,
        body: BlockIR { statements: vec![] },
        is_async: false,
    };
    generate_function_flutter(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("doSomething() {"));
}

#[test]
fn test_generate_function_with_mutation() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "increment".into(),
        visibility: Visibility::Private,
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![StatementIR::MutationStmt {
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
            }],
        },
        is_async: false,
    };
    generate_function_flutter(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("increment() {"));
    assert!(out.contains("setState(() {"));
    assert!(out.contains("count += 1;"));
    assert!(out.contains("});"));
}

// ── gen_component_dart tests ──────────────────────────────

/// Build IRProgram with a single module containing the given components.
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
fn test_empty_component() {
    let codegen = FlutterCodegen;
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
    let dart = codegen.gen_component_dart("App", comp_ref, &ir);
    assert!(dart.contains("import 'package:flutter/material.dart';"));
    assert!(dart.contains("class App extends StatefulWidget {"));
    assert!(dart.contains("const App({super.key});"));
    assert!(dart.contains("State<App> createState() => _AppState();"));
    assert!(dart.contains("class _AppState extends State<App> {"));
    assert!(dart.contains("@override"));
    assert!(dart.contains("Widget build(BuildContext context) {"));
    assert!(dart.contains("return Container("));
    assert!(dart.contains("child: Text('App'),"));
    assert!(dart.contains(");"));
}

#[test]
fn test_component_with_state() {
    let codegen = FlutterCodegen;
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
            fns: vec![FunctionIR {
                name: "increment".into(),
                visibility: Visibility::Private,
                params: vec![],
                return_type: None,
                body: BlockIR {
                    statements: vec![StatementIR::MutationStmt {
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
                    }],
                },
                is_async: false,
            }],
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
    let dart = codegen.gen_component_dart("Counter", comp_ref, &ir);
    assert!(dart.contains("int count = 0;"));
    assert!(dart.contains("increment() {"));
    assert!(dart.contains("setState(() {"));
    assert!(dart.contains("count += 1;"));
    assert!(dart.contains("});"));
    assert!(dart.contains("Widget build(BuildContext context) {"));
}

#[test]
fn test_component_with_props() {
    let codegen = FlutterCodegen;
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
    let dart = codegen.gen_component_dart("Greeting", comp_ref, &ir);
    assert!(dart.contains("required this.name,"));
    assert!(dart.contains("required this.age,"));
    assert!(dart.contains("final String name;"));
    assert!(dart.contains("final int age;"));
}

#[test]
fn test_component_with_mount_lifecycle() {
    let codegen = FlutterCodegen;
    let lc = LifecycleIntent {
        kind: LifecycleKind::Mount,
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::IdentifierExpr {
                    name: "init".into(),
                    type_ann: None,
                },
            }],
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
    let dart = codegen.gen_component_dart("App", comp_ref, &ir);
    assert!(dart.contains("@override"));
    assert!(dart.contains("void initState() {"));
    assert!(dart.contains("super.initState();"));
    assert!(dart.contains("init;"));
}

#[test]
fn test_component_with_unmount_lifecycle() {
    let codegen = FlutterCodegen;
    let lc = LifecycleIntent {
        kind: LifecycleKind::Unmount,
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::IdentifierExpr {
                    name: "cleanup".into(),
                    type_ann: None,
                },
            }],
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
    let dart = codegen.gen_component_dart("App", comp_ref, &ir);
    assert!(dart.contains("@override"));
    assert!(dart.contains("void dispose() {"));
    assert!(dart.contains("super.dispose();"));
}

// ── gen_store_dart tests ──────────────────────────────────

#[test]
fn test_store_dart_empty() {
    let codegen = FlutterCodegen;
    let store = StoreIR {
        name: "EmptyStore".into(),
        signals: HashMap::new(),
        signal_defaults: HashMap::new(),
        getters: HashMap::new(),
        actions: vec![],
        ..Default::default()
    };
    let store_dart = codegen.gen_store_dart("EmptyStore", &store);
    assert!(store_dart.contains("class EmptyStore {"));
    assert!(store_dart.contains("EmptyStore();"));
}

#[test]
fn test_store_dart_with_signals() {
    let codegen = FlutterCodegen;
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
    let store_dart = codegen.gen_store_dart("AppStore", &store);
    assert!(store_dart.contains("class AppStore extends ChangeNotifier {"));
    assert!(store_dart.contains("int count = 0;"));
    assert!(store_dart.contains("String name = 'world';"));
    // Setter methods
    assert!(store_dart.contains("void setCount(int value) {"));
    assert!(store_dart.contains("this.count = value;"));
    assert!(store_dart.contains("notifyListeners();"));
    assert!(store_dart.contains("void setName(String value) {"));
    assert!(store_dart.contains("this.name = value;"));
}

#[test]
fn test_store_dart_with_getters() {
    let codegen = FlutterCodegen;
    let mut getters = HashMap::new();
    getters.insert(
        "doubleCount".into(),
        ExprIR::BinaryOpExpr {
            op: "mul".into(),
            left: Box::new(ExprIR::IdentifierExpr {
                name: "count".into(),
                type_ann: None,
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
    let store_dart = codegen.gen_store_dart("CalcStore", &store);
    assert!(store_dart.contains("get doubleCount {"));
    assert!(store_dart.contains("return count * 2;"));
}

// ── Full pipeline through Codegen trait ───────────────────

#[test]
fn test_full_pipeline_empty() {
    let codegen = FlutterCodegen;
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
    let codegen = FlutterCodegen;
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
    assert_eq!(output.files.len(), 1);
    let file = &output.files[0];
    assert_eq!(file.path, "App.dart");
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_component", &file.content);
    });
}

#[test]
fn test_full_pipeline_with_store() {
    let codegen = FlutterCodegen;
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
    assert_eq!(output.files.len(), 1);
    let file = &output.files[0];
    assert_eq!(file.path, "AppStore_store.dart");
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_with_store", &file.content);
    });
}

#[test]
fn test_full_pipeline_component_with_store_ref() {
    let codegen = FlutterCodegen;
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
    assert_eq!(output.files.len(), 2); // App.dart + AppStore_store.dart
    insta::with_settings!({snapshot_path => "snapshots"}, {
        let mut snapshot_content = String::new();
        for f in &output.files {
            snapshot_content.push_str(&format!("=== {} ===\n", f.path));
            snapshot_content.push_str(&f.content);
            snapshot_content.push('\n');
        }
        insta::assert_snapshot!("test_full_pipeline_component_with_store_ref", &snapshot_content);
    });
}
