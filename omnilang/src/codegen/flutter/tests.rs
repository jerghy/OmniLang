//! Unit tests for flutter codegen.

use super::design::DesignResolver;
use super::*;
use crate::codegen::shared::build_component_map;
use crate::codegen::{format_json_value, JSON_FLUTTER};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
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

// ═══════════════════════════════════════════════════════════════
// Task 5.8 regression tests — showcase gate classes
// ═══════════════════════════════════════════════════════════════

/// Method table: the Dart family must map the language's method names
/// (.len()/.to_string()/.push()/.get()) to Dart natives.
#[test]
fn test_task58_method_name_mapping() {
    let method = |obj: &str, m: &str, args: &[&str]| ExprIR::MethodCallExpr {
        object: Box::new(ExprIR::IdentifierExpr {
            name: obj.into(),
            type_ann: None,
        }),
        method: m.into(),
        args: args
            .iter()
            .map(|a| ExprIR::LiteralExpr {
                value: serde_json::json!(a),
            })
            .collect(),
        is_optional: false,
    };
    assert_eq!(
        format_expr_ir_flutter(&method("input", "len", &[])),
        "input.length"
    );
    assert_eq!(
        format_expr_ir_flutter(&method("n", "to_string", &[])),
        "n.toString()"
    );
    assert_eq!(
        format_expr_ir_flutter(&method("items", "push", &["x"])),
        "items.add('x')"
    );
    assert_eq!(
        format_expr_ir_flutter(&method("items", "pop", &[])),
        "items.removeLast()"
    );
    assert_eq!(
        format_expr_ir_flutter(&method("rows", "get", &["0"])),
        "(rows)['0']!"
    );
    // Optional receiver: user?.len() -> user?.length
    let opt = ExprIR::MethodCallExpr {
        object: Box::new(ExprIR::IdentifierExpr {
            name: "user".into(),
            type_ann: None,
        }),
        method: "len".into(),
        args: vec![],
        is_optional: true,
    };
    assert_eq!(format_expr_ir_flutter(&opt), "user?.length");
}

/// String  coercion: 'x' + 5 -> 'x' + (5).toString(); a nested chain
/// keeps the tail operand coerced too.
#[test]
fn test_task58_string_concat_coercion() {
    let id = |n: &str| ExprIR::IdentifierExpr {
        name: n.into(),
        type_ann: None,
    };
    let add = |l: ExprIR, r: ExprIR| ExprIR::BinaryOpExpr {
        op: BinaryOpIR::Add,
        left: Box::new(l),
        right: Box::new(r),
    };
    let lit = |s: &str| ExprIR::LiteralExpr {
        value: serde_json::json!(s),
    };
    assert_eq!(
        format_expr_ir_flutter(&add(lit("x"), id("version"))),
        "'x' + (version).toString()"
    );
    // 'Hello ' + userName + ', ...' + version — the tail operand coerces.
    let chain = add(
        add(add(lit("Hello "), id("userName")), lit(", v")),
        id("version"),
    );
    assert!(format_expr_ir_flutter(&chain).ends_with("+ (version).toString()"));
}

/// Struct literals become named-parameter constructor calls.
#[test]
fn test_task58_struct_literal_constructor_form() {
    let expr = ExprIR::StructExpr {
        type_name: "StatisticsData".into(),
        fields: vec![
            (
                "name".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!("demo"),
                },
            ),
            (
                "count".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!(5),
                },
            ),
        ],
    };
    assert_eq!(
        format_expr_ir_flutter(&expr),
        "StatisticsData(name: 'demo', count: 5)"
    );
}

/// Event emits forward to the widget callback with a null-aware call.
#[test]
fn test_task58_event_emit_uses_widget_callback() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::EventEmitStmtIR {
        event_name: "onSwitch".into(),
        args: vec![ExprIR::LiteralExpr {
            value: serde_json::json!("overview"),
        }],
    };
    generate_statement_flutter(&mut w, &stmt);
    let out = w.into_string();
    assert!(
        out.contains("widget.onSwitch?.call('overview');"),
        "event emit must forward through the widget callback: {out}"
    );
}

/// println statements map to Dart's print.
#[test]
fn test_task58_println_maps_to_dart_print() {
    let expr = ExprIR::CallExpr {
        name: "println".into(),
        args: vec![ExprIR::LiteralExpr {
            value: serde_json::json!("hi"),
        }],
    };
    assert_eq!(format_expr_ir_flutter(&expr), "print('hi')");
}

/// measure() emits the DOMRect-parity wrapper.
#[test]
fn test_task58_measure_emits_wrapper() {
    let expr = ExprIR::RefOpExpr {
        ref_name: "box".into(),
        op: RefOpIR::Measure {
            what: MeasureWhat::Position,
        },
    };
    let out = format_expr_ir_flutter(&expr);
    assert!(
        out.starts_with("_omni_measure("),
        "measure must use the wrapper: {out}"
    );
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
        is_mut: false,
    };
    generate_statement_flutter(&mut w, &stmt);
    assert_eq!(w.into_string(), "final x = 42;\n");
}

#[test]
fn test_stmt_let_mut() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
        is_mut: true,
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
                is_mut: false,
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
                is_mut: false,
            }],
        }),
    };
    generate_statement_flutter(&mut w, &stmt);
    let out = w.into_string();
    assert!(out.contains("if (cond) {"));
    assert!(out.contains("final x = 1;"));
    assert!(out.contains("} else {"));
    assert!(out.contains("final x = 2;"));
    assert!(out.contains("}"));
}

// ── format_view_tree / format_element tests ───────────────

/// Format a view tree with an empty component/store context, returning
/// (formatted_string, warnings).
/// A component placeholder for ViewContext construction in tests (the
/// context's fn-arity lookups only matter for handler-wiring tests).
fn dummy_comp() -> ComponentIR {
    ComponentIR {
        form_intent: None,
        load_intent: vec![],
        route_intent: None,
        layout_intent: None,
        query_params: vec![],
        media_blocks: vec![],
        name: "Dummy".into(),
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        inline_stores: vec![],
        refs: vec![],
        events: vec![],
        slots: vec![],
        view: None,
        styles: vec![],
        source_file: None,
        source_span: None,
    }
}

fn fmt_view(node: &ViewTree) -> (String, Vec<crate::codegen::CodegenWarning>) {
    fmt_view_with_motions(node, &[])
}

/// Like [`fmt_view`] but with module-level motions available — needed by the
/// @drag-* extent derivation (deferral #19), which resolves the motion body
/// to compute the gesture extent from its keyframe range.
fn fmt_view_with_motions(
    node: &ViewTree,
    motions: &[MotionIntent],
) -> (String, Vec<crate::codegen::CodegenWarning>) {
    let ir = empty_program();
    fmt_view_in(node, motions, &ir)
}

/// Format a view tree with the given program (design blocks included) —
/// class:/variant: resolution (Task 4.3) needs the design IR.
fn fmt_view_with_program(
    node: &ViewTree,
    ir: &IRProgram,
) -> (String, Vec<crate::codegen::CodegenWarning>) {
    fmt_view_in(node, &[], ir)
}

/// Like [`fmt_view_with_program`] but with a component map (component
/// instantiation tests need the map to take the component-element path).
fn fmt_view_with_program_and_map<'a>(
    node: &ViewTree,
    ir: &'a IRProgram,
    component_map: &'a FxHashMap<&'a str, &'a ComponentIR>,
) -> (String, Vec<crate::codegen::CodegenWarning>) {
    let store_provider_map: FxHashMap<String, String> = FxHashMap::default();
    let mut warnings = Vec::new();
    let ref_usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver = DesignResolver::new(ir);
    let mut motion_counter = 0usize;
    let mut hoisted: Vec<(String, String)> = Vec::new();
    let mut hoist_next = 0usize;
    let mut ctx = ViewContext {
        component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir,
        design: &design_resolver,
        warnings: &mut warnings,
        file: None,
        motion_counter: &mut motion_counter,
        widget_names: &FxHashSet::default(),
        direct_write_signals: &FxHashSet::default(),
        errors: &mut Vec::new(),
        direct_write: &crate::codegen::shared::direct_write::DirectWriteSets::default(),
        fn_renames: &FxHashMap::default(),
        comp: &dummy_comp(),
        hoisted: &mut hoisted,
        hoist_next: &mut hoist_next,
        lazy_context: false,
        comp_type_scope: &FxHashMap::default(),
        load_builtins: &FxHashMap::default(),
        form_present: false,
        param_providers: &FxHashMap::default(),
    };
    let result = format_view_tree(node, &[], &mut ctx);
    (result, warnings)
}

fn fmt_view_in(
    node: &ViewTree,
    motions: &[MotionIntent],
    ir: &IRProgram,
) -> (String, Vec<crate::codegen::CodegenWarning>) {
    let component_map: FxHashMap<&str, &ComponentIR> = FxHashMap::default();
    let store_provider_map: FxHashMap<String, String> = FxHashMap::default();
    let mut warnings = Vec::new();
    let ref_usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let design_resolver = DesignResolver::new(ir);
    let mut motion_counter = 0usize;
    let mut hoisted: Vec<(String, String)> = Vec::new();
    let mut hoist_next = 0usize;
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir,
        design: &design_resolver,
        warnings: &mut warnings,
        file: None,
        motion_counter: &mut motion_counter,
        widget_names: &FxHashSet::default(),
        direct_write_signals: &FxHashSet::default(),
        errors: &mut Vec::new(),
        direct_write: &crate::codegen::shared::direct_write::DirectWriteSets::default(),
        fn_renames: &FxHashMap::default(),
        comp: &dummy_comp(),
        hoisted: &mut hoisted,
        hoist_next: &mut hoist_next,
        lazy_context: false,
        comp_type_scope: &FxHashMap::default(),
        load_builtins: &FxHashMap::default(),
        form_present: false,
        param_providers: &FxHashMap::default(),
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
        breakpoints: FxHashMap::default(),
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
    assert_eq!(expr, "const Text('Hello')");
}

#[test]
fn test_format_text_with_interpolation() {
    // `{`-leading TEXT is malformed-input fallback (parser interpolation
    // fallback) — rendered literally as plain text, never as Dart
    // interpolation code (Task 3.1 review CRITICAL round 2). Legit
    // interpolations arrive as ViewTree::Interpolation nodes.
    let node = ViewTree::Text("{name}".into());
    let (expr, _) = fmt_view(&node);
    assert_eq!(expr, "const Text('{name}')");
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container(child: const Text('Hello'))");
}

#[test]
fn test_drag_widget_derives_extent_from_keyframes() {
    // ADR-053 removed @drag-constraints — without constraints the drag extent
    // must be derived from the motion's own keyframe range (translateX
    // 0→300 ⇒ dragExtent 300.0) so the element follows its animation
    // (deferral #19) instead of dividing by Infinity.
    let motions = vec![MotionIntent {
        name: "slideX".into(),
        source_file: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        source_file: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
            .any(|w| w.message.contains("fadeOnly") && w.message.contains("literal translateX/Y")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr,
        "const Column(children: [const Text('First'), const Text('Second')])"
    );
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr,
        "Container(child: Column(children: [const Text('First'), const Text('Second')]))"
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(expr.contains("ElevatedButton("));
    // Task 5.8: FnRef handlers pass by tear-off (the closure form dropped
    // the event payload — `() { handleClick(); }` never forwarded
    // the argument and produced extra_positional_arguments).
    assert!(expr.contains("onPressed: handleClick"));
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Task 5.8: FnRef handlers pass by tear-off — a 0-arg fn is assignable
    // to VoidCallback (Dart function subtyping).
    assert!(
        expr.contains("onPressed: handleClick"),
        "VoidCallback handler must be a tear-off: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("VoidCallback")
                && w.message.contains("event field accesses")),
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
/// was emitted — a duplicate named argument). Task 3.3: the `onChanged`
/// required default must NOT render these bound controls disabled
/// (`onChanged: null` is gone) — the write-back closure is emitted instead
/// (`onChanged: (v) { setState(() { signal = v ?? signal; }); }`, the
/// `?? signal` fallback absorbing the nullable bool? callback param).
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
        content.contains(
            "Checkbox(onChanged: (v) { setState(() { active = v ?? active; }); }, value: active)"
        ),
        "bound value + write-back must both be present (no onChanged: null), got:\n{}",
        content
    );
    assert!(
        content.contains(
            "Switch(onChanged: (v) { setState(() { active = v ?? active; }); }, value: active)"
        ),
        "bound value + write-back must both be present (no onChanged: null), got:\n{}",
        content
    );
    assert!(
        content.contains("Slider(onChanged: (v) { setState(() { volume = v; }); }, value: volume)"),
        "bound value + write-back must both be present (no onChanged: null), got:\n{}",
        content
    );
    assert!(
        !content.contains("value: false, value:") && !content.contains("value: 0.0, value:"),
        "must not emit a duplicate value argument, got:\n{}",
        content
    );
    assert!(
        !content.contains("onChanged: null"),
        "bound controls must not render disabled, got:\n{}",
        content
    );
}

/// Task 3.3 (flutter HIGH): `bind:value` on a NON-ref'd input must not be
/// silently dropped — the write-back direction is wired through `onChanged`
/// (a setState signal assignment) so typing reaches the signal instead of
/// rendering a dead input (previously: binding warning + no callback at
/// all, the `onChanged: null` disabled-control form).
#[test]
fn bind_value_without_ref_emits_on_changed() {
    let source = r#"
component TextInputDemo {
    signal q: String = "";
    view {
        <input bind:value={q} />
    }
}
"#;
    let out = compile_flutter(source).unwrap();
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        !dart.contains("onChanged: null"),
        "disabled control: {}",
        dart
    );
    assert!(
        dart.contains("onChanged: (v)"),
        "missing write-back: {}",
        dart
    );
    assert!(
        dart.contains("q = v"),
        "signal write-back missing: {}",
        dart
    );
}

/// Review finding (task 3.3 follow-up): the write-back must target BARE
/// component signals only. A store-qualified target
/// (`bind:value={TodoStore.query}`) names a Notifier class / immutable
/// store state with no writable member — emitting the write-back would
/// produce uncompilable Dart (`setState(() { TodoStore.query = v; })`).
/// The shape must keep the original warning fallback (honest degradation,
/// ADR-054), never degrade a diagnosable warning into a raw analyzer error.
#[test]
fn bind_value_store_field_without_ref_warns_not_breaks() {
    let source = r#"
store TodoStore {
    signal query: String = "";
}
component StoreBindDemo {
    @store TodoStore;
    view {
        <input bind:value={TodoStore.query} />
    }
}
"#;
    let result = compile_flutter_full(source);
    assert!(
        result.success,
        "store-qualified bind:value must stay compilable (warning, not error): {:?}",
        result.errors
    );
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("TodoStore.query = v") && !content.contains("onChanged: (v)"),
        "write-back must not target a store field: {}",
        content
    );
    let warns: Vec<&str> = result.warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(
        warns
            .iter()
            .any(|w| w.contains("bind:value") && w.contains("skipped")),
        "store-qualified bind:value must warn (dropped), got: {:?}",
        warns
    );
}

/// Review finding (task 3.3 follow-up): the write-back must not fire for a
/// signal whose type cannot be assigned from the widget's onChanged payload.
/// A non-String signal on a TextField (`bind:value={count}` with an int
/// signal) would emit `count = v` with v: String — invalid_assignment.
/// Same fallback contract: warning, never uncompilable output.
#[test]
fn bind_value_type_mismatch_without_ref_warns_not_breaks() {
    let source = r#"
component IntBindDemo {
    signal count: i32 = 0;
    view {
        <input bind:value={count} />
    }
}
"#;
    let result = compile_flutter_full(source);
    assert!(
        result.success,
        "type-mismatched bind:value must stay compilable (warning, not error): {:?}",
        result.errors
    );
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("count = v") && !content.contains("onChanged: (v)"),
        "write-back must not fire for a String-callback / int-signal mismatch: {}",
        content
    );
    let warns: Vec<&str> = result.warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(
        warns
            .iter()
            .any(|w| w.contains("bind:value") && w.contains("skipped")),
        "type-mismatched bind:value must warn (dropped), got: {:?}",
        warns
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        warnings.iter().any(|w| w.message.contains("class 'active'")
            && w.message.contains("not declared by any design block")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // Static true class bindings resolve like static classes; an unknown
    // class warns and adds no wrapping Container.
    assert_eq!(expr, "Container()");
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("class 'active'")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // An unresolvable dynamic class leaves the element unwrapped (warned) —
    // the element itself is not const (class_map_bindings), the child text is.
    assert_eq!(
        expr, "Container(child: const Text('Hello'))",
        "unresolved class must not wrap the child: {}",
        expr
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("class 'active'")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
            w.message.contains("class 'RowLayout'")
                && w.message.contains("no decoration-mappable properties")
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
            .any(|w| w.message.contains("variant 'Nope'") && w.message.contains("not found")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let ir = design_test_program();
    let my_card = ComponentIR {
        form_intent: None,
        load_intent: vec![],
        route_intent: None,
        layout_intent: None,
        query_params: vec![],
        name: "MyCard".into(),
        view: None,
        ..Default::default()
    };
    let component_map: FxHashMap<&str, &ComponentIR> = FxHashMap::from_iter([("MyCard", &my_card)]);
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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

// ── T5.7: store state ==/hashCode + nullable-safe copyWith ──

/// A store with one non-nullable and one nullable signal. Nullable types
/// lower to `Option<T>` (the Dart formatter maps it to `T?`).
fn nullable_mixed_store() -> StoreIR {
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
        "note".into(),
        TypeIR {
            kind: "Option".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("count".into(), serde_json::json!(0));
    defaults.insert("note".into(), serde_json::Value::Null);
    StoreIR {
        name: "MixedStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    }
}

#[test]
fn test_store_state_equality_generated() {
    let codegen = FlutterCodegen;
    let store_dart =
        codegen.gen_store_dart("MixedStore", &nullable_mixed_store(), &IRProgram::default());
    assert!(
        store_dart.contains("bool operator ==(Object other)"),
        "state class must implement operator ==: {}",
        store_dart
    );
    assert!(
        store_dart.contains("runtimeType == other.runtimeType"),
        "== must guard on runtimeType: {}",
        store_dart
    );
    assert!(
        store_dart.contains("count == other.count &&") && store_dart.contains("note == other.note"),
        "== must compare all signal fields: {}",
        store_dart
    );
    assert!(
        store_dart.contains("int get hashCode => Object.hash(count, note)"),
        "hashCode must hash all fields via Object.hash: {}",
        store_dart
    );
}

#[test]
fn test_store_copy_with_sentinel_allows_nulling_nullable_fields() {
    let codegen = FlutterCodegen;
    let store_dart =
        codegen.gen_store_dart("MixedStore", &nullable_mixed_store(), &IRProgram::default());
    // Sentinel-based copyWith: `?? this.x` could never set a nullable field
    // back to null — the sentinel disambiguates "not passed" from "null".
    assert!(
        store_dart.contains("static const _unset = Object();"),
        "copyWith must use a _unset sentinel: {}",
        store_dart
    );
    assert!(
        store_dart.contains("Object? count = _unset")
            && store_dart.contains("Object? note = _unset"),
        "copyWith params must default to the sentinel: {}",
        store_dart
    );
    assert!(
        store_dart.contains("count: count == _unset ? this.count : count as int"),
        "non-nullable field must keep this.x when unset: {}",
        store_dart
    );
    assert!(
        store_dart.contains("note: note == _unset ? this.note : note as String?"),
        "nullable field must be settable to null via the sentinel: {}",
        store_dart
    );
    assert!(
        !store_dart.contains("?? this.note"),
        "the `?? this.x` form cannot null a nullable field and must be gone: {}",
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
        source_file: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };

    let view = ViewTree::Element(Box::new(el));

    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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

    // Field declarations — controllers are PER ELEMENT (T5.7: one controller
    // per motion NAME shared across elements made hovering one play both).
    // This element is the 0th motion element → `_fadeInController_0`.
    assert!(
        dart.contains("late final AnimationController _fadeInController_0;"),
        "should declare the per-element _fadeInController_0 field"
    );
    assert!(
        dart.contains("late final Animation<double> _fadeInOpacity_0;"),
        "should declare the per-element _fadeInOpacity_0 field: {}",
        dart
    );
    assert!(
        dart.contains("bool _fadeInActive_0 = false;"),
        "should declare the per-element active flag: {}",
        dart
    );

    // initState: AnimationController init
    assert!(
        dart.contains("_fadeInController_0 = AnimationController("),
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
        dart.contains("_fadeInOpacity_0 = Tween<double>(begin: 0.0, end: 1.0).chain(CurveTween(curve: Curves.easeOut)).animate(_fadeInController_0);"),
        "should create per-property Tween: {}",
        dart
    );

    // @enter → activate + forward() on mount
    assert!(
        dart.contains("_fadeInActive_0 = true;"),
        "should activate the motion on mount: {}",
        dart
    );
    assert!(
        dart.contains("_fadeInController_0.forward();"),
        "should call forward() for the @enter trigger: {}",
        dart
    );

    // AnimatedBuilder wrapping in build method — listens to the PER-ELEMENT
    // controller, reads per-property animation values, and gates the
    // wrappers on the per-element active flag (natural style while inactive)
    assert!(
        dart.contains("AnimatedBuilder("),
        "should emit AnimatedBuilder in build: {}",
        dart
    );
    assert!(
        dart.contains("animation: _fadeInController_0"),
        "AnimatedBuilder should listen to the per-element controller: {}",
        dart
    );
    assert!(
        dart.contains(
            "_fadeInActive_0 ? Opacity(opacity: _fadeInOpacity_0.value, child: child) : child ?? const SizedBox.shrink()"
        ),
        "builder body should gate per-property animation value on the per-element active flag: {}",
        dart
    );

    // dispose
    assert!(
        dart.contains("_fadeInController_0.dispose();"),
        "should dispose the per-element controller"
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
        source_file: None,
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
        // Per-element ordinal (T5.7): controllers are named per element
        // instance so hovering one element never plays another's animation.
        0,
    );

    assert!(
        result.contains("MouseRegion("),
        "should wrap with MouseRegion: {}",
        result
    );
    assert!(
        result.contains(
            "onEnter: (_) { _scaleOnHoverActive_0 = true; _scaleOnHoverController_0.forward(); }"
        ),
        "should activate and forward on mouse enter: {}",
        result
    );
    assert!(
        result.contains("onExit: (_) { _scaleOnHoverController_0.reverse().whenComplete(() { if (mounted && _scaleOnHoverController_0.value == 0.0) setState(() { _scaleOnHoverActive_0 = false; }); }); }"),
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
        source_file: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };

    let view = ViewTree::Element(Box::new(el));

    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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
        dart.contains("_popInController_0 = AnimationController("),
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
        dart.contains("_popInController_0.forward();"),
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
        dart.contains("Transform.scale(scale: _popInScale_0.value"),
        "should wrap with Transform.scale: {}",
        dart
    );
    assert!(
        dart.contains("Opacity(opacity: _popInOpacity_0.value"),
        "should wrap with Opacity: {}",
        dart
    );

    // dispose
    assert!(
        dart.contains("_popInController_0.dispose();"),
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
    assert_eq!(format_named_curve("elasticIn"), "Curves.elasticIn");
    assert_eq!(format_named_curve("elasticOut"), "Curves.elasticOut");
    assert_eq!(format_named_curve("elasticInOut"), "Curves.elasticInOut");

    // Unknown name → fallback
    assert_eq!(
        format_named_curve("nonexistent"),
        "Curves.linear",
        "unknown easing should fall back to Curves.linear"
    );
}

// ── T5.7: per-element motion controllers ─────────────────

#[test]
fn test_motion_two_elements_same_motion_get_separate_controllers() {
    // Two elements on the same motion NAME previously shared one controller —
    // hovering one played both. Each element instance gets its own controller
    // (React-style ordinal naming: `_{motion}Controller_{idx}`).
    let result = compile_flutter(
        "@motion fadeIn { from { opacity: 0 } to { opacity: 1 } over: 300ms }
         component App {
             view {
                 <div @enter={fadeIn}><p>First</p></div>
                 <div @enter={fadeIn}><p>Second</p></div>
             }
         }",
    )
    .expect("motion fixture must compile");
    let dart: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("late final AnimationController _fadeInController_0;")
            && dart.contains("late final AnimationController _fadeInController_1;"),
        "each motion element must declare its own controller: {}",
        dart
    );
    assert!(
        dart.contains("_fadeInController_0 = AnimationController(")
            && dart.contains("_fadeInController_1 = AnimationController("),
        "each controller must be created in initState: {}",
        dart
    );
    assert!(
        dart.contains("_fadeInActive_0 = true;") && dart.contains("_fadeInActive_1 = true;"),
        "each element's @enter must activate its own flag: {}",
        dart
    );
    assert!(
        dart.contains("_fadeInController_0.forward();")
            && dart.contains("_fadeInController_1.forward();"),
        "each element's @enter must play its own controller: {}",
        dart
    );
    assert!(
        dart.contains("animation: _fadeInController_0")
            && dart.contains("animation: _fadeInController_1"),
        "each AnimatedBuilder must listen to its own controller: {}",
        dart
    );
    assert!(
        dart.contains("_fadeInController_0.dispose();")
            && dart.contains("_fadeInController_1.dispose();"),
        "each controller must be disposed: {}",
        dart
    );
    // The old shared per-motion controller is gone
    assert!(
        !dart.contains("late final AnimationController _fadeInController;")
            && !dart.contains("_fadeInController.forward();"),
        "the shared per-motion controller must be gone: {}",
        dart
    );
}

#[test]
fn test_motion_unreferenced_and_unused_motions_are_pruned() {
    // Two motions are declared; App references only fadeIn; Quiet references
    // none. Only the referenced controller may be emitted (previously EVERY
    // module motion got a controller in EVERY component).
    let result = compile_flutter(
        "@motion fadeIn { from { opacity: 0 } to { opacity: 1 } over: 300ms }
         @motion slideIn { from { translateX: -50 } to { translateX: 0 } over: 300ms }
         component App {
             view { <div @enter={fadeIn}>Hello</div> }
         }
         component Quiet {
             view { <p>No motion here</p> }
         }",
    )
    .expect("motion fixture must compile");
    let app = result.files.get("App.dart").expect("App.dart generated");
    assert!(
        app.contains("_fadeInController_0"),
        "referenced motion must be emitted: {}",
        app
    );
    assert!(
        !app.contains("_slideInController"),
        "unreferenced motion must be pruned from App: {}",
        app
    );
    let quiet = result
        .files
        .get("Quiet.dart")
        .expect("Quiet.dart generated");
    assert!(
        !quiet.contains("AnimationController"),
        "motion-free component must declare no controllers: {}",
        quiet
    );
    assert!(
        !quiet.contains("TickerProviderStateMixin"),
        "motion-free component must not get the TickerProviderStateMixin: {}",
        quiet
    );
}

// ── T5.7: conservative const injection ──────────────────

#[test]
fn test_const_injected_on_literal_subtrees() {
    // Plain literal text nodes are always const
    let (expr, _) = fmt_view(&ViewTree::Text("Hello".into()));
    assert_eq!(expr, "const Text('Hello')");

    // A literal-only element subtree is const (children are const too —
    // nested const inside a const context is valid Dart) — BUT only for
    // widgets whose SDK constructor is const: Container is NOT
    // (const_with_non_const in this SDK), so a <div> stays non-const
    // while its literal text child is const.
    let mut el = props_el("div", vec![("padding", lit(serde_json::json!("16px")))]);
    el.children = vec![ViewTree::Text("Hi".into())];
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr,
        "Container(padding: EdgeInsets.all(16.0), child: const Text('Hi'))"
    );

    // A <p> with a single literal text child → const Text (Text IS const)
    let mut el = props_el("p", vec![]);
    el.children = vec![ViewTree::Text("Hello".into())];
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "const Text('Hello')");

    // A <column> with two literal text children → const Column (Column IS
    // const; the const context propagates to the children)
    let mut el = props_el("column", vec![]);
    el.children = vec![ViewTree::Text("A".into()), ViewTree::Text("B".into())];
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr,
        "const Column(children: [const Text('A'), const Text('B')])"
    );
}

#[test]
fn test_no_const_on_dynamic_subtrees() {
    // Interpolation child → not const
    let mut el = props_el("div", vec![]);
    el.children = vec![ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
        name: "count".into(),
        type_ann: None,
    }))];
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container(child: Text('${count}'))");

    // Event handler → not const
    let mut el = props_el("button", vec![]);
    el.events.insert(
        "click".into(),
        EventHandlerIR::FnRef {
            r#ref: "onClick".into(),
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        !expr.starts_with("const "),
        "event-bound elements must not be const: {}",
        expr
    );

    // Image.network has no const constructor → never const
    let el = props_el(
        "img",
        vec![("src", lit(serde_json::json!("https://example.com/x.png")))],
    );
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        !expr.starts_with("const "),
        "Image.network must never be const: {}",
        expr
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
    // Task C2b (#7): steps() easings map to the generated _OmniStepsCurve
    // class — no diagnostic anymore.
    assert!(
        easing_flutter_diagnostic("m", &Some(EasingIR::Named("steps(6,end)".into()))).is_none()
    );
    assert!(easing_flutter_diagnostic("m", &Some(EasingIR::Named("stepEnd".into()))).is_none());
}

#[test]
fn test_steps_curve_args() {
    assert_eq!(
        steps_curve_args("steps(6,end)"),
        Some(("6".to_string(), false))
    );
    assert_eq!(
        steps_curve_args("steps(4,start)"),
        Some(("4".to_string(), true))
    );
    assert_eq!(steps_curve_args("steps(2)"), Some(("2".to_string(), false)));
    assert_eq!(steps_curve_args("easeOut"), None);
    assert_eq!(steps_curve_args("steps(x)"), None);
    assert_eq!(
        format_named_curve("steps(6,end)"),
        "_OmniStepsCurve(6, start: false)"
    );
    assert_eq!(
        format_named_curve("steps(4,start)"),
        "_OmniStepsCurve(4, start: true)"
    );
    // Review MEDIUM-1: stepStart/stepEnd (and kebab forms) RESOLVE to
    // "steps(1,start|end)" in the shared easing table — the resolved form
    // must route through the steps-curve mapping too (the raw-name check
    // alone left them on Curves.linear while the class was emitted).
    assert_eq!(
        format_named_curve("stepStart"),
        "_OmniStepsCurve(1, start: true)"
    );
    assert_eq!(
        format_named_curve("stepEnd"),
        "_OmniStepsCurve(1, start: false)"
    );
    assert_eq!(
        format_named_curve("step-start"),
        "_OmniStepsCurve(1, start: true)"
    );
}

#[test]
fn test_prop_flutter_diagnostic() {
    assert!(prop_flutter_diagnostic("m", "opacity").is_none());
    assert!(prop_flutter_diagnostic("m", "translateX").is_none());

    let diag = prop_flutter_diagnostic("m", "width").unwrap();
    assert!(diag.contains("layout-dependent"), "width diag: {}", diag);

    // Task C2b (#7): skew composes into the Matrix4 transform — no diag.
    assert!(prop_flutter_diagnostic("m", "skewX").is_none());
    assert!(prop_flutter_diagnostic("m", "scaleX").is_none());
    assert!(prop_flutter_diagnostic("m", "rotateZ").is_none());

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
        source_file: None,
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
    let body = build_animated_builder_body(&motion, "popIn", 0);
    assert!(
        body.contains("Opacity(opacity: _popInOpacity_0.value"),
        "opacity wrapper: {}",
        body
    );
    assert!(
        body.contains("Transform.translate(offset: Offset(_popInTranslateX_0.value, 0.0)"),
        "translate wrapper: {}",
        body
    );
    assert!(
        body.contains("Transform.rotate(angle: _popInRotate_0.value"),
        "rotate wrapper: {}",
        body
    );
}

#[test]
fn test_build_animated_builder_body_color_and_size() {
    let motion = MotionIntent {
        name: "tint".into(),
        source_file: None,
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
    let body = build_animated_builder_body(&motion, "tint", 0);
    assert!(
        body.contains("Container(color: _tintBackgroundColor_0.value"),
        "backgroundColor wrapper: {}",
        body
    );
    assert!(
        body.contains("DefaultTextStyle.merge(style: TextStyle(color: _tintColor_0.value)"),
        "color wrapper (Task 3.9: must merge, not replace): {}",
        body
    );
    assert!(
        body.contains("SizedBox(width: _tintWidth_0.value"),
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
        source_file: None,
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
    let body = build_animated_builder_body(&motion, "soften", 0);
    assert!(
        body.contains(
            "Container(decoration: BoxDecoration(borderRadius: _softenBorderRadius_0.value)"
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
        body.contains("DefaultTextStyle.merge(style: TextStyle(fontSize: _softenFontSize_0.value"),
        "fontSize wrapper (Task 3.9: must merge, not replace): {}",
        body
    );
    assert!(
        body.contains("_softenActive_0 ?"),
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
        source_file: None,
        body: test_keyframe(&[
            (0.0, &[("fontSize", serde_json::json!(12))]),
            (1.0, &[("fontSize", serde_json::json!(16))]),
        ]),
        source_span: None,
    };
    let body = build_animated_builder_body(&motion, "growText", 0);
    assert_eq!(
        body,
        "_growTextActive_0 ? DefaultTextStyle.merge(style: TextStyle(fontSize: _growTextFontSize_0.value), \
         child: child ?? const SizedBox.shrink()) : child ?? const SizedBox.shrink()"
    );
}

#[test]
fn test_build_animated_builder_body_no_wrappers() {
    // Task C2b (#7): skewX now composes into a Matrix4 Transform — the
    // Flutter SDK has no Transform.skew, so the per-axis family builds
    // `Matrix4.identity()..setEntry(0, 1, tan(angle))`.
    let motion = MotionIntent {
        name: "skew".into(),
        source_file: None,
        body: test_keyframe(&[
            (0.0, &[("skewX", serde_json::json!(0))]),
            (1.0, &[("skewX", serde_json::json!(45))]),
        ]),
        source_span: None,
    };
    let body = build_animated_builder_body(&motion, "skew", 0);
    assert!(
        body.contains("Matrix4.identity()..setEntry(0, 1, tan(_skewSkewX_0.value))"),
        "skewX must compose into the Matrix4 transform, got: {body}"
    );

    // A motion with NO supported properties still produces a Widget-typed
    // body — the builder closure's `child` is `Widget?`.
    let none_motion = MotionIntent {
        name: "unsupported".into(),
        source_file: None,
        body: test_keyframe(&[
            (0.0, &[("blur", serde_json::json!(0))]),
            (1.0, &[("blur", serde_json::json!(10))]),
        ]),
        source_span: None,
    };
    assert_eq!(
        build_animated_builder_body(&none_motion, "unsupported", 0),
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
        source_file: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };

    let view = ViewTree::Element(Box::new(el));
    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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

    // Task C2b (#7): steps() easing → the generated _OmniStepsCurve class
    // (no warning anymore), with the class emitted in the component file.
    assert!(
        !warnings
            .iter()
            .any(|w| w.message.contains("steps() easing")),
        "steps() must no longer warn: {:?}",
        warnings
    );
    assert!(
        dart.contains("class _OmniStepsCurve extends Curve")
            && dart.contains("_OmniStepsCurve(6, start: false)"),
        "steps() must map to the emitted _OmniStepsCurve: {}",
        dart
    );
    // width → layout-dependent warning
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("'width'") && w.message.contains("layout-dependent")),
        "width warning: {:?}",
        warnings
    );

    // Per-property animations: opacity Tween + width Tween
    assert!(
        dart.contains("_pulseOpacity_0 = Tween<double>(begin: 0.0, end: 1.0)"),
        "opacity tween: {}",
        dart
    );
    assert!(
        dart.contains("_pulseWidth_0 = Tween<double>(begin: 100.0, end: 200.0)"),
        "width tween: {}",
        dart
    );

    // Playback: loop + delay → delayed repeat (mounted-guarded)
    assert!(
        dart.contains(
            "Future.delayed(Duration(milliseconds: 1000), () { if (mounted) { _pulseActive_0 = true; _pulseController_0.repeat(); } });"
        ),
        "delayed repeat: {}",
        dart
    );

    // Builder body wraps both properties
    assert!(
        dart.contains("SizedBox(width: _pulseWidth_0.value"),
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
        form_intent: None,
        load_intent: vec![],
        route_intent: None,
        layout_intent: None,
        query_params: vec![],
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
        form_intent: None,
        load_intent: vec![],
        route_intent: None,
        layout_intent: None,
        query_params: vec![],
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
            let _ = writeln!(snapshot_content, "=== {} ===", f.path);
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

/// Compile helper returning the full result (success + errors) so tests can
/// assert on diagnostics.
fn compile_flutter_full(source: &str) -> crate::InternalCompileResult {
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = crate::path_mapping::derive_path_mapping(&sources);
    crate::compile_multi_internal(
        sources,
        crate::codegen::Target::Flutter,
        pm,
        false,
        false,
        None,
    )
}

// ── Task 3.6: generic-element tap wrappers + multi-theme warning ──

/// Task C2b (#3): `@onEnter`/`@onLeave` on a generic element
/// (Container) wrap in a `MouseRegion(onEnter:/onExit:)` — the DOM
/// mouseenter/mouseleave analogue — instead of dropping with a warning.
#[test]
fn mouse_enter_leave_wraps_in_mouse_region() {
    let flutter = compile_flutter(
        r#"
component HoverDemo {
    signal hovering: bool = false;
    fn enter() { hovering = true; }
    fn exit() { hovering = false; }
    view {
        <div @onEnter={enter} @onLeave={exit}>x</div>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("MouseRegion(onEnter: (event) { enter(); }, onExit: (event) { exit(); }"),
        "mouseenter/mouseleave must wrap in MouseRegion(onEnter/onExit), got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'onHover'") || w.message.contains("'onMouseleave'")),
        "the mouse enter/leave events must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task 7: a MouseRegion-bound pointer handler (onHover) reading pointer
/// fields extracts `pointerId`/`buttons` directly from the Flutter
/// PointerEvent and derives x/y from localPosition (element-local
/// coordinates — offsetX/offsetY analogue).
#[test]
fn flutter_pointer_event_extracts_pointer_id_and_buttons() {
    let flutter = compile_flutter(
        r#"
component HoverDemo {
    signal lastId: i32 = 0;
    signal lastButtons: i32 = 0;
    signal hoverX: f64 = 0.0;
    view {
        <div @onHover={(e) => { lastId = e.pointerId; lastButtons = e.buttons; hoverX = e.x; }}>x</div>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("final pointerId = e.pointer"),
        "pointerId must map to the PointerEvent.pointer property (Flutter's unique \
         pointer id — the DOM pointerId analogue), got:\n{out}"
    );
    assert!(
        out.contains("final buttons = e.buttons;"),
        "buttons must map to the PointerEvent.buttons bitfield, got:\n{out}"
    );
    assert!(
        out.contains("final x = e.localPosition.dx;"),
        "x/y must derive from localPosition (element-local coordinates), got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("pointerId") || w.message.contains("buttons")),
        "real pointer-field mappings must not warn: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#3): `@onFocus`/`@onBlur` on a TextField merge into ONE
/// `Focus(onFocusChange:)` closure — the bool payload discriminates focus
/// gain vs loss.
#[test]
fn focus_blur_wraps_in_focus_on_focus_change() {
    let flutter = compile_flutter(
        r#"
component FocusDemo {
    fn focus() { }
    fn blur() { }
    view {
        <input placeholder="x" @onFocus={focus} @onBlur={blur}></input>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("Focus(onFocusChange: (v) { if (v) { focus(); } else { blur(); } }, child:"),
        "focus/blur must merge into one Focus(onFocusChange:), got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'onFocus'") || w.message.contains("'onBlur'")),
        "the focus/blur events must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#3): `@onKeyUp`/`@onKeyDown` with a zero-field handler wrap in
/// `Focus(onKeyEvent:)` under the Flutter contract `KeyEventResult
/// Function(FocusNode, KeyEvent)` — the closure returns
/// `KeyEventResult.ignored` so the event keeps propagating.
#[test]
fn key_event_wraps_in_focus_on_key_event() {
    let flutter = compile_flutter(
        r#"
component KeyDemo {
    signal count: i32 = 0;
    view {
        <div @onKeyUp={count = count + 1}>x</div>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("Focus(onKeyEvent: (node, event) {")
            && out.contains("return KeyEventResult.ignored;"),
        "key events must wrap in Focus(onKeyEvent:) with the KeyEventResult return, got:\n{out}"
    );
    // Review LOW-1: Focus.onKeyEvent fires for BOTH keydown and keyup —
    // an @onKeyUp handler must not run on presses (event-type guard).
    assert!(
        out.contains("if (event is! KeyUpEvent) { return KeyEventResult.ignored; }"),
        "the key wrapper must discriminate the event type, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'onKeyUp'")),
        "the key event must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#3): a param-bearing key handler (its `e.key`-style accesses
/// cannot read a raw KeyEvent) keeps its honest degradation warning.
#[test]
fn param_bearing_key_handler_stays_honest() {
    let flutter = compile_flutter_full(
        r#"
component KeyDemo2 {
    fn handleKey(e) { }
    view {
        <div @onKeyDown={handleKey}>x</div>
    }
}
"#,
    );
    assert!(
        flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("param-bearing handler 'handleKey'")),
        "the param-bearing key handler must warn, got: {:?}",
        flutter.warnings
    );
}

/// Review MEDIUM-1: the stepStart/stepEnd aliases (which RESOLVE to
/// steps(1,start|end) in the shared easing table) must emit the
/// _OmniStepsCurve — not a dead class + silent Curves.linear fallback.
#[test]
fn steps_alias_easing_emits_steps_curve() {
    let flutter = compile_flutter(
        r#"
@motion stepsAlias {
    from { opacity: 0 }
    to   { opacity: 1 }
    over: 800ms
    easing: stepStart
}

component StepsAliasDemo {
    view {
        <div @enter={stepsAlias}>x</div>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("_OmniStepsCurve(1, start: true)"),
        "stepStart must emit the steps curve, got:\n{out}"
    );
    assert!(
        !out.contains("_stepsAliasOpacity_0 = Tween<double>(begin: 0.0, end: 1.0).chain(CurveTween(curve: Curves.linear))"),
        "the alias must not fall back to Curves.linear, got:\n{out}"
    );
}

/// Review MEDIUM-3: a from-omitted scaleX keyframe synthesizes its begin
/// from the NATURAL 1.0 boundary (unscaled) — not 0.0 (collapsed).
#[test]
fn scale_x_missing_from_synthesizes_natural_one() {
    let motion = MotionIntent {
        name: "growX".into(),
        source_file: None,
        body: test_keyframe(&[(1.0, &[("scaleX", serde_json::json!(2.0))])]),
        source_span: None,
    };
    let tween = build_prop_tween_expr(&motion.body, "scaleX", false);
    assert!(
        tween.contains("begin: 1.0, end: 2.0"),
        "from-omitted scaleX must synthesize begin 1.0, got: {tween}"
    );
}

/// Review MEDIUM-2: an element with BOTH a ref binding and data-testid
/// must emit exactly ONE `key:` argument (the ref's GlobalKey wins; the
/// testid drop is an honest warning) — two key args would not compile.
#[test]
fn ref_and_data_testid_emit_single_key() {
    let flutter = compile_flutter(
        r#"
import omniview;
component RefTestIdDemo {
    ref box: HtmlElement;
    fn go() { box.focus(); }
    view {
        <div @ref={box} data-testid="card-a">x</div>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let key_count = out.matches("key: _").count() + out.matches("key: ValueKey(").count();
    assert_eq!(
        key_count, 1,
        "ref + data-testid must emit exactly ONE key arg, got:\n{out}"
    );
    assert!(
        flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'data-testid' is dropped")),
        "the testid drop must be an honest warning: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#7): per-axis transform motions (scaleX/rotateX/skewX...)
/// compose into a Matrix4 Transform — the Flutter SDK has no
/// per-axis transform constructors — and steps() easings map to the
/// emitted _OmniStepsCurve class.
#[test]
fn per_axis_motion_composes_matrix4_and_steps_curve() {
    let flutter = compile_flutter(
        r#"
@motion scaleXDemo {
    from { scaleX: 1 }
    to   { scaleX: 1.8 }
    over: 600ms
    easing: power2-out
}

@motion stepsMove {
    from { opacity: 0 }
    to   { opacity: 1 }
    over: 800ms
    easing: steps(4,end)
}

component MotionDemo {
    view {
        <div @enter={scaleXDemo}>x</div>
        <div @enter={stepsMove}>y</div>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("Matrix4.identity()..scaleByDouble(_scaleXDemoScaleX_0.value, 1.0, 1.0, 1.0)"),
        "scaleX must compose into the Matrix4 transform, got:\n{out}"
    );
    assert!(
        out.contains("class _OmniStepsCurve extends Curve")
            && out.contains("_OmniStepsCurve(4, start: false)"),
        "steps() must map to the emitted _OmniStepsCurve, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'scaleX'") || w.message.contains("steps()")),
        "scaleX/steps must no longer warn: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#5): a Text element with non-text children composes via
/// `Text.rich(TextSpan(children: [...]))` — text nodes become
/// TextSpan(text:), child widgets embed as WidgetSpan — instead of
/// degrading to a Column with a warning.
#[test]
fn mixed_content_text_composes_text_rich() {
    let flutter = compile_flutter(
        r#"
component RichDemo {
    signal result: String = "";
    view {
        <p>match 结果: <strong>{result}</strong></p>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("Text.rich(TextSpan(children: [TextSpan(text: 'match 结果:'), WidgetSpan(child: Container(child: Text('${result}')))]))"),
        "mixed content must compose Text.rich, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("non-text children")),
        "the Column degradation warning must be gone: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#8): Text inline `style: { color: ... }` maps into the
/// TextStyle wrapper (`style: TextStyle(color: ...)`) — hex, rgb() and
/// basic named colors convert; the mapping merges with prop-driven
/// TextStyle members.
#[test]
fn text_inline_color_maps_into_text_style() {
    let flutter = compile_flutter(
        r##"
component InlineStyleDemo {
    view {
        <p style: { color: "#f59e0b" }>x</p>
        <p fontSize="18" style: { color: "white" }>y</p>
    }
}
"##,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("style: TextStyle(color: Color(0xFFF59E0B))")
            && out.contains("style: TextStyle(fontSize: 18.0, color: Color(0xFFFFFFFF))"),
        "Text inline color must map into the TextStyle wrapper, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("inline style property 'color'")),
        "inline color on Text must no longer warn: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#9): a TextField whose `bind:value` shares the element with
/// a user @onChange keeps the two-way binding — the write-back MERGES into
/// the occupied onChanged handler instead of being dropped.
#[test]
fn bind_value_with_user_on_change_merges_write_back() {
    let flutter = compile_flutter(
        r#"
component MergeDemo {
    signal name: String = "";
    fn handleNameChange() { nameError = ""; }
    signal nameError: String = "";
    view {
        <input bind:value={name} @onChange={handleNameChange}></input>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("onChanged: (event) {  handleNameChange(); setState(() { name = event; }); }")
            || out.contains(
                "onChanged: (event) { handleNameChange(); setState(() { name = event; }); }"
            ),
        "the write-back must merge into the user's onChanged, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'bind:value'")),
        "bind:value with a user handler must no longer warn: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#4): `<textarea rows=N>` maps to TextField's `maxLines` as
/// an INT (a raw string would not type-check) and the DOM `max` maps to
/// `maxLength`.
#[test]
fn textarea_rows_and_max_map_to_int_params() {
    let flutter = compile_flutter(
        r#"
component RowsDemo {
    signal body: String = "";
    view {
        <textarea bind:value={body} rows="3"></textarea>
        <input type="number" max="120"></input>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("maxLines: 3") && out.contains("maxLength: 120"),
        "rows must map to maxLines and max to maxLength as ints, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'rows'") || w.message.contains("'max'")),
        "rows/max must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#4): `disabled` on button widgets (no `enabled` param in the
/// SDK) maps to the required param's null form — `onPressed: null` renders
/// the button disabled; `disabled={false}` emits nothing.
#[test]
fn button_disabled_maps_to_on_pressed_null() {
    let flutter = compile_flutter(
        r#"
component DisabledDemo {
    fn go() { }
    view {
        <button disabled @onPress={go}>Off</button>
        <button disabled={false} @onPress={go}>On</button>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("onPressed: null"),
        "disabled must map to onPressed: null, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'disabled'")),
        "disabled must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#4): `data-testid` maps to `key: ValueKey('...')` — the
/// Flutter testing hook. v4 终审 fix: the value is emitted through
/// `dart_string_literal` (shared `\`/`'`/`$` escaping) — the previous
/// `'`-only replace left `$`/`\` raw, so a testid containing `${expr}`
/// became executable Dart interpolation (injection class, same family as
/// the v4 1.2 rust HIGH text-path fix).
#[test]
fn data_testid_maps_to_value_key() {
    let flutter = compile_flutter(
        r#"
component TestIdDemo {
    view {
        <div data-testid="card-a">x</div>
        <input data-testid="name-field" placeholder="n"></input>
        <button data-testid="pay-$total"></button>
        <button data-testid="it's"></button>
        <button data-testid="a\\b"></button>
        <button data-testid="x-${alert(1)}"></button>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("key: ValueKey('card-a')") && out.contains("key: ValueKey('name-field')"),
        "data-testid must map to key: ValueKey('...'), got:\n{out}"
    );
    // v4 终审 fix: `$` → `\$` (a raw `$total` would interpolate the Dart
    // variable), `'` → `\'`, `\` → `\\`, and `${...}` must stay inert text.
    assert!(
        out.contains("key: ValueKey('pay-\\$total')"),
        "testid `$` must be escaped (Dart interpolation risk); got:\n{out}"
    );
    assert!(
        !out.contains("key: ValueKey('pay-$total')"),
        "testid must not emit raw `$` interpolation; got:\n{out}"
    );
    assert!(
        out.contains("key: ValueKey('it\\'s')"),
        "testid `'` must be escaped; got:\n{out}"
    );
    assert!(
        out.contains("key: ValueKey('a\\\\b')"),
        "testid `\\` must be escaped; got:\n{out}"
    );
    assert!(
        out.contains("key: ValueKey('x-\\${alert(1)}')"),
        "testid dollar-brace payload must stay inert text (escaped), not executable interpolation; got:\n{out}"
    );
    assert!(
        !out.contains("key: ValueKey('x-${alert(1)}')"),
        "testid must not emit raw dollar-brace interpolation; got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'data-testid'")),
        "data-testid must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#4): `title` on widgets without a title/tooltip param wraps
/// the element in a Tooltip — the DOM title attribute's Flutter analogue.
#[test]
fn title_maps_to_tooltip_wrapper() {
    let flutter = compile_flutter(
        r##"
component TitleDemo {
    fn go() { }
    view {
        <a href="#" title="Demo link" @onPress={go}>Click</a>
        <input title="Enter your age"></input>
    }
}
"##,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("Tooltip(message: 'Demo link', child:")
            && out.contains("Tooltip(message: 'Enter your age', child:"),
        "title must wrap in Tooltip(message:), got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'title'")),
        "title must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task C2b (#3): Image.network's `@onError` maps to the `errorBuilder`
/// builder — the handler fires on load failure, then a placeholder keeps
/// the widget tree valid (errorBuilder must return a Widget).
#[test]
fn image_on_error_maps_to_error_builder() {
    let flutter = compile_flutter(
        r#"
component ImgDemo {
    signal err: bool = false;
    view {
        <img src="https://example.com/x.png" @onError={err = true} alt="x" />
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("errorBuilder: (context, error, stackTrace) {")
            && out.contains("return const SizedBox.shrink();"),
        "img @onError must map to errorBuilder, got:\n{out}"
    );
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("'onError'")),
        "the error event must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task 3.6 (flutter HIGH): `@onPress` on a generic element (`div` →
/// Container) has no whitelisted callback — instead of silently dropping
/// the handler (ThemeSwitcher's theme cards emitted no tap target at all),
/// the rendered element must wrap in an InkWell/GestureDetector.
#[test]
fn generic_element_on_press_gets_tappable_wrapper() {
    let flutter = compile_flutter(
        r#"
component TapDemo {
    fn onCardPress() { }
    view {
        <div @onPress={onCardPress}>x</div>
    }
}
"#,
    )
    .expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("InkWell(") || out.contains("GestureDetector("),
        "no tappable wrapper: {}",
        out
    );
    assert!(
        out.contains(
            "GestureDetector(onTap: onCardPress, child: Container(child: const Text('x')))"
        ),
        "the wrapper must carry the tap handler around the rendered element, got: {}",
        out
    );
    // The handler is wired — the old 'not supported' drop is gone.
    assert!(
        !flutter
            .warnings
            .iter()
            .any(|w| w.message.contains("is not supported on 'Container'")),
        "the tap event must no longer be dropped: {:?}",
        flutter.warnings
    );
}

/// Task 3.6 (compiler MEDIUM, D-G): Flutter's native `ThemeMode` is
/// two-state — a design declaring more than light/dark must warn so extra
/// themes are never silently unreachable.
#[test]
fn multi_theme_flutter_warns() {
    let result = compile_flutter_full(r#"design AppDesign { @themes { light, dark, sepia } }"#);
    assert!(
        result
            .warnings
            .iter()
            .any(|d| d.message.contains("不可达") || d.message.contains("light/dark")),
        "missing multi-theme warning: {:?}",
        result.warnings
    );
}

// ── @for cross-target keyed contract (Task 5.5) ─────────────

/// T5.5: a non-scroll @for emits a `Column` (document flow), NOT an
/// unconditional `ListView.builder` (review CRITICAL: Flutter emitted
/// ListView.builder regardless of `[scroll]`).
#[test]
fn flutter_for_non_scroll_emits_column() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items { <div>{item}</div> }
    }
}
"#;
    let flutter = compile_flutter(source).unwrap();
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains(
            "Column(mainAxisSize: MainAxisSize.min, children: [for (final item in items)"
        ),
        "non-scroll @for must emit a Column, got:\n{dart}"
    );
    assert!(
        !dart.contains("ListView.builder"),
        "non-scroll @for must NOT emit ListView.builder, got:\n{dart}"
    );
}

/// T5.5: `key={expr}` → `KeyedSubtree(key: ValueKey(expr), ...)` per item —
/// the Column (non-scroll) path.
#[test]
fn flutter_for_key_expr_emits_value_key() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items key={item} { <div>{item}</div> }
    }
}
"#;
    let flutter = compile_flutter(source).unwrap();
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("KeyedSubtree(key: ValueKey(item), child:"),
        "key_expr must emit key: ValueKey(expr), got:\n{dart}"
    );
    assert!(
        !dart.contains("ListView.builder"),
        "keyed non-scroll @for must stay in a Column, got:\n{dart}"
    );
}

/// T5.5: `[scroll]` → ListView.builder dispatch, with `key: ValueKey(expr)`
/// on each item (the design doc 域 6 mapping row).
#[test]
fn flutter_for_scroll_emits_listview_builder_with_value_key() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items [scroll] key={item} { <div>{item}</div> }
    }
}
"#;
    let flutter = compile_flutter(source).unwrap();
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("ListView.builder("),
        "[scroll] @for must emit ListView.builder, got:\n{dart}"
    );
    assert!(
        dart.contains("key: ValueKey(item)"),
        "[scroll] keyed @for must carry key: ValueKey(expr) per item, got:\n{dart}"
    );
    // Task 3.8: the iterable is hoisted to a single-eval build local; both
    // itemCount and the itemBuilder reference it (O(N) instead of O(N²)).
    assert!(
        dart.contains("final __omniIter = items;"),
        "[scroll] iterable must be hoisted to a build local, got:\n{dart}"
    );
    assert!(
        dart.contains("itemCount: __omniIter.length")
            && dart.contains("var item = __omniIter[index];"),
        "[scroll] itemCount/itemBuilder must reference the hoisted local, got:\n{dart}"
    );
}

/// T5.5: nested @for (list inside list, non-scroll) must stay in document
/// flow — the inner list emits a nested Column, never a nested unbounded
/// ListView (design doc 域 6: 「@for 嵌套在可滚动父元素内 → Column」).
#[test]
fn flutter_nested_for_stays_in_document_flow() {
    let source = r#"
component MatrixPage {
    signal matrix: Array<Array<String>> = [];
    view {
        @for row in matrix {
            <div>@for cell in row { <span>{cell}</span> }</div>
        }
    }
}
"#;
    let flutter = compile_flutter(source).unwrap();
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("for (final row in matrix)"),
        "outer @for must iterate in document flow, got:\n{dart}"
    );
    assert!(
        dart.contains("for (final cell in row)"),
        "inner @for must iterate in document flow, got:\n{dart}"
    );
    assert!(
        !dart.contains("ListView.builder"),
        "nested non-scroll @for must not produce unbounded viewports, got:\n{dart}"
    );
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
        content.contains("late final Animation<double> _enterFadeUpOpacity_0;"),
        "opacity field: {}",
        content
    );
    assert!(
        content.contains("late final Animation<double> _enterFadeUpTranslateY_0;"),
        "translate field: {}",
        content
    );
    assert!(
        content.contains("late final Animation<Color> _multiStopDemoBackgroundColor_1;"),
        "color field: {}",
        content
    );

    // 2 stops → Tween; 3 stops → weight-proportional TweenSequence
    assert!(
        content.contains("_enterFadeUpOpacity_0 = Tween<double>(begin: 0.0, end: 1.0)"),
        "2-stop tween: {}",
        content
    );
    assert!(
        content.contains("_multiStopDemoOpacity_1 = TweenSequence<double>(["),
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
        content.contains("]).animate(CurvedAnimation(parent: _multiStopDemoController_1, curve: Curves.easeInOut));"),
        "color CurvedAnimation: {}",
        content
    );
    // rotate is declared in degrees → emitted as radians (360deg = 2π)
    assert!(
        content.contains("_spinLoopRotate_2 = Tween<double>(begin: 0.0, end: 6.283185307179586)"),
        "rotate radians: {}",
        content
    );

    // Playback: loop → repeat(); delay+fill backwards → active immediately,
    // delayed forward, deactivated (natural style) on completion
    assert!(
        content.contains("_spinLoopController_2.repeat();"),
        "loop: {}",
        content
    );
    assert!(
        content.contains("_fillBackwardsActive_3 = true;"),
        "fill backwards activation: {}",
        content
    );
    assert!(content.contains("Future.delayed(Duration(milliseconds: 500), () { if (mounted) { _fillBackwardsController_3.forward().whenComplete(() { if (mounted && _fillBackwardsController_3.value == 1.0) setState(() { _fillBackwardsActive_3 = false; }); }); } });"), "delay+fill: {}", content);

    // Builder body: per-property wrappers gated on the active flag
    assert!(
        content.contains("_enterFadeUpActive_0 ? Transform.translate(offset: Offset(0.0, _enterFadeUpTranslateY_0.value), child: Opacity(opacity: _enterFadeUpOpacity_0.value, child: child)) : child ?? const SizedBox.shrink()"),
        "enter body: {}",
        content
    );
    assert!(content.contains("_multiStopDemoActive_1 ? Container(color: _multiStopDemoBackgroundColor_1.value, child: Transform.scale(scale: _multiStopDemoScale_1.value, child: Opacity(opacity: _multiStopDemoOpacity_1.value, child: child))) : child ?? const SizedBox.shrink()"), "multi-property body: {}", content);

    // Task C2b (#7): steps() easing is now a real Curve — no warning, and
    // the component file carries the _OmniStepsCurve class.
    assert!(
        !output
            .warnings
            .iter()
            .any(|w| w.message.contains("steps() easing")),
        "steps must no longer warn: {:?}",
        output.warnings
    );
    assert!(
        content.contains("class _OmniStepsCurve extends Curve"),
        "steps must emit the _OmniStepsCurve class, got:\n{}",
        content
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
        content.contains("widget.header ?? const Text('Default Header')"),
        "Expected fallback-wrapped header render, got:\n{}",
        content
    );
}

/// `@slot header: String { ... }` is a deleted form (design D4): the parser
/// hard-errors with the migration hint — no deferral warning, no `String?`.
#[test]
fn slot_type_constraint_hard_error() {
    let source = r#"
component Badge {
    view {
        <div>
            @slot header: String { <span>Fallback</span> }
        </div>
    }
}
"#;
    let output = compile_flutter_full(source);
    assert!(
        !output.success,
        "typed slot must be a hard error, got success"
    );
    assert!(
        output
            .errors
            .iter()
            .any(|e| e.message.contains("类型约束与数据参数已删除")),
        "Expected deleted-typed-slot error, got: {:?}",
        output.errors
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
            "widget.header ?? Column(mainAxisSize: MainAxisSize.min, children: [const Text('Title'), const Text('Subtitle')])"
        ),
        "Expected Column-wrapped multi-node fallback, got:\n{}",
        content
    );
}

/// Consumer-side named-prop projections (`header={<h4/>}` — the only
/// consumer form since design D4) project into named constructor params at
/// the call site; plain children flow into the unnamed `children` param.
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
        <SlotBox
            header={<h4>Custom Header</h4>}
            body={<div><p>Body content</p></div>}
        >
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
        content.contains("header: const Text('Custom Header')"),
        "Expected named slot projected to header param, got:\n{}",
        content
    );
    assert!(
        content.contains("body: Container(child: const Text('Body content'))"),
        "Expected named slot projected to body param, got:\n{}",
        content
    );
    assert!(
        content.contains("children: const Text('Default child')"),
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
        consumer.contains("header: const Text('Custom Header')"),
        "Expected projected header param, got:\n{}",
        consumer
    );
    assert!(
        consumer.contains(
            "children: Column(mainAxisSize: MainAxisSize.min, children: [const Text('Body content'), const Text('Default child')])"
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
        <Box missing={<h4>Content</h4>}>
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
        content.contains("children: const Text('Content')"),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let view = ViewTree::Slot {
        name: None,
        fallback: vec![ViewTree::Element(Box::new(el))],
    };
    let usage = collect_motion_element_usage(&view, &FxHashMap::default());
    assert_eq!(
        usage.len(),
        1,
        "enter motion inside a slot fallback must be found"
    );
    assert_eq!(
        usage[0].motions,
        vec![("fadeIn".to_string(), true)],
        "the fallback element is element 0 referencing fadeIn with an @enter trigger"
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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

// ── C2b: unresolvable effect dep collapses condition and drops dead prev fields ──
//
// Regression (7.5b-L3 FilterBar): `@effect (searchText, setFilter)` — the
// store-action dep `setFilter` is not a component signal/prop, so the
// didUpdateWidget condition collapses to `if (true)` (Task 5.8a) and the
// resolvable `searchText` snapshot field would never be read — emitting
// `late String _prevsearchText;` triggered dart unused_field in the
// flutter-analyze gate. Both the field and the condition must be skipped.

#[test]
fn test_effect_unresolvable_dep_skips_dead_prev_fields() {
    let codegen = FlutterCodegen;
    let effect = EffectIntent {
        deps: vec!["searchText".into(), "setFilter".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::IdentifierExpr {
                    name: "log".into(),
                    type_ann: None,
                },
            }],
        },
    };
    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
            name: "App".into(),
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![SignalIntent {
                name: "searchText".into(),
                r#type: TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                initial_value: ExprIR::LiteralExpr {
                    value: serde_json::json!(""),
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

    assert!(
        !dart.contains("_prevsearchText"),
        "dead prev field must not be emitted: {}",
        dart
    );
    assert!(
        dart.contains("if (true) {"),
        "unresolvable dep collapses condition to if (true): {}",
        dart
    );
    assert!(
        !dart.contains("_prevsearchText != searchText"),
        "collapsed condition must not reference the snapshot field: {}",
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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
            form_intent: None,
            load_intent: vec![],
            route_intent: None,
            layout_intent: None,
            query_params: vec![],
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
        dart.contains("ref.watch(todoStoreProvider.select(((TodoStoreState s) => s.totalCount)))"),
        "getter read must become ref.watch select with the state-typed param \
         (Task 5.8a: a `dynamic` select closure made a collection-for over it \
         flag the next list element as dead_code): {}",
        dart
    );
    assert!(
        dart.contains(
            "ref.watch(todoStoreProvider.select(((TodoStoreState s) => s.filterStatus)))"
        ),
        "signal read must become ref.watch select with the state-typed param: {}",
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

// ── Task 3.7: ref.watch hoisted out of closures (Riverpod 3.0 scope) ────

/// Task 3.7 (flutter MEDIUM): a store read inside a `.where(...)` predicate
/// — `S.items.where(|x| x.contains(S.filter))` — lands in a LAZY closure.
/// `Iterable.where` evaluates the predicate when the iterable is consumed:
/// at build time for `{iter}.length` but at LAYOUT time inside a
/// ListView itemBuilder — where Riverpod 3.0 forbids `ref.watch` (watch
/// outside the build phase throws at runtime). The read must be hoisted to
/// a build local (`final __omniN = ref.watch(...)`) emitted ABOVE the
/// `.where` call; the closure then references the local.
#[test]
fn store_read_hoisted_out_of_filter_predicate() {
    let src = r#"
import omniview;
struct Todo { id: i32, text: String }
store S {
    signal items: Todo[] = [];
    signal filter: String = "";
}
component V {
    @store S;
    view {
        @for item in S.items.where(|x| x.contains(S.filter)) { <span>{item}</span> }
    }
}
"#;
    let out: String = compile_flutter(src)
        .expect("compile must succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let where_pos = out.find(".where(").unwrap();
    let watch_pos = out.find("ref.watch").unwrap();
    assert!(
        watch_pos < where_pos,
        "watch must be hoisted above predicate: {}",
        out
    );
    // The predicate is lazy — the region after `.where(` must contain NO
    // ref.watch at all (only the iterable read may watch, evaluated in build
    // scope before `.where`).
    let line_end = out[where_pos..]
        .find('\n')
        .map(|i| where_pos + i)
        .unwrap_or(out.len());
    assert!(
        !out[where_pos..line_end].contains("ref.watch"),
        "predicate region must not contain ref.watch: {}",
        out
    );
    assert!(
        out.contains("final __omni0 = ref.watch(sProvider.select(((SState s) => s.filter)))"),
        "predicate read must be hoisted to a build local: {}",
        out
    );
}

/// Task 3.7 ([scroll] path): `ListView.builder`'s itemBuilder executes at
/// LAYOUT time, after the build phase — NO `ref.watch` may appear anywhere
/// inside the itemBuilder body. Store reads in the item body are hoisted to
/// build locals; the itemBuilder closure references the locals by capture.
#[test]
fn scroll_item_builder_hoists_store_reads() {
    let src = r#"
import omniview;
store S {
    signal items: String[] = [];
    signal total: i32 = 0;
}
component V {
    @store S;
    view {
        @for item in S.items [scroll] {
            <span>{item}{S.total}</span>
        }
    }
}
"#;
    let out: String = compile_flutter(src)
        .expect("compile must succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let item_builder_pos = out
        .find("itemBuilder:")
        .unwrap_or_else(|| panic!("must emit ListView.builder with itemBuilder: {}", out));
    let item_builder_end = out.rfind('\n').unwrap_or(out.len());
    let body_region = &out[item_builder_pos..item_builder_end];
    assert!(
        !body_region.contains("ref.watch"),
        "itemBuilder body must not contain ref.watch (layout-phase watch \
         throws in Riverpod 3.0): {}",
        out
    );
    // The [scroll] iterable is referenced INSIDE the itemBuilder
    // (`var item = __omniIter[index];`) — its read must be hoisted too.
    assert!(
        out.contains("final __omni0 = ref.watch(sProvider.select(((SState s) => s.items)))"),
        "scroll iterable read must be hoisted (itemBuilder evaluates it at \
         layout): {}",
        out
    );
    // Task 3.8: the hoisted read feeds the single-evaluation iterable local
    // (`final __omniIter = __omni0;`) — the computed chain (none here, the
    // iterable is the bare read) is cached once per build.
    assert!(
        out.contains("final __omniIter = __omni0;"),
        "iterable must be hoisted to a single-eval build local: {}",
        out
    );
    // The item-body read must be hoisted as a build local too (its counter
    // slot is 2 — `__omniIter` occupies slot 1).
    assert!(
        out.contains("final __omni2 = ref.watch(sProvider.select(((SState s) => s.total)))"),
        "store read in item body must be hoisted to a build local: {}",
        out
    );
    // The item body must reference the locals instead of watching.
    assert!(
        body_region.contains("__omniIter") && body_region.contains("__omni2"),
        "item body must reference the hoisted locals: {}",
        out
    );
}

/// Task 3.8 (perf H2): a [scroll] @for with a COMPUTED iterable
/// (`items.filter(...)`) must evaluate the iterable exactly ONCE per build.
/// The old shape inlined it into `itemCount` AND the itemBuilder's
/// `var item = {iter}[index]` (layout-time, per item) — a plain Dart getter
/// re-evaluated N+1 times per build (O(N²) for filtered lists). The
/// iterable is hoisted to a build local (`final __omniIter = ...`) that all
/// references share; the itemBuilder closure captures it (same mechanism as
/// the 3.7 hoisted reads).
///
/// brief 适配说明：`<for [scroll] item in ...>` 与顶层 `view V {...}` 非法
/// （同 task-30 偏差：指令形式是 `@for item in ... [scroll]`、必须包在
/// component 内、需 import + store 语义）；`filter` 在 Dart 目标发射为
/// `.where(...).toList()`（ArrayMethodIR::Filter 映射），因此求值点计数
/// 锚定发射链 `items.where`。
#[test]
fn scroll_for_iterable_evaluated_once() {
    let src = r#"
import omniview;
component V {
    signal items: Array<i32> = [];
    view {
        @for item in items.filter(|x| x > 0) [scroll] { <span>{item}</span> }
    }
}
"#;
    let out: String = compile_flutter(src)
        .expect("compile must succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // iterable 表达式只能出现一次求值点（hoist 变量）
    let evals = out.matches("items.where").count();
    assert_eq!(
        evals, 1,
        "iterable evaluated {} times — O(N²): {}",
        evals, out
    );
    assert!(out.contains("final __omniIter"), "missing hoist: {}", out);
}

/// Task 3.8 (@empty path): with an @empty block the [scroll] shape would
/// inline the computed iterable a THIRD time (`iterable.isEmpty` in the
/// Builder wrapper) — the hoisted local must be shared there too.
#[test]
fn scroll_for_empty_wrapper_shares_hoisted_iterable() {
    let src = r#"
import omniview;
component V {
    signal items: Array<i32> = [];
    view {
        @for item in items.filter(|x| x > 0) [scroll] {
            <span>{item}</span>
        }
        @empty { <span>none</span> }
    }
}
"#;
    let out: String = compile_flutter(src)
        .expect("compile must succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let evals = out.matches("items.where").count();
    assert_eq!(
        evals, 1,
        "iterable evaluated {} times — O(N²): {}",
        evals, out
    );
    assert!(
        out.contains("__omniIter.isEmpty"),
        "@empty wrapper must reference the hoisted iterable: {}",
        out
    );
}

// ── Task 3.9: DefaultTextStyle.merge + IndexedStack page switcher ──────

/// Task 3.9 (flutter MEDIUM): a motion animating `color`/`fontSize` must
/// MERGE onto the inherited text style — the old
/// `DefaultTextStyle(style: TextStyle(color: ...))` REPLACED the whole
/// style, dropping the inherited size/weight/family while animating.
///
/// Brief fixture adapted per the 3.4 precedent: the bare `view V {...}`
/// shorthand and the element-level `@motion="m"` attribute have no parse
/// branch, and `from` must be paired with `to` — the @enter trigger shape
/// that exercises the wrapper is kept.
#[test]
fn motion_text_style_merges_inherited() {
    let out = compile_flutter(
        r#"import omniview;
component V { view { <div @enter={m}/> } } @motion m { from { color: red } to { color: blue } }"#,
    )
    .expect("motion fixture must compile");
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        out.contains("DefaultTextStyle.merge"),
        "must merge, not replace: {}",
        out
    );
}

/// Task 3.9 (flutter MEDIUM): a component-root @if group whose branches are
/// all component instances (the page-switch shape) emits an IndexedStack —
/// the old `cond ? child : SizedBox()` ternary unmounted the outgoing page
/// on every switch, dropping its scroll position/form input (the OmniView
/// runtime toggles visibility instead).
///
/// Brief fixture adapted per the 3.4 precedent: the bare `view App {...}`
/// shorthand does not parse — the branch components and the `page` signal
/// are declared explicitly.
#[test]
fn page_switcher_uses_indexed_stack() {
    let out = compile_flutter(
        r#"
component PageA { view { <div>Page A</div> } }
component PageB { view { <div>Page B</div> } }
component App {
    signal page: String = "a";
    view { @if page == "a" { <PageA/> } @else { <PageB/> } }
}
"#,
    )
    .expect("page-switcher fixture must compile");
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        out.contains("IndexedStack("),
        "tab switch unmounts state: {}",
        out
    );
}

/// Task 3.9: the page-switch shape requires EVERY branch AND the fallback
/// to be a component instance — a plain-content fallback (`@else { <div>...
/// }`) is not a page switch; converting would DROP the fallback content.
/// It must stay on the ternary path.
#[test]
fn page_switcher_keeps_non_component_fallback() {
    let out = compile_flutter(
        r#"
component PageA { view { <div>Page A</div> } }
component App {
    signal page: String = "a";
    view { @if page == "a" { <PageA/> } @else { <div>Fallback content</div> } }
}
"#,
    )
    .expect("conditional fixture must compile");
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        !out.contains("IndexedStack("),
        "plain-content fallback is not the page-switch shape: {}",
        out
    );
    assert!(
        out.contains("Fallback content"),
        "the non-component fallback must be kept on the ternary path: {}",
        out
    );
}

// ── Task 3.10: relative image paths + fontFamily stacks ──────

/// Task 3.10 (flutter LOW): a web-relative img `src` (`/dot.png`) thrown at
/// `Image.network` throws NetworkImageLoadException at runtime in a NATIVE
/// app (relative paths only resolve in the web harness). It must degrade to
/// `Image.asset` with an honest warning — never a silent network load that
/// fails at runtime.
#[test]
fn relative_image_path_warns_and_uses_asset_image() {
    let result = compile_flutter_full(
        r#"
component V {
    view { <img src="/dot.png"/> }
}
"#,
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|d| d.message.contains("Image.asset")),
        "relative path must warn honestly: {:?}",
        result.warnings
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("Image.asset('/dot.png'"),
        "relative path must emit Image.asset, not Image.network: {}",
        out
    );
}

/// Task 3.10: absolute http(s) URLs keep Image.network and stay silent —
/// the network-load path is the correct one for them.
#[test]
fn absolute_image_url_keeps_image_network_without_warning() {
    let result = compile_flutter_full(
        r#"
component V {
    view { <img src="https://example.com/dot.png"/> }
}
"#,
    );
    assert!(
        result
            .warnings
            .iter()
            .all(|d| !d.message.contains("Image.asset")),
        "absolute URL must not warn: {:?}",
        result.warnings
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("Image.network('https://example.com/dot.png'"),
        "absolute URL must keep Image.network: {}",
        out
    );
}

/// Task 3.10 (flutter LOW): a CSS font stack (`"system-ui, sans-serif"`) has
/// no single-font equivalent in Flutter's `fontFamily` string — the raw
/// stack silently falls back to the default font. The emitted constant must
/// carry the FIRST font and warn honestly.
#[test]
fn font_family_stack_uses_first_font_with_warning() {
    let result = compile_flutter_full(
        r#"
design App {
    token fontFamily: fontFamily = "system-ui, sans-serif";
}
"#,
    );
    assert!(
        result.warnings.iter().any(|d| d.message.contains("font")),
        "font stack must warn: {:?}",
        result.warnings
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("static const String fontFamily = 'system-ui';"),
        "the constant must carry only the first font: {}",
        out
    );
    assert!(
        !out.contains("'system-ui, sans-serif'"),
        "the raw stack must not reach the emitted constant: {}",
        out
    );
}

// ── Task 3.9 I-1: single-branch guard + index-chain fold ─────

/// Task 3.9 I-1 (3.10 顺手): a SINGLE-branch root-level @if with no @else is
/// NOT the page-switch shape — converting it to an IndexedStack would mount
/// the branch unconditionally (hidden when the condition is false), running
/// its initState/timers behind an offstage slot (the criterion-2
/// contradiction). It must stay on the ternary path.
#[test]
fn single_branch_conditional_stays_ternary_not_indexed_stack() {
    let out = compile_flutter(
        r#"
component PageA { view { <div>Page A</div> } }
component App {
    signal page: String = "a";
    view { @if page == "a" { <PageA/> } }
}
"#,
    )
    .expect("conditional fixture must compile");
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        !out.contains("IndexedStack("),
        "single-branch @if must NOT become an IndexedStack (the branch would be \
         mounted unconditionally): {}",
        out
    );
    assert!(
        out.contains("PageA("),
        "the conditional page must still render: {}",
        out
    );
}

/// Task 3.9 minor: 3+ branch @else-if chains fold into ONE index chain
/// (`a ? 0 : b ? 1 : c ? 2`) — every page gets its own mount slot.
#[test]
fn three_branch_page_switcher_folds_index_chain() {
    let out = compile_flutter(
        r#"
component PageA { view { <div>Page A</div> } }
component PageB { view { <div>Page B</div> } }
component PageC { view { <div>Page C</div> } }
component App {
    signal page: String = "a";
    view {
        @if page == "a" { <PageA/> }
        @else if page == "b" { <PageB/> }
        @else { <PageC/> }
    }
}
"#,
    )
    .expect("3-branch fixture must compile");
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        out.contains("IndexedStack("),
        "3-branch all-component @if must be a page switcher: {}",
        out
    );
    assert!(
        out.contains("? 0 :") && out.contains("? 1 :"),
        "the index chain must fold all branches: {}",
        out
    );
    assert!(
        out.contains("PageC("),
        "the @else fallback must stay mounted as the last slot: {}",
        out
    );
}

/// Task 3.7 (effect form): `@effect` bodies run from initState /
/// didUpdateWidget — OUTSIDE the build phase. Riverpod 3.0 forbids
/// `ref.watch` there; reads must be `ref.read` (event-time read). The
/// `_runEffect_N` method must contain no `ref.watch`.
#[test]
fn effect_body_uses_ref_read_not_watch() {
    let src = r#"
import omniview;
store S {
    signal filter: String = "";
}
component E {
    @store S;
    signal label: String = "";
    @effect(S.filter) {
        if S.filter != "" {
            label = S.filter;
        }
    }
    view {
        <span>{label}</span>
    }
}
"#;
    let out: String = compile_flutter(src)
        .expect("compile must succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let effect_pos = out
        .find("void _runEffect_0()")
        .unwrap_or_else(|| panic!("must emit _runEffect_0: {}", out));
    let effect_end = out
        .find("Widget build(")
        .unwrap_or_else(|| panic!("effect must precede build: {}", out));
    let effect_region = &out[effect_pos..effect_end];
    assert!(
        !effect_region.contains("ref.watch"),
        "effect body must use ref.read, not ref.watch (watch outside build \
         throws in Riverpod 3.0): {}",
        out
    );
    assert!(
        effect_region.contains("ref.read(sProvider.select(((SState s) => s.filter)))"),
        "effect body read must become ref.read: {}",
        out
    );
}

/// Task 3.7 (event-handler form): an inline handler's store READS become
/// `ref.watch` today — the handler runs at event time (outside build) and
/// the watch throws on click. Reads in handler bodies must be `ref.read`.
#[test]
fn inline_handler_uses_ref_read_not_watch() {
    let src = r#"
import omniview;
store S {
    signal count: i32 = 0;
}
component H {
    @store S;
    signal label: String = "";
    view {
        <button @onPress={() => { label = S.count.to_string(); }}/>
    }
}
"#;
    let out: String = compile_flutter(src)
        .expect("compile must succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("ref.watch"),
        "handler reads must use ref.read (watch at event time throws): {}",
        out
    );
    assert!(
        out.contains("ref.read(sProvider.select(((SState s) => s.count)))"),
        "handler read must become ref.read: {}",
        out
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        // Task C2b (#8): the basic CSS named colors ARE supported now —
        // use a design-token string (var(--...)) as the unmappable sample.
        inline_style: Some(serde_json::json!({ "color": "var(--primary)" })),
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
            .any(|w| w.message.contains("unsupported color value")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr, "Container()",
        "unmapped prop must not be silently included: {}",
        expr
    );
    assert!(
        warnings.iter().any(|w| w.message.contains("ariaLabel")
            && w.message.contains("not supported on 'Container'")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(expr, "Container()");
    assert!(
        !expr.contains("BoxDecoration"),
        "unknown classes must not emit any decoration stub: {}",
        expr
    );
    assert!(
        warnings.iter().any(|w| w.message.contains("class 'card'")
            && w.message.contains("not declared by any design block")),
        "unknown class must warn naming the class: {:?}",
        warnings
    );
}

/// Task 3.6: the click family on a generic element (Container) is no longer
/// dropped — the rendered element wraps in a `GestureDetector(onTap:)`
/// (ThemeSwitcher's theme cards were dead before). Events OUTSIDE the tap
/// family on whitelist-less widgets keep the warn-and-drop contract.
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
    let mut events2 = FxHashMap::default();
    events2.insert(
        "scroll".into(),
        EventHandlerIR::FnRef {
            r#ref: "handleScroll".into(),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert_eq!(
        expr, "GestureDetector(onTap: handleClick, child: Container())",
        "click on Container must wrap in a GestureDetector (task 3.6): {}",
        expr
    );
    assert!(
        !warnings
            .iter()
            .any(|w| w.message.contains("click")
                && w.message.contains("not supported on 'Container'")),
        "the tap-family wrap must not warn: {:?}",
        warnings
    );

    // Outside the tap family the old contract holds: warn and drop.
    let el2 = ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: events2,
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Component,
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr2, warnings2) = fmt_view(&ViewTree::Element(Box::new(el2)));
    assert_eq!(
        expr2, "Container()",
        "non-tap event on Container must still be dropped: {}",
        expr2
    );
    assert!(
        warnings2
            .iter()
            .any(|w| w.message.contains("scroll")
                && w.message.contains("not supported on 'Container'")),
        "non-tap event on unsupported widget should warn: {:?}",
        warnings2
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        // Task 5.8: FnRef handlers pass by tear-off.
        expr.contains("onPressed: handleClick"),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    // fontSize/fontWeight are TextStyle members — merged into a single style: arg,
    // with string literals converted to typed values (HIGH-4: `fontWeight: 'bold'`
    // was invalid Dart — TextStyle.fontWeight is FontWeight?, not String).
    assert!(
        expr.contains("style: TextStyle(")
            && expr.contains("fontSize: 16.0")
            && expr.contains("fontWeight: FontWeight.w700"),
        "Text style props must merge into TextStyle with TYPED values: {}",
        expr
    );
    assert!(
        !expr.contains("fontWeight: 'bold'"),
        "fontWeight must not stay a raw String literal: {}",
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

// ── T5.7: per-widget literal prop type conversion ──────────

/// Build a literal ExprIR value.
fn lit(value: serde_json::Value) -> ExprIR {
    ExprIR::LiteralExpr { value }
}

/// Build a minimal element with the given props.
fn props_el(tag: &str, props: Vec<(&str, ExprIR)>) -> ElementIR {
    ElementIR {
        class_map_bindings: FxHashMap::default(),
        tag: tag.into(),
        props: props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    }
}

#[test]
fn test_literal_prop_conversion_table() {
    // fontWeight string → FontWeight.*
    assert_eq!(
        convert_literal_prop_value("Text", "fontWeight", &lit(serde_json::json!("bold"))),
        Some("FontWeight.w700".into())
    );
    assert_eq!(
        convert_literal_prop_value("Text", "fontWeight", &lit(serde_json::json!("normal"))),
        Some("FontWeight.w400".into())
    );
    assert_eq!(
        convert_literal_prop_value("Text", "fontWeight", &lit(serde_json::json!("600"))),
        Some("FontWeight.w600".into())
    );
    assert_eq!(
        convert_literal_prop_value("Text", "fontWeight", &lit(serde_json::json!("w300"))),
        Some("FontWeight.w300".into())
    );
    // Unmappable weight → None (caller keeps the raw string + warns)
    assert_eq!(
        convert_literal_prop_value("Text", "fontWeight", &lit(serde_json::json!("bolder"))),
        None
    );
    // fontSize: CSS length string or number → double
    assert_eq!(
        convert_literal_prop_value("Text", "fontSize", &lit(serde_json::json!("16px"))),
        Some("16.0".into())
    );
    assert_eq!(
        convert_literal_prop_value("Text", "fontSize", &lit(serde_json::json!(14))),
        Some("14.0".into())
    );
    // color → Color(...)
    assert_eq!(
        convert_literal_prop_value("Container", "color", &lit(serde_json::json!("#ff0000"))),
        Some("Color(0xFFFF0000)".into())
    );
    // textAlign / overflow / fit → enum values
    assert_eq!(
        convert_literal_prop_value("Text", "textAlign", &lit(serde_json::json!("center"))),
        Some("TextAlign.center".into())
    );
    assert_eq!(
        convert_literal_prop_value("Text", "textAlign", &lit(serde_json::json!("left"))),
        Some("TextAlign.left".into())
    );
    assert_eq!(
        convert_literal_prop_value("Text", "overflow", &lit(serde_json::json!("hidden"))),
        Some("TextOverflow.clip".into())
    );
    assert_eq!(
        convert_literal_prop_value("Image.network", "fit", &lit(serde_json::json!("cover"))),
        Some("BoxFit.cover".into())
    );
    // lengths → double / EdgeInsets
    assert_eq!(
        convert_literal_prop_value("Container", "width", &lit(serde_json::json!("300px"))),
        Some("300.0".into())
    );
    assert_eq!(
        convert_literal_prop_value("Container", "padding", &lit(serde_json::json!("16px"))),
        Some("EdgeInsets.all(16.0)".into())
    );
    // Icon size → double
    assert_eq!(
        convert_literal_prop_value("Icon", "size", &lit(serde_json::json!(24))),
        Some("24.0".into())
    );
    // Non-literal expressions are never converted
    assert_eq!(
        convert_literal_prop_value(
            "Text",
            "fontWeight",
            &ExprIR::IdentifierExpr {
                name: "w".into(),
                type_ann: None
            }
        ),
        None
    );
    // Unmappable strings stay unconverted (caller decides)
    assert_eq!(
        convert_literal_prop_value("Text", "textAlign", &lit(serde_json::json!("middle"))),
        None
    );
}

#[test]
fn test_img_network_emits_positional_src_not_named_image() {
    // Task 3.10: an ABSOLUTE URL keeps Image.network (relative/asset paths
    // now degrade to Image.asset with a warning) — the positional-arg
    // mechanism under test stays on the network path.
    let el = props_el(
        "img",
        vec![
            (
                "src",
                lit(serde_json::json!("https://example.com/icon.png")),
            ),
            ("alt", lit(serde_json::json!("Settings icon"))),
        ],
    );
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.starts_with("Image.network('https://example.com/icon.png'"),
        "src must be the POSITIONAL arg — `Image.network(image: ...)` is invalid Dart: {}",
        expr
    );
    assert!(
        !expr.contains("image:"),
        "Image.network must not receive a named `image:` param: {}",
        expr
    );
    assert!(
        expr.contains("semanticLabel: 'Settings icon'"),
        "alt must map to semanticLabel: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "valid img props must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_img_named_image_prop_warns_and_is_skipped() {
    // A literal `image=` prop would emit `Image.network(image: ...)` — invalid
    // Dart (the whitelist must NOT contain `image`). It warns and is skipped.
    let el = props_el("img", vec![("image", lit(serde_json::json!("x.png")))]);
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        !expr.contains("image:"),
        "Image.network must not receive a named `image:` param: {}",
        expr
    );
    assert!(
        warnings.iter().any(|w| w.message.contains("'image'")),
        "unmappable image prop must warn: {:?}",
        warnings
    );
}

#[test]
fn test_icon_emits_icons_positional() {
    let el = props_el(
        "icon",
        vec![
            ("name", lit(serde_json::json!("home"))),
            ("size", lit(serde_json::json!(24))),
        ],
    );
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("Icon(Icons.home"),
        "icon name must map to the positional Icons.<name> arg: {}",
        expr
    );
    assert!(
        expr.contains("size: 24.0"),
        "icon size must be a typed double: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "known icon name must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_icon_unknown_name_warns_and_falls_back() {
    let el = props_el(
        "icon",
        vec![("name", lit(serde_json::json!("definitelyNotAnIcon")))],
    );
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("definitelyNotAnIcon")),
        "unmappable icon name must produce a diagnostic: {:?}",
        warnings
    );
    // The required positional `icon` must still be satisfied (Icon() with no
    // args is invalid Dart) — fall back to a visible placeholder icon.
    assert!(
        expr.contains("Icon(Icons.help_outline"),
        "unmappable icon name must fall back to a compile-able icon: {}",
        expr
    );
}

#[test]
fn test_container_color_prop_converted_to_color() {
    let el = props_el("div", vec![("color", lit(serde_json::json!("#ff0000")))]);
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("color: Color(0xFFFF0000)"),
        "color prop must convert to a Color literal — `color: '#ff0000'` is invalid Dart: {}",
        expr
    );
    assert!(
        warnings.is_empty(),
        "mappable color must not warn: {:?}",
        warnings
    );
}

#[test]
fn test_inline_style_color_respects_widget_whitelist() {
    // Task C2b (#8): `<p style: { color: "#f59e0b" }>` — Text has no
    // `color` param, so the inline color routes through the TextStyle
    // wrapper (`style: TextStyle(color: Color(0xFFF59E0B))`) instead of
    // emitting invalid Dart or warning.
    let mut el = props_el("p", vec![]);
    el.inline_style = Some(serde_json::json!({ "color": "#f59e0b" }));
    let (expr, warnings) = fmt_view(&ViewTree::Element(Box::new(el)));
    assert!(
        expr.contains("style: TextStyle(color: Color(0xFFF59E0B))"),
        "Text inline color must map into the TextStyle wrapper: {}",
        expr
    );
    assert!(
        !expr.contains("Text(color:"),
        "Text must not receive a bare color param: {}",
        expr
    );
    assert!(
        !warnings.iter().any(|w| w.message.contains("color")),
        "inline-style color on Text must not warn: {:?}",
        warnings
    );

    // Widgets whose whitelist DOES allow color keep the direct param.
    let mut div = props_el("div", vec![]);
    div.inline_style = Some(serde_json::json!({ "color": "#f59e0b" }));
    let (expr, _) = fmt_view(&ViewTree::Element(Box::new(div)));
    assert!(
        expr.contains("color: Color(0xFFF59E0B)"),
        "Container inline color stays a direct param: {}",
        expr
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
    // `@onPress={TodoStore.clear}` lowers to an InlineFn whose body contains a
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
    let mut motion_counter = 0usize;
    let mut hoisted: Vec<(String, String)> = Vec::new();
    let mut hoist_next = 0usize;
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver,
        warnings: &mut warnings,
        file: None,
        motion_counter: &mut motion_counter,
        widget_names: &FxHashSet::default(),
        direct_write_signals: &FxHashSet::default(),
        errors: &mut Vec::new(),
        direct_write: &crate::codegen::shared::direct_write::DirectWriteSets::default(),
        fn_renames: &FxHashMap::default(),
        comp: &dummy_comp(),
        hoisted: &mut hoisted,
        hoist_next: &mut hoist_next,
        lazy_context: false,
        comp_type_scope: &FxHashMap::default(),
        load_builtins: &FxHashMap::default(),
        form_present: false,
        param_providers: &FxHashMap::default(),
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
    // `@onPress={TodoStore.clear()}` (with parens) lowers the handler body to
    // a MethodCallExpr on the store identifier — same Riverpod rewrite.
    let _codegen = FlutterCodegen;
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
    let mut motion_counter = 0usize;
    let mut hoisted: Vec<(String, String)> = Vec::new();
    let mut hoist_next = 0usize;
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: &ref_usage,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver,
        warnings: &mut warnings,
        file: None,
        motion_counter: &mut motion_counter,
        widget_names: &FxHashSet::default(),
        direct_write_signals: &FxHashSet::default(),
        errors: &mut Vec::new(),
        direct_write: &crate::codegen::shared::direct_write::DirectWriteSets::default(),
        fn_renames: &FxHashMap::default(),
        comp: &dummy_comp(),
        hoisted: &mut hoisted,
        hoist_next: &mut hoist_next,
        lazy_context: false,
        comp_type_scope: &FxHashMap::default(),
        load_builtins: &FxHashMap::default(),
        form_present: false,
        param_providers: &FxHashMap::default(),
    };
    let expr = format_view_tree(&view, &[], &mut ctx);
    assert!(
        expr.contains("ref.read(todoStoreProvider.notifier).clear()"),
        "MethodCallExpr store action must rewrite to ref.read: {}",
        expr
    );
}

// ── Task 3.1: store action struct literals → typed constructors ──

/// CRITICAL (flutter): `items = [...items, { id: nextId, text, done: false }]`
/// in a store action must emit a typed `Todo(id: ...)` constructor call — NOT
/// a JSON-quoted Dart map `{ 'id': ... }`. The map literal is
/// `Map<String, Object>`, which makes the list `List<Object>` and breaks the
/// state's `copyWith` cast (`items as List<Todo>`) at runtime — the
/// TodoStore.add() TypeError that crashes the flagship demo's core loop.
#[test]
fn store_action_struct_literal_emits_constructor() {
    let src = r#"
        struct Todo { id: i32, text: String, done: bool }
        store TodoStore {
            signal todos: Todo[] = [];
            action add(nextId, text) { todos = [ { id: nextId, text, done: false }, ...todos ] }
        }
    "#;
    let out = compile_flutter(src);
    let dart: String = out
        .expect("flutter compile must succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !dart.contains(r"'id':"),
        "JSON-quoted keys in Dart: {}",
        dart
    );
    assert!(
        dart.contains("Todo(id:"),
        "expected typed constructor: {}",
        dart
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        form_intent: None,
        load_intent: vec![],
        route_intent: None,
        layout_intent: None,
        query_params: vec![],
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
    let mut motion_counter = 0usize;
    let mut hoisted: Vec<(String, String)> = Vec::new();
    let mut hoist_next = 0usize;
    let mut ctx = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver,
        warnings: &mut warnings,
        file: None,
        motion_counter: &mut motion_counter,
        widget_names: &FxHashSet::default(),
        direct_write_signals: &FxHashSet::default(),
        errors: &mut Vec::new(),
        direct_write: &crate::codegen::shared::direct_write::DirectWriteSets::default(),
        fn_renames: &FxHashMap::default(),
        comp: &dummy_comp(),
        hoisted: &mut hoisted,
        hoist_next: &mut hoist_next,
        lazy_context: false,
        comp_type_scope: &FxHashMap::default(),
        load_builtins: &FxHashMap::default(),
        form_present: false,
        param_providers: &FxHashMap::default(),
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
    let mut motion_counter = 0usize;
    let mut ctx2 = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: ref_usage2,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver2,
        warnings: &mut warnings2,
        file: None,
        motion_counter: &mut motion_counter,
        widget_names: &FxHashSet::default(),
        direct_write_signals: &FxHashSet::default(),
        errors: &mut Vec::new(),
        direct_write: &crate::codegen::shared::direct_write::DirectWriteSets::default(),
        fn_renames: &FxHashMap::default(),
        comp: &dummy_comp(),
        hoisted: &mut hoisted,
        hoist_next: &mut hoist_next,
        lazy_context: false,
        comp_type_scope: &FxHashMap::default(),
        load_builtins: &FxHashMap::default(),
        form_present: false,
        param_providers: &FxHashMap::default(),
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
    let mut motion_counter = 0usize;
    let mut ctx3 = ViewContext {
        component_map: &component_map,
        store_provider_map: &store_provider_map,
        ref_usage: ref_usage3,
        component_slots: &component_slots,
        ir: &ir,
        design: &design_resolver3,
        warnings: &mut warnings3,
        file: None,
        motion_counter: &mut motion_counter,
        widget_names: &FxHashSet::default(),
        direct_write_signals: &FxHashSet::default(),
        errors: &mut Vec::new(),
        direct_write: &crate::codegen::shared::direct_write::DirectWriteSets::default(),
        fn_renames: &FxHashMap::default(),
        comp: &dummy_comp(),
        hoisted: &mut hoisted,
        hoist_next: &mut hoist_next,
        lazy_context: false,
        comp_type_scope: &FxHashMap::default(),
        load_builtins: &FxHashMap::default(),
        form_present: false,
        param_providers: &FxHashMap::default(),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
            .any(|w| w.message.contains("searchbox") && w.message.contains("Semantics")),
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

/// a11y block (design doc 域 10) injection hardening — final T7.3 corpus
/// findings, both RED-verified before the fixes:
/// 1. the `label` path emitted raw Dart string interpolation (`${expr}` in
///    a `'...'` literal executes); 2. an unmappable `role` was embedded
///    verbatim in the degradation block comment, so a `*/` payload
///    terminated the comment and left code behind.
#[test]
fn test_a11y_label_dollar_escaped_and_role_comment_sanitized() {
    let codegen = FlutterCodegen;
    let el = ElementIR {
        tag: "input".to_string(),
        a11y: Some(A11yIR {
            label: Some("x${alert(1)}".into()),
            role: Some("x*/ alert(1) //".into()), // NOT in the Flutter subset
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    let view = ViewTree::Element(Box::new(el));
    let comp = ref_component("A11yInjectionDemo", &[], vec![], vec![], Some(view));
    let mut components = FxHashMap::default();
    components.insert("A11yInjectionDemo".into(), comp);
    let ir = make_ir(components);
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("A11yInjectionDemo")
        .unwrap();
    let component_map = build_component_map(&ir);
    let component_slots: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let (dart, warnings, errors) = codegen.gen_component_dart(
        "A11yInjectionDemo",
        comp_ref,
        &ir,
        &component_map,
        &component_slots,
    );
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    // 1. `$` must be escaped in the Dart label literal — a raw `${` would be
    //    interpolation (executable code), not content.
    assert!(
        dart.contains("label: 'x\\${alert(1)}'"),
        "label `$` must be escaped; got:
{}",
        dart
    );
    assert!(
        !dart.contains("label: 'x${alert(1)}'"),
        "label must not emit raw Dart interpolation; got:
{}",
        dart
    );
    // 2. The role must not terminate the degradation comment: `*/` is
    //    sanitized to `* /` inside the comment text.
    assert!(
        dart.contains("role 'x* / alert(1) //'"),
        "role `*/` must be sanitized inside the degradation comment; got:
{}",
        dart
    );
    assert!(
        !dart.contains("role 'x*/"),
        "raw role must not appear in the degradation comment; got:
{}",
        dart
    );
    // The unmappable role still warns on the T2.4 warnings channel.
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("role") && w.message.contains("Semantics")),
        "unmappable role must warn; got: {:?}",
        warnings
    );
}

/// v4 rust HIGH regression: `$` in raw text must be escaped in BOTH Flutter
/// text paths — the all-text concat (`Text('a$b')` construction,
/// view.rs:2193) and the rich-text TextSpan(text:) emission (view.rs:332)
/// were hand-written `'`-only replaces that skipped Dart `$`-escaping, so
/// `$total` / `${expr}` became Dart interpolation (silent wrong value /
/// executable expression; corpus point rawtext-multi). The `$total` shape
/// pins the `$ident` literal form the corpus is blind to (payloads only
/// break `${`, EXEC_PATTERNS only scans `alert(`-class code).
#[test]
fn text_dollar_escaped_in_concat_and_span() {
    // 拼接路径（all-text concat）：`价格 $total 元` 不得产生 Dart 插值。
    let flutter = compile_flutter(
        r#"
component App {
    view {
        <p>价格 $total 元</p>
    }
}
"#,
    )
    .expect("compile must succeed");
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("价格 \\$total 元"),
        "拼接路径未转义 $，got:\n{dart}"
    );
    assert!(
        !dart.contains("价格 $total"),
        "拼接路径不得发射裸 $total（Dart 插值），got:\n{dart}"
    );
    // TextSpan 路径（rich text）：混合内容段落走 Text.rich/TextSpan。
    let rich = compile_flutter(
        r#"
component App {
    view {
        <p>价格 $total <b>x</b></p>
    }
}
"#,
    )
    .expect("compile must succeed");
    let rich_dart: String = rich.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        rich_dart.contains("价格 \\$total"),
        "TextSpan 路径未转义 $，got:\n{rich_dart}"
    );
    assert!(
        !rich_dart.contains("价格 $total"),
        "TextSpan 路径不得发射裸 $total（Dart 插值），got:\n{rich_dart}"
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        warnings
            .iter()
            .any(|w| w.message.contains("alt 仅对 img 元素生效")),
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
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
    // Dep-scoped re-apply (T5.6): the value binding snapshots the bound
    // value and re-applies only on change — and skips the controller write
    // when it already holds the text (typing must not reset the caret).
    assert!(
        dart.contains("_prevValueInputBox")
            && dart.contains("syncValueInputBox != _prevValueInputBox"),
        "_syncSemanticBindings must compare against the previous value. Got:\n{}",
        dart
    );
    assert!(
        dart.contains("_inputBoxText.text != syncValueInputBox")
            && dart.contains("_syncingInputBoxText = true;")
            && dart.contains("_inputBoxText.text = syncValueInputBox;"),
        "_syncSemanticBindings must re-apply the signal to the controller \
         (guarded against echo). Got:\n{}",
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

    // Reactive sync method re-applying all binding kinds only when the
    // bound value CHANGED (T5.6 dep-scoped — parity with React's
    // useEffect-on-deps without re-yanking focus on unrelated rebuilds)
    assert!(dart.contains("void _syncSemanticBindings() {"));
    assert!(dart.contains("if (syncFocusInputBox != _prevFocusInputBox) {"));
    assert!(dart.contains("_inputBoxFocus.requestFocus();"));
    assert!(dart.contains("_inputBoxFocus.unfocus();"));
    assert!(dart.contains(
        "_paneScroll.animateTo(syncScrollYPane, duration: Duration(milliseconds: 300), curve: Curves.easeInOut);"
    ));
    assert!(dart.contains("if (syncScrollYPane != _prevScrollYPane) {"));
    assert!(dart.contains("_syncingInputBoxText = true;"));
    assert!(dart.contains("_inputBoxText.text = syncValueInputBox;"));
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

    // Measure — width / height / position. T5.7: the `as RenderBox?` cast
    // THROWS a TypeError for non-RenderBox render objects (RenderViewport,
    // ...) — the safe `is RenderBox ? box : null` pattern degrades to null
    // instead (matching React's `?.` semantics).
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::Measure {
                what: MeasureWhat::Width
            }
        )),
        "(_boxKey.currentContext?.findRenderObject() is RenderBox ? _boxKey.currentContext?.findRenderObject() as RenderBox : null)?.size.width ?? 0"
    );
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::Measure {
                what: MeasureWhat::Height
            }
        )),
        "(_boxKey.currentContext?.findRenderObject() is RenderBox ? _boxKey.currentContext?.findRenderObject() as RenderBox : null)?.size.height ?? 0"
    );
    assert_eq!(
        format_expr_ir_flutter(&op(
            "box",
            RefOpIR::Measure {
                what: MeasureWhat::Position
            }
        )),
        // Task 5.8: measure() emits the DOMRect-parity wrapper (the
        // language surface is x/y/width/height; a raw Offset has dx/dy).
        "_omni_measure((_boxKey.currentContext?.findRenderObject() is RenderBox ? _boxKey.currentContext?.findRenderObject() as RenderBox : null))"
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
        messages.iter().any(|m| {
            m.contains("Flutter 不支持指针捕获")
                && m.contains("GestureDetector")
                // ADR-073 decision 3: the hard error survives but the hint
                // points at the semantic replacement — @drag compiles to
                // the GestureDetector onPan wiring (D3 evolution).
                && m.contains("@drag")
        }),
        "expected the D3 hint pointing at @drag (ADR-073 decision 3), got: {:?}",
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
    let _content: String = output
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
fn body_slot_without_fallback_renders_bare() {
    let source = r#"
component Badge {
    @slot footer
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
        "expected nullable-guarded slot render, got:\n{}",
        content
    );
    assert!(
        content.contains("final Widget? footer;"),
        "slot param stays Widget? (parent-provided content), got:\n{}",
        content
    );
}

// ═══════════════════════════════════════════════════════════════
// Event field extraction diagnostics (Task 2.4 — Phase 2 Domain 7)
// ═══════════════════════════════════════════════════════════════

/// Extract a single field with a fresh warnings channel.
fn extract_with_warnings(
    kind: &EventDataKind,
    field: &str,
) -> (String, Vec<crate::codegen::CodegenWarning>) {
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
            .any(|w| w.message.contains("text") && w.message.contains("ClipboardEvent")),
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
            .any(|w| w.message.contains("files") && w.message.contains("DropEvent")),
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
            .any(|w| w.message.contains("text") && w.message.contains("DropEvent")),
        "drop text stub must warn with field + kind, got: {:?}",
        text_warnings
    );
    assert!(text_expr.contains("/*"), "degradation comment: {text_expr}");

    // The remaining silent-constant stubs.
    let (tilt_y, tilt_warnings) = extract_with_warnings(&EventDataKind::PointerEvent, "tiltY");
    assert!(
        tilt_warnings
            .iter()
            .any(|w| w.message.contains("tiltY") && w.message.contains("PointerEvent")),
        "tiltY stub must warn, got: {:?}",
        tilt_warnings
    );
    assert!(tilt_y.contains("/*"), "degradation comment: {tilt_y}");
    let (touches, touch_warnings) = extract_with_warnings(&EventDataKind::TouchEvent, "touches");
    assert!(
        touch_warnings
            .iter()
            .any(|w| w.message.contains("touches") && w.message.contains("TouchEvent")),
        "touches stub must warn, got: {:?}",
        touch_warnings
    );
    assert!(touches.contains("/*"), "degradation comment: {touches}");
    let (delta_mode, dm_warnings) = extract_with_warnings(&EventDataKind::WheelEvent, "deltaMode");
    assert!(
        dm_warnings
            .iter()
            .any(|w| w.message.contains("deltaMode") && w.message.contains("WheelEvent")),
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
            .any(|w| w.message.contains("values") && w.message.contains("SubmitEvent")),
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

/// Task 7: ScrollEvent.progress maps from ScrollMetrics — the SAME source
/// as x/y (metrics.pixels): `extentBefore / maxScrollExtent` with a 0
/// fallback when the extent is unreachable (maxScrollExtent == 0, content
/// fits — no scrolling). Replaces the Task 6 degradation placeholder.
#[test]
fn scroll_progress_maps_from_scroll_metrics() {
    let (expr, warnings) = extract_with_warnings(&EventDataKind::ScrollEvent, "progress");
    assert!(
        expr.contains("extentBefore") && expr.contains("maxScrollExtent"),
        "progress must map from ScrollMetrics extentBefore/maxScrollExtent, got: {expr}"
    );
    assert!(
        expr.contains("0.0"),
        "unreachable extent (maxScrollExtent == 0) must fall back to 0.0: {expr}"
    );
    assert!(
        warnings.is_empty(),
        "real mapping must not warn: {:?}",
        warnings
    );
    assert!(
        !expr.contains("/*"),
        "real mapping must not carry a degradation comment: {expr}"
    );
}

/// Task 7: PointerEvent.pointerId/buttons map directly from the Flutter
/// PointerEvent. `pointer` is the unique pointer id (Flutter names the
/// DOM-pointerId analogue `pointer`, NOT `pointerId` — the inline comment
/// documents the naming difference, like the pointerType arm); `buttons`
/// is the bitfield exposed as-is. x/y stay on localPosition (element-local
/// coordinates).
#[test]
fn pointer_event_extracts_pointer_id_and_buttons() {
    let (pid, pid_warnings) = extract_with_warnings(&EventDataKind::PointerEvent, "pointerId");
    assert!(
        pid.contains("e.pointer"),
        "pointerId must map to PointerEvent.pointer (Flutter's unique pointer id), got: {pid}"
    );
    assert!(
        pid_warnings.is_empty(),
        "mapped pointerId must not warn: {:?}",
        pid_warnings
    );
    let (btns, btns_warnings) = extract_with_warnings(&EventDataKind::PointerEvent, "buttons");
    assert!(
        btns.contains("e.buttons"),
        "buttons must map to the PointerEvent.buttons bitfield, got: {btns}"
    );
    assert!(
        btns_warnings.is_empty(),
        "mapped buttons must not warn: {:?}",
        btns_warnings
    );
    assert!(
        !btns.contains("/*"),
        "buttons (property name matches) must not carry a comment: {btns}"
    );
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
            .any(|w| w.message.contains("files") && w.message.contains("DropEvent")),
        "dropped 'files' must warn through the map emitter: {:?}",
        warnings
    );
    assert!(
        !warnings.iter().any(|w| w.message.contains("field 'x'")),
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
            "OmniScrollDrivenWidget(
  start: 0,
  end: 1000,"
        ),
        "expected typed start/end constructor args, got:
{}",
        &out[..out.len().min(1200)]
    );
    assert!(
        out.contains("class OmniScrollDrivenWidget extends StatefulWidget"),
        "scroll widget class must be emitted (in the shared runtime)"
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
                 <div @drag-horizontal constraints: [-100, 100] elastic: 0.5 threshold: 0.3 ={dragX}>drag me</div>
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
    // v4 5.8: velocity_max removed — the drag-velocity tracker (and its
    // velocityMax constructor arg) is gone with it.
    assert!(
        !out.contains("velocityMax"),
        "velocityMax must be removed from the widget, got:
{}",
        &out[..out.len().min(1200)]
    );
    assert!(
        !out.contains("_velocity"),
        "the velocity tracker must be pruned, got:
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
            "OmniScrollDrivenWidget(
  start:"
        ),
        "no typed start/end args when sub-params absent, got:
{}",
        &out[..out.len().min(800)]
    );
}

/// T5.6: the `_Omni*` trigger helper classes are emitted ONCE into the shared
/// `_omniview_runtime.dart` and imported by every component that uses
/// @in-view/@scroll/@drag triggers — the review MEDIUM flagged the helpers
/// being re-emitted verbatim into every component file (copy-paste divergence
/// across a multi-component app).
#[test]
fn trigger_helpers_shared_runtime_single_emission_and_imports() {
    let result = compile_flutter(
        "@motion fadeA { from { opacity: 0.3 } to { opacity: 1 } over: 500ms }
         @motion fadeB { from { opacity: 0.3 } to { opacity: 1 } over: 500ms }
         @motion drift { from { translateX: 0 } to { translateX: 100 } over: 300ms }
         component First {
             view {
                 <div @in-view={fadeA}>first</div>
             }
         }
         component Second {
             view {
                 <div @scroll start: 0 end: 1000 ={fadeB}>second</div>
                 <div @drag-horizontal={drift}>drag</div>
             }
         }
         component Plain {
             view {
                 <div>plain</div>
             }
         }",
    )
    .expect("trigger fixture must compile");
    let files = &result.files;
    // The shared runtime file exists — one copy for the whole program.
    let runtime = files.get("_omniview_runtime.dart").unwrap_or_else(|| {
        panic!(
            "shared trigger runtime missing; files: {:?}",
            files.keys().collect::<Vec<_>>()
        )
    });
    // Each helper class appears exactly once, in the shared file.
    for cls in [
        "class OmniInViewWidget",
        "class OmniScrollDrivenWidget",
        "class OmniDragWidget",
    ] {
        assert_eq!(
            runtime.matches(cls).count(),
            1,
            "'{cls}' must be emitted exactly once in the shared runtime"
        );
    }
    // Trigger-using components import the runtime instead of inlining ~90
    // lines of helper per file.
    let first = files.get("First.dart").unwrap();
    assert!(
        first.contains("import './_omniview_runtime.dart';"),
        "First must import the shared runtime, got:\n{first}"
    );
    assert!(
        !first.contains("class OmniInViewWidget"),
        "no inline helper duplication may remain in First, got:\n{first}"
    );
    let second = files.get("Second.dart").unwrap();
    assert!(
        second.contains("import './_omniview_runtime.dart';"),
        "Second must import the shared runtime, got:\n{second}"
    );
    assert!(
        !second.contains("class OmniScrollDrivenWidget")
            && !second.contains("class OmniDragWidget"),
        "no inline helper duplication may remain in Second, got:\n{second}"
    );
    // Components without triggers do not import the runtime.
    let plain = files.get("Plain.dart").unwrap();
    assert!(
        !plain.contains("_omniview_runtime.dart"),
        "Plain has no triggers and must not import the runtime, got:\n{plain}"
    );
}

/// Task 3.4 (flutter HIGH): the @in-view trigger must INSTANTIATE the
/// OmniInViewWidget wrapper with its trigger config — previously the helper
/// class was only ever imported, never constructed, so the AnimationController
/// never started and the animation silently never played.
///
/// Brief fixture adapted per the 2.9/2.10 precedent: the bare `view V {...}`
/// shorthand does not parse (component wrapper required) and the
/// element-level `@motion="m"` attribute has no parse_element branch (hard
/// error), so it is dropped — the assertion names and failure messages are
/// kept verbatim from the brief.
#[test]
fn in_view_trigger_instantiates_wrapper() {
    let src = r#"component V { view { <div @in-view={m}/> } } @motion m { from { opacity: 0 } to { opacity: 1 } }"#;
    let out = compile_flutter(src).unwrap();
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        out.contains("OmniInViewWidget("),
        "wrapper never instantiated: {}",
        out
    );
    assert!(
        out.contains("inViewOnce") || out.contains("inViewThreshold"),
        "trigger config missing: {}",
        out
    );
}

/// Task 3.4 (flutter HIGH): the @focus trigger must be WIRED — the element
/// is wrapped in `Focus(onFocusChange:)` so gaining focus plays the motion
/// (previously the trigger had zero listeners in the Flutter backend and
/// the animation silently never played).
#[test]
fn focus_trigger_wraps_with_focus_widget() {
    let src = r#"component V { view { <input @focus={m}/> } } @motion m { from { opacity: 0 } to { opacity: 1 } }"#;
    let out = compile_flutter(src).unwrap();
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        out.contains("Focus(\n  onFocusChange: (focused)"),
        "focus trigger must emit a Focus wrapper with onFocusChange, got:\n{}",
        out
    );
    assert!(
        out.contains(".forward();"),
        "focus gain must start the motion controller, got:\n{}",
        out
    );
}

/// T5.6: trigger configurations are typed — the runtime-parsed legacy string
/// paths ('top 80%' viewport configs, the drag `constraints: 'left:0 right:300'`
/// string) are gone from the emitted code; the legacy scroll fallback is
/// compiled to typed constants.
#[test]
fn trigger_widget_configs_typed_no_runtime_string_parsing() {
    let result = compile_flutter(
        "@motion fadeScroll { from { opacity: 0.3 } to { opacity: 1 } over: 500ms }
         @motion dragX { from { translateX: 0 } to { translateX: 300 } over: 300ms }
         component Test {
             view {
                 <div @scroll start: 0 end: 1000 scrub: true ={fadeScroll}>scroll me</div>
                 <div @drag-horizontal constraints: [-100, 100] ={dragX}>drag me</div>
             }
         }",
    )
    .expect("trigger fixture must compile");
    let files = &result.files;
    let runtime = files.get("_omniview_runtime.dart").unwrap();
    assert!(
        !runtime.contains("'top 80%'") && !runtime.contains("'bottom 20%'"),
        "legacy viewport string configs must be compiled to typed values, got:\n{runtime}"
    );
    assert!(
        !runtime.contains("split(' '))") && !runtime.contains("startCfg.split"),
        "no runtime string parsing may remain in the scroll widget, got:\n{runtime}"
    );
    assert!(
        !runtime.contains("final String constraints") && !runtime.contains("constraints.split"),
        "the stringly-typed drag constraints field and its parser must be gone, got:\n{runtime}"
    );
    // The legacy scroll fallback survives as a documented bridge with typed
    // constants (0.8 = 'top 80%', 0.2 = 'bottom 20%' — old-style usage still
    // emits start/end-less triggers).
    assert!(
        runtime.contains("0.8") && runtime.contains("0.2"),
        "legacy fallback constants must be typed, got:\n{runtime}"
    );
    // The drag call site no longer passes the dead `constraints: ''` arg.
    let test_dart = files.get("Test.dart").unwrap();
    assert!(
        !test_dart.contains("constraints: ''"),
        "no dead constraints string arg may be emitted, got:\n{test_dart}"
    );
    assert!(
        test_dart.contains("constraintsMin: -100,") && test_dart.contains("constraintsMax: 100,"),
        "typed constraints must flow to the constructor, got:\n{test_dart}"
    );
}

/// T5.6: the scroll scrub smoothing must derive elapsed time from the
/// ticker-driven frame clock instead of assuming 60fps
/// (`1.0 / (scrub * 60.0)` per frame over-smooths on 120Hz displays).
#[test]
fn scroll_scrub_uses_ticker_elapsed_time_not_60fps_hardcode() {
    let result = compile_flutter(
        "@motion fadeScroll { from { opacity: 0.3 } to { opacity: 1 } over: 500ms }
         component Test {
             view {
                 <div @scroll start: 0 end: 1000 scrub: true ={fadeScroll}>scroll me</div>
             }
         }",
    )
    .expect("scroll fixture must compile");
    let runtime = result.files.get("_omniview_runtime.dart").unwrap();
    assert!(
        runtime.contains("WidgetsBinding.instance.currentFrameTimeStamp"),
        "scrub must read the ticker-driven frame clock, got:\n{runtime}"
    );
    assert!(
        runtime.contains("math.exp"),
        "scrub must use frame-rate-independent exponential smoothing, got:\n{runtime}"
    );
    assert!(
        !runtime.contains("widget.scrub * 60.0"),
        "the 60fps hardcode must be gone, got:\n{runtime}"
    );
}

/// T5.6: the `_startPos` dead field flagged by the review must not exist in
/// the emitted drag widget.
#[test]
fn drag_widget_has_no_startpos_dead_field() {
    let result = compile_flutter(
        "@motion dragX { from { translateX: 0 } to { translateX: 300 } over: 300ms }
         component Test {
             view {
                 <div @drag-horizontal={dragX}>drag me</div>
             }
         }",
    )
    .expect("drag fixture must compile");
    let runtime = result.files.get("_omniview_runtime.dart").unwrap();
    assert!(
        !runtime.contains("_startPos"),
        "the dead _startPos field must not be emitted, got:\n{runtime}"
    );
}

/// T5.6 (review HIGH-2): `_syncSemanticBindings` must snapshot each bound
/// expression's value and re-apply focus/scroll/value ONLY when the value
/// CHANGED — an unrelated setState-driven rebuild must not re-request focus
/// (focus stealing) or replay the scroll animation (parity with React's
/// deps-scoped effect).
#[test]
fn semantic_binding_sync_snapshots_prev_values_before_reapply() {
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

    // Snapshot fields for every binding kind, typed from the bound signal.
    assert!(
        dart.contains("bool? _prevFocusInputBox;"),
        "missing focus snapshot field, got:\n{dart}"
    );
    assert!(
        dart.contains("double? _prevScrollYPane;"),
        "missing scroll snapshot field, got:\n{dart}"
    );
    assert!(
        dart.contains("String? _prevValueInputBox;"),
        "missing value snapshot field, got:\n{dart}"
    );

    // Each binding re-applies only when its value changed.
    assert!(
        dart.contains("if (syncFocusInputBox != _prevFocusInputBox) {"),
        "focus must be guarded by the prev-value comparison, got:\n{dart}"
    );
    assert!(
        dart.contains("_prevFocusInputBox = syncFocusInputBox;"),
        "focus snapshot must be recorded before re-applying, got:\n{dart}"
    );
    assert!(
        dart.contains("if (syncScrollYPane != _prevScrollYPane) {"),
        "scroll must be guarded by the prev-value comparison, got:\n{dart}"
    );
    assert!(
        dart.contains("if (syncValueInputBox != _prevValueInputBox) {"),
        "value must be guarded by the prev-value comparison, got:\n{dart}"
    );
    // requestFocus is only reachable through the change guard — the guard
    // wraps the apply, so a stale `if (active) {` (unconditional re-apply on
    // every rebuild) must not exist.
    assert!(
        !dart.contains("if (active) {"),
        "unconditional focus re-apply on every rebuild must be gone (focus stealing), got:\n{dart}"
    );
    // The value binding additionally skips the write when the controller
    // already holds the bound text — typing in the field must not reset the
    // caret on every rebuild.
    assert!(
        dart.contains("if (_inputBoxText.text != syncValueInputBox) {"),
        "value re-apply must skip when the controller already matches, got:\n{dart}"
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

// ═══════════════════════════════════════════════════════════
// Task 5.8a: u64 range warning, effect-guard collapse, pointer-capture
// truncation, store import dedupe, usage-gated design/types imports
// ═══════════════════════════════════════════════════════════

/// Task 5.8a (b): integer literals > i64::MAX warn on the Flutter target
/// (Dart int is signed 64-bit — u64-max is unrepresentable). Honest
/// degradation per ADR-054: compile-time warning, literal still emits.
#[test]
fn flutter_u64_max_literal_warns_chinese_range() {
    let source = r#"
component U64Demo {
    fn maxU64() -> u64 {
        return 18446744073709551615;
    }
}
"#;
    let output = compile_flutter(source).unwrap();
    assert!(
        output.warnings.iter().any(|w| w
            .message
            .contains("字面量超出 Dart int 范围（64 位有符号）")),
        "u64-max literal must warn on Flutter, got warnings: {:?}",
        output.warnings
    );
    // The literal still emits (the error is the developer's to resolve).
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("return 18446744073709551615;"),
        "the literal must still emit, got:\n{}",
        content
    );
}

/// Task 5.8a: an unresolvable effect dep makes the whole guard always-true —
/// `if (true || ...)` flagged the right operand as dead_code in dart
/// analyze (NotificationsPanel:43). Collapse to `if (true)`.
#[test]
fn flutter_effect_unresolvable_dep_collapses_guard() {
    let source = r#"
component EffectDemo {
    signal count: i32 = 0;
    @effect(unknownDep) {
        count = count + 1;
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
        !content.contains("true || true"),
        "the always-true guard must collapse, got:\n{}",
        content
    );
    assert!(
        content.contains("if (true) {") && content.contains("_runEffect_0();"),
        "the collapsed guard must still re-run the effect, got:\n{}",
        content
    );
}

/// Task 5.8a: pointer-capture ref ops emit `throw UnsupportedError(...)`
/// (design D3 / spec §10.3) — later statements in the same fn body are
/// unreachable and flagged dead_code in dart analyze (ElementRefDemo's
/// onPtrDown/onPtrUp). They are truncated after the throw.
#[test]
fn flutter_pointer_capture_truncates_dead_statements() {
    let source = r#"
import omniview;
component PtrDemo {
    ref box: HtmlElement;
    signal active: bool = false;
    signal status: String = "";
    fn onPtrDown() {
        box.capturePointer();
        active = true;
        status = "captured";
    }
    view {
        <div @ref={box}>Target</div>
    }
}
"#;
    // Pointer capture is a DELIBERATE hard error on Flutter (design D3 /
    // spec §10.3) — the compile result carries the diagnostic but still
    // emits the file, so compile directly and filter the expected error.
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
    let unexpected: Vec<String> = result
        .errors
        .iter()
        .filter(|d| !d.message.contains("Flutter 不支持指针捕获"))
        .map(|d| d.message.clone())
        .collect();
    assert!(
        unexpected.is_empty(),
        "only the pointer-capture error is expected, got: {:?}",
        unexpected
    );
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("throw UnsupportedError('capturePointer not supported on Flutter')"),
        "the throw must remain (honest degradation), got:\n{}",
        content
    );
    assert!(
        !content.contains("active = true"),
        "statements after the pointer-capture throw are unreachable and \
         must be truncated, got:\n{}",
        content
    );
}

/// Task 5.8a: store stdlib imports are emitted ONCE per store — the old
/// inline emission inside the per-expression scan duplicated
/// `shared_preferences` when several actions used storage_* (Notification
/// Store: duplicate_import).
#[test]
fn flutter_store_stdlib_import_emitted_once() {
    let source = r#"
import omniview;
import { storage_set } from "std/storage";

store PrefsStore {
    signal theme: String = "light";
    action saveTheme(mode: String) {
        storage_set("theme", mode);
    }
    action saveLang(lang: String) {
        storage_set("lang", lang);
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
    let count = content
        .matches("import 'package:shared_preferences/shared_preferences.dart';")
        .count();
    assert_eq!(
        count, 1,
        "shared_preferences must be imported exactly once, got:\n{}",
        content
    );
}

/// maintainability HIGH-1 (Phase 5.4): the store-action param shadowing on
/// Flutter — a param colliding with a store signal name must stay BARE
/// (`validate_all(items)` iterates the argument), never rewritten to
/// `state.items`. Port of the OmniView fn_locals pattern
/// (omniview/store.rs) to StoreActionExprFormatter's scope.
#[test]
fn store_action_param_shadows_signal_flutter() {
    let source = r#"
import omniview;
import { console_log } from "std/console";
store S {
    signal items: Array<i32> = [1];
    action validate_all(items) { console_log(items.length) }
}
"#;
    let output = compile_flutter(source).expect("compile must succeed");
    let out: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("state.items"),
        "param must not be rewritten to a store ref: {}",
        out
    );
    assert!(
        out.contains("print(items.length)"),
        "param must stay bare inside the call arg: {}",
        out
    );
}

/// Task 5.8a: bare local class names (`class: wrapper`) resolve to no
/// design block — they must NOT trigger the design.dart import (the old
/// conservative scan left it unused in ~30 showcase files).
#[test]
fn flutter_plain_class_names_do_not_import_design() {
    let source = r##"
import omniview;
design LearnDesign {
    token primary: color = "#6366f1";
    class Card {
        background: token(primary);
        border-radius: "8px";
    }
}
component PlainClasses {
    view {
        <div class: wrapper>
            <h3 class: section-title>Plain</h3>
        </div>
    }
}
"##;
    let output = compile_flutter(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("import 'design.dart';"),
        "plain class names must not import design.dart, got:\n{}",
        content
    );
}

/// Task 5.8a: a resolved design class DOES keep the import.
#[test]
fn flutter_resolved_design_class_keeps_import() {
    let source = r##"
import omniview;
design LearnDesign {
    token primary: color = "#6366f1";
    class Card {
        background: token(primary);
        border-radius: "8px";
    }
}
component DesignUser {
    view {
        <div class: LearnDesign.Card>Styled</div>
    }
}
"##;
    let output = compile_flutter(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("import 'design.dart';"),
        "resolved design classes must import design.dart, got:\n{}",
        content
    );
}

/// Task 5.8a: the types.dart import is gated on the fns ACTUALLY inlined
/// into each component — a component that references a struct-returning lib
/// fn imports it; a sibling that does not, does not (the old whole-program
/// scan imported types.dart into every component).
#[test]
fn flutter_types_import_gated_on_inlined_lib_fns() {
    let source = r#"
struct Point { x: i32, y: i32 }

pub fn make_point() -> Point {
    return Point { x: 1, y: 2 };
}

component UsesPoint {
    fn load() {
        let p = make_point();
    }
}

component PlainComp {
    signal n: i32 = 0;
}
"#;
    let output = compile_flutter(source).unwrap();
    let uses_point: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("UsesPoint.dart"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        uses_point.contains("import './types.dart';"),
        "a component inlining a struct-returning lib fn must import \
         types.dart, got:\n{}",
        uses_point
    );
    let plain: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("PlainComp.dart"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !plain.contains("import './types.dart';"),
        "a component referencing no types must not import types.dart, got:\n{}",
        plain
    );
}

// ═══════════════════════════════════════════════════════════════
// Task 3.5 (flutter HIGH): textarea/disabled mapping
// ═══════════════════════════════════════════════════════════════
//
// `<textarea>` rendered as an empty `Container()` (invisible — FormPage's
// message field) and `disabled` was silently dropped (the FormPage "Disabled
// field" input rendered ENABLED with zero warnings). Brief fixtures adapted
// per the 2.9/2.10 precedent: the bare `view V {...}` shorthand does not
// parse (component wrapper required); assertion names and failure messages
// are kept verbatim from the brief.

/// Task 3.5: `<textarea/>` must map to a multiline `TextField` — the
/// multiline intent is first-class (not a bare `Container()` shell).
#[test]
fn textarea_maps_to_multiline_text_field() {
    let out = compile_flutter(r#"component V { view { <textarea/> } }"#).unwrap();
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        !out.contains("child: Container()"),
        "textarea rendered as empty container: {}",
        out
    );
    assert!(out.contains("maxLines:"), "multiline intent lost: {}", out);
}

/// Task 3.5: the boolean `disabled` attribute must emit `enabled: false` —
/// a disabled field must not render as an enabled TextField (previously the
/// prop mapped to "" and was dropped with zero warnings).
#[test]
fn disabled_emits_enabled_false() {
    let out = compile_flutter(r#"component V { view { <input disabled/> } }"#).unwrap();
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        out.contains("enabled: false"),
        "disabled silently dropped: {}",
        out
    );
}

/// Task 3.5: `disabled={false}` must map to the NEGATED `enabled` value —
/// `enabled: true` (the DOM boolean is inverted on the way to Flutter's
/// param; forwarding it verbatim would disable the field on an explicit
/// "not disabled").
#[test]
fn disabled_false_emits_enabled_true() {
    let out = compile_flutter(r#"component V { view { <input disabled={false}/> } }"#).unwrap();
    let out: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        out.contains("enabled: true"),
        "disabled={{false}} must map to enabled: true: {}",
        out
    );
    assert!(
        !out.contains("enabled: false"),
        "negation inverted: {}",
        out
    );
}

// ── @drag draggable-intent wiring (ADR-073) ──────────

/// @drag/@dragEnd lower into ElementIR.drag_binding; Flutter must wrap the
/// rendered element in a GestureDetector whose pan callbacks synthesize the
/// DragIntent payload (drag is not a DOM event — ADR-073 decision 3, capture
/// is a compiler-internal detail, never user-facing syntax):
///   onPanStart → snapshot `d.localPosition` onto the State field `_dragStart`
///   onPanUpdate → drag-guarded move-handler call with
///                 `{ x, y, dx, dy, pointerId, startX, startY }` computed
///                 against the snapshot (dx/dy = delta from start)
///   onPanEnd   → optional end-handler call (same DragIntent shape read from
///                 the snapshot `s`) + clear `_dragStart`
#[test]
fn test_drag_binding_emits_gesture_detector_pan() {
    let source = r#"
component DragBox {
  signal lastDx: Float = 0
  signal lastDy: Float = 0
  signal lastX: Float = 0
  signal lastP: Int = 0
  ref box: Element
  fn move(e) {
    lastDx = e.dx
    lastDy = e.dy
    lastX = e.x
    lastP = e.pointerId
  }
  fn end(e) {
    lastX = e.startX
  }
  view {
    <div ref:box @drag={move} @dragEnd={end}>content</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    // the drag-start snapshot lives in ONE State field (shared by all drag
    // elements — Flutter's class-instance state space)
    assert!(
        out.contains("Offset? _dragStart;"),
        "the drag-start snapshot must be a nullable Offset State field. Got:\n{}",
        out
    );
    // onPanStart: snapshot the start point
    assert!(
        out.contains("onPanStart: (d) { _dragStart = d.localPosition; }"),
        "onPanStart must snapshot d.localPosition. Got:\n{}",
        out
    );
    // the element wraps in a GestureDetector carrying the pan callbacks
    // (wrapper params are sorted deterministically — onPanEnd sorts first)
    assert!(
        out.contains("GestureDetector(onPanEnd:")
            && out.contains("onPanStart:")
            && out.contains("onPanUpdate:"),
        "the element must wrap in a GestureDetector with the pan callbacks. Got:\n{}",
        out
    );
    // the DragIntent payload is a TYPED _DragEvent instance (named-arg
    // construction) — handler bodies read e.dx as dot-accesses, which a
    // Map literal cannot serve (no getters; Fix round 1 of the ADR-073
    // review)
    assert!(
        out.contains("class _DragEvent {"),
        "the _DragEvent carrier class must be emitted. Got:\n{}",
        out
    );
    // onPanUpdate: drag-guarded, dx/dy computed against the snapshot
    assert!(
        out.contains(
            "onPanUpdate: (d) { final s = _dragStart; if (s == null) return; move(_DragEvent("
        ),
        "onPanUpdate must be drag-guarded and call the move handler with a _DragEvent. Got:\n{}",
        out
    );
    assert!(
        out.contains("dx: d.localPosition.dx - s.dx"),
        "dx must be computed against the start snapshot. Got:\n{}",
        out
    );
    assert!(
        out.contains("dy: d.localPosition.dy - s.dy"),
        "dy must be computed against the start snapshot. Got:\n{}",
        out
    );
    assert!(
        out.contains("pointerId: 0"),
        "pointerId has no Flutter analogue — must degrade to 0. Got:\n{}",
        out
    );
    // onPanEnd: optional end handler (reads the same DragIntent shape from
    // the snapshot `s`) + clear
    assert!(
        out.contains("onPanEnd: (d) { final s = _dragStart; if (s != null) { end(_DragEvent("),
        "onPanEnd should call the end handler from the snapshot. Got:\n{}",
        out
    );
    assert!(
        out.contains("startX: s.dx"),
        "startX must be read from the start snapshot. Got:\n{}",
        out
    );
    assert!(
        out.contains("_dragStart = null; }"),
        "onPanEnd must clear the start snapshot. Got:\n{}",
        out
    );
}

/// @drag without @dragEnd: onPanEnd still clears the snapshot — no end
/// callback is wired.
#[test]
fn test_drag_binding_without_drag_end_clears_start() {
    let source = r#"
component DragBox {
  signal lastDx: Float = 0
  ref box: Element
  fn move(e) {
    lastDx = e.dx
  }
  view {
    <div ref:box @drag={move}>content</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("onPanEnd: (d) { _dragStart = null; }"),
        "onPanEnd without @dragEnd must clear the snapshot only. Got:\n{}",
        out
    );
    assert!(
        !out.contains("end(_DragEvent("),
        "no end handler may be wired without @dragEnd. Got:\n{}",
        out
    );
}

/// A zero-param move handler is called BARE — `move();` (a `move({})` call
/// would be extra_positional_arguments on the 0-arity method).
#[test]
fn test_drag_binding_zero_param_handler_called_bare() {
    let source = r#"
component DragBox {
  signal moved: bool = false
  ref box: Element
  fn move() {
    moved = true
  }
  view {
    <div ref:box @drag={move}>content</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("if (s == null) return; move();"),
        "a zero-param handler must be called bare. Got:\n{}",
        out
    );
}

/// Fix round 1 regression (ADR-073 review CRITICAL): the DragIntent payload
/// must be a TYPED `_DragEvent(...)` instance — NOT a Dart `Map<String,num>`
/// literal. Handler bodies read `e.dx`/`e.pointerId` as dynamic dot-accesses;
/// maps have no getters, so the pre-fix Map wiring NoSuchMethodError'd on the
/// first pan. Compiles a real @drag component through the crate API and
/// asserts the compiled artifact carries the class declaration, constructs
/// instances via named args, and contains NO quoted-key Map-literal payload.
#[test]
fn test_drag_payload_is_typed_object_not_map_literal() {
    let source = r#"
component DragBox {
  signal lastDx: Float = 0
  signal lastP: Int = 0
  ref box: Element
  fn move(e) {
    lastDx = e.dx
    lastP = e.pointerId
  }
  view {
    <div ref:box @drag={move}>content</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // the typed carrier class is emitted declaring ONLY the union of
    // fields the handlers read (Fix round 2: move reads dx + pointerId —
    // never the hardcoded all-7 shape that produced 5
    // unused_element_parameter warnings in dart analyze)
    let class_start = out
        .find("class _DragEvent {")
        .expect("the _DragEvent carrier class must be emitted");
    let class_end = class_start + out[class_start..].find("\n}\n").expect("class closed");
    assert_eq!(
        &out[class_start..class_end],
        concat!(
            "class _DragEvent {\n",
            "        final double dx;\n",
            "        final int pointerId;\n",
            "        const _DragEvent({\n",
            "            this.dx = 0,\n",
            "            this.pointerId = 0,\n",
            "        });",
        ),
        "the carrier class must declare exactly the used-fields union (dx + pointerId). Got:\n{}",
        out
    );
    // the pan wiring constructs instances via named args (used_fields
    // filter: move reads dx + pointerId)
    assert!(
        out.contains("move(_DragEvent(dx: d.localPosition.dx - s.dx, pointerId: 0));"),
        "onPanUpdate must construct a _DragEvent with named args. Got:\n{}",
        out
    );
    // NO Map-literal payload may survive — the quoted-key shape is the
    // pre-fix form that NoSuchMethodError'd at runtime
    assert!(
        !out.contains("'dx': d.localPosition.dx - s.dx") && !out.contains("{'x':"),
        "the pan wiring must not pass a Map literal (maps have no getters). Got:\n{}",
        out
    );
    // the handler body keeps its source-level dot-access — valid because the
    // untyped (dynamic) param receives a _DragEvent instance that HAS the
    // getters (dynamic dispatch), unlike the Map it used to receive
    assert!(
        out.contains("lastDx = e.dx;"),
        "the handler body's e.dx must survive compilation (dynamic dispatch on _DragEvent). Got:\n{}",
        out
    );
}

/// Fix round 2 (ADR-073 review): the `_DragEvent` carrier class declares
/// ONLY the union of fields the component's drag handlers actually read —
/// a hardcoded all-7-fields declaration produced 5
/// `unused_element_parameter` warnings in dart analyze once a real demo
/// (@drag on ElementRefDemo) read only dx/dy. The union spans @drag +
/// @dragEnd handlers and multiple elements; zero-param handlers stay bare
/// calls and contribute no fields.
#[test]
fn test_drag_event_class_declares_only_used_fields() {
    let source = r#"
component DragBox {
  signal lastDx: Float = 0
  signal lastY: Float = 0
  signal lastSX: Float = 0
  signal moved: bool = false
  ref box: Element
  ref box2: Element
  fn move(e) {
    lastDx = e.dx
    lastY = e.y
  }
  fn end(e) {
    lastSX = e.startX
  }
  fn ping() {
    moved = true
  }
  view {
    <div ref:box @drag={move} @dragEnd={end}>content</div>
    <div ref:box2 @drag={ping}>tap</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // union of {move: dx, y} ∪ {end: startX} = {y, dx, startX} in schema
    // order — nothing more (ping is zero-param ⇒ bare call, no fields)
    let class_start = out
        .find("class _DragEvent {")
        .expect("the _DragEvent carrier class must be emitted");
    let class_end = class_start + out[class_start..].find("\n}\n").expect("class closed");
    assert_eq!(
        &out[class_start..class_end],
        concat!(
            "class _DragEvent {\n",
            "        final double y;\n",
            "        final double dx;\n",
            "        final double startX;\n",
            "        const _DragEvent({\n",
            "            this.y = 0,\n",
            "            this.dx = 0,\n",
            "            this.startX = 0,\n",
            "        });",
        ),
        "the carrier class must declare exactly the used-fields union (y, dx, startX). Got:\n{}",
        out
    );
    assert!(
        out.contains("ping();"),
        "a zero-param handler must stay a bare call. Got:\n{}",
        out
    );
}

/// Fix round 2 edge: an arity-1 handler that ignores `e` (used_fields =
/// Some([])) still receives a `_DragEvent()` carrier — the class must exist
/// but declare ZERO fields (the call references `_DragEvent()`).
#[test]
fn test_drag_binding_ignored_event_emits_empty_carrier_class() {
    let source = r#"
component DragBox {
  signal moved: bool = false
  ref box: Element
  fn move(e) {
    moved = true
  }
  view {
    <div ref:box @drag={move}>content</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("move(_DragEvent());"),
        "an arity-1 handler that reads no fields must still receive the (empty) carrier. Got:\n{}",
        out
    );
    assert!(
        out.contains("class _DragEvent {\n        const _DragEvent();\n}"),
        "the carrier class must exist with an empty constructor. Got:\n{}",
        out
    );
}

/// Fix round 2 edge: when EVERY drag handler is zero-param, no `_DragEvent`
/// reference exists in the emitted code — the carrier class must NOT be
/// emitted (an unreferenced private class would itself be a dart
/// `unused_element` warning).
#[test]
fn test_drag_binding_all_zero_param_emits_no_carrier_class() {
    let source = r#"
component DragBox {
  signal moved: bool = false
  ref box: Element
  fn move() {
    moved = true
  }
  view {
    <div ref:box @drag={move}>content</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("if (s == null) return; move();"),
        "zero-param handlers must be called bare. Got:\n{}",
        out
    );
    assert!(
        !out.contains("class _DragEvent"),
        "no _DragEvent reference exists — the carrier class must not be emitted. Got:\n{}",
        out
    );
}

/// @drag + a tap-family event on the same element MERGE into ONE
/// GestureDetector (onTap + onPan*) — never two nested wrappers.
#[test]
fn test_drag_binding_merges_into_tap_wrapper() {
    let source = r#"
component DragTapBox {
  signal lastDx: Float = 0
  ref box: Element
  fn onCardPress() { }
  fn move(e) {
    lastDx = e.dx
  }
  view {
    <div ref:box @onPress={onCardPress} @drag={move}>content</div>
  }
}
"#;
    let flutter = compile_flutter(source).expect("compile must succeed");
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("GestureDetector(onPanEnd:")
            && out.contains("onPanStart:")
            && out.contains("onPanUpdate:")
            && out.contains("onTap: onCardPress"),
        "@drag must merge into the existing tap wrapper (one GestureDetector), got:\n{}",
        out
    );
    assert_eq!(
        out.matches("GestureDetector(").count(),
        1,
        "exactly ONE GestureDetector wrapper expected, got:\n{}",
        out
    );
}

// ── Task 12: shared signal-domain classification (direct_write) ──────────
// The D1/D2/B2 classification pass lives in shared/direct_write.rs
// (target-independent) — this test proves the Flutter backend can consume
// it on IR produced by the same pipeline the Flutter compile path runs.
// Fixture mirrors the React drag-demo contract: entry-only reads → D1,
// mutation-RHS self-reads → D2, render-path (class) reads → stays out.

/// Compile a single .omni source through the pipeline and return its first
/// component IR (run_pipeline keys the single-file module "input.omni").
fn compile_component_ir(source: &str) -> ComponentIR {
    let out = omnilang_core::run_pipeline(source);
    assert!(!out.has_errors, "pipeline errors: {:?}", out.diagnostics);
    let mut comps = out
        .ir
        .modules
        .get("input.omni")
        .unwrap()
        .components
        .values();
    comps.next().unwrap().clone()
}

#[test]
fn test_flutter_classifies_drag_signals() {
    use crate::codegen::shared::direct_write::classify_direct_write;

    let source = r#"
component DragBox {
  signal dragging: Bool = false
  signal dragX: f64 = 0
  signal dragOriginX: f64 = 0
  ref box: Element
  fn start() { dragging = true; dragOriginX = 10 }
  fn move(e) { dragX = e.dx - dragOriginX }
  view {
    <div @ref={box} @onPress={start} @drag={move}
         style: { transform: "translate(" + dragX + "px, 0px)" }
         class: { Active: dragging } />
  }
}
"#;

    // The same fixture must compile through the Flutter target (the future
    // direct-write channel consumer).
    let flutter = compile_flutter_full(source);
    assert!(
        flutter.success,
        "fixture must compile for Flutter: {:?}",
        flutter.errors
    );

    // Classification runs on the target-independent component IR.
    let comp = compile_component_ir(source);
    let sets = classify_direct_write(&comp);
    assert!(
        sets.d1.contains("dragX"),
        "dragX should be D1 (entry-only read): {:?}",
        sets.d1
    );
    assert!(
        sets.d2.contains("dragOriginX"),
        "dragOriginX should be D2 (mutation-RHS self-read only): {:?}",
        sets.d2
    );
    assert!(
        !sets.d1.contains("dragging") && !sets.d2.contains("dragging"),
        "dragging has a class-binding read and must stay out of D1/D2"
    );
    assert_eq!(sets.entries.len(), 1, "one active entry (transform)");
    assert_eq!(sets.entries[0].ref_name, "box");
    assert_eq!(sets.entries[0].prop, "transform");
    assert!(
        sets.b2_entries.is_empty(),
        "all-D1 entry is not B2: {:?}",
        sets.b2_entries
    );
}

// ── Task 13: D1/D2 ValueNotifier storage + mutation direct write ──────────
// ADR-071 decision 2 (Phase 1, Flutter): signals whose reads are confined to
// dynamic-style entries (D1) or that are write-only (D2) stop being plain
// State fields + setState mutations — they become
// `final _name = ValueNotifier<T>(init);` fields; mutations write
// `_name.value = ...` with NO setState wrapper (zero whole-tree rebuild).
// Render-path signals (dragging) keep the setState form.

#[test]
fn test_flutter_direct_write_value_notifier() {
    let source = r#"
component DragBox {
  signal dragging: Bool = false
  signal dragX: f64 = 0
  signal dragOriginX: f64 = 0
  ref box: Element
  fn start() { dragging = true; dragOriginX = 10 }
  fn move(e) { dragX = e.dx - dragOriginX }
  view {
    <div @ref={box} @onPress={start} @drag={move}
         style: { transform: "translate(" + dragX + "px, 0px)" }
         class: { Active: dragging } />
  }
}
"#;
    let flutter = compile_flutter_full(source);
    assert!(flutter.success, "compile failed: {:?}", flutter.errors);
    let out: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    // D1/D2 storage: `_`-prefixed ValueNotifier fields — `final _dragX =
    // ValueNotifier<double>(0);` (ADR-071). Plain signals keep fields.
    assert!(
        out.contains("final _dragX = ValueNotifier<double>(0);"),
        "D1 storage must be a ValueNotifier field, got:\n{}",
        out
    );
    assert!(
        out.contains("final _dragOriginX = ValueNotifier<double>(0);"),
        "D2 storage must be a ValueNotifier field, got:\n{}",
        out
    );
    assert!(
        out.contains("bool dragging = false;"),
        "render-path signal stays a plain field, got:\n{}",
        out
    );
    assert!(
        !out.contains("double dragX = 0;"),
        "D1 signal must not remain a plain State field"
    );
    assert!(
        !out.contains("double dragOriginX = 0;"),
        "D2 signal must not remain a plain State field"
    );

    // Mutations: direct `.value` writes — no setState wrapper.
    assert!(
        out.contains("_dragOriginX.value = 10;"),
        "D2 mutation must write the notifier directly, got:\n{}",
        out
    );
    assert!(
        out.contains("_dragX.value = e.dx - _dragOriginX.value;"),
        "D1 mutation must write the notifier directly (RHS self-read via \
         `.value`), got:\n{}",
        out
    );

    // The D1/D2 writes must sit OUTSIDE any setState closure. start() also
    // rebuilds dragging (render-path) — assert the D2 write lands AFTER the
    // setState block closes in that method, and move() has no setState at all.
    // (Void-returning fns emit no return-type prefix — `start() {`.)
    let start_body = out
        .split("start() {")
        .nth(1)
        .and_then(|s| s.split("\n}").next())
        .unwrap_or("");
    let post_setstate = start_body.rsplit("});").next().unwrap_or("");
    assert!(
        post_setstate.contains("_dragOriginX.value = 10;"),
        "D2 write must come after the setState block in start(), got:\n{}",
        start_body
    );
    let move_body = out
        .split("move(e) {")
        .nth(1)
        .and_then(|s| s.split("\n}").next())
        .unwrap_or("");
    assert!(
        !move_body.contains("setState"),
        "move() must not wrap the D1 write in setState, got:\n{}",
        move_body
    );

    // The render-path mutation (dragging = true) still rebuilds via setState.
    assert!(
        out.contains("setState(() {"),
        "render-path signal mutation keeps the setState wrapper"
    );
}

// ── Task 14: Flutter 动态 style 表达式化（ADR-071 Phase 1 直接写入 channel）──

/// Dynamic `style` on a `@ref`'d element with an active entry compiles to a
/// ListenableBuilder wrap driven by the entry's read signals (ValueNotifier
/// names, D1/D2 storage from Task 13) + a Transform.translate mapping of the
/// `"translate(xpx, ypx)"` concat shape.
#[test]
fn test_flutter_style_binding_listenable_builder() {
    let flutter = compile_flutter_full(
        r#"
component DragBox {
  signal dragX: f64 = 0
  signal dragY: f64 = 0
  ref box: Element
  fn move(e) { dragX = e.dx; dragY = e.dy }
  view {
    <div @ref={box} @drag={move}
         style: { transform: "translate(" + dragX + "px, " + dragY + "px)" } />
  }
}
"#,
    );
    assert!(
        flutter.success,
        "fixture must compile: {:?}",
        flutter.errors
    );
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("ListenableBuilder("),
        "expected ListenableBuilder wrap, got:\n{}",
        dart
    );
    assert!(
        dart.contains("listenable: Listenable.merge([_dragX, _dragY])"),
        "expected merged listenables, got:\n{}",
        dart
    );
    assert!(
        dart.contains("Transform.translate("),
        "expected Transform.translate, got:\n{}",
        dart
    );
    assert!(
        dart.contains("offset: Offset(_dragX.value, _dragY.value)"),
        "expected Offset from .value reads, got:\n{}",
        dart
    );
}

/// A dynamic style value that cannot map to any Flutter widget parameter
/// (here `letterSpacing`) must produce a compile-time diagnostic instead of
/// the old silent drop. 批次 3 收口（12155e1a）按 ADR-054 诚实降级将
/// 无参数映射的 CSS 属性从编译错误降为警告（style 条目不发射）——
/// 断言随之更新为警告通道。
#[test]
fn test_flutter_unmappable_style_diagnostic() {
    let flutter = compile_flutter_full(
        r#"
component LetterBox {
  signal ls: f64 = 0
  ref box: Element
  fn bump() { ls = ls + 1 }
  view {
    <div @ref={box} @onPress={bump} style: { letterSpacing: ls } />
  }
}
"#,
    );
    assert!(
        flutter.success,
        "unmappable dynamic style degrades to a warning, not a compile failure"
    );
    let msg = flutter
        .warnings
        .iter()
        .map(|e| e.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("letterSpacing"),
        "diagnostic must mention the property: {}",
        msg
    );
}

#[test]
fn test_flutter_b2_style_entry_bare_names_no_listenable() {
    // The entry reads `shifted` — a COMPUTED signal (never D1/D2, keeps its
    // render-path derivation; Task 12 boundary) → the entry is B2: bare
    // name reads, no ListenableBuilder, setState rebuild channel.
    let flutter = compile_flutter_full(
        r#"
component DragBox {
  signal posX: f64 = 0
  computed shifted: f64 = posX + 10
  ref box: Element
  fn move(e) { posX = e.dx }
  view {
    <div @ref={box} @drag={move}
         style: { transform: "translate(" + shifted + "px, 0px)" } />
  }
}
"#,
    );
    assert!(
        flutter.success,
        "B2 fixture must compile: {:?}",
        flutter.errors
    );
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("Transform.translate(offset: Offset(shifted, 0.0)"),
        "B2 must use bare name (render-path/computed signal), got:\n{}",
        dart
    );
    assert!(
        !dart.contains("ListenableBuilder"),
        "B2 must not wrap in ListenableBuilder, got:\n{}",
        dart
    );
}

/// A String-typed dynamic style value cannot map to Flutter's `double?`
/// width/height params (`Container(width: <String>)` fails dart analyze
/// with argument_type_not_assignable) — the compiler must produce a
/// compile-time diagnostic pointing at the String signal/computed instead
/// of emitting broken Dart (regression: ElementRefDemo 2×
/// argument_type_not_assignable, Task 14 follow-up).
#[test]
fn test_flutter_string_style_value_diagnostic() {
    let flutter = compile_flutter_full(
        r#"
component SizeBox {
  signal boxWidthNum: f64 = 200
  computed boxWidthPx: String = boxWidthNum.to_string() + "px"
  ref box: Element
  fn bump() { boxWidthNum = 300 }
  view {
    <div @ref={box} @onPress={bump} style: { width: boxWidthPx } />
  }
}
"#,
    );
    assert!(
        !flutter.success,
        "String-typed dynamic style must fail compilation (cannot map to double param)"
    );
    let msg = flutter
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("width") && msg.contains("String"),
        "diagnostic must mention the property and the String/type mismatch: {}",
        msg
    );
}

/// The same String check applies to the transform translate shape: Offset
/// x/y are doubles; a String x/y read must be diagnosed, not emitted.
#[test]
fn test_flutter_string_translate_component_diagnostic() {
    let flutter = compile_flutter_full(
        r#"
component DragBox {
  signal dragX: String = "10px"
  signal dragY: f64 = 0
  ref box: Element
  fn move(e) { dragX = "20px" }
  view {
    <div @ref={box} @drag={move}
         style: { transform: "translate(" + dragX + "px, " + dragY + "px)" } />
  }
}
"#,
    );
    assert!(
        !flutter.success,
        "String-typed translate component must fail compilation"
    );
    let msg = flutter
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("translate") && msg.contains("String"),
        "diagnostic must mention translate and the String mismatch: {}",
        msg
    );
}

/// Review HIGH-2: a MIXED entry — D1 signal `dragX` + computed `shifted`
/// (render-path read) — must resolve PER-SIGNAL: the D1 member reads
/// `_dragX.value` (ValueNotifier storage) even though the entry is
/// B2-classified (computed read), and its listenable still enters the
/// merge list. Previously the entry-level B2 decision emitted bare `dragX`
/// (undefined_identifier) and dropped the listenable.
#[test]
fn test_flutter_mixed_d1_computed_entry_per_signal_resolution() {
    let flutter = compile_flutter_full(
        r#"
component DragBox {
  signal dragX: f64 = 0
  signal posX: f64 = 0
  computed shifted: f64 = posX + 10
  ref box: Element
  fn move(e) { dragX = e.dx }
  fn drift() { posX = posX + 5 }
  view {
    <div @ref={box} @drag={move} @onPress={drift}
         style: { transform: "translate(" + dragX + "px, " + shifted + "px)" } />
  }
}
"#,
    );
    assert!(
        flutter.success,
        "mixed fixture must compile: {:?}",
        flutter.errors
    );
    let dart: String = flutter
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        dart.contains("Transform.translate(offset: Offset(_dragX.value, shifted)"),
        "D1 member must read .value, computed stays bare, got:\n{}",
        dart
    );
    assert!(
        dart.contains("listenable: Listenable.merge([_dragX])"),
        "D1 member must be in the merge list, computed excluded, got:\n{}",
        dart
    );
}

// ═══════════════════════════════════════════════════════════
// Task 5: @form Form+validator + @load FutureProvider/AsyncValue
// ═══════════════════════════════════════════════════════════

const FORM_SRC: &str = r#"
component App {
  signal name: string = "";
  signal email: string = "";
  fn submit(e) { }
  view {
    <form @form @onSubmit={submit}>
      <input bind:value={name} validate:{required: true} />
      <input bind:value={email} validate:{format: "email"} />
      <button type="submit">Submit</button>
      @if form.errors.name { <span id="err-name">{form.errors.name}</span> }
    </form>
  }
}
"#;

const LOAD_SRC: &str = r#"
component App {
  fn fetchTodos() -> Array<String> { }
  view {
    @load={fetchTodos}
    @if loading { <div>loading</div> }
    @if error != "" { <div>{error}</div> }
    @for item in data { <div>{item}</div> }
  }
}
"#;

#[test]
fn flutter_form_emits_form_and_validators() {
    let out = compile_flutter(FORM_SRC).expect("form fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("Form("),
        "@form must emit a Form widget, got:\n{}",
        dart
    );
    assert!(
        dart.contains("TextFormField"),
        "bound fields must emit TextFormField, got:\n{}",
        dart
    );
    // The validator CHAIN must be a real `validator:` constructor arg —
    // `validator: (v) => cond ? 'msg' : ...` — not a comment mentioning
    // errorText. Errors surface Flutter-natively via errorText when the
    // validator returns a non-null String.
    assert!(
        dart.contains("validator: (v) =>"),
        "bound fields must carry a real validator chain, got:\n{}",
        dart
    );
}

#[test]
fn flutter_load_emits_future_provider() {
    let out = compile_flutter(LOAD_SRC).expect("load fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("FutureProvider"),
        "@load must emit a FutureProvider, got:\n{}",
        dart
    );
    assert!(
        dart.contains("AsyncValue"),
        "the three-state signal must be an AsyncValue, got:\n{}",
        dart
    );
}

/// validator 规则链：required → 空值拦截；format: "email" → 正则；
/// 错误消息与 React 目标一致（跨目标一致）；validator 返回 String?。
#[test]
fn flutter_form_validator_rules_and_messages() {
    let out = compile_flutter(FORM_SRC).expect("form fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("此项必填"),
        "required must map to the shared 此项必填 message, got:\n{}",
        dart
    );
    assert!(
        dart.contains("邮箱格式不正确"),
        "format: email must map to the shared 邮箱格式不正确 message, got:\n{}",
        dart
    );
    // format is a CLOSED name set — "email" must dispatch to the CONCRETE
    // email regex, never the literal name-as-pattern (`RegExp(r'email')`
    // would match the substring "email", not email addresses).
    assert!(
        dart.contains(r"RegExp(r'^[^\s@]+@[^\s@]+\.[^\s@]+')"),
        "format: email must emit the concrete email regex, got:\n{}",
        dart
    );
    assert!(
        !dart.contains("RegExp(r'email')"),
        "format names must never be emitted as literal patterns, got:\n{}",
        dart
    );
    assert!(
        dart.contains("GlobalKey<FormState>"),
        "the form must carry a GlobalKey<FormState>, got:\n{}",
        dart
    );
    assert!(
        dart.contains("autovalidateMode"),
        "TextFormField must set AutovalidateMode explicitly, got:\n{}",
        dart
    );
}

/// submit 拦截：按钮 onPressed 前校验 `_formKey.currentState!.validate()`。
#[test]
fn flutter_form_submit_interception_guards_on_pressed() {
    let out = compile_flutter(FORM_SRC).expect("form fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("if (!_formKey.currentState!.validate()) return;"),
        "submit must be guarded by validate(), got:\n{}",
        dart
    );
}

/// 语言面表单信号（跨目标一致）：form.errors.name → formErrors["name"]；
/// form.valid/form.submitting 引用时才发射（reachability-gated，避免
/// 未使用局部警告）；form.errors 发射为常量空 map（Flutter 错误经
/// errorText 展示）。
#[test]
fn flutter_form_builtin_signals_render_and_gate() {
    let out = compile_flutter(FORM_SRC).expect("form fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("formErrors[\"name\"]"),
        "form.errors.name must render formErrors[\"name\"], got:\n{}",
        dart
    );
    assert!(
        dart.contains("formErrors"),
        "form.errors must emit the formErrors map, got:\n{}",
        dart
    );

    // reachability：无内建引用时不得发射常量（dart analyze unused warning 门禁）
    let bare = compile_flutter(
        r#"
component App {
  signal name: string = "";
  fn submit(e) { }
  view {
    <form @form @onSubmit={submit}>
      <input bind:value={name} validate:{required: true} />
    </form>
  }
}
"#,
    )
    .expect("bare form fixture must compile");
    let bare_dart: String = bare.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        !bare_dart.contains("formSubmitting"),
        "formSubmitting must be reachability-gated (unused → absent), got:\n{}",
        bare_dart
    );
}

/// 三态接线：loading → .isLoading；error → (.error?.toString() ?? "")；
/// data → .asData?.value ?? const []；@if error != "" 必须整体括号化
/// （Dart ?? 优先级低于 !=）。
#[test]
fn flutter_load_wires_three_states_and_provider() {
    let out = compile_flutter(LOAD_SRC).expect("load fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains(
            "final fetchTodosProvider = FutureProvider<List<String>>((ref) async => fetchTodos());"
        ),
        "provider scaffold must wrap the load fn, got:\n{}",
        dart
    );
    assert!(
        dart.contains("fetchTodosAsync.isLoading"),
        "loading builtin must map to AsyncValue.isLoading, got:\n{}",
        dart
    );
    assert!(
        dart.contains("(fetchTodosAsync.error?.toString() ?? \"\") != ''"),
        "error builtin must map to a parenthesized error string, got:\n{}",
        dart
    );
    assert!(
        dart.contains("fetchTodosAsync.asData?.value ?? const []"),
        "data builtin must map to asData?.value with an empty-list fallback, got:\n{}",
        dart
    );
    assert!(
        dart.contains("ref.watch(fetchTodosProvider)"),
        "the build method must watch the provider, got:\n{}",
        dart
    );
    assert!(
        dart.contains("import 'package:flutter_riverpod/flutter_riverpod.dart';"),
        "@load must pull in flutter_riverpod, got:\n{}",
        dart
    );
    assert!(
        dart.contains("ConsumerStatefulWidget"),
        "ref access requires a Consumer stateful widget, got:\n{}",
        dart
    );
}

/// retry 回调别名 → ref.invalidate(fetchTodosProvider)（Flutter 端重试）。
#[test]
fn flutter_load_retry_maps_to_ref_invalidate() {
    let out = compile_flutter(
        r#"
component App {
  fn fetchTodos() -> Array<String> { }
  view {
    @load={fetchTodos}
    <button @onPress={retry}>Retry</button>
  }
}
"#,
    )
    .expect("retry fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("ref.invalidate(fetchTodosProvider)"),
        "retry must invalidate the provider, got:\n{}",
        dart
    );
}

/// retry 表达式层：`retry()` 作为 CALL 出现在内联 handler 体内时发射
/// 原始动作 `ref.invalidate(...);`（FnRef 是绑定层；表达式层经
/// format_call 分派 — 共享 CallExpr 臂传原始名字，不能退化为裸标识符）。
#[test]
fn flutter_load_retry_call_in_inline_handler_maps_to_action() {
    let out = compile_flutter(
        r#"
component App {
  fn fetchTodos() -> Array<String> { }
  view {
    @load={fetchTodos}
    <button @onPress={() => { retry(); }}>Retry</button>
  }
}
"#,
    )
    .expect("retry call fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("ref.invalidate(fetchTodosProvider);"),
        "retry() must emit the invalidate action in the handler body, got:\n{}",
        dart
    );
}

// ==== 批次 4 Task 8：@load 多例/参数 watch/Reset AsyncNotifier 渐进升级 + null 兜底 ====

/// 多例化（spec §2.1/§4.2）：2 个命名 @load → 每实例独立 provider
/// （`{name}Provider`）+ 每实例 AsyncValue 提升（`{name}Async`）+ 命名
/// 三态/retry/reset 回调（`{name}Data`/`{name}Retry`/`{name}Reset`）。
#[test]
fn flutter_multi_load_per_instance_providers() {
    let out = compile_flutter(
        r#"
component App {
  fn fetchItems() -> Array<String> { }
  fn fetchStats() -> Array<String> { }
  view {
    @load items={fetchItems}
    @load stats={fetchStats}
    <div>{statsData}</div>
    <button @onPress={itemsRetry}>Retry</button>
    <button @onPress={itemsReset}>Reset</button>
    @for item in statsData { <div>{item}</div> }
  }
}
"#,
    )
    .expect("multi-load fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("final itemsProvider = AsyncNotifierProvider<ItemsNotifier, List<String>>(() => ItemsNotifier());"),
        "the reset-using named instance must emit its own upgraded provider, got:\n{}",
        dart
    );
    assert!(
        dart.contains("class ItemsNotifier extends AsyncNotifier<List<String>>"),
        "the reset-using named instance needs its AsyncNotifier class, got:\n{}",
        dart
    );
    assert!(
        dart.contains(
            "final statsProvider = FutureProvider<List<String>>((ref) async => fetchStats());"
        ),
        "each named instance must emit its own provider, got:\n{}",
        dart
    );
    assert!(
        dart.contains("final AsyncValue<List<String>> itemsAsync = ref.watch(itemsProvider);"),
        "the build method must hoist a per-instance AsyncValue, got:\n{}",
        dart
    );
    assert!(
        dart.contains("final AsyncValue<List<String>> statsAsync = ref.watch(statsProvider);"),
        "the build method must hoist a per-instance AsyncValue, got:\n{}",
        dart
    );
    assert!(
        dart.contains("statsAsync.asData?.value ?? const []"),
        "named data builtin must map to {{name}}Async.asData?.value with null fallback, got:\n{}",
        dart
    );
    assert!(
        dart.contains("ref.invalidate(itemsProvider)"),
        "named retry must invalidate the instance provider, got:\n{}",
        dart
    );
    assert!(
        dart.contains("ref.read(itemsProvider.notifier).reset()"),
        "named reset must clear the instance state, got:\n{}",
        dart
    );
}

/// 参数信号绑定（spec §2.1 fn_ref call_expr）：`@load stats={fetchStats(token)}`
/// → provider body 内 `ref.watch(tokenProvider)`——参数变化自动重载（Riverpod
/// 官方响应式数据流，spec §4）。参数信号升级为模块级 NotifierProvider。
#[test]
fn flutter_param_signal_watched_in_provider_body() {
    let out = compile_flutter(
        r#"
component App {
  signal token: String = "a";
  fn fetchStats(token: String) -> Array<String> { }
  view {
    @load stats={fetchStats(token)}
    <div>{statsData}</div>
  }
}
"#,
    )
    .expect("param-load fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("fetchStats(ref.watch(tokenProvider))"),
        "the provider body must watch the param signal, got:\n{}",
        dart
    );
    assert!(
        dart.contains(
            "final tokenProvider = NotifierProvider<TokenNotifier, String>(TokenNotifier.new);"
        ),
        "the param signal must be providerized at module level, got:\n{}",
        dart
    );
    assert!(
        dart.contains("class TokenNotifier extends Notifier<String>"),
        "the param signal needs a top-level Notifier class, got:\n{}",
        dart
    );
}

/// Task 8 fix round 1（Riverpod 3.0 布局期防线）：scroll @for 的
/// itemBuilder 在 LAYOUT 期执行——item 体引用参数信号 `token` 时，读取
/// 必须像 store 读取一样提升为 build 局部量（`ref.watch(tokenProvider)`
/// 提升至 build 方法体），itemBuilder 闭包内不得出现 `ref.watch`
/// （否则 Riverpod 3.0 抛 StateError）。
#[test]
fn flutter_scroll_item_builder_hoists_param_reads() {
    let out = compile_flutter(
        r#"
component App {
  signal token: String = "a";
  fn fetchStats(token: String) -> Array<String> { }
  view {
    @load stats={fetchStats(token)}
    @for item in statsData [scroll] { <div>{item}{token}</div> }
  }
}
"#,
    )
    .expect("scroll + param fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    // 参数读取被提升为 build 局部量（`__omniIter` 已占用 hoist 计数，故为
    // `__omni1`），provider body 照常 watch 参数（变化自动重载）。
    assert!(
        dart.contains("fetchStats(ref.watch(tokenProvider))"),
        "the provider body must watch the param signal, got:\n{}",
        dart
    );
    assert!(
        dart.contains("final __omni1 = ref.watch(tokenProvider);"),
        "param read in the scroll item body must be hoisted to a build local, got:\n{}",
        dart
    );
    let item_builder_pos = dart
        .find("itemBuilder:")
        .unwrap_or_else(|| panic!("must emit ListView.builder with itemBuilder: {}", dart));
    let body_region = &dart[item_builder_pos..];
    assert!(
        !body_region.contains("ref.watch"),
        "itemBuilder body must not contain ref.watch (layout-phase watch \
         throws in Riverpod 3.0): {}",
        dart
    );
    // item 体引用提升后的局部量，而非闭包内 watch。
    assert!(
        body_region.contains("__omni1"),
        "item body must reference the hoisted param local: {}",
        dart
    );
}

/// Reset 渐进升级（P4/spec §2.1 注记）：组件含 Reset 用点 → 该实例升级
/// AsyncNotifierProvider + 顶层 Notifier 类，Reset 以 setState(AsyncData(null))
/// 实现（清空不重载，ref.invalidate ≠ Reset）；无 Reset 的组件维持
/// FutureProvider（渐进式，不整段重写）。
#[test]
fn flutter_reset_upgrades_to_async_notifier() {
    let out = compile_flutter(
        r#"
component App {
  fn fetchTodos() -> Array<String> { }
  view {
    @load={fetchTodos}
    <button @onPress={reset}>Reset</button>
    @for item in data { <div>{item}</div> }
  }
}
"#,
    )
    .expect("reset fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("AsyncNotifierProvider<FetchTodosNotifier, List<String>>"),
        "Reset usage must upgrade the instance to AsyncNotifierProvider, got:\n{}",
        dart
    );
    assert!(
        dart.contains("class FetchTodosNotifier extends AsyncNotifier<List<String>>"),
        "the upgrade needs a top-level AsyncNotifier class, got:\n{}",
        dart
    );
    assert!(
        dart.contains("state = AsyncData(null);"),
        "Reset must clear to AsyncData(null) — never auto-reload, got:\n{}",
        dart
    );
    assert!(
        !dart.contains("ref.invalidate(fetchTodosProvider);"),
        "Reset must NOT map to ref.invalidate (that is retry), got:\n{}",
        dart
    );
    // 渐进式：同批 2 个组件，仅含 Reset 的升级，另一个维持 FutureProvider。
    let no_reset = compile_flutter(
        r#"
component App {
  fn fetchTodos() -> Array<String> { }
  view {
    @load={fetchTodos}
    @for item in data { <div>{item}</div> }
  }
}
"#,
    )
    .expect("no-reset fixture must compile");
    let plain: String = no_reset
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !plain.contains("AsyncNotifier"),
        "without Reset usage the instance must stay on FutureProvider, got:\n{}",
        plain
    );
}

/// ★ 兼容性检查点（spec §2.1 注记）：@for item in data 且 data 为 null →
/// `asData?.value ?? const []` 产物——三态 data 为 null 时迭代空列表，
/// 不崩溃、不显脏 null。
#[test]
fn flutter_for_over_null_data_falls_back_empty() {
    let out = compile_flutter(
        r#"
component App {
  fn fetchTodos() -> Array<String> { }
  view {
    @load={fetchTodos}
    @for item in data { <div>{item}</div> }
  }
}
"#,
    )
    .expect("null-data fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("fetchTodosAsync.asData?.value ?? const []"),
        "@for over possibly-null data must fall back to an empty list, got:\n{}",
        dart
    );
}

/// 匿名 @load → reset（v0 无）：匿名实例的 Reset 回调为裸 `reset`，
/// 清空匿名 provider 状态（data→null、error→null、loading→false），
/// 不触发重载。
#[test]
fn flutter_anon_reset_emitted() {
    let out = compile_flutter(
        r#"
component App {
  fn fetchTodos() -> Array<String> { }
  view {
    @load={fetchTodos}
    <button @onPress={reset}>Reset</button>
    @for item in data { <div>{item}</div> }
  }
}
"#,
    )
    .expect("anon-reset fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("ref.read(fetchTodosProvider.notifier).reset()"),
        "anonymous reset must clear the anonymous provider, got:\n{}",
        dart
    );
}

// ==== 批次 2 Task 7：GoRouter 路由表 + redirect/ShellRoute + pathParameters 注入 + navigate/back 翻译 ====

#[test]
fn test_flutter_gorouter_table_emission() {
    let out = compile_flutter(
        r#"component TodosPage {
  @route "/todos";
  view { <div>Todos</div> }
}

component TodoDetail {
  prop id: String;
  @route "/todos/:id";
  view { <div>Detail {id}</div> }
}
"#,
    )
    .expect("route fixtures must compile");
    let router = out
        .files
        .get("router.dart")
        .expect("router.dart must be emitted when @route present");
    assert!(
        router.contains("GoRouter("),
        "router.dart must contain the GoRouter table, got:\n{router}"
    );
    assert!(
        router.contains("initialLocation: '/',"),
        "GoRouter must pin initialLocation to '/', got:\n{router}"
    );
    assert!(
        router.contains("GoRoute(path: '/todos',"),
        "static path route missing, got:\n{router}"
    );
    assert!(
        router.contains("GoRoute(path: '/todos/:id',"),
        "param path route missing, got:\n{router}"
    );
    let main = out
        .files
        .get("main.dart")
        .expect("main.dart must be emitted when no @theme shell");
    assert!(
        main.contains("MaterialApp.router(routerConfig: router);"),
        "main.dart must wire MaterialApp.router, got:\n{main}"
    );
}

#[test]
fn test_flutter_guard_redirect() {
    // Guard MUST be a component fn (V9: is_valid_guard 只查组件 fn).
    let out = compile_flutter(
        r#"component AdminPage {
  @route "/admin" guard: isAuthorized redirect: "/login";
  fn isAuthorized() -> Bool {
    return true;
  }
  view { <div>Admin</div> }
}
"#,
    )
    .expect("guard fixture must compile");
    let router = out
        .files
        .get("router.dart")
        .expect("router.dart must be emitted");
    assert!(
        router.contains("redirect: (_, __) => isAuthorized() ? null : '/login',"),
        "guard must translate to the redirect form, got:\n{router}"
    );
    assert!(
        router.contains("// 守卫：组件 fn 翻译为顶层 Dart 函数（spec §4.1）"),
        "guard must carry the component-fn translation note, got:\n{router}"
    );
    assert!(
        router.contains("bool isAuthorized() {"),
        "guard fn must be emitted as a top-level Dart fn, got:\n{router}"
    );
}

#[test]
fn test_flutter_shell_route_nested() {
    let out = compile_flutter(
        r#"component AdminLayout {
  @layout "/admin";
  view { @slot default }
}

component AdminUsers {
  @route "/admin/users";
  view { <div>Users</div> }
}
"#,
    )
    .expect("layout fixture must compile");
    let router = out
        .files
        .get("router.dart")
        .expect("router.dart must be emitted");
    assert!(
        router.contains("ShellRoute("),
        "layout must emit a ShellRoute, got:\n{router}"
    );
    assert!(
        router.contains("builder: (_, __, child) => AdminLayout(children: child),"),
        "layout must receive its slot content via the children param, got:\n{router}"
    );
    assert!(
        router.contains("GoRoute(path: '/admin/users',"),
        "layout-owned child route must use its full path, got:\n{router}"
    );
}

#[test]
fn test_flutter_param_injection() {
    let out = compile_flutter(
        r#"component TodoDetail {
  prop id: String;
  @route "/todos/:id";
  view { <div>Detail {id}</div> }
}
"#,
    )
    .expect("param fixture must compile");
    let router = out
        .files
        .get("router.dart")
        .expect("router.dart must be emitted");
    assert!(
        router.contains("builder: (_, s) => TodoDetail(id: s.pathParameters['id']!),"),
        "path param must inject via pathParameters, got:\n{router}"
    );
}

#[test]
fn test_flutter_query_param() {
    let out = compile_flutter(
        r#"component TodosPage {
  prop filter: String = "all";
  @route "/todos";
  @query filter: String = "all";
  view { <div>Todos {filter}</div> }
}
"#,
    )
    .expect("query fixture must compile");
    let router = out
        .files
        .get("router.dart")
        .expect("router.dart must be emitted");
    assert!(
        router.contains("filter: s.uri.queryParameters['filter'] ?? 'all'"),
        "query param must inject via uri.queryParameters with default, got:\n{router}"
    );
}

#[test]
fn test_flutter_navigate_back_translation() {
    let out = compile_flutter(
        r#"component TodosPage {
  @route "/todos";
  prop id: String = "1";
  fn goDetail() { navigate("/todos/1"); }
  fn goDetailDynamic() { navigate("/users/" + id); }
  fn goBack() { back(); }
  view { <div>Todos</div> }
}
"#,
    )
    .expect("navigate/back fixture must compile");
    let dart: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        dart.contains("context.push('/todos/1')"),
        "navigate must translate to context.push, got:\n{dart}"
    );
    assert!(
        dart.contains("context.push('/users/' + (widget.id).toString())"),
        "dynamic-path navigate WITHOUT options must translate to context.push (not raw navigate(...)), got:\n{dart}"
    );
    assert!(
        !dart.contains("navigate("),
        "raw navigate(...) must never survive into generated Dart, got:\n{dart}"
    );
    assert!(
        dart.contains("context.pop()"),
        "back must translate to context.pop, got:\n{dart}"
    );
    assert!(
        dart.contains("栈空时 GoRouter pop 对空栈为安全 no-op"),
        "back must carry the empty-stack no-op degradation note (ADR-054), got:\n{dart}"
    );
    assert!(
        dart.contains("import 'package:go_router/go_router.dart';"),
        "component file must import go_router when navigate/back used, got:\n{dart}"
    );
}

#[test]
fn test_flutter_lazy_mobile_noop_note() {
    let out = compile_flutter(
        r#"component TodosPage {
  @route "/todos" lazy: true;
  view { <div>Todos</div> }
}
"#,
    )
    .expect("lazy fixture must compile");
    let router = out
        .files
        .get("router.dart")
        .expect("router.dart must be emitted");
    assert!(
        router.contains("移动端") && router.contains("无按需加载"),
        "lazy must emit the mobile no-op degradation note (spec §6 边界 5), got:\n{router}"
    );
    assert!(
        router.contains("页面保挂载") && router.contains("StatefulShellRoute.indexedStack"),
        "router.dart must carry the keep-alive difference note (spec §6 边界 7), got:\n{router}"
    );
}

#[test]
fn test_flutter_no_route_no_router() {
    let out = compile_flutter(
        r#"component PlainPage {
  view { <div>plain</div> }
}
"#,
    )
    .expect("plain fixture must compile");
    assert!(
        !out.files.contains_key("router.dart"),
        "no @route -> no router.dart (zero-regression), got: {:?}",
        out.files.keys().collect::<Vec<_>>()
    );
    assert!(
        !out.files.contains_key("main.dart"),
        "no @route -> no main.dart (zero-regression), got: {:?}",
        out.files.keys().collect::<Vec<_>>()
    );
}

// ── 批次 3 响应式：bp() → MediaQuery.sizeOf / bp 常量 / W-302 降级 ──

#[test]
fn bp_emits_media_query_sizeof() {
    let src = r#"
        design D { breakpoint md: 768; }
        component App {
            view {
                @if (bp("md")) { <Container>Desktop</Container> } @else { <Container>Mobile</Container> }
            }
        }
    "#;
    let out = compile_flutter_full(src);
    assert!(out.success, "compile failed: {:?}", out.errors);
    let joined = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        joined.contains("MediaQuery.sizeOf(context).width >= 768"),
        "got: {joined}"
    );
}

#[test]
fn design_emits_bp_constants() {
    let src = r#"
        design D {
            breakpoint sm: 640;
            breakpoint md: 768;
            breakpoint lg: 1024;
        }
        component App { view { <Text>x</Text> } }
    "#;
    let out = compile_flutter_full(src);
    assert!(out.success, "compile failed: {:?}", out.errors);
    let joined = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        joined.contains("static const bpSm = 640.0;"),
        "got: {joined}"
    );
    assert!(
        joined.contains("static const bpMd = 768.0;"),
        "got: {joined}"
    );
    assert!(
        joined.contains("static const bpLg = 1024.0;"),
        "got: {joined}"
    );
}

#[test]
fn media_block_degrades_with_w302_and_no_residue() {
    let src = r#"
        component App {
            style {
                .card { padding: 16; }
                @media (min-width: 768px) {
                    .card { padding: 32; }
                }
            }
            view { <div class: card>c</div> }
        }
    "#;
    let out = compile_flutter_full(src);
    assert!(out.success, "compile failed: {:?}", out.errors);
    let joined = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        !joined.contains("@media"),
        "Flutter 生成物不得残留 @media 语法: {joined}"
    );
    assert!(!joined.contains("min-width"), "块内规则不得生效: {joined}");
    let w302 = out
        .warnings
        .iter()
        .any(|w| w.message.contains("不生效") && w.message.contains("bp()"));
    assert!(w302, "W-302 缺失: {:?}", out.warnings);
}
