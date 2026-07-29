//! Lowering for view blocks and view nodes.
//!
//! Includes: `lower_view_block`, `lower_view_node`.

use super::expr::lower_expr;
use super::stmt::lower_block;
use crate::ir::logic::FunctionIR;
use crate::ir::types::*;
use crate::ir::types_motion::*;
use crate::ir::TriggerKind;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::cell::Cell;

// ═══════════════════════════════════════════════════════
// View lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_view_block(
    view: &ViewBlock,
    string_signals: &[String],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
) -> ViewTree {
    if let Some(first) = view.children.first() {
        let sig_idx = Cell::new(0usize);
        lower_view_node(
            first,
            string_signals,
            Some(&sig_idx),
            variant_map,
            semantics,
            fns,
        )
    } else {
        ViewTree::Text("".into())
    }
}

/// Scan an event handler AST block for `e.preventDefault()` and
/// `e.stopPropagation()` method calls.  Returns (needs_prevent_default,
/// needs_stop_propagation).
///
/// This replaces the field-collection half of the old `EventFieldCollector`;
/// event field names are now collected during `lower_expr` traversal.
fn scan_event_methods(block: &Block, event_param: &str) -> (bool, bool) {
    let mut needs_pd = false;
    let mut needs_sp = false;
    for stmt in &block.stmts {
        scan_stmt_for_event_methods(stmt, &mut needs_pd, &mut needs_sp, event_param);
    }
    if let Some(ref expr) = block.trailing_expr {
        scan_expr_for_event_methods(expr, &mut needs_pd, &mut needs_sp, event_param);
    }
    (needs_pd, needs_sp)
}

fn scan_stmt_for_event_methods(stmt: &Stmt, pd: &mut bool, sp: &mut bool, event_param: &str) {
    match stmt {
        Stmt::ExprStmt { expr, .. } => scan_expr_for_event_methods(expr, pd, sp, event_param),
        Stmt::VarDecl {
            init: Some(init), ..
        } => scan_expr_for_event_methods(init, pd, sp, event_param),
        Stmt::Assign { value, .. } => scan_expr_for_event_methods(value, pd, sp, event_param),
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            scan_expr_for_event_methods(condition, pd, sp, event_param);
            for s in &then_body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
            for (_, elsif_block) in else_ifs {
                for s in &elsif_block.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
            if let Some(else_b) = else_body {
                for s in &else_b.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            scan_expr_for_event_methods(condition, pd, sp, event_param);
            for s in &body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
        }
        Stmt::ForIn { iterable, body, .. } => {
            scan_expr_for_event_methods(iterable, pd, sp, event_param);
            for s in &body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
        }
        Stmt::Return { value: Some(v), .. } => scan_expr_for_event_methods(v, pd, sp, event_param),
        Stmt::Print { arg, .. } => scan_expr_for_event_methods(arg, pd, sp, event_param),
        Stmt::Throw { expr, .. } => scan_expr_for_event_methods(expr, pd, sp, event_param),
        Stmt::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
            if let Some(catch_b) = catch_body {
                for s in &catch_b.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
            if let Some(finally_b) = finally_body {
                for s in &finally_b.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
        }
        _ => {}
    }
}

fn scan_expr_for_event_methods(expr: &Expr, pd: &mut bool, sp: &mut bool, event_param: &str) {
    match expr {
        Expr::MethodCall { object, method, .. } => {
            if let Expr::VarRef(name, _) = object.as_ref() {
                if name == event_param {
                    match method.as_str() {
                        "preventDefault" => *pd = true,
                        "stopPropagation" => *sp = true,
                        _ => {}
                    }
                }
            }
        }
        // Recurse into expressions that may contain nested method calls
        Expr::BinaryOp { left, right, .. } => {
            scan_expr_for_event_methods(left, pd, sp, event_param);
            scan_expr_for_event_methods(right, pd, sp, event_param);
        }
        Expr::UnaryOp { operand, .. } => scan_expr_for_event_methods(operand, pd, sp, event_param),
        Expr::FuncCall { func, args, .. } => {
            scan_expr_for_event_methods(func, pd, sp, event_param);
            for arg in args {
                scan_expr_for_event_methods(arg, pd, sp, event_param);
            }
        }
        Expr::Block { body, .. } => {
            for s in &body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
        }
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        }
        | Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            scan_expr_for_event_methods(condition, pd, sp, event_param);
            scan_expr_for_event_methods(then_expr, pd, sp, event_param);
            scan_expr_for_event_methods(else_expr, pd, sp, event_param);
        }
        _ => {}
    }
}

pub(super) fn lower_view_node(
    node: &ViewNode,
    string_signals: &[String],
    sig_idx: Option<&Cell<usize>>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
) -> ViewTree {
    match node {
        ViewNode::Element(el) => {
            lower_element(el, string_signals, sig_idx, variant_map, semantics, fns)
        }
        ViewNode::Text(s) => ViewTree::Text(s.clone()),
        ViewNode::Interpolation(expr) => {
            // Lower the inner expression to ExprIR so codegen can emit proper JS
            ViewTree::Interpolation(Box::new(lower_expr(expr, variant_map, None, semantics)))
        }
        ViewNode::IfDirective(if_dir) => {
            lower_if_directive(if_dir, string_signals, variant_map, semantics, fns)
        }
        ViewNode::ForDirective(for_dir) => {
            lower_for_directive(for_dir, string_signals, variant_map, semantics, fns)
        }
        ViewNode::SlotDirective {
            name,
            default_content: _,
        } => ViewTree::Slot(name.clone()),
    }
}

// ── Element lowering ──────────────────────────────────────

/// Lowers an `Element` AST node into `ViewTree::Element(ElementIR)`.
/// Handles props, bindings, events, class/style, motion, state, scroll, drag,
/// and hero attributes.
fn lower_element(
    el: &Element,
    string_signals: &[String],
    sig_idx: Option<&Cell<usize>>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
) -> ViewTree {
    let mut props: Option<FxHashMap<String, ExprIR>> = None;
    let mut bindings: Option<FxHashMap<String, SignalRefIR>> = None;
    let mut events: Option<FxHashMap<String, EventHandlerIR>> = None;
    let mut inline_style: Option<serde_json::Value> = None;
    let mut has_value_binding = false;
    let mut class_map_bindings_from_prop: Option<FxHashMap<String, ExprIR>> = None;
    let mut motion_triggers: Vec<TriggerIntent> = Vec::new();
    let mut paused_override: Option<ExprIR> = None;
    let mut in_view_once: Option<ExprIR> = None;
    let mut in_view_repeat: Option<ExprIR> = None;
    let mut in_view_threshold: Option<ExprIR> = None;
    let mut scroll_start: Option<String> = None;
    let mut scroll_end: Option<String> = None;
    let mut scroll_scrub: Option<ExprIR> = None;
    let mut drag_constraints: Option<String> = None;
    let mut drag_elastic: Option<ExprIR> = None;
    let mut drag_threshold: Option<ExprIR> = None;
    let mut drag_velocity_max: Option<ExprIR> = None;

    for prop in &el.props {
        match prop {
            ElementProp::Static { name, value } => {
                props.get_or_insert_with(Default::default).insert(
                    name.clone(),
                    lower_expr(value, variant_map, None, semantics),
                );
            }
            ElementProp::Bind { name, signal_path } => {
                if name == "value" {
                    has_value_binding = true;
                }
                bindings.get_or_insert_with(Default::default).insert(
                    name.clone(),
                    SignalRefIR {
                        owner: None,
                        name: signal_path.clone(),
                    },
                );
            }
            ElementProp::Event { name, handler } => {
                // Collect event data kind
                let event_data = map_event_data_kind(name);
                let mut needs_pd = false;
                let mut needs_sp = false;
                let ir_handler = match handler {
                    EventHandler::InlineFn { params, body, .. } => {
                        let event_param = params.first().map(|p| p.name.as_str()).unwrap_or("e");
                        if event_data.is_some() {
                            let (needs_pd_from_scan, needs_sp_from_scan) =
                                scan_event_methods(body, event_param);
                            needs_pd = needs_pd || needs_pd_from_scan;
                            needs_sp = needs_sp_from_scan;

                            // Collect event field names during lowering instead
                            // of a separate pre-walk.
                            let mut event_fields = FxHashSet::default();
                            // Save and set event param name so lower_expr can detect field access
                            let prev_param = semantics.event_param_name.take();
                            semantics.event_param_name = Some(event_param.to_string());
                            let block_ir =
                                lower_block(body, variant_map, Some(&mut event_fields), semantics);
                            // Restore previous event param name
                            semantics.event_param_name = prev_param;

                            EventHandlerIR::InlineFn {
                                body: block_ir,
                                params: params.iter().map(|p| p.name.clone()).collect(),
                                event_data: event_data.clone(),
                                used_fields: Some(event_fields.into_iter().collect()),
                                needs_prevent_default: needs_pd,
                                needs_stop_propagation: needs_sp,
                            }
                        } else {
                            EventHandlerIR::InlineFn {
                                body: lower_block(body, variant_map, None, semantics),
                                params: params.iter().map(|p| p.name.clone()).collect(),
                                event_data: None,
                                used_fields: None,
                                needs_prevent_default: needs_pd,
                                needs_stop_propagation: needs_sp,
                            }
                        }
                    }
                    EventHandler::FnRef(fn_name) => {
                        // Look up the referenced function's body to find which
                        // event fields are used (e.g. `e.value`, `e.key`).
                        let used_fields = event_data.as_ref().and_then(|kind| {
                            let schema = event_data_schema(kind);
                            let valid_names: Vec<&str> =
                                schema.iter().map(|f| f.name.as_str()).collect();

                            let func_ir = fns.iter().find(|f| f.name == *fn_name).or_else(|| {
                                fns.iter().find(|f| {
                                    f.name == *fn_name || f.name.ends_with(&format!("_{}", fn_name))
                                })
                            });

                            func_ir.map(|func| {
                                let mut used = Vec::new();
                                // Extract event parameter name (default "e" if no params)
                                let fn_param_name =
                                    func.params.first().map(|p| p.name.as_str()).unwrap_or("e");
                                collect_ir_event_fields(&func.body, &mut used, fn_param_name);
                                used.into_iter()
                                    .filter(|f| valid_names.contains(&f.as_str()))
                                    .collect()
                            })
                        });

                        EventHandlerIR::FnRef {
                            r#ref: fn_name.clone(),
                            params: vec![],
                            event_data: event_data.clone(),
                            used_fields,
                            needs_prevent_default: needs_pd,
                            needs_stop_propagation: needs_sp,
                        }
                    }
                    EventHandler::StoreAction { store, action } => {
                        // Store actions receive no event data
                        EventHandlerIR::StoreAction {
                            r#ref: format!("{}.{}", store, action),
                            event_data: None,
                        }
                    }
                };
                events
                    .get_or_insert_with(Default::default)
                    .insert(name.clone(), ir_handler);
            }
            ElementProp::Class { value } => {
                // class: styleRef or class: "literal-class"
                let mut class_map_bindings: FxHashMap<String, ExprIR> = FxHashMap::default();
                let class_str = match value {
                    ClassValue::StyleRef(s) => s.clone(),
                    ClassValue::StringLiteral(s) => s.clone(),
                    ClassValue::DesignClass {
                        design_name: _,
                        class_name,
                    } => {
                        // Design class names are emitted directly (CSS is scoped
                        // via [data-design~="Name"] on the root element, not via
                        // component-level class name mangling).
                        class_name.clone()
                    }
                    ClassValue::ClassMap(entries) => {
                        // Include classes based on their literal value:
                        //   Bool(true)  → static truth  → include the class
                        //   Bool(false) → static falsy → skip the class
                        //   dynamic     → unknown at compile time → JS handles
                        let mut static_classes: Vec<String> = Vec::new();
                        for (key, val_expr) in entries {
                            let mut skip = false;
                            if let Expr::Literal(Literal::Bool(false), _) = val_expr {
                                // Explicitly false: exclude the class entirely
                                skip = true;
                            }
                            // For true or dynamic values, include the class.
                            // Dynamic toggling will be handled by JS later.
                            if !skip {
                                static_classes.push(key.clone());
                            }
                            // Collect dynamic bindings (non-literal values) for reactive JS toggling
                            if !matches!(val_expr, Expr::Literal(Literal::Bool(_), _)) {
                                class_map_bindings.insert(
                                    key.clone(),
                                    lower_expr(val_expr, variant_map, None, semantics),
                                );
                            }
                        }
                        static_classes.join(" ")
                    }
                };
                props.get_or_insert_with(Default::default).insert(
                    "class".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::Value::String(class_str),
                    },
                );
                // Pass class_map_bindings through the element
                class_map_bindings_from_prop = Some(class_map_bindings);
            }
            ElementProp::Style { entries } => {
                // style: { key: value, ... } — passed as inline_style
                // Lower each entry to a JSON representation for codegen
                let style_map: serde_json::Map<String, serde_json::Value> = entries
                    .iter()
                    .map(|(key, expr)| {
                        let ir_expr = lower_expr(expr, variant_map, None, semantics);
                        let json_val = super::expr::lower_expr_to_json_from_ir(&ir_expr);
                        (key.clone(), json_val)
                    })
                    .collect();
                inline_style = Some(serde_json::Value::Object(style_map));
            }
            ElementProp::Variant {
                design_name,
                variant_name,
                dimension_values,
            } => {
                // Encode variant binding as a synthetic prop that codegen will
                // pick up and expand via `expand_variant()`.
                let dim_map: FxHashMap<String, String> = dimension_values.iter().cloned().collect();
                let json_val = serde_json::json!({
                    "design_name": design_name,
                    "variant_name": variant_name,
                    "dimension_values": dim_map,
                });
                props.get_or_insert_with(Default::default).insert(
                    "_design_variant".into(),
                    ExprIR::LiteralExpr { value: json_val },
                );
            }
            ElementProp::TestId(id) => {
                // test-id="..." attribute
                props.get_or_insert_with(Default::default).insert(
                    "data-testid".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::Value::String(id.clone()),
                    },
                );
            }
            ElementProp::Ref(r) => {
                // ref="..." attribute
                props.get_or_insert_with(Default::default).insert(
                    "data-ref".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::Value::String(r.clone()),
                    },
                );
            }
            // Motion animation triggers: on:hover={fadeIn}, etc.
            ElementProp::MotionTrigger(mt) => {
                let motion_ref = match &mt.motion_expr {
                    Expr::VarRef(name, _) => Some(name.clone()),
                    _ => None,
                };
                motion_triggers.push(TriggerIntent {
                    kind: match &mt.trigger {
                        crate::parser::ast::TriggerKind::Enter => TriggerKind::Enter,
                        crate::parser::ast::TriggerKind::Hover => TriggerKind::Hover,
                        crate::parser::ast::TriggerKind::Press => TriggerKind::Press,
                        crate::parser::ast::TriggerKind::InView => TriggerKind::InView,
                        crate::parser::ast::TriggerKind::Scroll => TriggerKind::Scroll,
                        crate::parser::ast::TriggerKind::DragHorizontal => {
                            TriggerKind::DragHorizontal
                        }
                        crate::parser::ast::TriggerKind::DragVertical => TriggerKind::DragVertical,
                    },
                    motion_ref,
                    inline_motion: None,
                    source_span: None,
                });
            }
            // Paused override: @paused={expr}
            ElementProp::PausedOverride(expr) => {
                paused_override = Some(lower_expr(expr, variant_map, None, semantics));
            }
            // InView trigger: once
            ElementProp::InViewOnce(expr) => {
                in_view_once = Some(lower_expr(expr, variant_map, None, semantics));
            }
            // InView trigger: repeat
            ElementProp::InViewRepeat(expr) => {
                in_view_repeat = Some(lower_expr(expr, variant_map, None, semantics));
            }
            // InView trigger: threshold
            ElementProp::InViewThreshold(expr) => {
                in_view_threshold = Some(lower_expr(expr, variant_map, None, semantics));
            }
            // Scroll sub-attributes
            ElementProp::ScrollStart(s) => {
                scroll_start = Some(s.clone());
            }
            ElementProp::ScrollEnd(s) => {
                scroll_end = Some(s.clone());
            }
            ElementProp::ScrollScrub(expr) => {
                scroll_scrub = Some(lower_expr(expr, variant_map, None, semantics));
            }
            // Drag sub-attributes
            ElementProp::DragConstraints(s) => {
                drag_constraints = Some(s.clone());
            }
            ElementProp::DragElastic(expr) => {
                drag_elastic = Some(lower_expr(expr, variant_map, None, semantics));
            }
            ElementProp::DragThreshold(expr) => {
                drag_threshold = Some(lower_expr(expr, variant_map, None, semantics));
            }
            ElementProp::DragVelocityMax(expr) => {
                drag_velocity_max = Some(lower_expr(expr, variant_map, None, semantics));
            }
            // DragScrub and DragMomentum are no longer supported — silently ignore
            ElementProp::DragScrub(_) | ElementProp::DragMomentum(_) => {}
        }
    }

    // Auto-bind <input> elements to String signals if no explicit bind:value
    // and no existing value={expr} prop (one-way reactive binding).
    // Skip checkbox/radio inputs — they use "checked" property, not "value".
    let has_value_prop = props.as_ref().is_some_and(|p| p.contains_key("value"));
    let is_checkbox_or_radio = props.as_ref().is_some_and(|p| {
        p.get("type").is_some_and(|v| {
            matches!(v, ExprIR::LiteralExpr { value: serde_json::Value::String(s) } if s == "checkbox" || s == "radio")
        })
    });
    if let Some(sig_counter) = sig_idx {
        let idx = sig_counter.get();
        if el.tag == "input"
            && !has_value_binding
            && !has_value_prop
            && !is_checkbox_or_radio
            && idx < string_signals.len()
        {
            let sig_name = string_signals[idx].clone();
            sig_counter.set(idx + 1);
            bindings.get_or_insert_with(Default::default).insert(
                "value".into(),
                SignalRefIR {
                    owner: None,
                    name: sig_name,
                },
            );
        }
    }

    let children: Vec<ViewTree> = el
        .children
        .iter()
        .map(|c| lower_view_node(c, string_signals, sig_idx, variant_map, semantics, fns))
        .collect();

    ViewTree::Element(ElementIR {
        tag: el.tag.clone(),
        props: props.unwrap_or_default(),
        bindings: bindings.unwrap_or_default(),
        events: events.unwrap_or_default(),
        style_classes: None,
        inline_style,
        children,
        construction_kind: ConstructionKind::TagMapping,
        class_map_bindings: class_map_bindings_from_prop.unwrap_or_default(),
        source_span: Some(el.span),
        motion: {
            let has_any = !motion_triggers.is_empty()
                || paused_override.is_some()
                || in_view_once.is_some()
                || in_view_repeat.is_some()
                || in_view_threshold.is_some();
            if has_any {
                Some(Box::new(MotionBehavior {
                    triggers: motion_triggers,
                    paused_override,
                    in_view_once,
                    in_view_repeat,
                    in_view_threshold,
                    source_span: None,
                }))
            } else {
                None
            }
        },
        scroll: {
            if scroll_start.is_some() || scroll_end.is_some() || scroll_scrub.is_some() {
                Some(Box::new(ScrollBehavior {
                    start: scroll_start,
                    end: scroll_end,
                    scrub: scroll_scrub,
                    source_span: None,
                }))
            } else {
                None
            }
        },
        drag: {
            if drag_constraints.is_some()
                || drag_elastic.is_some()
                || drag_threshold.is_some()
                || drag_velocity_max.is_some()
            {
                Some(Box::new(DragBehavior {
                    constraints: drag_constraints,
                    elastic: drag_elastic,
                    threshold: drag_threshold,
                    velocity_max: drag_velocity_max,
                    source_span: None,
                }))
            } else {
                None
            }
        },
    })
}

// ── If-directive lowering ─────────────────────────────────

/// Lowers an `IfDirective` AST node into `ViewTree::Conditional`.
/// Creates `ConditionalBranch` entries for the primary condition,
/// any `else if` chains, and the optional `else` fallback.
fn lower_if_directive(
    if_dir: &IfDirective,
    string_signals: &[String],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
) -> ViewTree {
    let branches: Vec<ConditionalBranch> = std::iter::once(ConditionalBranch {
        condition: lower_expr(&if_dir.condition, variant_map, None, semantics),
        body: Some(make_fragment_element(
            if_dir
                .body
                .iter()
                .map(|n| lower_view_node(n, string_signals, None, variant_map, semantics, fns))
                .collect(),
            ConstructionKind::Conditional,
        )),
    })
    .chain(if_dir.else_ifs.iter().map(|(cond, body)| {
        ConditionalBranch {
            condition: lower_expr(cond, variant_map, None, semantics),
            body: Some(make_fragment_element(
                body.iter()
                    .map(|n| lower_view_node(n, string_signals, None, variant_map, semantics, fns))
                    .collect(),
                ConstructionKind::Conditional,
            )),
        }
    }))
    .collect();

    let fallback = if_dir.else_body.as_ref().map(|body| {
        make_fragment_element(
            body.iter()
                .map(|n| lower_view_node(n, string_signals, None, variant_map, semantics, fns))
                .collect(),
            ConstructionKind::Conditional,
        )
    });

    ViewTree::Conditional(Box::new(ConditionalIntent {
        branches,
        fallback,
        transition_mode: if_dir.transition_mode.clone(),
    }))
}

// ── For-directive lowering ────────────────────────────────

/// Lowers a `ForDirective` AST node into `ViewTree::ListRendering`.
/// Wraps body nodes in a fragment with `ConstructionKind::List`.
fn lower_for_directive(
    for_dir: &ForDirective,
    string_signals: &[String],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
) -> ViewTree {
    let lower_body = |nodes: &[ViewNode], s: &mut SemanticResult| -> Option<ViewTree> {
        if nodes.is_empty() {
            None
        } else {
            Some(make_fragment_element(
                nodes
                    .iter()
                    .map(|n| lower_view_node(n, string_signals, None, variant_map, s, fns))
                    .collect(),
                ConstructionKind::List,
            ))
        }
    };

    ViewTree::ListRendering(Box::new(ListRenderingIntent {
        iterable: lower_expr(&for_dir.iterable, variant_map, None, semantics),
        item_binding: BindingIR {
            name: for_dir.item_name.clone(),
            r#type: None,
        },
        body: lower_body(&for_dir.body, semantics),
        key_expr: for_dir
            .key_expr
            .as_ref()
            .map(|e| lower_expr(e, variant_map, None, semantics)),
        empty_body: for_dir
            .empty_body
            .as_ref()
            .and_then(|nodes| lower_body(nodes.as_slice(), semantics)),
    }))
}

// ═══════════════════════════════════════════════════════
// Event field helpers (moved from component.rs — inline
// fn_ref backfill replaces the old two-phase mutation pass)
// ═══════════════════════════════════════════════════════

/// Walk a BlockIR statement list for FieldAccessExpr where object is IdentifierExpr(param_name).
fn collect_ir_event_fields(block: &BlockIR, fields: &mut Vec<String>, param_name: &str) {
    for stmt in &block.statements {
        match stmt {
            StatementIR::ExprStmtIR { expr } => {
                walk_expr_event_fields(expr, fields, param_name);
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                walk_expr_event_fields(condition, fields, param_name);
                collect_ir_event_fields(then_body, fields, param_name);
                for (_, block) in else_ifs {
                    collect_ir_event_fields(block, fields, param_name);
                }
                if let Some(block) = else_body {
                    collect_ir_event_fields(block, fields, param_name);
                }
            }
            StatementIR::LetStmtIR {
                value: Some(expr), ..
            } => {
                walk_expr_event_fields(expr, fields, param_name);
            }
            StatementIR::ReturnStmtIR { value: Some(expr) } => {
                walk_expr_event_fields(expr, fields, param_name);
            }
            StatementIR::BlockStmtIR { body } => {
                collect_ir_event_fields(body, fields, param_name);
            }
            StatementIR::WhileStmtIR { condition, body } => {
                walk_expr_event_fields(condition, fields, param_name);
                collect_ir_event_fields(body, fields, param_name);
            }
            StatementIR::ForInStmtIR { iterable, body, .. } => {
                walk_expr_event_fields(iterable, fields, param_name);
                collect_ir_event_fields(body, fields, param_name);
            }
            StatementIR::ThrowStmtIR { expr } => {
                walk_expr_event_fields(expr, fields, param_name);
            }
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                collect_ir_event_fields(try_body, fields, param_name);
                if let Some(body) = catch_body {
                    collect_ir_event_fields(body, fields, param_name);
                }
                if let Some(body) = finally_body {
                    collect_ir_event_fields(body, fields, param_name);
                }
            }
            StatementIR::EventEmitStmtIR { args, .. } => {
                for arg in args {
                    walk_expr_event_fields(arg, fields, param_name);
                }
            }
            StatementIR::MutationStmt {
                intent: Some(intent),
            } => match intent {
                MutationIntent::DirectAssign { value, .. } => {
                    if let Some(value) = value {
                        walk_expr_event_fields(value, fields, param_name);
                    }
                }
                MutationIntent::Increment { amount, .. }
                | MutationIntent::Decrement { amount, .. }
                | MutationIntent::Multiply { amount, .. }
                | MutationIntent::Divide { amount, .. }
                | MutationIntent::Modulo { amount, .. } => {
                    if let Some(amount) = amount {
                        walk_expr_event_fields(amount, fields, param_name);
                    }
                }
                MutationIntent::Push { item, .. } => {
                    if let Some(item) = item {
                        walk_expr_event_fields(item, fields, param_name);
                    }
                }
                MutationIntent::Pop { .. } => {}
            },
            _ => {}
        }
    }
}

/// Walk an ExprIR tree for `FieldAccessExpr { object: Box(IdentifierExpr(param_name)), field }`.
fn walk_expr_event_fields(expr: &ExprIR, fields: &mut Vec<String>, param_name: &str) {
    match expr {
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if name == param_name && !fields.contains(field) {
                    fields.push(field.clone());
                }
            }
            walk_expr_event_fields(object, fields, param_name);
        }
        ExprIR::OptionalChainExpr { object, .. } => {
            walk_expr_event_fields(object, fields, param_name);
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr_event_fields(object, fields, param_name);
            for arg in args {
                walk_expr_event_fields(arg, fields, param_name);
            }
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr_event_fields(arg, fields, param_name);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_event_fields(left, fields, param_name);
            walk_expr_event_fields(right, fields, param_name);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_event_fields(operand, fields, param_name),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_event_fields(cond, fields, param_name);
            walk_expr_event_fields(then_expr, fields, param_name);
            walk_expr_event_fields(else_expr, fields, param_name);
        }
        ExprIR::StructExpr {
            fields: map_fields, ..
        } => {
            for (_, v) in map_fields {
                walk_expr_event_fields(v, fields, param_name);
            }
        }
        ExprIR::ArrayExpr { elements, .. } => {
            for e in elements {
                walk_expr_event_fields(e, fields, param_name);
            }
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr_event_fields(object, fields, param_name);
            walk_expr_event_fields(index, fields, param_name);
        }
        ExprIR::ClosureExpr { body, .. } => {
            walk_expr_event_fields(body, fields, param_name);
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                match stmt {
                    StatementIR::ExprStmtIR { expr } => {
                        walk_expr_event_fields(expr, fields, param_name);
                    }
                    StatementIR::LetStmtIR {
                        value: Some(expr), ..
                    } => {
                        walk_expr_event_fields(expr, fields, param_name);
                    }
                    _ => {}
                }
            }
            walk_expr_event_fields(tail, fields, param_name);
        }
        ExprIR::AwaitExpr { operand } => walk_expr_event_fields(operand, fields, param_name),
        ExprIR::SpreadExpr { operand } => walk_expr_event_fields(operand, fields, param_name),
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr_event_fields(matched, fields, param_name);
            for arm in arms {
                collect_ir_event_fields(&arm.body, fields, param_name);
            }
        }
        ExprIR::VariantLiteral {
            fields: variant_fields,
            ..
        } => {
            for (_, v) in variant_fields {
                walk_expr_event_fields(v, fields, param_name);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                walk_expr_event_fields(arg, fields, param_name);
            }
        }
        _ => {}
    }
}

// ── Helper factories ─────────────────────────────────────

/// Creates a "fragment" `ElementIR` that wraps child view nodes.
/// Used by conditional/list directives that need a synthetic wrapper element.
fn make_fragment_element(children: Vec<ViewTree>, construction_kind: ConstructionKind) -> ViewTree {
    ViewTree::Element(ElementIR {
        tag: "fragment".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children,
        construction_kind,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
    })
}
