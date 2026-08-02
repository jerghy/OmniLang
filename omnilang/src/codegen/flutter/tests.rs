//! Unit tests for flutter codegen.

use super::design::DesignResolver;
use super::*;
use crate::codegen::shared::build_component_map;
use crate::codegen::{format_json_value, JSON_FLUTTER};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::collections::HashMap;
use std::fmt::Write;
use std::process::Command;

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
    fmt_view_with_motions(node, &[])
}

/// Like [`fmt_view`] but with module-level motions available — needed by the
/// @drag-* extent derivation (deferral #19), which resolves the motion body
/// to compute the gesture extent from its keyframe range.
fn fmt_view_with_motions(node: &ViewTree, motions: &[MotionIntent]) -> (String, Vec<String>) {
    let ir = empty_program();
    fmt_view_in(node, motions, &ir)
}

/// Format a view tree with the given program (design blocks included) —
/// class:/variant: resolution (Task 4.3) needs the design IR.
fn fmt_view_with_program(node: &ViewTree, ir: &IRProgram) -> (String, Vec<String>) {
    fmt_view_in(node, &[], ir)
}

/// Like [`fmt_view_with_program`] but with a component map (component
/// instantiation tests need the map to take the component-element path).
fn fmt_view_with_program_and_map<'a>(
    node: &ViewTree,
    ir: &'a IRProgram,
    component_map: &'a FxHashMap<&'a str, &'a ComponentIR>,
) -> (String, Vec<String>) {
    let store_provider_map: FxHashMap<String, String> = FxHashMap::default();
    let mut warnings = Vec::new();
    let ref_usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver = DesignResolver::new(ir);
    let mut ctx = ViewContext {
        component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir,
        design: &design_resolver,
        warnings: &mut warnings,
    };
    let result = format_view_tree(node, &[], &mut ctx);
    (result, warnings)
}

fn fmt_view_in(node: &ViewTree, motions: &[MotionIntent], ir: &IRProgram) -> (String, Vec<String>) {
    let component_map: FxHashMap<&str, &ComponentIR> = FxHashMap::default();
    let store_provider_map: FxHashMap<String, String> = FxHashMap::default();
    let mut warnings = Vec::new();
    let ref_usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver = DesignResolver::new(ir);
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir,
        design: &design_resolver,
        warnings: &mut warnings,
    };
    let result = format_view_tree(node, motions, &mut ctx);
    (result, warnings)
}

/// A small design IR for class:/variant: resolution tests (Task 4.3):
/// tokens `primary`/`surface`/`radiusMd`, classes `Card` (background +
/// border-radius) and `Active` (background), variant `Button`
/// (color: primary|danger, base border-radius, branches set background).
fn design_test_program() -> IRProgram {
    use crate::ir::design::*;
    let mut tokens = FxHashMap::default();
    for (name, value, token_type) in [
        ("primary", "#6366f1", TokenTypeIR::Color),
        ("surface", "#ffffff", TokenTypeIR::Color),
        ("radiusMd", "8px", TokenTypeIR::Radius),
    ] {
        tokens.insert(
            name.to_string(),
            TokenIR {
                value: TokenValueIR::Static(value.to_string()),
                token_type,
                theme_values: FxHashMap::default(),
                source_span: None,
            },
        );
    }
    let classes = vec![
        ClassIR {
            name: "Card".to_string(),
            properties: vec![
                ("background".to_string(), "var(--surface)".to_string()),
                ("border-radius".to_string(), "var(--radiusMd)".to_string()),
            ]
            .into_iter()
            .collect(),
            pseudo_states: vec![],
            source_span: None,
        },
        ClassIR {
            name: "Active".to_string(),
            properties: vec![("background".to_string(), "var(--primary)".to_string())]
                .into_iter()
                .collect(),
            pseudo_states: vec![],
            source_span: None,
        },
        ClassIR {
            name: "RowLayout".to_string(),
            properties: vec![("display".to_string(), "flex".to_string())]
                .into_iter()
                .collect(),
            pseudo_states: vec![],
            source_span: None,
        },
    ];
    let variants = vec![VariantIR {
        name: "Button".to_string(),
        dimensions: vec![VariantDimIR {
            name: "color".to_string(),
            values: vec!["primary".to_string(), "danger".to_string()],
            source_span: None,
        }],
        defaults: vec![("color".to_string(), "primary".to_string())]
            .into_iter()
            .collect(),
        base: vec![("border-radius".to_string(), "var(--radiusMd)".to_string())]
            .into_iter()
            .collect(),
        branches: vec![
            VariantBranchIR {
                dimension: "color".to_string(),
                value: "primary".to_string(),
                properties: vec![("background".to_string(), "var(--primary)".to_string())]
                    .into_iter()
                    .collect(),
                source_span: None,
            },
            VariantBranchIR {
                dimension: "color".to_string(),
                value: "danger".to_string(),
                properties: vec![("background".to_string(), "#ef4444".to_string())]
                    .into_iter()
                    .collect(),
                source_span: None,
            },
        ],
        source_span: None,
    }];
    let design = DesignIR {
        name: "App".to_string(),
        themes: vec![],
        theme_signal: None,
        default_theme: None,
        tokens,
        classes,
        variants,
        source_span: None,
        source_file: Some("src/design.omni".to_string()),
    };
    IRProgram {
        version: "1.0".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                designs: vec![design],
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
fn test_format_text_node() {
    let node = ViewTree::Text("Hello".into());
    let (expr, _) = fmt_view(&node);
    assert_eq!(expr, "Text('Hello')");
}

#[test]
fn test_format_text_with_interpolation() {
    // `{`-leading TEXT is malformed-input fallback (parser interpolation
    // fallback) — rendered literally as plain text, never as Dart
    // interpolation code (Task 3.1 review CRITICAL round 2). Legit
    // interpolations arrive as ViewTree::Interpolation nodes.
    let node = ViewTree::Text("{name}".into());
    let (expr, _) = fmt_view(&node);
    assert_eq!(expr, "Text('{name}')");
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container(child: Text('Hello'))");
}

#[test]
fn test_drag_widget_derives_extent_from_keyframes() {
    // ADR-053 removed @drag-constraints — without constraints the drag extent
    // must be derived from the motion's own keyframe range (translateX
    // 0→300 ⇒ dragExtent 300.0) so the element follows its animation
    // (deferral #19) instead of dividing by Infinity.
    let motions = vec![MotionIntent {
        name: "slideX".into(),
        body: KeyframeIR {
            stops: vec![
                KeyframeStopIR {
                    position: 0.0,
                    properties: FxHashMap::from_iter([(
                        "translateX".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
                KeyframeStopIR {
                    position: 1.0,
                    properties: FxHashMap::from_iter([(
                        "translateX".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(300),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
            ],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!("300ms"),
            },
            ease: None,
            playback: PlaybackIR::default(),
            source_span: None,
        },
        source_span: None,
    }];
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![ViewTree::Text("drag me".into())],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::DragHorizontal {
                    constraints: None,
                    elastic: None,
                    threshold: None,
                    velocity_max: None,
                },
                motion_ref: Some("slideX".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, _) = fmt_view_with_motions(&ViewTree::Element(Box::new(el)), &motions);
    assert!(
        expr.contains("dragExtent: 300.0"),
        "expected keyframe-derived dragExtent, got: {}",
        expr
    );
    assert!(
        !expr.contains("WARNING"),
        "derived extent must not carry a fallback warning, got: {}",
        expr
    );
}

#[test]
fn test_drag_widget_without_translate_falls_back_unbounded() {
    // A motion with no translate range must not emit a bogus extent — it
    // falls back to the previous unbounded (inert) behavior with a warning.
    let motions = vec![MotionIntent {
        name: "fadeOnly".into(),
        body: KeyframeIR {
            stops: vec![
                KeyframeStopIR {
                    position: 0.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
                KeyframeStopIR {
                    position: 1.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
            ],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!("300ms"),
            },
            ease: None,
            playback: PlaybackIR::default(),
            source_span: None,
        },
        source_span: None,
    }];
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![ViewTree::Text("drag me".into())],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::DragVertical {
                    constraints: None,
                    elastic: None,
                    threshold: None,
                    velocity_max: None,
                },
                motion_ref: Some("fadeOnly".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, warnings) = fmt_view_with_motions(&ViewTree::Element(Box::new(el)), &motions);
    assert!(
        expr.contains("dragExtent: 0.0"),
        "no-translate motion must fall back to the unbounded extent, got: {}",
        expr
    );
    assert!(
        expr.contains("WARNING"),
        "fallback must carry a warning comment, got: {}",
        expr
    );
    assert!(
        !expr.contains("NaN"),
        "fallback must not emit NaN, got: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("fadeOnly") && w.contains("literal translateX/Y")),
        "fallback must push a compiler warning through the warnings channel, got: {:?}",
        warnings
    );
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Column(children: [Text('First'), Text('Second')])");
}

#[test]
fn test_format_element_div_multiple_children_wraps_column() {
    // div → Container is a single-child widget: multiple children must be
    // hosted via an inner Column — `Container(children: [...])` is invalid Dart.
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr,
        "Container(child: Column(children: [Text('First'), Text('Second')]))"
    );
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(expr.contains("ElevatedButton("));
    assert!(expr.contains("onPressed: () { handleClick(); }"));
}

/// Handlers on VoidCallback params (onPressed/onLongPress/...) receive no
/// event argument in Flutter — a FnRef handler whose referenced function
/// uses event fields must warn about the dropped data (symmetric with the
/// InlineFn path) and still emit a parameterless closure.
#[test]
fn test_format_element_void_callback_fn_ref_warns_on_dropped_event_fields() {
    let mut events = FxHashMap::default();
    events.insert(
        "click".into(),
        EventHandlerIR::FnRef {
            r#ref: "handleClick".into(),
            params: vec![],
            event_data: Some(EventDataKind::PointerEvent),
            used_fields: Some(vec!["value".into()]),
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("onPressed: () { handleClick(); }"),
        "VoidCallback handler must be parameterless: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("VoidCallback") && w.contains("event field accesses")),
        "expected dropped-event-fields warning, got: {:?}",
        warnings
    );
}

/// A component instantiated inside a consumer-side slot projection must be
/// imported in the CONSUMER's file — the projected widget expression is
/// emitted there (regression: only the slot-holding component was
/// imported, leaving the projected component undefined in the consumer).
#[test]
fn slot_projected_component_is_imported() {
    let source = r#"
component Avatar {
    view {
        <div>AV</div>
    }
}

component Card {
    view {
        <div>
            @slot header { <h3>Default</h3> }
        </div>
    }
}

component Consumer {
    view {
        <Card>
            @slot header { <Avatar /> }
        </Card>
    }
}
"#;
    let output = compile_flutter(source).unwrap();
    let consumer: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("Consumer.dart"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        consumer.contains("import './Avatar.dart';"),
        "projected component must be imported in the consumer file, got:\n{}",
        consumer
    );
    assert!(
        consumer.contains("import './Card.dart';"),
        "slot-holding component must be imported, got:\n{}",
        consumer
    );
    assert!(
        consumer.contains("header: Avatar()"),
        "projected content must render in the consumer file, got:\n{}",
        consumer
    );
}

/// `bind:value` must satisfy the required `value:` param instead of
/// duplicating it — the required-parameter pass runs after bindings so a
/// bound param wins (regression: `Checkbox(value: false, value: active)`
/// was emitted — a duplicate named argument). The other required params
/// (Checkbox/Switch/Slider `onChanged`, Slider `value`) still get their
/// disabled defaults.
#[test]
fn bind_value_satisfies_required_value_param() {
    let source = r#"
component ToggleDemo {
    signal active: bool = false;
    signal volume: f64 = 0.5;
    view {
        <checkbox bind:value={active} />
        <switch bind:value={active} />
        <slider bind:value={volume} />
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
    assert!(
        content.contains("Checkbox(value: active, onChanged: null)"),
        "bound value must satisfy the required value param (no duplicate), got:\n{}",
        content
    );
    assert!(
        content.contains("Switch(value: active, onChanged: null)"),
        "bound value must satisfy the required value param (no duplicate), got:\n{}",
        content
    );
    assert!(
        content.contains("Slider(value: volume, onChanged: null)"),
        "bound value must satisfy the required value param (no duplicate), got:\n{}",
        content
    );
    assert!(
        !content.contains("value: false, value:") && !content.contains("value: 0.0, value:"),
        "must not emit a duplicate value argument, got:\n{}",
        content
    );
}

#[test]
fn test_format_element_with_class_map_bindings_warns_unknown_class() {
    // A dynamic class that no design declares cannot resolve to a decoration
    // — it must warn naming the class (spec §10.3) instead of silently
    // toggling an empty BoxDecoration.
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr, "Container()",
        "unresolved class must not wrap: {}",
        expr
    );
    assert!(
        !expr.contains("BoxDecoration"),
        "no empty BoxDecoration stub may be emitted: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("class 'active'") && w.contains("not declared by any design block")),
        "unknown dynamic class must warn: {:?}",
        warnings
    );
}

#[test]
fn test_format_element_with_class_map_bindings_static_true_warns_unknown_class() {
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Static true class bindings resolve like static classes; an unknown
    // class warns and adds no wrapping Container.
    assert_eq!(expr, "Container()");
    assert!(
        warnings.iter().any(|w| w.contains("class 'active'")),
        "unknown always-on class must warn: {:?}",
        warnings
    );
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Static false class bindings are skipped entirely — no warning either.
    assert_eq!(expr, "Container()");
    assert!(
        warnings.is_empty(),
        "never-applied classes must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_format_element_with_class_map_bindings_and_child_warns_unknown_class() {
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // An unresolvable dynamic class leaves the element unwrapped (warned).
    assert_eq!(
        expr, "Container(child: Text('Hello'))",
        "unresolved class must not wrap the child: {}",
        expr
    );
    assert!(
        warnings.iter().any(|w| w.contains("class 'active'")),
        "unknown dynamic class must warn: {:?}",
        warnings
    );
    assert!(
        !expr.contains("isOpen") && !expr.contains("BoxDecoration"),
        "unresolved dynamic classes must emit no conditional stub: {}",
        expr
    );
}

// ── Design class:/variant: resolution (Task 4.3) ──────────

/// Element with a `class` prop holding a literal class list.
fn el_with_class(classes: &str) -> ElementIR {
    let mut props = FxHashMap::default();
    props.insert(
        "class".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!(classes),
        },
    );
    ElementIR {
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    }
}

#[test]
fn test_design_class_resolves_to_factory_decoration() {
    // class: AppDesign.Card (lowered to the literal class prop "Card") must
    // emit `Container(decoration: AppDesign.card(), ...)` — the design doc's
    // hand-written form — NOT the old empty `BoxDecoration()` stub.
    let ir = design_test_program();
    let (expr, warnings) =
        fmt_view_with_program(&ViewTree::Element(Box::new(el_with_class("Card"))), &ir);
    assert!(
        expr.contains("Container(decoration: AppDesign.card(), child: Container())"),
        "static design class must resolve to the generated factory: {}",
        expr
    );
    assert!(
        !expr.contains("BoxDecoration(") || expr.contains("AppDesign.card()"),
        "no inline stub may replace the factory reference: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "a resolvable design class must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_design_class_dynamic_conditional_resolves_real_decoration() {
    // class: { Active: isOpen } — the dynamic class resolves to REAL values
    // (`AppDesign.active()`), toggled by the condition.
    let mut bindings = FxHashMap::default();
    bindings.insert(
        "Active".to_string(),
        ExprIR::IdentifierExpr {
            name: "isOpen".to_string(),
            type_ann: None,
        },
    );
    let mut el = el_with_class("");
    el.class_map_bindings = bindings;
    let ir = design_test_program();
    let (expr, warnings) = fmt_view_with_program(&ViewTree::Element(Box::new(el)), &ir);
    assert!(
        expr.contains("decoration: isOpen ? BoxDecoration(color: AppTokens.primary) : null"),
        "dynamic class must toggle a REAL decoration (not an empty stub): {}",
        expr
    );
    assert!(
        !expr.contains("BoxDecoration()"),
        "the empty stub is gone: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "a resolvable dynamic class must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_design_multi_class_merges_with_last_wins() {
    // class: "Card Active" — the merged decoration keeps the Card radius and
    // the Active background (later class wins on conflict).
    let ir = design_test_program();
    let (expr, _) = fmt_view_with_program(
        &ViewTree::Element(Box::new(el_with_class("Card Active"))),
        &ir,
    );
    assert!(
        expr.contains("decoration: BoxDecoration("),
        "multi-class must compose into one inline decoration: {}",
        expr
    );
    assert!(
        expr.contains("color: AppTokens.primary"),
        "the later class must win the color field: {}",
        expr
    );
    assert!(
        expr.contains("borderRadius: BorderRadius.circular(AppTokens.radiusMd)"),
        "the earlier class's radius must survive: {}",
        expr
    );
    assert!(
        !expr.contains("AppDesign.card") && !expr.contains("AppDesign.active"),
        "composed decorations inline the fields instead of referencing a single factory: {}",
        expr
    );
}

#[test]
fn test_design_variant_resolves_to_real_decoration() {
    // variant: AppDesign.Button(color=primary) — lowered to the
    // `_design_variant` prop — merges base + selected branch into a REAL
    // inline BoxDecoration.
    let mut props = FxHashMap::default();
    props.insert(
        "_design_variant".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!({
                "design_name": "App",
                "variant_name": "Button",
                "dimension_values": { "color": "primary" },
            }),
        },
    );
    let mut el = el_with_class("");
    el.props = props;
    let ir = design_test_program();
    let (expr, warnings) = fmt_view_with_program(&ViewTree::Element(Box::new(el)), &ir);
    assert!(
        expr.contains("decoration: BoxDecoration("),
        "variant must resolve to an inline decoration: {}",
        expr
    );
    assert!(
        expr.contains("color: AppTokens.primary"),
        "variant branch background must resolve: {}",
        expr
    );
    assert!(
        expr.contains("borderRadius: BorderRadius.circular(AppTokens.radiusMd)"),
        "variant base border-radius must resolve: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "a resolvable variant must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_design_static_plus_dynamic_swaps_decorations() {
    // class: "Card" + class: { Active: cond } — the decoration swaps between
    // the static set and the full set while the condition holds.
    let mut bindings = FxHashMap::default();
    bindings.insert(
        "Active".to_string(),
        ExprIR::IdentifierExpr {
            name: "isOpen".to_string(),
            type_ann: None,
        },
    );
    let mut el = el_with_class("Card");
    el.class_map_bindings = bindings;
    let ir = design_test_program();
    let (expr, _) = fmt_view_with_program(&ViewTree::Element(Box::new(el)), &ir);
    assert!(
        expr.contains("decoration: isOpen ? "),
        "static+dynamic must emit the ternary: {}",
        expr
    );
    assert!(
        expr.contains("BoxDecoration(color: AppTokens.primary, borderRadius: BorderRadius.circular(AppTokens.radiusMd))")
            && expr.contains("BoxDecoration(color: AppTokens.surface, borderRadius: BorderRadius.circular(AppTokens.radiusMd))"),
        "both branches carry REAL values: {}",
        expr
    );
}

#[test]
fn test_design_layout_only_class_warns_and_skips() {
    let ir = design_test_program();
    let (expr, warnings) = fmt_view_with_program(
        &ViewTree::Element(Box::new(el_with_class("RowLayout"))),
        &ir,
    );
    assert_eq!(
        expr, "Container()",
        "layout-only class must not wrap: {}",
        expr
    );
    assert!(
        warnings.iter().any(|w| {
            w.contains("class 'RowLayout'") && w.contains("no decoration-mappable properties")
        }),
        "layout-only class must warn: {:?}",
        warnings
    );
}

#[test]
fn test_design_variant_unknown_warns() {
    let mut props = FxHashMap::default();
    props.insert(
        "_design_variant".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!({
                "design_name": "App",
                "variant_name": "Nope",
                "dimension_values": {},
            }),
        },
    );
    let mut el = el_with_class("");
    el.props = props;
    let ir = design_test_program();
    let (expr, warnings) = fmt_view_with_program(&ViewTree::Element(Box::new(el)), &ir);
    assert_eq!(expr, "Container()");
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("variant 'Nope'") && w.contains("not found")),
        "unknown variant must warn: {:?}",
        warnings
    );
}

#[test]
fn test_design_class_on_component_element_wraps_in_decoration() {
    // class: on a component instantiation resolves to a decoration on a
    // wrapping Container — and the class prop never reaches the child
    // constructor (invalid Dart).
    let mut props = FxHashMap::default();
    props.insert(
        "class".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("Card"),
        },
    );
    let el = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "MyCard".into(),
        props,
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let ir = design_test_program();
    let my_card = ComponentIR {
        name: "MyCard".into(),
        view: None,
        ..Default::default()
    };
    let component_map: FxHashMap<&str, &ComponentIR> =
        FxHashMap::from_iter([("MyCard".into(), &my_card)]);
    let (expr, _) =
        fmt_view_with_program_and_map(&ViewTree::Element(Box::new(el)), &ir, &component_map);
    assert!(
        expr.contains("Container(decoration: AppDesign.card(), child: MyCard())"),
        "component element must be wrapped with the resolved decoration: {}",
        expr
    );
    assert!(
        !expr.contains("MyCard(class:"),
        "class must not leak into the child constructor: {}",
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("App", comp_ref, &ir, &component_map, &component_slots);
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("Counter", comp_ref, &ir, &component_map, &component_slots);
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("Greeting", comp_ref, &ir, &component_map, &component_slots);
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("App", comp_ref, &ir, &component_map, &component_slots);
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("App", comp_ref, &ir, &component_map, &component_slots);
    assert!(dart.contains("@override"));
    assert!(dart.contains("void dispose() {"));
    assert!(dart.contains("super.dispose();"));
}

// ── gen_store_dart tests ──────────────────────────────────

#[test]
fn test_store_dart_empty() {
    let codegen = FlutterCodegen;
    let ir_program = IRProgram::default();
    let store = StoreIR {
        name: "EmptyStore".into(),
        signals: FxHashMap::default(),
        signal_defaults: FxHashMap::default(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let store_dart = codegen.gen_store_dart("EmptyStore", &store, &ir_program);
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
    let ir_program = IRProgram::default();
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
    let store_dart = codegen.gen_store_dart("AppStore", &store, &ir_program);
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
    let ir_program = IRProgram::default();
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
    let store_dart = codegen.gen_store_dart("CalcStore", &store, &ir_program);
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("App", comp_ref, &ir, &component_map, &component_slots);

    // Field declarations
    assert!(
        dart.contains("late final AnimationController _fadeInController;"),
        "should declare _fadeInController field"
    );
    assert!(
        dart.contains("late final Animation<double> _fadeInOpacity;"),
        "should declare per-property _fadeInOpacity field: {}",
        dart
    );
    assert!(
        dart.contains("bool _fadeInActive = false;"),
        "should declare the motion active flag: {}",
        dart
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

    // Per-property Tween + Curve
    assert!(
        dart.contains("_fadeInOpacity = Tween<double>(begin: 0.0, end: 1.0).chain(CurveTween(curve: Curves.easeOut)).animate(_fadeInController);"),
        "should create per-property Tween: {}",
        dart
    );

    // @enter → activate + forward() on mount
    assert!(
        dart.contains("_fadeInActive = true;"),
        "should activate the motion on mount: {}",
        dart
    );
    assert!(
        dart.contains("_fadeInController.forward();"),
        "should call forward() for @enter trigger: {}",
        dart
    );

    // AnimatedBuilder wrapping in build method — listens to the shared
    // controller, reads per-property animation values, and gates the
    // wrappers on the active flag (natural style while inactive)
    assert!(
        dart.contains("AnimatedBuilder("),
        "should emit AnimatedBuilder in build: {}",
        dart
    );
    assert!(
        dart.contains("animation: _fadeInController"),
        "AnimatedBuilder should listen to the shared controller: {}",
        dart
    );
    assert!(
        dart.contains(
            "_fadeInActive ? Opacity(opacity: _fadeInOpacity.value, child: child) : child ?? const SizedBox.shrink()"
        ),
        "builder body should gate per-property animation value on the active flag: {}",
        dart
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
        result.contains(
            "onEnter: (_) { _scaleOnHoverActive = true; _scaleOnHoverController.forward(); }"
        ),
        "should activate and forward on mouse enter: {}",
        result
    );
    assert!(
        result.contains("onExit: (_) { _scaleOnHoverController.reverse().whenComplete(() { if (mounted && _scaleOnHoverController.value == 0.0) setState(() { _scaleOnHoverActive = false; }); }); }"),
        "should reverse and deactivate on mouse exit: {}",
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("App", comp_ref, &ir, &component_map, &component_slots);

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
    // — per-property animations drive each wrapper independently
    assert!(
        dart.contains("AnimatedBuilder("),
        "should emit AnimatedBuilder: {}",
        dart
    );
    assert!(
        dart.contains("Transform.scale(scale: _popInScale.value"),
        "should wrap with Transform.scale: {}",
        dart
    );
    assert!(
        dart.contains("Opacity(opacity: _popInOpacity.value"),
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

    // Bare numeric string "0.5" → seconds semantics (CSS duration): 500ms
    // (previously emitted invalid `Duration(milliseconds: 0.5)`)
    assert_eq!(
        extract_duration_ms(&ExprIR::LiteralExpr {
            value: serde_json::json!("0.5")
        }),
        "500"
    );

    // Fractional millisecond values round to an int — Dart's
    // `Duration(milliseconds:)` takes an `int`, so `250.5` would not compile.
    assert_eq!(
        extract_duration_ms(&ExprIR::LiteralExpr {
            value: serde_json::json!("250.5ms")
        }),
        "251"
    );

    // Garbage → default fallback "300"
    assert_eq!(
        extract_duration_ms(&ExprIR::LiteralExpr {
            value: serde_json::json!("not-a-duration")
        }),
        "300"
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

/// Build a KeyframeIR from (position, property → json value) triples.
fn test_keyframe(stops: &[(f64, &[(&str, serde_json::Value)])]) -> KeyframeIR {
    KeyframeIR {
        stops: stops
            .iter()
            .map(|(pos, props)| KeyframeStopIR {
                position: *pos,
                properties: FxHashMap::from_iter(
                    props
                        .iter()
                        .map(|(k, v)| (k.to_string(), ExprIR::LiteralExpr { value: v.clone() })),
                ),
                ease: None,
                source_span: None,
            })
            .collect(),
        over: ExprIR::LiteralExpr {
            value: serde_json::json!("300ms"),
        },
        ease: None,
        playback: PlaybackIR::default(),
        source_span: None,
    }
}

#[test]
fn test_build_prop_tween_expr_two_stops() {
    // 2 stops → plain Tween
    let kf = test_keyframe(&[
        (0.0, &[("opacity", serde_json::json!(0.0))]),
        (1.0, &[("opacity", serde_json::json!(1.0))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", false),
        "Tween<double>(begin: 0.0, end: 1.0)"
    );
}

#[test]
fn test_build_prop_tween_expr_three_stops_sequence() {
    // 3 stops → TweenSequence with weight-proportional segments (50/50)
    let kf = test_keyframe(&[
        (0.0, &[("opacity", serde_json::json!(0.0))]),
        (0.5, &[("opacity", serde_json::json!(0.5))]),
        (1.0, &[("opacity", serde_json::json!(1.0))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", false),
        "TweenSequence<double>([TweenSequenceItem(tween: Tween<double>(begin: 0.0, end: 0.5), weight: 50), \
         TweenSequenceItem(tween: Tween<double>(begin: 0.5, end: 1.0), weight: 50)])"
    );
}

#[test]
fn test_build_prop_tween_expr_nonuniform_weights() {
    // Stops at 0/0.25/1 → weights proportional to spans (25/75)
    let kf = test_keyframe(&[
        (0.0, &[("opacity", serde_json::json!(0.0))]),
        (0.25, &[("opacity", serde_json::json!(0.5))]),
        (1.0, &[("opacity", serde_json::json!(1.0))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", false),
        "TweenSequence<double>([TweenSequenceItem(tween: Tween<double>(begin: 0.0, end: 0.5), weight: 25), \
         TweenSequenceItem(tween: Tween<double>(begin: 0.5, end: 1.0), weight: 75)])"
    );
}

#[test]
fn test_build_prop_tween_expr_boundary_synthesis() {
    // To-only (single stop at 1.0): begin defaults to the natural value
    // (opacity is naturally fully visible — 1.0, not 0.0)
    let kf = test_keyframe(&[(1.0, &[("opacity", serde_json::json!(1.0))])]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", false),
        "Tween<double>(begin: 1.0, end: 1.0)"
    );
    // To-only scale animates from the natural scale 1.0
    let kf = test_keyframe(&[(1.0, &[("scale", serde_json::json!(1.05))])]);
    assert_eq!(
        build_prop_tween_expr(&kf, "scale", false),
        "Tween<double>(begin: 1.0, end: 1.05)"
    );
    // To-only rotate interpolates from the natural rotation 0.0
    let kf = test_keyframe(&[(1.0, &[("rotate", serde_json::json!(90))])]);
    assert_eq!(
        build_prop_tween_expr(&kf, "rotate", false),
        "Tween<double>(begin: 0.0, end: 1.5707963267948966)"
    );

    // From-only (single stop at 0.0): end defaults to the natural value
    // (opacity naturally fully visible — 1.0)
    let kf = test_keyframe(&[(0.0, &[("opacity", serde_json::json!(0.0))])]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", false),
        "Tween<double>(begin: 0.0, end: 1.0)"
    );
    // From-only translate returns to the natural offset 0.0
    let kf = test_keyframe(&[(0.0, &[("translateX", serde_json::json!(20))])]);
    assert_eq!(
        build_prop_tween_expr(&kf, "translateX", false),
        "Tween<double>(begin: 20.0, end: 0.0)"
    );

    // Partial boundaries (0% and 50% only) → synthesized 100% end becomes a
    // third TweenSequence point
    let kf = test_keyframe(&[
        (0.0, &[("opacity", serde_json::json!(0.0))]),
        (0.5, &[("opacity", serde_json::json!(1.0))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", false),
        "TweenSequence<double>([TweenSequenceItem(tween: Tween<double>(begin: 0.0, end: 1.0), weight: 50), \
         TweenSequenceItem(tween: Tween<double>(begin: 1.0, end: 1.0), weight: 50)])"
    );
}

#[test]
fn test_build_prop_tween_expr_color() {
    // Color property → Tween<Color> with css_color_to_dart conversions
    let kf = test_keyframe(&[
        (0.0, &[("backgroundColor", serde_json::json!("#ffffff"))]),
        (1.0, &[("backgroundColor", serde_json::json!("#ff0000"))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "backgroundColor", false),
        "Tween<Color>(begin: Color(0xFFFFFFFF), end: Color(0xFFFF0000))"
    );
}

#[test]
fn test_build_prop_tween_expr_border_radius() {
    // borderRadius → BorderRadiusTween (Flutter SDK tween over
    // BorderRadius?, lerped via BorderRadius.lerp) — Task 5.2 semantic set.
    let kf = test_keyframe(&[
        (0.0, &[("borderRadius", serde_json::json!(8))]),
        (1.0, &[("borderRadius", serde_json::json!(16))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "borderRadius", false),
        "BorderRadiusTween(begin: BorderRadius.all(Radius.circular(8.0)), \
         end: BorderRadius.all(Radius.circular(16.0)))"
    );
    // Unit-bearing string values strip the px unit.
    let kf = test_keyframe(&[
        (0.0, &[("borderRadius", serde_json::json!("8px"))]),
        (1.0, &[("borderRadius", serde_json::json!("16px"))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "borderRadius", false),
        "BorderRadiusTween(begin: BorderRadius.all(Radius.circular(8.0)), \
         end: BorderRadius.all(Radius.circular(16.0)))"
    );
    // 3 stops → TweenSequence of BorderRadiusTween items.
    let kf = test_keyframe(&[
        (0.0, &[("borderRadius", serde_json::json!(0))]),
        (0.5, &[("borderRadius", serde_json::json!(8))]),
        (1.0, &[("borderRadius", serde_json::json!(16))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "borderRadius", false),
        "TweenSequence<BorderRadius?>([TweenSequenceItem(tween: \
         BorderRadiusTween(begin: BorderRadius.all(Radius.circular(0.0)), \
         end: BorderRadius.all(Radius.circular(8.0))), weight: 50), \
         TweenSequenceItem(tween: BorderRadiusTween(begin: \
         BorderRadius.all(Radius.circular(8.0)), end: \
         BorderRadius.all(Radius.circular(16.0))), weight: 50)])"
    );
}

#[test]
fn test_build_prop_tween_expr_font_size() {
    // fontSize → Tween<double> applied through TextStyle(fontSize: ...) —
    // the per-property equivalent of TextStyle.lerp (Task 5.2).
    let kf = test_keyframe(&[
        (0.0, &[("fontSize", serde_json::json!(12))]),
        (1.0, &[("fontSize", serde_json::json!(16))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "fontSize", false),
        "Tween<double>(begin: 12.0, end: 16.0)"
    );
    // Unit-bearing string values strip the px unit.
    let kf = test_keyframe(&[
        (0.0, &[("fontSize", serde_json::json!("12px"))]),
        (1.0, &[("fontSize", serde_json::json!("16px"))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "fontSize", false),
        "Tween<double>(begin: 12.0, end: 16.0)"
    );
}

#[test]
fn test_build_prop_tween_expr_per_property_isolation() {
    // A property present at only two of three stops interpolates over its own
    // span — other properties' extra stops don't affect it.
    let kf = test_keyframe(&[
        (0.0, &[("opacity", serde_json::json!(0.0))]),
        (0.5, &[("scale", serde_json::json!(0.5))]),
        (
            1.0,
            &[
                ("opacity", serde_json::json!(1.0)),
                ("scale", serde_json::json!(1.0)),
            ],
        ),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", false),
        "Tween<double>(begin: 0.0, end: 1.0)"
    );
    // scale's missing 0% boundary synthesizes its natural value 1.0
    assert_eq!(
        build_prop_tween_expr(&kf, "scale", false),
        "TweenSequence<double>([TweenSequenceItem(tween: Tween<double>(begin: 1.0, end: 0.5), weight: 50), \
         TweenSequenceItem(tween: Tween<double>(begin: 0.5, end: 1.0), weight: 50)])"
    );
}

#[test]
fn test_build_prop_tween_expr_reverse() {
    // `direction: reverse` + loop → swapped Tween direction (Flutter can
    // only repeat forward passes)
    let kf = test_keyframe(&[
        (0.0, &[("opacity", serde_json::json!(0.0))]),
        (1.0, &[("opacity", serde_json::json!(1.0))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", true),
        "Tween<double>(begin: 1.0, end: 0.0)"
    );

    // TweenSequence: segments reverse and swap begin/end
    let kf = test_keyframe(&[
        (0.0, &[("opacity", serde_json::json!(0.0))]),
        (0.5, &[("opacity", serde_json::json!(0.5))]),
        (1.0, &[("opacity", serde_json::json!(1.0))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "opacity", true),
        "TweenSequence<double>([TweenSequenceItem(tween: Tween<double>(begin: 1.0, end: 0.5), weight: 50), \
         TweenSequenceItem(tween: Tween<double>(begin: 0.5, end: 0.0), weight: 50)])"
    );
}

#[test]
fn test_build_prop_tween_expr_rotate_radians() {
    // rotate is declared in degrees but Transform.rotate takes radians:
    // 45deg → 0.7853981633974483, 360deg → 6.283185307179586
    let kf = test_keyframe(&[
        (0.0, &[("rotate", serde_json::json!(0))]),
        (1.0, &[("rotate", serde_json::json!(360))]),
    ]);
    assert_eq!(
        build_prop_tween_expr(&kf, "rotate", false),
        "Tween<double>(begin: 0.0, end: 6.283185307179586)"
    );
}

#[test]
fn test_prop_anim_field_naming() {
    assert_eq!(prop_anim_field("fadeIn", "opacity"), "_fadeInOpacity");
    assert_eq!(prop_anim_field("fadeIn", "scale"), "_fadeInScale");
    assert_eq!(prop_anim_field("fadeIn", "translateX"), "_fadeInTranslateX");
    assert_eq!(
        prop_anim_field("fadeIn", "backgroundColor"),
        "_fadeInBackgroundColor"
    );
    assert_eq!(prop_anim_type("opacity"), "double");
    assert_eq!(prop_anim_type("backgroundColor"), "Color");
}

#[test]
fn test_emit_playback_start_default() {
    // Default fill (forwards-like): activate at play start, no reset
    let pb = PlaybackIR::default();
    assert_eq!(
        emit_playback_start("_fadeInController", Some("_fadeInActive"), &pb),
        vec!["_fadeInActive = true; _fadeInController.forward();"]
    );
}

#[test]
fn test_emit_playback_start_loop() {
    let pb = PlaybackIR {
        loop_count: Some(LoopValue::Infinite),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true;", "_c.repeat();"]
    );

    let pb = PlaybackIR {
        loop_count: Some(LoopValue::Count(3)),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true;", "_c.repeat(count: 3);"]
    );

    // loop + alternate → reversing repeat (WAAPI: F,R,F,R)
    let pb = PlaybackIR {
        loop_count: Some(LoopValue::Infinite),
        direction: Some(DirectionValue::Alternate),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true;", "_c.repeat(reverse: true);"]
    );

    // loop + alternate-reverse → preset to 1.0 so the first pass is reversed
    // (WAAPI: R,F,R,F)
    let pb = PlaybackIR {
        loop_count: Some(LoopValue::Infinite),
        direction: Some(DirectionValue::AlternateReverse),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec![
            "_cActive = true;",
            "_c.value = 1.0;",
            "_c.repeat(reverse: true);"
        ]
    );
}

#[test]
fn test_emit_playback_start_direction() {
    // WAAPI with iterations=1: reverse/alternate-reverse play a single
    // reversed pass; alternate plays a single forward pass
    let pb = PlaybackIR {
        direction: Some(DirectionValue::Reverse),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true; _c.reverse();"]
    );

    let pb = PlaybackIR {
        direction: Some(DirectionValue::Alternate),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true; _c.forward();"]
    );

    let pb = PlaybackIR {
        direction: Some(DirectionValue::AlternateReverse),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true; _c.reverse();"]
    );
}

#[test]
fn test_emit_playback_start_fill() {
    // forwards/both (and default) hold the end value — no reset
    let pb = PlaybackIR {
        fill: Some(FillValue::Forwards),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true; _c.forward();"]
    );

    // none/backwards → deactivate the animated wrapper on completion so the
    // element returns to its natural style (NOT `value = 0` — that would
    // leave a fade-in permanently invisible)
    let pb = PlaybackIR {
        fill: Some(FillValue::None),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["_cActive = true; _c.forward().whenComplete(() { if (mounted && _c.value == 1.0) setState(() { _cActive = false; }); });"]
    );
}

#[test]
fn test_emit_playback_start_no_wrappers() {
    // Motions with no supported properties (e.g. skew-only) have no active
    // flag — playback statements must not reference one.
    let pb = PlaybackIR::default();
    assert_eq!(emit_playback_start("_c", None, &pb), vec!["_c.forward();"]);
    let pb = PlaybackIR {
        fill: Some(FillValue::None),
        ..Default::default()
    };
    assert_eq!(emit_playback_start("_c", None, &pb), vec!["_c.forward();"]);
    let pb = PlaybackIR {
        loop_count: Some(LoopValue::Infinite),
        ..Default::default()
    };
    assert_eq!(emit_playback_start("_c", None, &pb), vec!["_c.repeat();"]);
}

#[test]
fn test_emit_playback_start_delay() {
    // delay → Future.delayed with a mounted guard; activation inside the
    // callback (natural style during the delay)
    let pb = PlaybackIR {
        delay: Some(ExprIR::LiteralExpr {
            value: serde_json::json!("500ms"),
        }),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["Future.delayed(Duration(milliseconds: 500), () { if (mounted) { _cActive = true; _c.forward(); } });"]
    );

    // delay + fill backwards → active immediately (begin state during the
    // delay), deactivated after completion
    let pb = PlaybackIR {
        fill: Some(FillValue::Backwards),
        delay: Some(ExprIR::LiteralExpr {
            value: serde_json::json!("500ms"),
        }),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec![
            "_cActive = true;",
            "Future.delayed(Duration(milliseconds: 500), () { if (mounted) { _c.forward().whenComplete(() { if (mounted && _c.value == 1.0) setState(() { _cActive = false; }); }); } });"
        ]
    );

    // delay + loop
    let pb = PlaybackIR {
        loop_count: Some(LoopValue::Infinite),
        delay: Some(ExprIR::LiteralExpr {
            value: serde_json::json!("1s"),
        }),
        ..Default::default()
    };
    assert_eq!(
        emit_playback_start("_c", Some("_cActive"), &pb),
        vec!["Future.delayed(Duration(milliseconds: 1000), () { if (mounted) { _cActive = true; _c.repeat(); } });"]
    );
}

#[test]
fn test_easing_flutter_diagnostic() {
    assert!(easing_flutter_diagnostic("m", &Some(EasingIR::Named("easeOut".into()))).is_none());
    assert!(easing_flutter_diagnostic(
        "m",
        &Some(EasingIR::CubicBezier {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0
        })
    )
    .is_none());
    // steps(N,dir) strings lower from steps() easing — no Flutter equivalent
    let diag = easing_flutter_diagnostic("m", &Some(EasingIR::Named("steps(6,end)".into())));
    assert!(diag.is_some());
    assert!(diag.unwrap().contains("steps()"));
    // Named step aliases (stepStart/stepEnd lower to steps(1,...))
    assert!(easing_flutter_diagnostic("m", &Some(EasingIR::Named("stepEnd".into()))).is_some());
}

#[test]
fn test_prop_flutter_diagnostic() {
    assert!(prop_flutter_diagnostic("m", "opacity").is_none());
    assert!(prop_flutter_diagnostic("m", "translateX").is_none());

    let diag = prop_flutter_diagnostic("m", "width").unwrap();
    assert!(diag.contains("layout-dependent"), "width diag: {}", diag);

    let diag = prop_flutter_diagnostic("m", "skewX").unwrap();
    assert!(diag.contains("skew"), "skew diag: {}", diag);

    let diag = prop_flutter_diagnostic("m", "blur").unwrap();
    assert!(
        diag.contains("unsupported property 'blur'"),
        "unknown diag: {}",
        diag
    );
}

#[test]
fn test_build_animated_builder_body_per_property() {
    let motion = MotionIntent {
        name: "popIn".into(),
        body: test_keyframe(&[
            (
                0.0,
                &[
                    ("opacity", serde_json::json!(0.0)),
                    ("translateX", serde_json::json!(-50)),
                    ("rotate", serde_json::json!(0)),
                ],
            ),
            (
                1.0,
                &[
                    ("opacity", serde_json::json!(1.0)),
                    ("translateX", serde_json::json!(0)),
                    ("rotate", serde_json::json!(45)),
                ],
            ),
        ]),
        source_span: None,
    };
    let body = build_animated_builder_body(&motion, "popIn");
    assert!(
        body.contains("Opacity(opacity: _popInOpacity.value"),
        "opacity wrapper: {}",
        body
    );
    assert!(
        body.contains("Transform.translate(offset: Offset(_popInTranslateX.value, 0.0)"),
        "translate wrapper: {}",
        body
    );
    assert!(
        body.contains("Transform.rotate(angle: _popInRotate.value"),
        "rotate wrapper: {}",
        body
    );
}

#[test]
fn test_build_animated_builder_body_color_and_size() {
    let motion = MotionIntent {
        name: "tint".into(),
        body: test_keyframe(&[
            (
                0.0,
                &[
                    ("backgroundColor", serde_json::json!("#ffffff")),
                    ("width", serde_json::json!(100)),
                    ("color", serde_json::json!("#000000")),
                ],
            ),
            (
                1.0,
                &[
                    ("backgroundColor", serde_json::json!("#ff0000")),
                    ("width", serde_json::json!(200)),
                    ("color", serde_json::json!("#ffffff")),
                ],
            ),
        ]),
        source_span: None,
    };
    let body = build_animated_builder_body(&motion, "tint");
    assert!(
        body.contains("Container(color: _tintBackgroundColor.value"),
        "backgroundColor wrapper: {}",
        body
    );
    assert!(
        body.contains("DefaultTextStyle(style: TextStyle(color: _tintColor.value)"),
        "color wrapper: {}",
        body
    );
    assert!(
        body.contains("SizedBox(width: _tintWidth.value"),
        "width wrapper: {}",
        body
    );
}

#[test]
fn test_build_animated_builder_body_border_radius_and_font_size() {
    // Task 5.2 semantic set: borderRadius → Container with BoxDecoration,
    // fontSize → DefaultTextStyle (text-style family, same wrapper as color).
    let motion = MotionIntent {
        name: "soften".into(),
        body: test_keyframe(&[
            (
                0.0,
                &[
                    ("borderRadius", serde_json::json!(8)),
                    ("fontSize", serde_json::json!(12)),
                ],
            ),
            (
                1.0,
                &[
                    ("borderRadius", serde_json::json!(16)),
                    ("fontSize", serde_json::json!(16)),
                ],
            ),
        ]),
        source_span: None,
    };
    let body = build_animated_builder_body(&motion, "soften");
    assert!(
        body.contains(
            "Container(decoration: BoxDecoration(borderRadius: _softenBorderRadius.value)"
        ),
        "borderRadius wrapper: {}",
        body
    );
    assert!(
        body.contains("clipBehavior: Clip.antiAlias"),
        "borderRadius wrapper must clip the child so the animated radius is \
         visible on a container with a background (a bare BoxDecoration with \
         no color paints nothing — the motion would be visually inert): {}",
        body
    );
    assert!(
        body.contains("DefaultTextStyle(style: TextStyle(fontSize: _softenFontSize.value"),
        "fontSize wrapper: {}",
        body
    );
    assert!(
        body.contains("_softenActive ?"),
        "wrappers gated on the active flag: {}",
        body
    );
    // Field types: BorderRadius? (BorderRadiusTween) / double (Tween<double>).
    assert_eq!(prop_anim_type("borderRadius"), "BorderRadius?");
    assert_eq!(prop_anim_type("fontSize"), "double");
    assert!(is_supported_motion_prop("borderRadius"));
    assert!(is_supported_motion_prop("fontSize"));
    assert!(prop_flutter_diagnostic("soften", "borderRadius").is_none());
    assert!(prop_flutter_diagnostic("soften", "fontSize").is_none());

    // fontSize alone → DefaultTextStyle must coalesce the builder's nullable
    // `child` (DefaultTextStyle.child is a required non-nullable Widget) —
    // the dart analyze gate catches this in generated output.
    let motion = MotionIntent {
        name: "growText".into(),
        body: test_keyframe(&[
            (0.0, &[("fontSize", serde_json::json!(12))]),
            (1.0, &[("fontSize", serde_json::json!(16))]),
        ]),
        source_span: None,
    };
    let body = build_animated_builder_body(&motion, "growText");
    assert_eq!(
        body,
        "_growTextActive ? DefaultTextStyle(style: TextStyle(fontSize: _growTextFontSize.value), \
         child: child ?? const SizedBox.shrink()) : child ?? const SizedBox.shrink()"
    );
}

#[test]
fn test_build_animated_builder_body_no_wrappers() {
    // Motion whose only property is unsupported (skew) must still produce a
    // Widget-typed body — the builder closure's `child` is `Widget?`.
    let motion = MotionIntent {
        name: "skew".into(),
        body: test_keyframe(&[
            (0.0, &[("skewX", serde_json::json!(0))]),
            (1.0, &[("skewX", serde_json::json!(45))]),
        ]),
        source_span: None,
    };
    assert_eq!(
        build_animated_builder_body(&motion, "skew"),
        "child ?? const SizedBox.shrink()"
    );
}

#[test]
fn test_motion_pipeline_warnings_and_playback() {
    // Full pipeline: motion with steps() easing + width property + loop +
    // delay must produce warnings and playback honoring loop/delay.
    let codegen = FlutterCodegen;
    let motion = MotionIntent {
        name: "pulse".into(),
        body: KeyframeIR {
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
                            "width".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(100),
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
                            "width".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(200),
                            },
                        ),
                    ]),
                    ease: None,
                    source_span: None,
                },
            ],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!("300ms"),
            },
            ease: Some(EasingIR::Named("steps(6,end)".into())),
            playback: PlaybackIR {
                loop_count: Some(LoopValue::Infinite),
                delay: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!("1s"),
                }),
                ..Default::default()
            },
            source_span: None,
        },
        source_span: None,
    };

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
                motion_ref: Some("pulse".into()),
                inline_motion: None,
                source_span: None,
            }],
            ..Default::default()
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, warnings, _errors) =
        codegen.gen_component_dart("App", comp_ref, &ir, &component_map, &component_slots);

    // steps() easing → diagnostic warning
    assert!(
        warnings.iter().any(|w| w.contains("steps() easing")),
        "steps() warning: {:?}",
        warnings
    );
    // width → layout-dependent warning
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("'width'") && w.contains("layout-dependent")),
        "width warning: {:?}",
        warnings
    );

    // Per-property animations: opacity Tween + width Tween
    assert!(
        dart.contains("_pulseOpacity = Tween<double>(begin: 0.0, end: 1.0)"),
        "opacity tween: {}",
        dart
    );
    assert!(
        dart.contains("_pulseWidth = Tween<double>(begin: 100.0, end: 200.0)"),
        "width tween: {}",
        dart
    );

    // steps() falls back to Curves.linear
    assert!(
        dart.contains("CurveTween(curve: Curves.linear)"),
        "steps fallback: {}",
        dart
    );

    // Playback: loop + delay → delayed repeat (mounted-guarded)
    assert!(
        dart.contains(
            "Future.delayed(Duration(milliseconds: 1000), () { if (mounted) { _pulseActive = true; _pulseController.repeat(); } });"
        ),
        "delayed repeat: {}",
        dart
    );

    // Builder body wraps both properties
    assert!(
        dart.contains("SizedBox(width: _pulseWidth.value"),
        "width wrapper: {}",
        dart
    );
}

#[test]
fn test_store_dart_with_action() {
    let codegen = FlutterCodegen;
    let ir_program = IRProgram::default();
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
    let store_dart = codegen.gen_store_dart("AppStore", &store, &ir_program);
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

/// Task 4.3 fixture: a component using `class: Design.Card`, a dynamic
/// `class: { Active: cond }`, and `variant: Design.Button(color=primary)` —
/// the Flutter output must carry REAL decoration values (the generated
/// `AppDesign` factory / token constants), never the old empty
/// `BoxDecoration()` stub, and the component must import design.dart.
#[test]
fn design_classes_and_variants_render_real_flutter_decorations() {
    let source = r##"
import omniview;

design App {
    token primary: color = "#6366f1";
    token surface: color = "#ffffff";
    token radiusMd: radius = "8px";

    class Card {
        background: token(surface);
        border-radius: token(radiusMd);
        border: "1px solid #d4d4d8";
    }
    class Active {
        background: token(primary);
    }

    variant Button {
        @dim color: primary | danger;
        @default { color: primary; }
        base {
            border-radius: token(radiusMd);
        }
        color.primary { background: token(primary); }
        color.danger { background: "#ef4444"; }
    }
}

component DesignDemo {
    signal isActive: bool = false;
    view {
        <div class: App.Card>
            <div class: { Active: isActive }>Toggle</div>
        </div>
        <button variant: App.Button(color=primary)>Go</button>
    }
}
"##;
    let output = compile_flutter(source).unwrap();
    let dart: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    // Static class → the generated factory, exactly as the design doc
    // promises (`Container(decoration: AppDesign.card(), ...)`).
    assert!(
        dart.contains("Container(decoration: AppDesign.card(), child: Container("),
        "static design class must resolve to the AppDesign factory. Got:\n{}",
        dart
    );
    // Dynamic class → REAL values toggled by the condition.
    assert!(
        dart.contains("decoration: isActive ? BoxDecoration(color: AppTokens.primary) : null"),
        "dynamic class must toggle real decoration values. Got:\n{}",
        dart
    );
    // Variant → merged base + branch values.
    assert!(
        dart.contains("decoration: BoxDecoration(color: AppTokens.primary, borderRadius: BorderRadius.circular(AppTokens.radiusMd))"),
        "variant must resolve to real values. Got:\n{}",
        dart
    );
    // No empty stub, no TODO, no leftover `class`/`_design_variant` props.
    assert!(
        !dart.contains("BoxDecoration()"),
        "the empty BoxDecoration stub is gone. Got:\n{}",
        dart
    );
    assert!(
        !dart.contains("TODO"),
        "the TODO comment is gone. Got:\n{}",
        dart
    );
    assert!(
        !dart.contains("_design_variant") && !dart.contains(": 'Card'"),
        "design props must not leak into generated widgets. Got:\n{}",
        dart
    );
    // The component imports design.dart (the factories it references).
    let demo = output
        .files
        .get("DesignDemo.dart")
        .expect("DesignDemo.dart should be generated");
    assert!(
        demo.contains("import 'design.dart';"),
        "components using design classes must import design.dart. Got:\n{}",
        demo
    );
    // design.dart itself contains the factory.
    let design_dart = output
        .files
        .get("design.dart")
        .expect("design.dart should be generated");
    assert!(
        design_dart.contains("static BoxDecoration card() {")
            && design_dart.contains("color: AppTokens.surface,"),
        "design.dart must contain the card factory. Got:\n{}",
        design_dart
    );
}

/// Full-pipeline motion coverage (Task 5.4): real @motion syntax through
/// parse → IR → Flutter codegen. Verifies per-property Tween/TweenSequence,
/// playback properties, color mapping, and motion diagnostics.
#[test]
fn motion_full_pipeline_per_property_and_playback() {
    let source = r##"
import omniview;

@motion enterFadeUp {
    from { opacity: 0, translateY: 20 }
    to   { opacity: 1, translateY: 0 }
    over: 500, easing: power2Out
}

@motion multiStopDemo {
    0%   { opacity: 0, scale: 0.5, backgroundColor: "#6366f1" }
    50%  { opacity: 1, scale: 1.2, backgroundColor: "#10b981" }
    100% { opacity: 1, scale: 1, backgroundColor: "#f59e0b" }
    over: 1200, easing: easeInOut
}

@motion spinLoop {
    to { rotate: 360 }
    over: 2000, easing: linear
    loop
}

@motion fillBackwards {
    from { opacity: 0 }
    to   { opacity: 1 }
    over: 600, easing: power2Out
    delay 500
    fill backwards
}

@motion stepsMotion {
    from { opacity: 0 }
    to   { opacity: 1 }
    over: 600, easing: steps(6, end)
}

component MotionCheck {
    view {
        <div>
            <div @enter={enterFadeUp}>hello</div>
            <div @enter={multiStopDemo}>multi</div>
            <div @enter={spinLoop}>spin</div>
            <div @enter={fillBackwards}>fill</div>
            <div @enter={stepsMotion}>steps</div>
        </div>
    }
}
    "##;
    let output = compile_flutter(source).expect("motion pipeline compile failed");
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    // Per-property Animation fields (lowerCamelCase, no underscores mid-name)
    assert!(
        content.contains("late final Animation<double> _enterFadeUpOpacity;"),
        "opacity field: {}",
        content
    );
    assert!(
        content.contains("late final Animation<double> _enterFadeUpTranslateY;"),
        "translate field: {}",
        content
    );
    assert!(
        content.contains("late final Animation<Color> _multiStopDemoBackgroundColor;"),
        "color field: {}",
        content
    );

    // 2 stops → Tween; 3 stops → weight-proportional TweenSequence
    assert!(
        content.contains("_enterFadeUpOpacity = Tween<double>(begin: 0.0, end: 1.0)"),
        "2-stop tween: {}",
        content
    );
    assert!(
        content.contains("_multiStopDemoOpacity = TweenSequence<double>(["),
        "3-stop sequence: {}",
        content
    );
    assert!(
        content.contains("weight: 50)"),
        "weight-proportional items: {}",
        content
    );
    assert!(
        content.contains("TweenSequence<Color>(["),
        "color sequence: {}",
        content
    );
    // Color tweens use CurvedAnimation — CurveTween is Animatable<double>
    // and cannot chain onto Tween<Color>
    assert!(
        content.contains("]).animate(CurvedAnimation(parent: _multiStopDemoController, curve: Curves.easeInOut));"),
        "color CurvedAnimation: {}",
        content
    );
    // rotate is declared in degrees → emitted as radians (360deg = 2π)
    assert!(
        content.contains("_spinLoopRotate = Tween<double>(begin: 0.0, end: 6.283185307179586)"),
        "rotate radians: {}",
        content
    );

    // Playback: loop → repeat(); delay+fill backwards → active immediately,
    // delayed forward, deactivated (natural style) on completion
    assert!(
        content.contains("_spinLoopController.repeat();"),
        "loop: {}",
        content
    );
    assert!(
        content.contains("_fillBackwardsActive = true;"),
        "fill backwards activation: {}",
        content
    );
    assert!(content.contains("Future.delayed(Duration(milliseconds: 500), () { if (mounted) { _fillBackwardsController.forward().whenComplete(() { if (mounted && _fillBackwardsController.value == 1.0) setState(() { _fillBackwardsActive = false; }); }); } });"), "delay+fill: {}", content);

    // Builder body: per-property wrappers gated on the active flag
    assert!(
        content.contains("_enterFadeUpActive ? Transform.translate(offset: Offset(0.0, _enterFadeUpTranslateY.value), child: Opacity(opacity: _enterFadeUpOpacity.value, child: child)) : child ?? const SizedBox.shrink()"),
        "enter body: {}",
        content
    );
    assert!(content.contains("_multiStopDemoActive ? Container(color: _multiStopDemoBackgroundColor.value, child: Transform.scale(scale: _multiStopDemoScale.value, child: Opacity(opacity: _multiStopDemoOpacity.value, child: child))) : child ?? const SizedBox.shrink()"), "multi-property body: {}", content);

    // Diagnostics: steps() easing warning
    assert!(
        output
            .warnings
            .iter()
            .any(|w| w.message.contains("steps() easing")),
        "steps warning: {:?}",
        output.warnings
    );
}

#[test]
fn slot_unnamed_emits_children_param() {
    let source = r#"
component Card {
    view {
        <div class="card">@slot</div>
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
    assert!(
        content.contains("final Widget? children;"),
        "Expected children slot field, got:\n{}",
        content
    );
    assert!(
        content.contains("widget.children ?? SizedBox.shrink()"),
        "Expected widget.children render with a nullable-guarding fallback \
         (widget.children is Widget?; a bare entry would be invalid in \
         List<Widget> contexts), got:\n{}",
        content
    );
    assert!(
        !output.warnings.iter().any(|w| w.message.contains("slot")),
        "Slot warning should be gone, got: {:?}",
        output.warnings
    );
}

/// Named slot with fallback: `final Widget? header;` + `widget.header ?? fallback`.
#[test]
fn slot_named_emits_param_with_fallback() {
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
    let output = compile_flutter(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("final Widget? header;"),
        "Expected header slot field, got:\n{}",
        content
    );
    assert!(
        content.contains("this.header,"),
        "Expected header constructor param, got:\n{}",
        content
    );
    assert!(
        content.contains("widget.header ?? Text('Default Header')"),
        "Expected fallback-wrapped header render, got:\n{}",
        content
    );
}

/// `@slot header: String { ... }` types the param as `String?` instead of `Widget?`.
#[test]
fn slot_type_constraint_defers_to_widget_with_warning() {
    let source = r#"
component Badge {
    view {
        <div>
            @slot header: String { <span>Fallback</span> }
        </div>
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
    // Typed slots are deferred: the param stays `Widget?` so the
    // `widget.header ?? <fallback widget>` render stays statically valid.
    assert!(
        content.contains("final Widget? header;"),
        "Expected Widget? header param (type constraint deferred), got:\n{}",
        content
    );
    assert!(
        !content.contains("final String? header;"),
        "Must not emit a String-typed slot field, got:\n{}",
        content
    );
    assert!(
        output
            .warnings
            .iter()
            .any(|w| w.message.contains("type constraint")),
        "Expected type-constraint deferral warning, got: {:?}",
        output.warnings
    );
}

/// A slot whose name collides with a prop is skipped — a duplicate
/// constructor param/field would be invalid Dart (mirrors the React
/// backend's collision guard).
#[test]
fn slot_colliding_with_prop_is_skipped() {
    let source = r#"
component Card {
    prop title: String = "T"
    view {
        <div>
            @slot title {
                <span>Fallback</span>
            }
        </div>
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
    assert_eq!(
        content.matches("this.title,").count(),
        1,
        "Constructor must declare exactly one title param (the prop), got:\n{}",
        content
    );
    assert!(
        !content.contains("final Widget? title;"),
        "Colliding slot must not emit a duplicate field, got:\n{}",
        content
    );
    assert!(
        content.contains("final String title;"),
        "Prop field must still be emitted, got:\n{}",
        content
    );
}

/// Multi-node fallbacks wrap in a Column so the `??` expression stays a
/// single widget (Flutter's analogue of React's fragment).
#[test]
fn slot_multi_node_fallback_wraps_in_column() {
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
    let output = compile_flutter(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains(
            "widget.header ?? Column(mainAxisSize: MainAxisSize.min, children: [Text('Title'), Text('Subtitle')])"
        ),
        "Expected Column-wrapped multi-node fallback, got:\n{}",
        content
    );
}

/// Consumer-side `slot="name"` children project into named constructor
/// params at the call site (with the `slot` attribute stripped); plain
/// children flow into the unnamed `children` param.
#[test]
fn slot_projection_at_call_site() {
    let source = r#"
component SlotBox {
    view {
        <div>
            <div>@slot header</div>
            <div>@slot body</div>
            <div>@slot</div>
        </div>
    }
}

component Consumer {
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
    let output = compile_flutter(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("header: Text('Custom Header')"),
        "Expected named slot projected to header param, got:\n{}",
        content
    );
    assert!(
        content.contains("body: Container(child: Text('Body content'))"),
        "Expected named slot projected to body param, got:\n{}",
        content
    );
    assert!(
        content.contains("children: Text('Default child')"),
        "Expected default children param, got:\n{}",
        content
    );
    assert!(
        !content.contains("slot:"),
        "slot prop must be stripped after projection, got:\n{}",
        content
    );
}

/// Consumer-position `@slot name { ... }` directives project into the named
/// param (the fallback IS the projected content); the consumer component
/// itself must not declare slot params for projections.
#[test]
fn slot_projection_directive_at_call_site() {
    let source = r#"
component SlotBox {
    view {
        <div>
            <div>@slot header</div>
            <div>@slot</div>
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
    let output = compile_flutter(source).unwrap();
    let consumer: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("Consumer.dart"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        consumer.contains("header: Text('Custom Header')"),
        "Expected projected header param, got:\n{}",
        consumer
    );
    assert!(
        consumer.contains(
            "children: Column(mainAxisSize: MainAxisSize.min, children: [Text('Body content'), Text('Default child')])"
        ),
        "Expected Column-wrapped default children projection, got:\n{}",
        consumer
    );
    assert!(
        !consumer.contains("final Widget? header;"),
        "Consumer must not declare params for projections, got:\n{}",
        consumer
    );
}

/// An unknown slot name at the call site warns and falls back to the
/// default children instead of emitting invalid Dart.
#[test]
fn slot_projection_unknown_slot_warns_and_falls_back() {
    let source = r#"
component Box {
    view {
        <div>
            <div>@slot header</div>
            <div>@slot</div>
        </div>
    }
}

component Consumer {
    view {
        <Box>
            <h4 slot="missing">Content</h4>
        </Box>
    }
}
"#;
    let output = compile_flutter(source).unwrap();
    assert!(
        output
            .warnings
            .iter()
            .any(|w| w.message.contains("no slot 'missing'")),
        "Expected unknown-slot warning, got: {:?}",
        output.warnings
    );
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("children: Text('Content')"),
        "Unknown slot content should fall back to default children, got:\n{}",
        content
    );
}

/// A component without an unnamed slot drops default children with a
/// warning instead of emitting invalid Dart.
#[test]
fn slot_default_children_dropped_when_no_default_slot() {
    let source = r#"
component Box {
    view {
        <div>
            <div>@slot header</div>
        </div>
    }
}

component Consumer {
    view {
        <Box>
            <span>Orphan child</span>
        </Box>
    }
}
"#;
    let output = compile_flutter(source).unwrap();
    assert!(
        output
            .warnings
            .iter()
            .any(|w| w.message.contains("declares no default slot")),
        "Expected no-default-slot warning, got: {:?}",
        output.warnings
    );
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("children:"),
        "Must not emit children param for slot-less default, got:\n{}",
        content
    );
}

/// Enter-triggered motions inside slot fallbacks must be found so their
/// controllers are created in initState — otherwise the fallback's @enter
/// animation never plays (Task 5.3 review fix).
#[test]
fn enter_motion_in_slot_fallback_is_found() {
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
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,
                source_span: None,
            }],
            ..Default::default()
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
    };
    let view = ViewTree::Slot {
        name: None,
        fallback: vec![ViewTree::Element(Box::new(el))],
        type_constraint: None,
        data_param: None,
    };
    let motions = find_enter_motions(&view);
    assert_eq!(
        motions,
        vec!["fadeIn".to_string()],
        "enter motion inside a slot fallback must be found"
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("App", comp_ref, &ir, &component_map, &component_slots);

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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("Greeting", comp_ref, &ir, &component_map, &component_slots);
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
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("FilterBar", comp_ref, &ir, &component_map, &component_slots);
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
fn test_class_prop_unknown_class_warns_and_skips() {
    // `class: "card"` with no design declaring "card" resolves to nothing —
    // the element stays unwrapped and a warning names the class (spec §10.3,
    // Task 4.3: unknown classes must not vanish silently).
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
        ref_binding: None,
        semantic_bindings: style,
        a11y: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container()");
    assert!(
        !expr.contains("BoxDecoration"),
        "unknown classes must not emit any decoration stub: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("class 'card'") && w.contains("not declared by any design block")),
        "unknown class must warn naming the class: {:?}",
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
    let ref_usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver = DesignResolver::new(&ir);
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver,
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
    let ref_usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver = DesignResolver::new(&ir);
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver,
        warnings: &mut warnings,
    };
    let expr = format_view_tree(&view, &[], &mut ctx);
    assert!(
        expr.contains("ref.read(todoStoreProvider.notifier).clear()"),
        "MethodCallExpr store action must rewrite to ref.read: {}",
        expr
    );
}

// ── Ref codegen tests (Task 5.2) ──────────────────────────

/// Build a minimal element with a tag, optional @ref binding, and semantic
/// bindings (kind → expression).
fn ref_el(tag: &str, ref_binding: Option<&str>, semantic: &[(&str, ExprIR)]) -> ElementIR {
    ElementIR {
        tag: tag.to_string(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Normal,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        ref_binding: ref_binding.map(String::from),
        semantic_bindings: semantic
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        a11y: None,
    }
}

/// Build a component with the given refs, signals, fns, and view.
fn ref_component(
    name: &str,
    refs: &[&str],
    signals: Vec<SignalIntent>,
    fns: Vec<FunctionIR>,
    view: Option<ViewTree>,
) -> ComponentIR {
    ComponentIR {
        name: name.into(),
        refs: refs
            .iter()
            .map(|n| RefDeclIR {
                name: n.to_string(),
                element_tag: None,
            })
            .collect(),
        signals,
        fns,
        view,
        ..Default::default()
    }
}

#[test]
fn test_ref_global_key_and_on_demand_controllers() {
    // One ref used with a focus op, one with a scroll op, one unused.
    let codegen = FlutterCodegen;
    let focus_fn = FunctionIR {
        name: "focusIt".into(),
        visibility: Visibility::Private,
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::RefOpExpr {
                    ref_name: "inputBox".into(),
                    op: RefOpIR::Focus,
                },
            }],
        },
        is_async: false,
        source_span: None,
    };
    let scroll_fn = FunctionIR {
        name: "scrollIt".into(),
        visibility: Visibility::Private,
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::RefOpExpr {
                    ref_name: "pane".into(),
                    op: RefOpIR::ScrollTo {
                        x: None,
                        y: Some(Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(100),
                        })),
                    },
                },
            }],
        },
        is_async: false,
        source_span: None,
    };
    let comp = ref_component(
        "RefDemo",
        &["inputBox", "pane", "unused"],
        vec![],
        vec![focus_fn, scroll_fn],
        None,
    );
    let mut components = FxHashMap::default();
    components.insert("RefDemo".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("RefDemo")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("RefDemo", comp_ref, &ir, &component_map, &component_slots);

    // GlobalKey for every declared ref
    assert!(dart.contains("final GlobalKey _inputBoxKey = GlobalKey();"));
    assert!(dart.contains("final GlobalKey _paneKey = GlobalKey();"));
    assert!(dart.contains("final GlobalKey _unusedKey = GlobalKey();"));

    // On-demand controllers: only for used capabilities
    assert!(dart.contains("final FocusNode _inputBoxFocus = FocusNode();"));
    assert!(dart.contains("final ScrollController _paneScroll = ScrollController();"));
    assert!(
        !dart.contains("_unusedFocus"),
        "unused ref must not get a FocusNode: {dart}"
    );
    assert!(
        !dart.contains("_inputBoxScroll"),
        "focus-only ref must not get a ScrollController: {dart}"
    );
    assert!(
        !dart.contains("_inputBoxText"),
        "focus-only ref must not get a TextEditingController: {dart}"
    );

    // Ref ops format to Flutter idioms in fn bodies
    assert!(dart.contains("_inputBoxFocus.requestFocus();"));
    assert!(dart.contains("if (_paneScroll.hasClients) { _paneScroll.jumpTo(100); }"));

    // dispose() disposes only declared controllers
    assert!(dart.contains("void dispose() {"));
    assert!(dart.contains("_inputBoxFocus.dispose();"));
    assert!(dart.contains("_paneScroll.dispose();"));
    assert!(!dart.contains("_unusedFocus.dispose();"));
}

#[test]
fn test_ref_key_on_widget_and_controller_attachment() {
    // input (needs_text + needs_focus) → key/controller/focusNode params
    let el = ref_el("input", Some("inputBox"), &[]);
    let mut usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    usage.insert(
        "inputBox".into(),
        RefUsage {
            needs_focus: true,
            needs_text: true,
            ..Default::default()
        },
    );
    let ref_usage: &FxHashMap<String, RefUsage> = &usage;
    let mut warnings = Vec::new();
    let ir = empty_program();
    let component_map: FxHashMap<&str, &ComponentIR> = FxHashMap::default();
    let store_provider_map: FxHashMap<String, String> = FxHashMap::default();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver = DesignResolver::new(&ir);
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver,
        warnings: &mut warnings,
    };
    let view = ViewTree::Element(Box::new(el));
    let expr = format_view_tree(&view, &[], &mut ctx);
    assert!(expr.contains("key: _inputBoxKey"), "missing key: {expr}");
    assert!(
        expr.contains("controller: _inputBoxText"),
        "missing text controller: {expr}"
    );
    assert!(
        expr.contains("focusNode: _inputBoxFocus"),
        "missing focusNode: {expr}"
    );

    // non-TextField element with focus need → Focus wrapper
    let el2 = ref_el("div", Some("box"), &[]);
    let mut usage2: FxHashMap<String, RefUsage> = FxHashMap::default();
    usage2.insert(
        "box".into(),
        RefUsage {
            needs_focus: true,
            ..Default::default()
        },
    );
    let ref_usage2: &FxHashMap<String, RefUsage> = &usage2;
    let mut warnings2 = Vec::new();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver2 = DesignResolver::new(&ir);
    let mut ctx2 = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: ref_usage2,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver2,
        warnings: &mut warnings2,
    };
    let view2 = ViewTree::Element(Box::new(el2));
    let expr2 = format_view_tree(&view2, &[], &mut ctx2);
    assert!(
        expr2.contains("Focus(focusNode: _boxFocus, child: Container("),
        "non-TextField focus needs a Focus wrapper: {expr2}"
    );

    // scrollable element with scroll need → ScrollController attachment
    let el3 = ref_el("listview", Some("list"), &[]);
    let mut usage3: FxHashMap<String, RefUsage> = FxHashMap::default();
    usage3.insert(
        "list".into(),
        RefUsage {
            needs_scroll: true,
            ..Default::default()
        },
    );
    let ref_usage3: &FxHashMap<String, RefUsage> = &usage3;
    let mut warnings3 = Vec::new();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver3 = DesignResolver::new(&ir);
    let mut ctx3 = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: ref_usage3,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver3,
        warnings: &mut warnings3,
    };
    let view3 = ViewTree::Element(Box::new(el3));
    let expr3 = format_view_tree(&view3, &[], &mut ctx3);
    assert!(
        expr3.contains("controller: _listScroll"),
        "scrollable ref needs a ScrollController: {expr3}"
    );
}

/// a11y block (design doc 域 10): an unmappable role must surface on the
/// T2.4 warnings channel AND leave an honest degradation comment in the
/// generated Dart (never a silent drop — the review's MEDIUM-2 was exactly
/// the silent loss of a11y intent on Flutter).
#[test]
fn test_a11y_unmappable_role_warns_and_comments() {
    let codegen = FlutterCodegen;
    let el = ElementIR {
        tag: "input".to_string(),
        a11y: Some(A11yIR {
            label: Some("Search".into()),
            role: Some("searchbox".into()), // NOT in the Flutter subset
            hidden: None,
            live: None,
            alt: None,
        }),
        children: vec![],
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        construction_kind: ConstructionKind::Normal,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let view = ViewTree::Element(Box::new(el));
    let comp = ref_component("A11yDemo", &[], vec![], vec![], Some(view));
    let mut components = FxHashMap::default();
    components.insert("A11yDemo".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("A11yDemo")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, warnings, errors) =
        codegen.gen_component_dart("A11yDemo", comp_ref, &ir, &component_map, &component_slots);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    // The label still emits a Semantics node (the a11y intent survives).
    assert!(
        dart.contains("Semantics(label: 'Search'"),
        "label must still emit Semantics; got:
{}",
        dart
    );
    // No flag for the unmappable role — but the degradation is stated:
    assert!(
        !dart.contains("textField: true") && !dart.contains("button: true"),
        "unmappable role must not emit a Semantics flag; got:
{}",
        dart
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("searchbox") && w.contains("Semantics")),
        "unmappable role must warn on the T2.4 warnings channel; got: {:?}",
        warnings
    );
    assert!(
        dart.contains("/* a11y: role 'searchbox' has no Flutter Semantics flag"),
        "degradation comment must be present in the generated code; got:
{}",
        dart
    );
}

/// a11y block (design doc 域 10): `label` + `alt` on a NON-image element
/// used to emit TWO `label:` args into one `Semantics(...)` call —
/// duplicate named argument, invalid Dart. `label` wins (it IS the
/// accessible name); `alt` (image-specific intent on a non-image element)
/// warns on the T2.4 warnings channel and is dropped.
#[test]
fn test_a11y_label_alt_collision_non_image_dedupes() {
    let codegen = FlutterCodegen;
    let el = ElementIR {
        tag: "div".to_string(),
        a11y: Some(A11yIR {
            label: Some("Close".into()),
            role: None,
            hidden: None,
            live: None,
            alt: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("close icon"),
            }),
        }),
        children: vec![],
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        construction_kind: ConstructionKind::Normal,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let view = ViewTree::Element(Box::new(el));
    let comp = ref_component("A11yDemo", &[], vec![], vec![], Some(view));
    let mut components = FxHashMap::default();
    components.insert("A11yDemo".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("A11yDemo")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, warnings, errors) =
        codegen.gen_component_dart("A11yDemo", comp_ref, &ir, &component_map, &component_slots);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    // Exactly ONE label: arg in the Semantics call — the label wins.
    assert_eq!(
        dart.matches("label: 'Close'").count(),
        1,
        "label must win over alt — exactly one label: arg (duplicate named \
         args are invalid Dart); got:
{}",
        dart
    );
    assert!(
        !dart.contains("close icon"),
        "alt must not leak into the Semantics call; got:
{}",
        dart
    );
    assert!(
        warnings.iter().any(|w| w.contains("alt 仅对 img 元素生效")),
        "alt-on-non-image with a label present must warn on the T2.4 \
         warnings channel; got: {:?}",
        warnings
    );
}

/// a11y block (design doc 域 10): `alt` ALONE on a non-image element keeps
/// the documented degradation to the accessible-name `label` (no warning —
/// image alt text IS the accessible name; no information is lost).
#[test]
fn test_a11y_alt_alone_non_image_degrades_to_label() {
    let codegen = FlutterCodegen;
    let el = ElementIR {
        tag: "div".to_string(),
        a11y: Some(A11yIR {
            label: None,
            role: None,
            hidden: None,
            live: None,
            alt: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("close icon"),
            }),
        }),
        children: vec![],
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        construction_kind: ConstructionKind::Normal,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    };
    let view = ViewTree::Element(Box::new(el));
    let comp = ref_component("A11yDemo", &[], vec![], vec![], Some(view));
    let mut components = FxHashMap::default();
    components.insert("A11yDemo".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("A11yDemo")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, warnings, errors) =
        codegen.gen_component_dart("A11yDemo", comp_ref, &ir, &component_map, &component_slots);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    assert!(
        dart.contains("Semantics(label: 'close icon'"),
        "alt alone on a non-image element degrades to the accessible-name \
         label; got:
{}",
        dart
    );
    assert!(
        warnings.is_empty(),
        "alt alone on a non-image element must not warn (documented \
         degradation); got: {:?}",
        warnings
    );
}

/// Design D2: `bind:value` (ElementIR.bindings) on a ref'd text field is the
/// sole two-way form — it must install the guarded controller listener (user
/// edits → setState write-back) and the `_syncSemanticBindings` re-apply,
/// exactly as the removed `value={sig}` semantic binding did.
#[test]
fn test_bind_value_ref_text_field_two_way() {
    let codegen = FlutterCodegen;
    let el_input = ElementIR {
        tag: "input".to_string(),
        ref_binding: Some("inputBox".to_string()),
        semantic_bindings: FxHashMap::default(),
        bindings: FxHashMap::from_iter([(
            "value".to_string(),
            SignalRefIR {
                name: "text".to_string(),
            },
        )]),
        children: vec![],
        props: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        construction_kind: ConstructionKind::Normal,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        a11y: None,
    };
    let view = ViewTree::Element(Box::new(el_input));
    let signals = vec![SignalIntent {
        name: "text".into(),
        r#type: TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
        initial_value: ExprIR::LiteralExpr {
            value: serde_json::json!(""),
        },
    }];
    let comp = ref_component("BindDemo", &["inputBox"], signals, vec![], Some(view));
    let mut components = FxHashMap::default();
    components.insert("BindDemo".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("BindDemo")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("BindDemo", comp_ref, &ir, &component_map, &component_slots);

    // Controller + guard flag declared (bind:value needs the text controller).
    assert!(
        dart.contains("final TextEditingController _inputBoxText = TextEditingController();"),
        "bind:value must declare the text controller. Got:\n{}",
        dart
    );
    assert!(
        dart.contains("bool _syncingInputBoxText = false;"),
        "bind:value must declare the sync guard. Got:\n{}",
        dart
    );
    // Listener write-back: user edits flow into the signal via setState.
    assert!(
        dart.contains("_inputBoxText.addListener(() {"),
        "bind:value must install the controller listener. Got:\n{}",
        dart
    );
    assert!(
        dart.contains("setState(() {") && dart.contains("text = _inputBoxText.text;"),
        "bind:value must write back through setState. Got:\n{}",
        dart
    );
    // The widget attaches the controller and the sync re-applies signal → text.
    assert!(
        dart.contains("controller: _inputBoxText"),
        "TextField must attach the controller. Got:\n{}",
        dart
    );
    assert!(
        dart.contains("_syncingInputBoxText = true;")
            && dart.contains("_inputBoxText.text = text;"),
        "_syncSemanticBindings must re-apply the signal to the controller. Got:\n{}",
        dart
    );
}

#[test]
fn test_semantic_binding_post_frame_setup() {
    // input with focus={active} + bind:value={text}; div with scroll-y={pos}
    // (the value={sig} semantic form is removed — design D2 — bind:value
    // drives the text controller machinery)
    let codegen = FlutterCodegen;
    let mut el_input = ref_el(
        "input",
        Some("inputBox"),
        &[("focus", ExprIR::ident("active"))],
    );
    el_input.bindings.insert(
        "value".to_string(),
        SignalRefIR {
            name: "text".to_string(),
        },
    );
    let el_pane = ref_el("div", Some("pane"), &[("scroll-y", ExprIR::ident("pos"))]);
    let view = ViewTree::Fragment(vec![
        ViewTree::Element(Box::new(el_input)),
        ViewTree::Element(Box::new(el_pane)),
    ]);
    let signals = vec![
        SignalIntent {
            name: "active".into(),
            r#type: TypeIR {
                kind: "bool".into(),
                type_args: vec![],
                is_nullable: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!(false),
            },
        },
        SignalIntent {
            name: "text".into(),
            r#type: TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!(""),
            },
        },
        SignalIntent {
            name: "pos".into(),
            r#type: TypeIR {
                kind: "f64".into(),
                type_args: vec![],
                is_nullable: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!(0.0),
            },
        },
    ];
    let comp = ref_component(
        "BindDemo",
        &["inputBox", "pane"],
        signals,
        vec![],
        Some(view),
    );
    let mut components = FxHashMap::default();
    components.insert("BindDemo".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("BindDemo")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("BindDemo", comp_ref, &ir, &component_map, &component_slots);

    // Controllers + guard flag declared
    assert!(dart.contains("final FocusNode _inputBoxFocus = FocusNode();"));
    assert!(dart.contains("final TextEditingController _inputBoxText = TextEditingController();"));
    assert!(dart.contains("final ScrollController _paneScroll = ScrollController();"));
    assert!(dart.contains("bool _syncingInputBoxText = false;"));

    // value listener with guard: user edits flow into the signal via
    // setState (a bare field write would leave consumers of the signal stale)
    assert!(dart.contains("_inputBoxText.addListener(() {"));
    assert!(dart.contains("if (_syncingInputBoxText) {"));
    assert!(dart.contains("setState(() {"));
    assert!(dart.contains("text = _inputBoxText.text;"));

    // Reactive sync method re-applying all binding kinds after every
    // rebuild (parity with React's useEffect-on-deps)
    assert!(dart.contains("void _syncSemanticBindings() {"));
    assert!(dart.contains("if (active) {"));
    assert!(dart.contains("_inputBoxFocus.requestFocus();"));
    assert!(dart.contains("_inputBoxFocus.unfocus();"));
    assert!(dart.contains(
        "_paneScroll.animateTo(pos, duration: Duration(milliseconds: 300), curve: Curves.easeInOut);"
    ));
    assert!(dart.contains("_syncingInputBoxText = true;"));
    assert!(dart.contains("_inputBoxText.text = text;"));
    assert!(dart.contains("_syncingInputBoxText = false;"));

    // build registers the sync post-frame with a mounted guard (prevents
    // use-after-dispose on fast unmount)
    assert!(dart.contains("WidgetsBinding.instance.addPostFrameCallback((_) {"));
    assert!(dart.contains("if (!mounted) return;"));
    assert!(dart.contains("_syncSemanticBindings();"));

    // dispose() covers all controllers
    assert!(dart.contains("_inputBoxFocus.dispose();"));
    assert!(dart.contains("_inputBoxText.dispose();"));
    assert!(dart.contains("_paneScroll.dispose();"));
}

#[test]
fn test_format_ref_op_flutter_variants() {
    let op = |ref_name: &str, op: RefOpIR| ExprIR::RefOpExpr {
        ref_name: ref_name.into(),
        op,
    };

    // Focus / Blur
    assert_eq!(
        format_expr_ir_flutter(&op("box", RefOpIR::Focus)),
        "_boxFocus.requestFocus()"
    );
    assert_eq!(
        format_expr_ir_flutter(&op("box", RefOpIR::Blur)),
        "_boxFocus.unfocus()"
    );

    // ScrollTo — y wins; guarded by hasClients
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::ScrollTo {
                x: Some(Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(10),
                })),
                y: Some(Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(100),
                })),
            }
        )),
        "if (_boxScroll.hasClients) { _boxScroll.jumpTo(100); }"
    );
    // ScrollTo — x only
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::ScrollTo {
                x: Some(Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(10),
                })),
                y: None,
            }
        )),
        "if (_boxScroll.hasClients) { _boxScroll.jumpTo(10); }"
    );

    // ScrollBy — relative delta via animateTo with duration/curve
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::ScrollBy {
                dx: None,
                dy: Some(Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(50),
                })),
            }
        )),
        "if (_boxScroll.hasClients) { _boxScroll.animateTo(_boxScroll.position.pixels + 50, duration: Duration(milliseconds: 200), curve: Curves.easeInOut); }"
    );

    // Measure — width / height / position
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::Measure {
                what: MeasureWhat::Width
            }
        )),
        "(_boxKey.currentContext?.findRenderObject() as RenderBox?)?.size.width"
    );
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::Measure {
                what: MeasureWhat::Height
            }
        )),
        "(_boxKey.currentContext?.findRenderObject() as RenderBox?)?.size.height"
    );
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::Measure {
                what: MeasureWhat::Position
            }
        )),
        "(_boxKey.currentContext?.findRenderObject() as RenderBox?)?.localToGlobal(Offset.zero)"
    );

    // Value read (the imperative write op was removed — bind:value covers it)
    assert_eq!(
        format_expr_ir_flutter(&op("box", RefOpIR::Value)),
        "_boxText.text"
    );

    // Select / SetSelection
    assert_eq!(
        format_expr_ir_flutter(&op("box", RefOpIR::Select)),
        "_boxText.selection = TextSelection(baseOffset: 0, extentOffset: _boxText.text.length)"
    );
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::SetSelection {
                start: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                end: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(5),
                }),
            }
        )),
        "_boxText.selection = TextSelection(baseOffset: 1, extentOffset: 5)"
    );

    // Pointer capture — unsupported on Flutter (design D3 / spec §10.3):
    // the formatting emits a runtime `throw UnsupportedError` (loud even if
    // the compile diagnostic is bypassed). `throw` is a valid Dart
    // expression, so the no-trailing-semicolon convention holds.
    assert_eq!(
        format_expr_ir_flutter(&op("box", RefOpIR::CapturePointer)),
        "throw UnsupportedError('capturePointer not supported on Flutter')"
    );
    assert_eq!(
        format_expr_ir_flutter(&op("box", RefOpIR::ReleasePointer)),
        "throw UnsupportedError('releasePointer not supported on Flutter')"
    );
}

#[test]
fn test_ref_ops_in_effect_and_inline_handler_are_detected() {
    // @effect body calling ref.scrollTo → ScrollController declared even
    // though no fn body references it.
    let codegen = FlutterCodegen;
    let el = ref_el("listview", Some("list"), &[]);
    let view = ViewTree::Element(Box::new(el));
    let mut comp = ref_component("EffectRef", &["list"], vec![], vec![], Some(view));
    comp.effects = vec![EffectIntent {
        deps: vec!["count".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::RefOpExpr {
                    ref_name: "list".into(),
                    op: RefOpIR::ScrollTo {
                        x: None,
                        y: Some(Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(42),
                        })),
                    },
                },
            }],
        },
    }];
    let mut components = FxHashMap::default();
    components.insert("EffectRef".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("EffectRef")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, _warnings, _errors) =
        codegen.gen_component_dart("EffectRef", comp_ref, &ir, &component_map, &component_slots);
    assert!(
        dart.contains("final ScrollController _listScroll = ScrollController();"),
        "effect-body ref op must trigger the controller declaration: {dart}"
    );
    assert!(
        dart.contains("_listScroll.jumpTo(42)"),
        "missing scroll op in effect body: {dart}"
    );
}

/// Test: `capturePointer()`/`releasePointer()` are a hard ERROR on Flutter
/// (design D3 / spec §10.3) — the compile fails with the verbatim hint AND
/// the emitted Dart contains a runtime `throw UnsupportedError` so failure
/// stays loud even if the diagnostic is bypassed.
#[test]
fn test_pointer_capture_is_hard_error_on_flutter() {
    let source = r#"
component PtrBox {
  ref box: Element
  fn onDown() {
    box.capturePointer()
  }
  fn onUp() {
    box.releasePointer()
  }
  view {
    <div ref:box @onPointerDown={onDown} @onPointerUp={onUp}>hold me</div>
  }
}
"#;
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
    assert!(
        !result.success,
        "pointer capture must fail the Flutter compile, got success with: {:?}",
        result.errors
    );
    let messages: Vec<&str> = result.errors.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("Flutter 不支持指针捕获") && m.contains("GestureDetector")),
        "expected the verbatim §10.3 hint, got: {:?}",
        messages
    );
    let dart: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("throw UnsupportedError('capturePointer not supported on Flutter');")
            && dart.contains("throw UnsupportedError('releasePointer not supported on Flutter');"),
        "emitted Dart must throw UnsupportedError (not a TODO comment). Got:\n{}",
        dart
    );
    assert!(
        !dart.contains("TODO(flutter)"),
        "no placeholder TODO comments may remain. Got:\n{}",
        dart
    );
}

#[test]
fn test_end_to_end_ref_component_compiles() {
    // Full pipeline: .omni source with a ref declaration, semantic bindings,
    // and a ref op in a fn → valid Dart StatefulWidget with GlobalKey +
    // on-demand controllers.
    let source = r##"
component SearchBox {
  signal active: Bool = false
  signal pos: Float = 0
  signal text: String = ""
  ref field: Element
  fn focusField() {
    field.focus()
  }
  view {
    <input ref:field focus={active} scroll-y={pos} bind:value={text} />
  }
}
"##;
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
    assert!(
        result.success,
        "flutter compile failed: {:?}",
        result.errors
    );
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(content.contains("final GlobalKey _fieldKey = GlobalKey();"));
    assert!(content.contains("final FocusNode _fieldFocus = FocusNode();"));
    assert!(content.contains("final ScrollController _fieldScroll = ScrollController();"));
    assert!(content.contains("final TextEditingController _fieldText = TextEditingController();"));
    assert!(content.contains("bool _syncingFieldText = false;"));
    assert!(content.contains("key: _fieldKey"));
    assert!(content.contains("focusNode: _fieldFocus"));
    assert!(content.contains("controller: _fieldText"));
    assert!(content.contains("_fieldFocus.requestFocus();"));
    assert!(content.contains("WidgetsBinding.instance.addPostFrameCallback((_) {"));
    assert!(content.contains("if (!mounted) return;"));
    assert!(content.contains("_syncSemanticBindings();"));
    assert!(content.contains("void _syncSemanticBindings() {"));
    assert!(content.contains("_fieldText.addListener(() {"));
    assert!(content.contains("_fieldFocus.dispose();"));
    assert!(content.contains("_fieldText.dispose();"));
}

/// ADR-053 D5: body-level `@slot` declarations become `Widget?` constructor
/// params and their `{name}` interpolations render `widget.name ?? fallback`
/// — mirroring view-level slots.
#[test]
fn body_slot_declarations_emit_widget_params_with_fallback() {
    let source = r#"
component Modal {
    @slot header { <h4>Default header</h4> }
    @slot { <p>Default body</p> }
    view {
        <div>
            {header}
            {children}
        </div>
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
    assert!(
        content.contains("this.header,") && content.contains("this.children,"),
        "Expected body slots as constructor params, got:\n{}",
        content
    );
    assert!(
        content.contains("final Widget? header;") && content.contains("final Widget? children;"),
        "Expected Widget? slot fields, got:\n{}",
        content
    );
    assert!(
        content.contains("widget.header ??") && content.contains("widget.children ??"),
        "Expected fallback-wrapped slot renders, got:\n{}",
        content
    );
    assert!(
        content.contains("Default header") && content.contains("Default body"),
        "Expected declared fallbacks in output, got:\n{}",
        content
    );
}

/// ADR-053 D5: consumer named-prop projection (`header={<div/>}`) projects
/// into the body-declared slot param; a bare instantiation keeps the
/// declared fallbacks.
#[test]
fn consumer_named_prop_projects_into_body_declared_slot() {
    let source = r#"
component Modal {
    @slot header { <h4>Default header</h4> }
    @slot { <p>Default body</p> }
    view {
        <div>
            {header}
            {children}
        </div>
    }
}

component App {
    view {
        <Modal header={<div>Custom title</div>}>
            <p>Custom body</p>
        </Modal>
        <Modal />
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
    let app: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("App.dart"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        app.contains("header: "),
        "Expected named-prop projection in consumer, got:\n{}",
        app
    );
    assert!(
        app.contains("Custom title") && app.contains("Custom body"),
        "Expected projected content, got:\n{}",
        app
    );
    assert!(
        app.contains("Modal()"),
        "Expected bare instantiation to keep declared fallbacks, got:\n{}",
        app
    );
}

/// ADR-053 D5: a typed body slot without fallback renders
/// `widget.<name> ?? SizedBox.shrink()` — the param is `Widget?`, so a
/// bare `widget.<name>` would be a static error in `List<Widget>`
/// contexts; the shrink box renders nothing when no content is projected.
#[test]
fn body_typed_slot_without_fallback_renders_bare() {
    let source = r#"
component Badge {
    @slot footer: String
    view {
        <div>{footer}</div>
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
    assert!(
        content.contains("widget.footer ?? SizedBox.shrink()"),
        "expected nullable-guarded typed-slot render, got:\n{}",
        content
    );
    assert!(
        content.contains("final Widget? footer;"),
        "typed slot param stays Widget? (constraint deferred), got:\n{}",
        content
    );
}

// ═══════════════════════════════════════════════════════════════
// Event field extraction diagnostics (Task 2.4 — Phase 2 Domain 7)
// ═══════════════════════════════════════════════════════════════

/// Extract a single field with a fresh warnings channel.
fn extract_with_warnings(kind: &EventDataKind, field: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let expr = field_extract_flutter(kind, field, "e", &mut warnings);
    (expr, warnings)
}

/// Silent-stub contract (ADR-054 honest degradation): every unmapped or
/// dropped event field must emit a warning naming the field AND the event
/// kind, and the generated code must keep a degradation comment.
#[test]
fn unmapped_event_fields_warn_not_silent() {
    // ClipboardEvent.text was a silent '' stub.
    let (expr, warnings) = extract_with_warnings(&EventDataKind::ClipboardEvent, "text");
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("text") && w.contains("ClipboardEvent")),
        "clipboard text stub must warn with field + kind, got: {:?}",
        warnings
    );
    assert!(
        expr.contains("/*") && expr.contains("*/"),
        "degradation comment missing from generated code: {expr}"
    );

    // DropEvent.files / text — same contract.
    let (files_expr, files_warnings) = extract_with_warnings(&EventDataKind::DropEvent, "files");
    assert!(
        files_warnings
            .iter()
            .any(|w| w.contains("files") && w.contains("DropEvent")),
        "drop files stub must warn with field + kind, got: {:?}",
        files_warnings
    );
    assert!(
        files_expr.contains("/*"),
        "degradation comment: {files_expr}"
    );
    let (text_expr, text_warnings) = extract_with_warnings(&EventDataKind::DropEvent, "text");
    assert!(
        text_warnings
            .iter()
            .any(|w| w.contains("text") && w.contains("DropEvent")),
        "drop text stub must warn with field + kind, got: {:?}",
        text_warnings
    );
    assert!(text_expr.contains("/*"), "degradation comment: {text_expr}");

    // The remaining silent-constant stubs.
    let (tilt_y, tilt_warnings) = extract_with_warnings(&EventDataKind::PointerEvent, "tiltY");
    assert!(
        tilt_warnings
            .iter()
            .any(|w| w.contains("tiltY") && w.contains("PointerEvent")),
        "tiltY stub must warn, got: {:?}",
        tilt_warnings
    );
    assert!(tilt_y.contains("/*"), "degradation comment: {tilt_y}");
    let (touches, touch_warnings) = extract_with_warnings(&EventDataKind::TouchEvent, "touches");
    assert!(
        touch_warnings
            .iter()
            .any(|w| w.contains("touches") && w.contains("TouchEvent")),
        "touches stub must warn, got: {:?}",
        touch_warnings
    );
    assert!(touches.contains("/*"), "degradation comment: {touches}");
    let (delta_mode, dm_warnings) = extract_with_warnings(&EventDataKind::WheelEvent, "deltaMode");
    assert!(
        dm_warnings
            .iter()
            .any(|w| w.contains("deltaMode") && w.contains("WheelEvent")),
        "deltaMode stub must warn, got: {:?}",
        dm_warnings
    );
    assert!(
        delta_mode.contains("/*"),
        "degradation comment: {delta_mode}"
    );
    let (values, values_warnings) = extract_with_warnings(&EventDataKind::SubmitEvent, "values");
    assert!(
        values_warnings
            .iter()
            .any(|w| w.contains("values") && w.contains("SubmitEvent")),
        "submit values stub must warn, got: {:?}",
        values_warnings
    );
    assert!(values.contains("/*"), "degradation comment: {values}");
}

/// Real mappings must stay silent — no warning, no degradation comment.
#[test]
fn mapped_event_fields_do_not_warn() {
    for (kind, field) in [
        (EventDataKind::PointerEvent, "x"),
        (EventDataKind::PointerEvent, "y"),
        (EventDataKind::PointerEvent, "button"),
        (EventDataKind::PointerEvent, "pressure"),
        (EventDataKind::KeyEvent, "key"),
        (EventDataKind::InputEvent, "value"),
        (EventDataKind::MediaEvent, "currentTime"),
        (EventDataKind::DragEvent, "x"),
    ] {
        let (expr, warnings) = extract_with_warnings(&kind, field);
        assert!(
            warnings.is_empty(),
            "mapped field '{field}' of {kind:?} must not warn: {:?}",
            warnings
        );
        assert!(
            !expr.contains("/*"),
            "mapped field '{field}' must not carry a degradation comment: {expr}"
        );
    }
}

/// KeyEvent.code must use a STABLE mapping — the DOM-analogue physical key
/// id — never debugName (a debug-oriented string that changes with the
/// Flutter engine and is not stable data).
#[test]
fn key_code_uses_stable_mapping() {
    let (expr, warnings) = extract_with_warnings(&EventDataKind::KeyEvent, "code");
    assert!(
        expr.contains("physicalKey.keyId"),
        "code must map to the stable physical key id, got: {expr}"
    );
    assert!(
        !expr.contains("debugName"),
        "debugName must never be emitted as event data: {expr}"
    );
    assert!(
        warnings.is_empty(),
        "keyCode is a real mapping — no warning expected: {:?}",
        warnings
    );
}

/// The map-literal emitter threads the warnings channel too: a handler that
/// uses stubbed fields emits field + kind warnings, and the generated map
/// carries degradation comments while mapped fields stay silent.
#[test]
fn emit_event_object_flutter_warns_for_dropped_fields() {
    let mut warnings = Vec::new();
    let obj = emit_event_object_flutter(
        &Some(EventDataKind::DropEvent),
        &Some(vec!["files".to_string(), "x".to_string()]),
        "event",
        &mut warnings,
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("files") && w.contains("DropEvent")),
        "dropped 'files' must warn through the map emitter: {:?}",
        warnings
    );
    assert!(
        !warnings.iter().any(|w| w.contains("field 'x'")),
        "mapped 'x' must not warn: {:?}",
        warnings
    );
    assert!(
        obj.contains("'files':"),
        "map must still emit the field key"
    );
    assert!(obj.contains("'x':"), "map must still emit mapped fields");
    assert!(obj.contains("/*"), "degradation comment in map: {obj}");
}

/// Task 4.4: the spec §10.3 design diagnostics fire through the FULL
/// pipeline (parse → semantic → IR → Flutter codegen): `display: grid` and
/// `float` are hard errors, `cursor` and `zIndex` are warnings, and the
/// ZIndex token no longer emits a meaningless `static const int`.
#[test]
fn design_spec_10_3_diagnostics_flow_through_full_pipeline() {
    let source = r##"
import omniview;

design App {
    token zModal: zIndex = "100";
    token primary: color = "#6366f1";

    class GridPanel {
        display: grid;
        background: token(primary);
    }
    class Floaty {
        float: left;
    }
    class Clickable {
        cursor: pointer;
        background: token(primary);
    }
}

component Demo {
    view {
        <div class: App.GridPanel>grid</div>
        <div class: App.Clickable>click</div>
    }
}
"##;
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
    // grid + float → hard errors (the compile must NOT be marked successful).
    let grid_err = result
        .errors
        .iter()
        .find(|e| e.message.contains("Flutter 不支持 CSS grid"));
    assert!(
        grid_err.is_some(),
        "display:grid must hard-error through the pipeline. Got:\n{:?}",
        result.errors
    );
    assert_eq!(grid_err.unwrap().severity, crate::error::Severity::Error);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("Flutter widget 布局无 float 概念")),
        "float must hard-error through the pipeline. Got:\n{:?}",
        result.errors
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.message.contains("cursor 在 Flutter 中无效果")),
        "cursor must warn through the pipeline. Got:\n{:?}",
        result.warnings
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.message.contains("token 'zModal'")
                && w.message.contains("Flutter z 序由 widget 顺序决定")),
        "zIndex must warn through the pipeline. Got:\n{:?}",
        result.warnings
    );
    // The design runtime still emits (warnings do not block files), but the
    // ZIndex constant is gone.
    let design_dart = result
        .files
        .get("design.dart")
        .expect("design.dart should still be emitted");
    assert!(
        !design_dart.contains("zModal"),
        "no meaningless ZIndex constant may be emitted. Got:\n{}",
        design_dart
    );
}

#[test]
fn scroll_trigger_passes_typed_start_end_to_widget() {
    // v1 control dimensions (Task 5.1): numeric start/end flow into the
    // _OmniScrollDrivenWidget constructor as typed doubles.
    let result = compile_flutter(
        "@motion fadeScroll { from { opacity: 0.3 } to { opacity: 1 } over: 500ms }
         component Test {
             view {
                 <div @scroll start: 0 end: 1000 scrub: true ={fadeScroll}>scroll me</div>
             }
         }",
    )
    .expect("scroll fixture must compile");
    let out = result.files.values().cloned().collect::<Vec<_>>().join(
        "
",
    );
    assert!(
        out.contains(
            "_OmniScrollDrivenWidget(
  start: 0,
  end: 1000,"
        ),
        "expected typed start/end constructor args, got:
{}",
        &out[..out.len().min(1200)]
    );
    assert!(
        out.contains("class _OmniScrollDrivenWidget extends StatefulWidget"),
        "scroll widget class must be emitted"
    );
    assert!(
        out.contains("final double? start;"),
        "widget must accept typed nullable start"
    );
}

#[test]
fn drag_trigger_passes_typed_constraints_to_widget() {
    // v1 control dimensions (Task 5.1): constraints: [min, max] flow into
    // the _OmniDragWidget constructor as typed constraintsMin/constraintsMax.
    let result = compile_flutter(
        "@motion dragX { from { translateX: 0 } to { translateX: 300 } over: 300ms }
         component Test {
             view {
                 <div @drag-horizontal constraints: [-100, 100] elastic: 0.5 threshold: 0.3 velocity_max: 500 ={dragX}>drag me</div>
             }
         }",
    )
    .expect("drag fixture must compile");
    let out = result.files.values().cloned().collect::<Vec<_>>().join(
        "
",
    );
    assert!(
        out.contains("constraintsMin: -100,") && out.contains("constraintsMax: 100,"),
        "expected typed constraintsMin/constraintsMax constructor args, got:
{}",
        &out[..out.len().min(1200)]
    );
    assert!(
        out.contains("final double? constraintsMin;"),
        "widget must accept typed nullable constraintsMin"
    );
    assert!(
        out.contains("velocityMax: 500,"),
        "velocity_max must flow into the widget constructor, got:
{}",
        &out[..out.len().min(1200)]
    );
    assert!(
        out.contains("_velocity = (dp / dt * 1000).clamp(0.0, widget.velocityMax);"),
        "widget must track drag velocity clamped at velocityMax, got:
{}",
        &out[..out.len().min(1200)]
    );
}

#[test]
fn scroll_without_params_omits_typed_args() {
    // No sub-params: the widget falls back to the legacy string configs.
    let result = compile_flutter(
        "@motion fadeScroll { from { opacity: 0.3 } to { opacity: 1 } over: 500ms }
         component Test {
             view {
                 <div @scroll={fadeScroll}>scroll me</div>
             }
         }",
    )
    .expect("scroll fixture must compile");
    let out = result.files.values().cloned().collect::<Vec<_>>().join(
        "
",
    );
    assert!(
        !out.contains(
            "_OmniScrollDrivenWidget(
  start:"
        ),
        "no typed start/end args when sub-params absent, got:
{}",
        &out[..out.len().min(800)]
    );
}

/// Review fix (fix round 1/5, HIGH): the numeric @scroll path must map
/// progress from the SCROLL OFFSET alone — the previous emitted computation
/// summed the viewport-relative elementTop into the document-relative
/// scrollOffset, producing a constant "document position" that never changed
/// while scrolling (a silent no-op). This test extracts the numeric branch
/// VERBATIM from the generated Dart and executes it under `dart run`,
/// driving scroll offsets 0/500/2000 against a 0→1000 range and asserting
/// progress 0.0/0.5/1.0. Skips when the Dart SDK is unavailable.
#[test]
fn flutter_scroll_numeric_progress_changes_with_scroll_offset() {
    let result = compile_flutter(
        "@motion fadeScroll { from { opacity: 0.3 } to { opacity: 1 } over: 500ms }\n\
         component Test {\n\
             view {\n\
                 <div @scroll start: 0 end: 1000 ={fadeScroll}>scroll me</div>\n\
             }\n\
         }",
    )
    .expect("scroll fixture must compile");
    let out = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    // Extract the emitted numeric branch verbatim: from the typedStart
    // declaration through the first `progress = raw.clamp(0.0, 1.0);`
    // (the if-block closes with a brace we append — the emitted ` else {`
    // legacy branch is excluded).
    let marker = "final double? typedStart = widget.start;";
    let start_idx = out
        .find(marker)
        .unwrap_or_else(|| panic!("numeric branch marker not found in output:\n{}", out));
    let fragment_start = &out[start_idx..];
    let end_idx = fragment_start
        .find("progress = raw.clamp(0.0, 1.0);")
        .expect("progress assignment not found in numeric branch")
        + "progress = raw.clamp(0.0, 1.0);".len();
    // The emitted numeric branch declares `double progress;` locally — the
    // harness provides the accumulator at top level so the driver can
    // observe it; the bare declaration carries no computation.
    let fragment = fragment_start[..end_idx]
        .lines()
        .filter(|l| l.trim() != "double progress;")
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    let fragment = format!(
        "{}
}}",
        fragment
    );

    let script = format!(
        "class WidgetStub {{\n\
         \x20 final double? start;\n\
         \x20 final double? end;\n\
         \x20 const WidgetStub(this.start, this.end);\n\
         }}\n\
         class ScrollPos {{\n\
         \x20 final double? pixels;\n\
         \x20 const ScrollPos(this.pixels);\n\
         }}\n\
         WidgetStub widget = const WidgetStub(0, 1000);\n\
         ScrollPos? _scrollPosition;\n\
         double progress = 0;\n\
         \n\
         void updateProgress() {{\n\
         {fragment}\n\
         }}\n\
         \n\
         void main() {{\n\
         \x20 _scrollPosition = const ScrollPos(0); updateProgress(); print('p0=$progress');\n\
         \x20 _scrollPosition = const ScrollPos(500); updateProgress(); print('p500=$progress');\n\
         \x20 _scrollPosition = const ScrollPos(2000); updateProgress(); print('p2000=$progress');\n\
         }}\n"
    );

    // Locate a usable `dart` (the gate falls back to `cmd /C dart` on
    // Windows where only dart.bat exists).
    let mut dart_cmd = Command::new("dart");
    let mut use_cmd_fallback = false;
    if dart_cmd.arg("--version").output().is_err() {
        #[cfg(windows)]
        {
            if Command::new("cmd")
                .args(["/C", "dart", "--version"])
                .output()
                .is_ok_and(|o| o.status.success())
            {
                use_cmd_fallback = true;
            }
        }
    }
    if !use_cmd_fallback && dart_cmd.arg("--version").output().is_err() {
        eprintln!("SKIP: dart not found on PATH — cannot execute the emitted progress computation");
        return;
    }

    let script_dir = std::env::temp_dir().join("omni_flutter_scroll_check");
    let _ = std::fs::create_dir_all(&script_dir);
    let script_path = script_dir.join("check.dart");
    std::fs::write(&script_path, &script).expect("write dart check script");
    let mut run = if use_cmd_fallback {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("dart").arg("run").arg(&script_path);
        cmd
    } else {
        let mut cmd = Command::new("dart");
        cmd.arg("run").arg(&script_path);
        cmd
    };
    let output = run.output().expect("failed to execute dart run");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "dart run failed:\nstdout: {}\nstderr: {}\nscript:\n{}",
        stdout,
        stderr,
        script
    );
    assert!(
        stdout.contains("p0=0.0") && stdout.contains("p500=0.5") && stdout.contains("p2000=1.0"),
        "progress must move with the scroll offset (0→0.0, 500→0.5, 2000→1.0), got:\n{}",
        stdout
    );
}
