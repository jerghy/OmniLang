//! Unit tests for flutter codegen.

use super::*;
use crate::codegen::shared::build_component_map;
use crate::codegen::{format_json_value, JSON_FLUTTER};
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::fmt::Write;

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
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::Add, false, false),
        "+"
    );
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::Eq, false, false),
        "=="
    );
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::And, false, false),
        "&&"
    );
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::Or, false, false),
        "||"
    );
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::Neq, false, false),
        "!="
    );
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
    assert_eq!(format_expr_ir_flutter(&expr), "a + b");
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
    assert_eq!(format_expr_ir_flutter(&expr), "x ? 1 : 0");
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
    assert_eq!(format_expr_ir_flutter(&expr), "obj.prop");
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
        intent: Some(MutationIntent::Increment {
            target: SignalRefIR {
                name: "count".into(),
            },

            amount: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
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
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "name".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            })),
            field_object: None,
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
        intent: Some(MutationIntent::Decrement {
            target: SignalRefIR {
                name: "count".into(),
            },

            amount: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
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

/// Format a view tree with an empty component/store context, returning
/// (formatted_string, warnings).
fn fmt_view(node: &ViewTree) -> (String, Vec<String>) {
    let ir = empty_program();
    let component_map: FxHashMap<&str, &ComponentIR> = FxHashMap::default();
    let store_provider_map: FxHashMap<String, String> = FxHashMap::default();
    let mut warnings = Vec::new();
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ir: &ir,
        warnings: &mut warnings,
    };
    let result = format_view_tree(node, &[], &mut ctx);
    (result, warnings)
}

#[test]
fn test_format_text_node() {
    let node = ViewTree::Text("Hello".into());
    let (expr, _) = fmt_view(&node);
    assert_eq!(expr, "Text('Hello')");
}

#[test]
fn test_format_text_with_interpolation() {
    let node = ViewTree::Text("{name}".into());
    let (expr, _) = fmt_view(&node);
    assert_eq!(expr, "Text('$name')");
}

#[test]
fn test_format_element_empty() {
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container()");
}

#[test]
fn test_format_element_with_text_child() {
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![ViewTree::Text("Hello".into())],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container(child: Text('Hello'))");
}

#[test]
fn test_format_element_with_multiple_children() {
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "column".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![
            ViewTree::Text("First".into()),
            ViewTree::Text("Second".into()),
        ],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Column(children: [Text('First'), Text('Second')])");
}

#[test]
fn test_format_element_with_event() {
    let mut events = FxHashMap::default();
    events.insert(
        "click".into(),
        EventHandlerIR::FnRef {
            r#ref: "handleClick".into(),
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "button".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events,
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(expr.contains("ElevatedButton("));
    assert!(expr.contains("onPressed: () { handleClick(); }"));
}

#[test]
fn test_format_element_with_class_map_bindings() {
    let mut bindings = FxHashMap::default();
    bindings.insert(
        "active".to_string(),
        ExprIR::IdentifierExpr {
            name: "isOpen".to_string(),
            type_ann: None,
        },
    );
    let el = ElementIR {
        class_map_bindings: bindings,
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Should emit conditional decoration that uses the binding expression
    assert!(
        expr.contains("isOpen"),
        "should contain the condition expression: {}",
        expr
    );
    assert!(
        expr.contains("BoxDecoration"),
        "should contain conditional BoxDecoration: {}",
        expr
    );
    assert!(
        expr.contains("Container("),
        "should wrap in Container: {}",
        expr
    );
}

#[test]
fn test_format_element_with_class_map_bindings_static_true() {
    let mut bindings = FxHashMap::default();
    bindings.insert(
        "active".to_string(),
        ExprIR::LiteralExpr {
            value: serde_json::json!(true),
        },
    );
    let el = ElementIR {
        class_map_bindings: bindings,
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Static true class bindings should not add wrapping Container
    assert_eq!(expr, "Container()");
}

#[test]
fn test_format_element_with_class_map_bindings_static_false() {
    let mut bindings = FxHashMap::default();
    bindings.insert(
        "inactive".to_string(),
        ExprIR::LiteralExpr {
            value: serde_json::json!(false),
        },
    );
    let el = ElementIR {
        class_map_bindings: bindings,
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Static false class bindings should be skipped entirely
    assert_eq!(expr, "Container()");
}

#[test]
fn test_format_element_with_class_map_bindings_and_child() {
    let mut bindings = FxHashMap::default();
    bindings.insert(
        "active".to_string(),
        ExprIR::IdentifierExpr {
            name: "isOpen".to_string(),
            type_ann: None,
        },
    );
    let el = ElementIR {
        class_map_bindings: bindings,
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![ViewTree::Text("Hello".into())],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Should wrap the child Container inside a conditional Container
    assert!(
        expr.contains("Container("),
        "should contain outer Container: {}",
        expr
    );
    assert!(
        expr.contains("child: Container(child: Text('Hello'))"),
        "should contain inner Container with child: {}",
        expr
    );
    assert!(
        expr.contains("isOpen"),
        "should contain condition expression: {}",
        expr
    );
    assert!(
        expr.contains("BoxDecoration"),
        "should contain conditional decoration: {}",
        expr
    );
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

        source_span: None,
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
                intent: Some(MutationIntent::Increment {
                    target: SignalRefIR {
                        name: "count".into(),
                    },

                    amount: Some(Box::new(ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
                    })),
                }),
            }],
        },
        is_async: false,

        source_span: None,
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
fn make_ir(components: FxHashMap<String, ComponentIR>) -> IRProgram {
    IRProgram {
        version: "1.0".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                ..Default::default()
            },
        )]),
        stdlib_imports: Default::default(),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        source_span: None,
    }
}

/// Empty IRProgram (no modules) for isolated view-formatting tests.
fn empty_program() -> IRProgram {
    IRProgram {
        version: "1.0".into(),
        modules: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        source_span: None,
    }
}

/// Build an IRProgram with a single module containing the given stores.
fn make_ir_with_stores(
    components: FxHashMap<String, ComponentIR>,
    stores: FxHashMap<String, StoreIR>,
) -> IRProgram {
    IRProgram {
        version: "1.0".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                stores,
                ..Default::default()
            },
        )]),
        stdlib_imports: Default::default(),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        source_span: None,
    }
}

#[test]
fn test_empty_component() {
    let codegen = FlutterCodegen;
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
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("App", comp_ref, &ir, &component_map);
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
            fns: vec![FunctionIR {
                name: "increment".into(),
                visibility: Visibility::Private,
                params: vec![],
                return_type: None,
                body: BlockIR {
                    statements: vec![StatementIR::MutationStmt {
                        intent: Some(MutationIntent::Increment {
                            target: SignalRefIR {
                                name: "count".into(),
                            },

                            amount: Some(Box::new(ExprIR::LiteralExpr {
                                value: serde_json::json!(1),
                            })),
                        }),
                    }],
                },
                is_async: false,

                source_span: None,
            }],
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
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("Counter", comp_ref, &ir, &component_map);
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
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("Greeting", comp_ref, &ir, &component_map);
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
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("App", comp_ref, &ir, &component_map);
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
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("App", comp_ref, &ir, &component_map);
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
        signals: FxHashMap::default(),
        signal_defaults: FxHashMap::default(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let store_dart = codegen.gen_store_dart("EmptyStore", &store);
    assert!(store_dart.contains("import 'package:flutter_riverpod/flutter_riverpod.dart';"));
    assert!(
        store_dart.contains("Empty store"),
        "empty store should have comment: {}",
        store_dart
    );
}

#[test]
fn test_store_dart_with_signals() {
    let codegen = FlutterCodegen;
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
    let store_dart = codegen.gen_store_dart("AppStore", &store);
    // Riverpod 3.0: immutable state class
    assert!(
        store_dart.contains("class AppStoreState {"),
        "should emit state class: {}",
        store_dart
    );
    assert!(
        store_dart.contains("final int count;"),
        "should emit count field: {}",
        store_dart
    );
    assert!(
        store_dart.contains("final String name;"),
        "should emit name field: {}",
        store_dart
    );
    assert!(
        store_dart.contains("required this.count,"),
        "should emit required count constructor param: {}",
        store_dart
    );
    assert!(
        store_dart.contains("AppStoreState copyWith({"),
        "should emit copyWith: {}",
        store_dart
    );
    // Notifier class + provider
    assert!(
        store_dart.contains("class AppStore extends Notifier<AppStoreState> {"),
        "should emit Notifier class: {}",
        store_dart
    );
    assert!(
        store_dart.contains("AppStoreState build() {"),
        "should emit build(): {}",
        store_dart
    );
    assert!(
        store_dart.contains("count: 0") && store_dart.contains("name: 'world'"),
        "should emit signal defaults in build(): {}",
        store_dart
    );
    assert!(
        store_dart.contains(
            "final appStoreProvider = NotifierProvider<AppStore, AppStoreState>(AppStore.new);"
        ),
        "should emit NotifierProvider: {}",
        store_dart
    );
    // No stale Riverpod 2.x StateProvider API
    assert!(
        !store_dart.contains("StateProvider"),
        "should not emit Riverpod 2.x StateProvider: {}",
        store_dart
    );
}

#[test]
fn test_store_dart_with_getters() {
    let codegen = FlutterCodegen;
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
        getter_types: FxHashMap::from_iter([(
            "doubleCount".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        actions: vec![],
        ..Default::default()
    };
    let store_dart = codegen.gen_store_dart("CalcStore", &store);
    // Riverpod 3.0: getters are computed properties on the immutable state class
    assert!(store_dart.contains("class CalcStoreState {"));
    assert!(
        store_dart.contains("int get doubleCount => count * 2;"),
        "should emit getter on state class: {}",
        store_dart
    );
    assert!(store_dart.contains(
        "final calcStoreProvider = NotifierProvider<CalcStore, CalcStoreState>(CalcStore.new);"
    ));
    assert!(
        !store_dart.contains("Provider<dynamic>"),
        "should not emit Riverpod 2.x Provider<dynamic>: {}",
        store_dart
    );
}

// ── Motion codegen tests ───────────────────────────────

#[test]
fn test_motion_enter_controller_init() {
    let codegen = FlutterCodegen;

    // Create a keyframe motion: opacity 0→1 over 300ms
    let motion = MotionIntent {
        name: "fadeIn".into(),
        body: KeyframeIR {
            stops: vec![KeyframeStopIR {
                position: 0.0,
                properties: FxHashMap::from_iter([(
                    "opacity".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(0.0),
                    },
                )]),
                ease: None,
                source_span: None,
            }],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!("300ms"),
            },
            ease: Some(EasingIR::Named("easeOut".into())),
            playback: PlaybackIR::default(),
            source_span: None,
        },
        source_span: None,
    };

    // View element with @enter trigger
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![ViewTree::Text("Hello".into())],
        construction_kind: ConstructionKind::Component,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::Enter,
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,
                source_span: None,
            }],
            ..Default::default()
        })),
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };

    let view = ViewTree::Element(Box::new(el));

    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            name: "App".into(),
            view: Some(view),
            ..Default::default()
        },
    );

    let ir = IRProgram {
        version: "1.0".into(),
        modules: FxHashMap::from_iter([(
            "main".into(),
            ModuleIR {
                name: "main".into(),
                components,
                motions: vec![motion],
                ..Default::default()
            },
        )]),
        stdlib_imports: Default::default(),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        source_span: None,
    };

    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("App")
        .unwrap();
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("App", comp_ref, &ir, &component_map);

    // Field declarations
    assert!(
        dart.contains("late final AnimationController _fadeInController;"),
        "should declare _fadeInController field"
    );
    assert!(
        dart.contains("late final Animation<double> _fadeInAnimation;"),
        "should declare _fadeInAnimation field"
    );

    // initState: AnimationController init
    assert!(
        dart.contains("_fadeInController = AnimationController("),
        "should init AnimationController: {}",
        dart
    );
    assert!(
        dart.contains("duration: Duration(milliseconds: 300)"),
        "should set duration: {}",
        dart
    );

    // Tween + Curve
    assert!(
        dart.contains("Tween<double>(begin: 0.0, end: 1.0)"),
        "should create Tween: {}",
        dart
    );
    assert!(
        dart.contains("CurveTween(curve: Curves.easeOut)"),
        "should apply easing: {}",
        dart
    );

    // @enter → forward() on mount
    assert!(
        dart.contains("_fadeInController.forward();"),
        "should call forward() for @enter trigger: {}",
        dart
    );

    // AnimatedBuilder wrapping in build method
    assert!(
        dart.contains("AnimatedBuilder("),
        "should emit AnimatedBuilder in build: {}",
        dart
    );
    assert!(
        dart.contains("animation: _fadeInAnimation"),
        "AnimatedBuilder should reference animation"
    );

    // dispose
    assert!(
        dart.contains("_fadeInController.dispose();"),
        "should dispose controller"
    );
}

#[test]
fn test_motion_hover_mouse_region_wrapping() {
    let triggers = vec![TriggerIntent {
        kind: TriggerKind::Hover,
        motion_ref: Some("scaleOnHover".into()),
        inline_motion: None,
        source_span: None,
    }];

    // Motion with scale animation
    let motions = vec![MotionIntent {
        name: "scaleOnHover".into(),
        body: KeyframeIR {
            stops: vec![KeyframeStopIR {
                position: 0.0,
                properties: FxHashMap::from_iter([(
                    "scale".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(1.0),
                    },
                )]),
                ease: None,
                source_span: None,
            }],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!("200ms"),
            },
            ease: None,
            playback: PlaybackIR::default(),
            source_span: None,
        },
        source_span: None,
    }];

    let result = wrap_with_motion_widgets(
        "Container(child: Text('Hover me'))".into(),
        &triggers,
        &motions,
    );

    assert!(
        result.contains("MouseRegion("),
        "should wrap with MouseRegion: {}",
        result
    );
    assert!(
        result.contains("onEnter: (_) => _scaleOnHoverController.forward()"),
        "should call forward on mouse enter: {}",
        result
    );
    assert!(
        result.contains("onExit: (_) => _scaleOnHoverController.reverse()"),
        "should call reverse on mouse exit: {}",
        result
    );
}

#[test]
fn test_motion_inline_body_handling() {
    let codegen = FlutterCodegen;

    // Create a multi-property motion simulating an inline body definition
    let motion = MotionIntent {
        name: "popIn".into(),
        body: KeyframeIR {
            stops: vec![KeyframeStopIR {
                position: 0.0,
                properties: FxHashMap::from_iter([
                    (
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0.0),
                        },
                    ),
                    (
                        "scale".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0.5),
                        },
                    ),
                ]),
                ease: None,
                source_span: None,
            }],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!(400),
            },
            ease: Some(EasingIR::Named("power1Out".into())),
            playback: PlaybackIR::default(),
            source_span: None,
        },
        source_span: None,
    };

    // Element with @enter trigger referencing the motion
    // inline_motion is set to simulate an inline body definition (Task 8 Step 10 pattern)
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::Enter,
                motion_ref: Some("popIn".into()),
                inline_motion: Some(KeyframeIR {
                    stops: vec![
                        KeyframeStopIR {
                            position: 0.0,
                            properties: FxHashMap::from_iter([
                                (
                                    "opacity".into(),
                                    ExprIR::LiteralExpr {
                                        value: serde_json::json!(0.0),
                                    },
                                ),
                                (
                                    "scale".into(),
                                    ExprIR::LiteralExpr {
                                        value: serde_json::json!(0.5),
                                    },
                                ),
                            ]),
                            ease: None,
                            source_span: None,
                        },
                        KeyframeStopIR {
                            position: 1.0,
                            properties: FxHashMap::from_iter([
                                (
                                    "opacity".into(),
                                    ExprIR::LiteralExpr {
                                        value: serde_json::json!(1.0),
                                    },
                                ),
                                (
                                    "scale".into(),
                                    ExprIR::LiteralExpr {
                                        value: serde_json::json!(1.0),
                                    },
                                ),
                            ]),
                            ease: None,
                            source_span: None,
                        },
                    ],
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(400),
                    },
                    ease: Some(EasingIR::Named("power1Out".into())),
                    playback: PlaybackIR::default(),
                    source_span: None,
                }),
                source_span: None,
            }],
            ..Default::default()
        })),
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };

    let view = ViewTree::Element(Box::new(el));

    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            name: "App".into(),
            view: Some(view),
            ..Default::default()
        },
    );

    let ir = IRProgram {
        version: "1.0".into(),
        modules: FxHashMap::from_iter([(
            "main".into(),
            ModuleIR {
                name: "main".into(),
                components,
                motions: vec![motion],
                ..Default::default()
            },
        )]),
        stdlib_imports: Default::default(),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        source_span: None,
    };

    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("App")
        .unwrap();
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("App", comp_ref, &ir, &component_map);

    // Numeric duration init
    assert!(
        dart.contains("_popInController = AnimationController("),
        "should init AnimationController for popIn"
    );
    assert!(
        dart.contains("duration: Duration(milliseconds: 400)"),
        "should set numeric duration: {}",
        dart
    );

    // Power1Out easing → Curves.easeOutQuad via shared NAMED_EASINGS
    assert!(
        dart.contains("CurveTween(curve: Curves.easeOutQuad)"),
        "should resolve power1Out easing: {}",
        dart
    );

    // @enter → forward()
    assert!(
        dart.contains("_popInController.forward();"),
        "should call forward() for @enter"
    );

    // Tween with begin/end values from keyframe stops
    // (begin value depends on FxHashMap iteration order — test the general pattern)
    assert!(
        dart.contains("Tween<double>(begin:"),
        "should create Tween: {}",
        dart
    );
    assert!(
        dart.contains("end: 1.0)"),
        "should have end value 1.0: {}",
        dart
    );

    // AnimatedBuilder with multi-property wrapping (Opacity + Transform.scale)
    assert!(
        dart.contains("AnimatedBuilder("),
        "should emit AnimatedBuilder: {}",
        dart
    );
    assert!(
        dart.contains("Transform.scale(scale: _popInAnimation.value"),
        "should wrap with Transform.scale: {}",
        dart
    );
    assert!(
        dart.contains("Opacity(opacity: _popInAnimation.value"),
        "should wrap with Opacity: {}",
        dart
    );

    // dispose
    assert!(
        dart.contains("_popInController.dispose();"),
        "should dispose popIn controller"
    );
}

#[test]
fn test_motion_format_named_curve() {
    // CSS standard keywords
    assert_eq!(format_named_curve("linear"), "Curves.linear");
    assert_eq!(format_named_curve("easeOut"), "Curves.easeOut");
    assert_eq!(format_named_curve("easeInOut"), "Curves.easeInOut");

    // GSAP power1 (quadratic) — Task 11: shared NAMED_EASINGS delegation
    assert_eq!(
        format_named_curve("power1Out"),
        "Curves.easeOutQuad",
        "power1Out should map to Curves.easeOutQuad via shared table"
    );
    assert_eq!(format_named_curve("power1In"), "Curves.easeInQuad");
    assert_eq!(format_named_curve("power1InOut"), "Curves.easeInOutQuad");

    // GSAP power4 (quintic)
    assert_eq!(format_named_curve("power4Out"), "Curves.easeOutQuint");

    // Back / Bounce / Elastic
    assert_eq!(format_named_curve("backOut"), "Curves.easeOutBack");
    assert_eq!(format_named_curve("bounceOut"), "Curves.bounceOut");
    assert_eq!(format_named_curve("elasticOut"), "Curves.easeOutElastic");

    // Unknown name → fallback
    assert_eq!(
        format_named_curve("nonexistent"),
        "Curves.linear",
        "unknown easing should fall back to Curves.linear"
    );
}

#[test]
fn test_motion_extract_duration_ms() {
    // String "300ms" → strip "ms" suffix
    assert_eq!(
        extract_duration_ms(&ExprIR::LiteralExpr {
            value: serde_json::json!("300ms")
        }),
        "300"
    );

    // String "1.5s" → converted to milliseconds (was passed through unparsed,
    // producing invalid `Duration(milliseconds: 1.5s)` Dart)
    assert_eq!(
        extract_duration_ms(&ExprIR::LiteralExpr {
            value: serde_json::json!("1.5s")
        }),
        "1500"
    );

    // Numeric → stringified
    assert_eq!(
        extract_duration_ms(&ExprIR::LiteralExpr {
            value: serde_json::json!(300)
        }),
        "300"
    );
    assert_eq!(
        extract_duration_ms(&ExprIR::LiteralExpr {
            value: serde_json::json!(400)
        }),
        "400"
    );

    // IdentifierExpr (non-literal) → default fallback "300"
    assert_eq!(
        extract_duration_ms(&ExprIR::IdentifierExpr {
            name: "customDur".into(),
            type_ann: None,
        }),
        "300"
    );
}

#[test]
fn test_store_dart_with_action() {
    let codegen = FlutterCodegen;
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
        name: "AppStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "increment".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent::Increment {
                        target: SignalRefIR {
                            name: "count".into(),
                        },
                        amount: Some(Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        })),
                    }),
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let store_dart = codegen.gen_store_dart("AppStore", &store);
    // Riverpod 3.0: actions are Notifier methods mutating state via copyWith
    assert!(
        store_dart.contains("void increment() {"),
        "should emit action as Notifier method: {}",
        store_dart
    );
    assert!(
        store_dart.contains("state = state.copyWith(count: state.count + 1);"),
        "should mutate via copyWith: {}",
        store_dart
    );
    assert!(
        !store_dart.contains("WidgetRef ref"),
        "actions should not take WidgetRef: {}",
        store_dart
    );
}

// ── Full pipeline through Codegen trait ───────────────────

#[test]
fn test_full_pipeline_empty() {
    let codegen = FlutterCodegen;
    let ir = IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::default(),
                stores: FxHashMap::default(),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
    };
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert!(output.files.is_empty());
}

#[test]
fn test_full_pipeline_component() {
    let codegen = FlutterCodegen;
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
    let ir = IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::from_iter([("App".into(), comp)]),
                stores: FxHashMap::default(),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
    };
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
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
        signals: FxHashMap::from_iter([(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: FxHashMap::from_iter([("count".into(), serde_json::json!(0))]),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::default(),
                stores: FxHashMap::from_iter([("AppStore".into(), store)]),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
    };
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
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
        signals: FxHashMap::from_iter([(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: FxHashMap::from_iter([("count".into(), serde_json::json!(0))]),
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
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::from_iter([("App".into(), comp)]),
                stores: FxHashMap::from_iter([("AppStore".into(), store)]),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        motion_index: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
    };
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 2); // App.dart + AppStore_store.dart
    insta::with_settings!({snapshot_path => "snapshots"}, {
        let mut snapshot_content = String::new();
        for f in &output.files {
            let _ = write!(snapshot_content, "=== {} ===\n", f.path);
            snapshot_content.push_str(&f.content);
            snapshot_content.push('\n');
        }
        insta::assert_snapshot!("test_full_pipeline_component_with_store_ref", &snapshot_content);
    });
}

// ═══════════════════════════════════════════════════════════
// Integration tests (full pipeline)
// ═══════════════════════════════════════════════════════════

/// Compile a single .omni source string to Flutter, returning the InternalCompileResult.
fn compile_flutter(source: &str) -> Option<crate::InternalCompileResult> {
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = crate::path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::Flutter,
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

#[test]
fn warns_on_slot_in_flutter() {
    let source = r#"
component Card {
    view {
        <div class="card">@slot</div>
    }
}
"#;
    let output = compile_flutter(source).unwrap();
    // Slot warnings now flow through the compile result diagnostics
    assert!(
        output
            .warnings
            .iter()
            .any(|w| w.message.contains("slot") && w.message.contains("not supported")),
        "Expected slot warning in compile output, got: {:?}",
        output.warnings
    );
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("SizedBox.shrink()"),
        "slot should degrade to SizedBox.shrink(): {}",
        content
    );
}

#[test]
fn css_padding_emits_valid_dart() {
    let source = r#"
component Styled {
    view {
        <div style: { padding: "16px", margin: "8px" }>hello</div>
    }
}
"#;
    let output = compile_flutter(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // Must contain valid Dart EdgeInsets, not raw CSS strings
    assert!(
        content.contains("EdgeInsets.all(16.0)") || content.contains("EdgeInsets.all(16)"),
        "Expected EdgeInsets.all(16.0), got: {}",
        content
    );
    assert!(
        !content.contains("EdgeInsets.all(\"16px\")"),
        "Must not emit CSS unit string, got: {}",
        content
    );
    assert!(
        !content.contains("EdgeInsets.all(\"8px\")"),
        "Must not emit CSS unit string for margin, got: {}",
        content
    );
}

// ── C2: @effect generates real _runEffect_N method ────────

#[test]
fn test_effect_emits_real_method_and_lifecycle_wiring() {
    let codegen = FlutterCodegen;
    let effect = EffectIntent {
        deps: vec!["count".into(), "title".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::IdentifierExpr {
                    name: "log".into(),
                    type_ann: None,
                },
            }],
        },
    };
    let mut props = FxHashMap::default();
    props.insert(
        "title".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            name: "App".into(),
            props,
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
        .get("App")
        .unwrap();
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("App", comp_ref, &ir, &component_map);

    // Real method body (was: undefined stub call + inline body + false comment)
    assert!(
        dart.contains("void _runEffect_0() {"),
        "should define _runEffect_0: {}",
        dart
    );
    assert!(
        dart.contains("log;"),
        "should emit effect body statements: {}",
        dart
    );
    assert!(
        !dart.contains("re-executes on setState"),
        "false comment removed: {}",
        dart
    );

    // initState calls the method
    assert!(
        dart.contains("_runEffect_0();"),
        "should call _runEffect_0 from initState: {}",
        dart
    );

    // didUpdateWidget re-runs after deps equality check
    assert!(
        dart.contains("void didUpdateWidget(App oldWidget) {"),
        "should emit didUpdateWidget with concrete widget type: {}",
        dart
    );
    assert!(
        dart.contains("oldWidget.title != widget.title"),
        "prop dep compared via oldWidget vs widget: {}",
        dart
    );
    assert!(
        dart.contains("_prevcount != count"),
        "signal dep compared via snapshot field: {}",
        dart
    );
    assert!(
        dart.contains("late int _prevcount;"),
        "should declare snapshot field: {}",
        dart
    );
    assert!(
        dart.contains("_prevcount = count;"),
        "should snapshot dep after run: {}",
        dart
    );
}

// ── C3: const CompName({ constructor ──────────────────────

#[test]
fn test_component_constructor_includes_class_name() {
    let codegen = FlutterCodegen;
    let mut props = FxHashMap::default();
    props.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
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
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("Greeting", comp_ref, &ir, &component_map);
    assert!(
        dart.contains("const Greeting({"),
        "constructor must include class name, got: {}",
        dart
    );
    assert!(
        !dart.contains("const {"),
        "must not emit bare 'const {{': {}",
        dart
    );
}

// ── C4: store reads in views → ref.watch(...) ─────────────

#[test]
fn test_view_store_field_read_uses_ref_watch() {
    let codegen = FlutterCodegen;
    let store = StoreIR {
        name: "TodoStore".into(),
        signals: FxHashMap::from_iter([(
            "filterStatus".into(),
            TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: FxHashMap::from_iter([("filterStatus".into(), serde_json::json!("all"))]),
        getters: FxHashMap::from_iter([(
            "totalCount".into(),
            ExprIR::MethodCallExpr {
                object: Box::new(ExprIR::IdentifierExpr {
                    name: "items".into(),
                    type_ann: None,
                }),
                method: "len".into(),
                args: vec![],
                is_optional: false,
            },
        )]),
        actions: vec![FunctionIR {
            name: "fetchReminders".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let view = ViewTree::Fragment(vec![
        // Getter read → ref.watch(provider.select((s) => s.totalCount))
        ViewTree::Interpolation(Box::new(ExprIR::StoreFieldAccessExpr {
            store_alias: "TodoStore".into(),
            field_name: "totalCount".into(),
            call_args: vec![],
        })),
        // Signal read → ref.watch(provider.select((s) => s.filterStatus))
        ViewTree::Interpolation(Box::new(ExprIR::StoreFieldAccessExpr {
            store_alias: "TodoStore".into(),
            field_name: "filterStatus".into(),
            call_args: vec![],
        })),
        // Action with no args → ref.read(provider.notifier).fetchReminders()
        ViewTree::Interpolation(Box::new(ExprIR::StoreFieldAccessExpr {
            store_alias: "TodoStore".into(),
            field_name: "fetchReminders".into(),
            call_args: vec![],
        })),
    ]);
    let mut components = FxHashMap::default();
    components.insert(
        "FilterBar".into(),
        ComponentIR {
            name: "FilterBar".into(),
            store_refs: vec!["TodoStore".into()],
            view: Some(view),
            ..Default::default()
        },
    );
    let ir = make_ir_with_stores(
        components,
        FxHashMap::from_iter([("TodoStore".into(), store)]),
    );
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("FilterBar")
        .unwrap();
    let component_map = build_component_map(&ir);
    let (dart, _warnings) = codegen.gen_component_dart("FilterBar", comp_ref, &ir, &component_map);
    assert!(
        dart.contains("ref.watch(todoStoreProvider.select(((s) => s.totalCount)))"),
        "getter read must become ref.watch select: {}",
        dart
    );
    assert!(
        dart.contains("ref.watch(todoStoreProvider.select(((s) => s.filterStatus)))"),
        "signal read must become ref.watch select: {}",
        dart
    );
    assert!(
        dart.contains("ref.read(todoStoreProvider.notifier).fetchReminders()"),
        "action call must become ref.read notifier: {}",
        dart
    );
    assert!(
        !dart.contains("TodoStore.totalCount"),
        "must not emit bare store.field access: {}",
        dart
    );
}

// ── C5: CSS color conversion ──────────────────────────────

#[test]
fn test_css_hex_to_dart_color() {
    assert_eq!(
        css_hex_to_dart_color("#ff0000").as_deref(),
        Some("Color(0xFFFF0000)")
    );
    assert_eq!(
        css_hex_to_dart_color("#f00").as_deref(),
        Some("Color(0xFFFF0000)")
    );
    assert_eq!(
        css_hex_to_dart_color("#00ff00").as_deref(),
        Some("Color(0xFF00FF00)")
    );
    assert_eq!(
        css_hex_to_dart_color("  #abcdef  ").as_deref(),
        Some("Color(0xFFABCDEF)")
    );
    assert_eq!(
        css_hex_to_dart_color("#ffff").as_deref(),
        None,
        "4-digit hex unsupported"
    );
    assert_eq!(
        css_hex_to_dart_color("red").as_deref(),
        None,
        "named colors unsupported"
    );
    assert_eq!(
        css_hex_to_dart_color("#gg0000").as_deref(),
        None,
        "non-hex digits rejected"
    );
}

#[test]
fn test_css_color_to_dart_rgb() {
    assert_eq!(
        css_color_to_dart("rgb(255, 0, 0)").as_deref(),
        Some("Color.fromARGB(255, 255, 0, 0)")
    );
    assert_eq!(
        css_color_to_dart("rgba(0, 128, 255, 0.5)").as_deref(),
        Some("Color.fromARGB(128, 0, 128, 255)")
    );
    assert_eq!(css_color_to_dart("50%").as_deref(), None);
}

#[test]
fn test_inline_style_hex_color_emits_color_literal() {
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: Some(serde_json::json!({ "color": "#ff0000" })),
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, _warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("color: Color(0xFFFF0000)"),
        "hex color must convert to Color literal: {}",
        expr
    );
}

#[test]
fn test_inline_style_unsupported_color_warns() {
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: Some(serde_json::json!({ "color": "red" })),
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        !expr.contains("color:"),
        "unsupported color must be skipped: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("unsupported color value")),
        "unsupported color should warn: {:?}",
        warnings
    );
}

// ── C6: per-widget parameter whitelist ────────────────────

#[test]
fn test_unmapped_prop_warns_and_is_skipped() {
    let mut props = FxHashMap::default();
    props.insert(
        "ariaLabel".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("close"),
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props,
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr, "Container()",
        "unmapped prop must not be silently included: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("ariaLabel") && w.contains("not supported on 'Container'")),
        "unmapped prop should produce a compiler warning: {:?}",
        warnings
    );
}

#[test]
fn test_slug_props_skipped_silently() {
    let mut props = FxHashMap::default();
    props.insert(
        "class".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("card"),
        },
    );
    let mut style = FxHashMap::default();
    style.insert(
        "padding".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("8px"),
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props,
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: style,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container()");
    assert!(
        !warnings.iter().any(|w| w.contains("class")),
        "slug props (class/style) are intentionally dropped without warnings: {:?}",
        warnings
    );
}

#[test]
fn test_event_not_supported_on_widget_warns() {
    let mut events = FxHashMap::default();
    events.insert(
        "click".into(),
        EventHandlerIR::FnRef {
            r#ref: "handleClick".into(),
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events,
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr, "Container()",
        "Container has no onPressed — event must not be emitted: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("click") && w.contains("not supported on 'Container'")),
        "event on unsupported widget should warn: {:?}",
        warnings
    );
}

#[test]
fn test_event_on_button_is_allowed() {
    let mut events = FxHashMap::default();
    events.insert(
        "click".into(),
        EventHandlerIR::FnRef {
            r#ref: "handleClick".into(),
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "button".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events,
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("onPressed: () { handleClick(); }"),
        "click on ElevatedButton maps to onPressed: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "allowed event should not warn: {:?}",
        warnings
    );
}

#[test]
fn test_store_action_handler_uses_provider() {
    assert_eq!(
        format_store_action_handler("TodoStore.setFilter"),
        "() { ref.read(todoStoreProvider.notifier).setFilter(); }"
    );
}

// ── C6: TextStyle/InputDecoration wrapper emission ────────

#[test]
fn test_text_style_props_wrapped_in_text_style() {
    let mut props = FxHashMap::default();
    props.insert(
        "fontSize".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!(16),
        },
    );
    props.insert(
        "fontWeight".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("bold"),
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "p".into(),
        props,
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // fontSize/fontWeight are TextStyle members — merged into a single style: arg
    assert!(
        expr.contains("style: TextStyle(")
            && expr.contains("fontSize: 16")
            && expr.contains("fontWeight: 'bold'"),
        "Text style props must merge into TextStyle: {}",
        expr
    );
    assert!(
        !expr.contains("Text(fontSize"),
        "must not emit fontSize as a direct Text param: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "wrapped props must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_text_semantics_label_rename() {
    let mut props = FxHashMap::default();
    props.insert(
        "alt".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("icon"),
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "p".into(),
        props,
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("semanticsLabel: 'icon'"),
        "alt on Text must map to semanticsLabel (Text param): {}",
        expr
    );
    assert!(
        !expr.contains("semanticLabel"),
        "must not emit semanticLabel on Text: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "renamed prop must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_text_field_placeholder_wrapped_in_decoration() {
    let mut props = FxHashMap::default();
    props.insert(
        "placeholder".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("Search..."),
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "input".into(),
        props,
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("decoration: InputDecoration(hintText: 'Search...')"),
        "hintText must be wrapped in InputDecoration: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "wrapped prop must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_store_action_in_inline_handler_body() {
    // `@onClick={TodoStore.clear}` lowers to an InlineFn whose body contains a
    // bare store-action reference — it must rewrite to ref.read(...notifier).
    let store = StoreIR {
        name: "TodoStore".into(),
        signals: FxHashMap::from_iter([(
            "items".into(),
            TypeIR {
                kind: "array".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: FxHashMap::from_iter([("items".into(), serde_json::json!([]))]),
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "clear".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let mut events = FxHashMap::default();
    events.insert(
        "click".into(),
        EventHandlerIR::InlineFn {
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: ExprIR::FieldAccessExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "TodoStore".into(),
                            type_ann: None,
                        }),
                        field: "clear".into(),
                    },
                }],
            },
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "button".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events,
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let view = ViewTree::Element(Box::new(el));
    let ir = make_ir_with_stores(
        FxHashMap::default(),
        FxHashMap::from_iter([("TodoStore".into(), store)]),
    );
    let mut warnings = Vec::new();
    let component_map: FxHashMap<&str, &ComponentIR> = FxHashMap::default();
    let store_provider_map: FxHashMap<String, String> =
        FxHashMap::from_iter([("TodoStore".into(), "todoStoreProvider".into())]);
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ir: &ir,
        warnings: &mut warnings,
    };
    let expr = format_view_tree(&view, &[], &mut ctx);
    assert!(
        expr.contains("ref.read(todoStoreProvider.notifier).clear();"),
        "store action in handler body must rewrite to ref.read: {}",
        expr
    );
}

#[test]
fn test_store_action_method_call_form_in_handler() {
    // `@onClick={TodoStore.clear()}` (with parens) lowers the handler body to
    // a MethodCallExpr on the store identifier — same Riverpod rewrite.
    let codegen = FlutterCodegen;
    let store = StoreIR {
        name: "TodoStore".into(),
        signals: FxHashMap::from_iter([(
            "items".into(),
            TypeIR {
                kind: "array".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: FxHashMap::from_iter([("items".into(), serde_json::json!([]))]),
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "clear".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let mut events = FxHashMap::default();
    events.insert(
        "click".into(),
        EventHandlerIR::InlineFn {
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: ExprIR::MethodCallExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "TodoStore".into(),
                            type_ann: None,
                        }),
                        method: "clear".into(),
                        args: vec![],
                        is_optional: false,
                    },
                }],
            },
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "button".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events,
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let view = ViewTree::Element(Box::new(el));
    let ir = make_ir_with_stores(
        FxHashMap::default(),
        FxHashMap::from_iter([("TodoStore".into(), store)]),
    );
    let mut warnings = Vec::new();
    let component_map: FxHashMap<&str, &ComponentIR> = FxHashMap::default();
    let store_provider_map: FxHashMap<String, String> =
        FxHashMap::from_iter([("TodoStore".into(), "todoStoreProvider".into())]);
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ir: &ir,
        warnings: &mut warnings,
    };
    let expr = format_view_tree(&view, &[], &mut ctx);
    assert!(
        expr.contains("ref.read(todoStoreProvider.notifier).clear()"),
        "MethodCallExpr store action must rewrite to ref.read: {}",
        expr
    );
}
