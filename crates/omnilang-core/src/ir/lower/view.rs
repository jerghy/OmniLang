//! Lowering for view blocks and view nodes.
//!
//! Includes: `lower_view_block`, `lower_view_node`.

use super::component::lower_type;
use super::expr::lower_expr;
use super::stmt::lower_block;
use crate::ir::logic::FunctionIR;
use crate::ir::types::*;
use crate::ir::types_motion::*;
use crate::ir::walk;
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
    sig_idx: &Cell<usize>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
) -> ViewTree {
    if view.children.is_empty() {
        return ViewTree::Text("".into());
    }
    let mut lowered: Vec<ViewTree> = Vec::with_capacity(view.children.len());
    for child in &view.children {
        lowered.push(lower_view_node(
            child,
            string_signals,
            Some(sig_idx),
            variant_map,
            semantics,
            fns,
        ));
    }
    // A single child lowers to that child directly; multiple top-level view
    // nodes lower to a fragment so no node is silently dropped (previously
    // only `children.first()` was lowered — multi-root views lost everything
    // after the first element).
    if lowered.len() == 1 {
        lowered.pop().expect("len == 1")
    } else {
        ViewTree::Fragment(lowered)
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
            default_content,
            type_constraint,
        } => {
            let shape = SlotDeclShape {
                name,
                default_content,
                type_constraint,
            };
            lower_slot_common(&shape, string_signals, sig_idx, variant_map, semantics, fns)
                .into_view_tree()
        }
    }
}

/// Shared slot declaration shape passed to `lower_slot_common` — used by
/// both view-level `@slot` directives (`ViewNode::SlotDirective`) and
/// component body slot declarations (`SlotDecl`), ADR-053 D5.
pub(super) struct SlotDeclShape<'a> {
    pub name: &'a Option<String>,
    pub default_content: &'a Option<Vec<ViewNode>>,
    pub type_constraint: &'a Option<TypeAnnotation>,
}

/// Lower the shared slot declaration shape (name + fallback + type
/// constraint) into `SlotDeclIR`.
pub(super) fn lower_slot_common(
    slot: &SlotDeclShape,
    string_signals: &[String],
    sig_idx: Option<&Cell<usize>>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
) -> SlotDeclIR {
    let fallback = slot
        .default_content
        .as_ref()
        .map(|nodes| {
            nodes
                .iter()
                .map(|n| lower_view_node(n, string_signals, sig_idx, variant_map, semantics, fns))
                .collect()
        })
        .unwrap_or_default();
    let (tc, dp) = if let Some(tc) = slot.type_constraint {
        (
            Some(lower_type(tc)),
            Some(slot.name.clone().unwrap_or_default()),
        )
    } else {
        (None, None)
    };
    SlotDeclIR {
        name: slot.name.clone(),
        fallback,
        type_constraint: tc,
        data_param: dp,
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
    // Sub-attribute values (in-view once/repeat/threshold, scroll start/end/
    // scrub, drag constraints/elastic/threshold/velocity-max) were removed as
    // @-prefixed tokens per semantic-level DSL spec §3.2. The IR behavior
    // structs keep their fields for now (always None) until Phase 1
    // restructures them. paused_override was deleted outright (ADR-053).
    let in_view_once: Option<ExprIR> = None;
    let in_view_repeat: Option<ExprIR> = None;
    let in_view_threshold: Option<ExprIR> = None;
    let scroll_start: Option<String> = None;
    let scroll_end: Option<String> = None;
    let scroll_scrub: Option<ExprIR> = None;
    let drag_constraints: Option<String> = None;
    let drag_elastic: Option<ExprIR> = None;
    let drag_threshold: Option<ExprIR> = None;
    let drag_velocity_max: Option<ExprIR> = None;
    let mut ref_binding: Option<String> = None;
    let mut semantic_bindings: Option<FxHashMap<String, ExprIR>> = None;
    // Named slot projections (ADR-053 D5): `header={<div/>}` on a component
    // call lowers to `ViewTree::Slot { name: Some(name), fallback: content }`
    // inserted into the element's children — codegen consumes the projection
    // from the consumer position exactly like a view-level `@slot name {...}`.
    let mut slot_projections: Vec<ViewTree> = Vec::new();

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
                            // Bool(true)  → static truth  → include in HTML
                            // Bool(false) → static falsy → exclude entirely
                            // dynamic     → unknown at compile time → JS handles with classList.toggle
                            if let Expr::Literal(Literal::Bool(true), _) = val_expr {
                                static_classes.push(key.clone());
                            }
                            // Collect dynamic bindings (non-literal values) for reactive JS toggling
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
                ref_binding = Some(r.clone());
            }
            // Semantic ref bindings: focus={sig}, scroll-x={sig}, scroll-y={sig},
            // value={sig}. Extracted into ElementIR.semantic_bindings (consumed
            // by codegen backends in later phases); the semantic pass enforces
            // that they appear only on @ref-bound elements.
            ElementProp::RefBinding { binding, signal } => {
                semantic_bindings
                    .get_or_insert_with(Default::default)
                    .insert(
                        binding.clone(),
                        lower_expr(signal, variant_map, None, semantics),
                    );
            }
            // Motion animation triggers: on:hover={fadeIn}, etc.
            ElementProp::MotionTrigger(mt) => {
                // Two forms: a named motion reference (@enter={fadeIn}) or an
                // inline tween body (@enter={{ opacity: 0, scale: 0.8 }} —
                // parsed as a MapLiteral). Inline bodies are lowered to a
                // KeyframeIR here and hoisted into the module motion table by
                // the lower_inline_motions pass.
                let (motion_ref, inline_motion) = match &mt.motion_expr {
                    Expr::VarRef(name, _) => (Some(name.clone()), None),
                    Expr::MapLiteral { entries, .. } => (
                        None,
                        Some(lower_inline_motion_map(entries, variant_map, semantics)),
                    ),
                    _ => (None, None),
                };
                motion_triggers.push(TriggerIntent {
                    kind: match &mt.trigger {
                        crate::parser::ast::TriggerKind::Enter => TriggerKind::Enter,
                        crate::parser::ast::TriggerKind::Hover => TriggerKind::Hover,
                        crate::parser::ast::TriggerKind::Press => TriggerKind::Press,
                        crate::parser::ast::TriggerKind::Focus => TriggerKind::Focus,
                        crate::parser::ast::TriggerKind::InView => TriggerKind::InView,
                        crate::parser::ast::TriggerKind::Scroll => TriggerKind::Scroll,
                        crate::parser::ast::TriggerKind::DragHorizontal => {
                            TriggerKind::DragHorizontal
                        }
                        crate::parser::ast::TriggerKind::DragVertical => TriggerKind::DragVertical,
                    },
                    motion_ref,
                    inline_motion,
                    source_span: None,
                });
            } // Deleted sub-attribute ElementProps (@paused, in-view once/repeat/
            // threshold, scroll start/end/scrub, drag constraints/elastic/scrub/
            // momentum/threshold/velocity-max) no longer exist — they are parsed
            // as regular ident-based attrs per semantic-level DSL spec §3.2 and
            // will be lowered again once the new attr syntax lands (Tasks 0.3-0.4).
            // Named slot projection: <Modal header={<div>Title</div>} />
            // The projected content is the fallback of the consumer-position
            // ViewTree::Slot (the parent will override the declaration's
            // fallback by slot name).
            ElementProp::SlotContent { name, content } => {
                let fallback = content
                    .iter()
                    .map(|n| {
                        lower_view_node(n, string_signals, sig_idx, variant_map, semantics, fns)
                    })
                    .collect();
                slot_projections.push(ViewTree::Slot {
                    name: Some(name.clone()),
                    fallback,
                    type_constraint: None,
                    data_param: None,
                });
            }
        }
    }

    // Auto-bind <input> elements to String signals if no explicit bind:value
    // and no existing value={expr} prop or value semantic binding (one-way
    // reactive binding).
    // Skip checkbox/radio inputs — they use "checked" property, not "value".
    let has_value_prop = props.as_ref().is_some_and(|p| p.contains_key("value"))
        || semantic_bindings
            .as_ref()
            .is_some_and(|s| s.contains_key("value"));
    let is_checkbox_or_radio = props.as_ref().is_some_and(|p| {
        p.get("type").is_some_and(|v| {
            matches!(v, ExprIR::LiteralExpr { value: serde_json::Value::String(s) } if s == "checkbox" || s == "radio")
        })
    });
    // Skip auto-bind for <input> elements that already have an explicit
    // @onInput event handler — the handler is the developer's chosen approach
    // for managing input state (e.g., reading via ref), and an auto-inserted
    // bind:value would conflict by binding to an unrelated string signal.
    let has_explicit_oninput = events.as_ref().is_some_and(|e| {
        e.keys().any(|k| {
            let lower = k.to_lowercase();
            lower == "oninput" || lower == "input"
        })
    });
    if let Some(sig_counter) = sig_idx {
        let idx = sig_counter.get();
        if el.tag == "input"
            && !has_value_binding
            && !has_value_prop
            && !has_explicit_oninput
            && !is_checkbox_or_radio
            && idx < string_signals.len()
        {
            let sig_name = string_signals[idx].clone();
            sig_counter.set(idx + 1);
            bindings
                .get_or_insert_with(Default::default)
                .insert("value".into(), SignalRefIR { name: sig_name });
        }
    }

    let mut children: Vec<ViewTree> = el
        .children
        .iter()
        .map(|c| lower_view_node(c, string_signals, sig_idx, variant_map, semantics, fns))
        .collect();
    // Slot projections are appended after regular children — codegen looks
    // them up by slot name, so relative order does not affect projection.
    children.extend(slot_projections);

    ViewTree::Element(Box::new(ElementIR {
        tag: el.tag.clone(),
        props: props.unwrap_or_default(),
        bindings: bindings.unwrap_or_default(),
        events: events.unwrap_or_default(),
        style_classes: None,
        inline_style,
        children,
        construction_kind: ConstructionKind::Component,
        class_map_bindings: class_map_bindings_from_prop.unwrap_or_default(),
        source_span: Some(el.span),
        motion: {
            let has_any = !motion_triggers.is_empty()
                || in_view_once.is_some()
                || in_view_repeat.is_some()
                || in_view_threshold.is_some();
            if has_any {
                Some(Box::new(MotionBehavior {
                    triggers: motion_triggers,
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
        ref_binding,
        semantic_bindings: semantic_bindings.unwrap_or_default(),
    }))
}

/// Lower an inline motion tween body (`@enter={{ opacity: 0, scale: 0.8 }}`)
/// into a `KeyframeIR`.
///
/// The map is a single "from" stop at position 0.0: the element animates from
/// these values to its natural state when the trigger fires. Inline tweens
/// carry no `over`/`easing` clauses, so the duration defaults to 400ms — the
/// same default used by the inline-motion codegen fixtures. The
/// `lower_inline_motions` pass later hoists the body into the module motion
/// table under a synthesized name.
fn lower_inline_motion_map(
    entries: &[(Expr, Expr)],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> KeyframeIR {
    let mut properties = FxHashMap::default();
    for (key, value) in entries {
        if let Expr::Literal(Literal::String(name), _) = key {
            properties.insert(
                name.clone(),
                lower_expr(value, variant_map, None, semantics),
            );
        }
    }
    KeyframeIR {
        stops: vec![KeyframeStopIR {
            position: 0.0,
            properties,
            ease: None,
            source_span: None,
        }],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(400),
        },
        ease: None,
        playback: PlaybackIR::default(),
        source_span: None,
    }
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
        scroll_hint: for_dir.scroll_hint,
    }))
}

// ═══════════════════════════════════════════════════════
// Event field helpers (moved from component.rs — inline
// fn_ref backfill replaces the old two-phase mutation pass)
// ═══════════════════════════════════════════════════════

/// Walk a BlockIR statement list for FieldAccessExpr where object is IdentifierExpr(param_name).
/// Uses the shared `walk_stmt` to traverse all expressions within the block.
fn collect_ir_event_fields(block: &BlockIR, fields: &mut Vec<String>, param_name: &str) {
    for stmt in &block.statements {
        walk::walk_stmt(stmt, &mut |expr| {
            if let ExprIR::FieldAccessExpr { object, field } = expr {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if name == param_name && !fields.contains(field) {
                        fields.push(field.clone());
                    }
                }
            }
        });
    }
}

// ── Helper factories ─────────────────────────────────────

/// Creates a "fragment" `ElementIR` that wraps child view nodes.
/// Used by conditional/list directives that need a synthetic wrapper element.
fn make_fragment_element(children: Vec<ViewTree>, construction_kind: ConstructionKind) -> ViewTree {
    ViewTree::Element(Box::new(ElementIR {
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
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Span;
    use crate::semantic::symbols::SymbolTable;

    #[test]
    fn test_lower_element_with_ref_binding() {
        // Verify that ElementProp::Ref("box") sets ref_binding on ElementIR
        // instead of emitting a "data-ref" prop.
        let el = Element {
            tag: "div".into(),
            props: vec![ElementProp::Ref("box".to_string())],
            children: vec![],
            self_closing: true,
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                col: 1,
            },
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
        };
        let result = lower_element(&el, &[], None, &FxHashMap::default(), &mut semantics, &[]);
        match result {
            ViewTree::Element(el_ir) => {
                assert_eq!(
                    el_ir.ref_binding,
                    Some("box".to_string()),
                    "ref_binding should be set from ElementProp::Ref"
                );
                assert!(
                    !el_ir.props.contains_key("data-ref"),
                    "no data-ref prop should be emitted"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    #[test]
    fn test_lower_element_ref_binding_lowers_to_semantic_bindings() {
        // RefBinding props lower into ElementIR.semantic_bindings — not into
        // the props map. Codegen backends consume semantic_bindings directly
        // (focus={sig}, scroll-x={sig}, scroll-y={sig}, value={sig}).
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let el = Element {
            tag: "div".into(),
            props: vec![ElementProp::RefBinding {
                binding: "scroll-y".into(),
                signal: Expr::VarRef("pos".into(), span),
            }],
            children: vec![],
            self_closing: true,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
        };
        let result = lower_element(&el, &[], None, &FxHashMap::default(), &mut semantics, &[]);
        match result {
            ViewTree::Element(el_ir) => {
                assert!(
                    el_ir.semantic_bindings.contains_key("scroll-y"),
                    "scroll-y should be lowered into semantic_bindings"
                );
                assert!(
                    matches!(
                        el_ir.semantic_bindings.get("scroll-y"),
                        Some(ExprIR::IdentifierExpr { name, .. }) if name == "pos"
                    ),
                    "signal expr should lower to IdentifierExpr(pos)"
                );
                assert!(
                    !el_ir.props.contains_key("scroll-y"),
                    "scroll-y should NOT be lowered into props"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    #[test]
    fn test_auto_bind_skipped_when_semantic_value_binding_present() {
        // <input value={sig} /> is a semantic binding — the auto-bind logic
        // must not also insert a bind:value for a String signal.
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let el = Element {
            tag: "input".into(),
            props: vec![ElementProp::RefBinding {
                binding: "value".into(),
                signal: Expr::VarRef("text".into(), span),
            }],
            children: vec![],
            self_closing: true,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
        };
        let string_signals = vec!["unrelated".to_string()];
        let result = lower_element(
            &el,
            &string_signals,
            Some(&Cell::new(0usize)),
            &FxHashMap::default(),
            &mut semantics,
            &[],
        );
        match result {
            ViewTree::Element(el_ir) => {
                assert!(
                    el_ir.semantic_bindings.contains_key("value"),
                    "value should be a semantic binding"
                );
                assert!(
                    !el_ir.bindings.contains_key("value"),
                    "auto-bind must not add bind:value when value={{sig}} exists"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    /// Regression: a view block with multiple top-level nodes must lower ALL
    /// of them (previously only `children.first()` was lowered, silently
    /// dropping every node after the first element).
    #[test]
    fn test_multi_root_view_block_preserves_all_children() {
        let source = r#"
component MultiRoot {
    view {
        <div class: a>A</div>
        <div class: b @hover={pulse}>B</div>
    }
}
"#;
        let tokens = crate::lexer::tokenize(source);
        let (program, errors) = crate::parser::parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
        };
        let ir = crate::ir::lower(&program, &mut semantics, None);
        let module = ir.modules.get("input.omni").expect("module");
        let comp = module.components.get("MultiRoot").expect("component");
        let view = comp.view.as_ref().expect("view");
        match view {
            ViewTree::Fragment(children) => {
                assert_eq!(children.len(), 2, "both top-level nodes must survive");
                let texts: Vec<&str> = children
                    .iter()
                    .filter_map(|c| match c {
                        ViewTree::Element(el) => el.children.iter().find_map(|g| match g {
                            ViewTree::Text(t) => Some(t.as_str()),
                            _ => None,
                        }),
                        _ => None,
                    })
                    .collect();
                assert_eq!(texts, vec!["A", "B"], "both elements must be present");
            }
            _ => panic!("Expected ViewTree::Fragment for a multi-root view"),
        }
    }
}
