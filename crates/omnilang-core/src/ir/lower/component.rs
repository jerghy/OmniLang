//! Lowering for component and store definitions.
//!
//! Includes: `lower_component`, `lower_store`, `lower_type`,
//! and pass-2 helpers for fn_ref event field backfilling.

use super::expr::lower_expr;
use super::motion::{lower_easing, lower_motion_def, lower_timeline_def};
use super::stmt::lower_block;
use super::view::lower_view_block;
use crate::ir::logic::*;
use crate::ir::types::*;
use crate::ir::types_motion::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;

// Disambiguate conflicting names between parser AST and IR
use crate::ir::types::LifecycleKind as IRLifecycleKind;
use crate::parser::ast::view::LifecycleKind as ParserLifecycleKind;

// ═══════════════════════════════════════════════════════
// Component lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_component(
    comp: &ComponentDef,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> ComponentIR {
    let states: Vec<SignalIntent> = comp
        .signals
        .iter()
        .map(|s| SignalIntent {
            name: s.name.clone(),
            r#type: lower_type(&s.type_ann),
            initial_value: lower_expr(&s.init, variant_map, None, semantics),
            is_copy: false,
        })
        .collect();

    let mut computeds: Vec<ComputedIntent> = comp
        .computeds
        .iter()
        .map(|c| ComputedIntent {
            name: c.name.clone(),
            r#type: lower_type(&c.type_ann),
            body: lower_expr(&c.body, variant_map, None, semantics),
            deps: vec![],
        })
        .collect();

    // Lower inline-store getters into computed properties
    for g in &comp.getters {
        computeds.push(ComputedIntent {
            name: g.name.clone(),
            r#type: lower_type(&g.type_ann),
            body: lower_expr(&g.body, variant_map, None, semantics),
            deps: vec![],
        });
    }

    let effects: Vec<EffectIntent> = comp
        .lifecycles
        .iter()
        .filter(|lc| matches!(lc.kind, ParserLifecycleKind::Effect))
        .map(|lc| EffectIntent {
            deps: lc.deps.clone(),
            body: lower_block(&lc.body, variant_map, None, semantics),
        })
        .collect();

    let lifecycles: Vec<LifecycleIntent> = comp
        .lifecycles
        .iter()
        .filter(|lc| !matches!(lc.kind, ParserLifecycleKind::Effect))
        .map(|lc| LifecycleIntent {
            kind: match lc.kind {
                ParserLifecycleKind::Mount => IRLifecycleKind::Mount,
                ParserLifecycleKind::Unmount => IRLifecycleKind::Unmount,
                ParserLifecycleKind::Effect => unreachable!(
                    "Effect lifecycle should not reach codegen; filtered into comp.effects during IR lowering"
                ),
            },
            body: lower_block(&lc.body, variant_map, None, semantics),
        })
        .collect();

    let mut fns: Vec<FunctionIR> = comp
        .fns
        .iter()
        .map(|f| {
            semantics.symbols.push_scope(&format!("fn:{}", f.name));
            let result = FunctionIR {
                name: f.name.clone(),
                visibility: Visibility::Private,
                params: f
                    .params
                    .iter()
                    .map(|p| ParamIR {
                        name: p.name.clone(),
                        r#type: p.type_ann.as_ref().map(lower_type),
                    })
                    .collect(),
                return_type: f.return_type.as_ref().map(lower_type),
                body: lower_block(&f.body, variant_map, None, semantics),
                is_async: f.is_async,
                source_span: Some(f.span),
            };
            semantics.symbols.pop_scope();
            result
        })
        .collect();
    // Also include component actions as regular functions
    for a in &comp.actions {
        semantics.symbols.push_scope(&format!("fn:{}", a.name));
        fns.push(FunctionIR {
            name: a.name.clone(),
            visibility: Visibility::Private,
            params: a
                .params
                .iter()
                .map(|p| ParamIR {
                    name: p.name.clone(),
                    r#type: p.type_ann.as_ref().map(lower_type),
                })
                .collect(),
            return_type: a.return_type.as_ref().map(lower_type),
            body: lower_block(&a.body, variant_map, None, semantics),
            is_async: a.is_async,
            source_span: Some(a.span),
        });
        semantics.symbols.pop_scope();
    }

    let props: FxHashMap<String, TypeIR> = comp
        .props
        .iter()
        .map(|p| (p.name.clone(), lower_type(&p.type_ann)))
        .collect();

    let prop_defaults: FxHashMap<String, ExprIR> = comp
        .props
        .iter()
        .filter_map(|p| {
            p.default
                .as_ref()
                .map(|d| (p.name.clone(), lower_expr(d, variant_map, None, semantics)))
        })
        .collect();

    // Collect input-binding signal names for <input> element auto-binding.
    // Only auto-bind signals whose name suggests they're for input binding.
    // If none exist, fall back to all String signals for backward compatibility.
    let input_signals: Vec<String> = {
        let string_signals: Vec<_> = comp.signals.iter()
            .filter(|s| matches!(s.type_ann, TypeAnnotation::Named { ref name, .. } if name == "String"))
            .map(|s| s.name.clone())
            .collect();
        // Prefer signals whose name suggests they're for input binding
        let candidates: Vec<_> = string_signals
            .iter()
            .filter(|n| n.contains("input") || n.contains("Input"))
            .cloned()
            .collect();
        if candidates.is_empty() {
            string_signals
        } else {
            candidates
        }
    };

    let mut view = comp
        .view
        .as_ref()
        .map(|v| lower_view_block(v, &input_signals, variant_map, semantics));

    let styles: Vec<StyleRuleIR> = comp
        .styles
        .iter()
        .map(|s| {
            let properties: FxHashMap<String, ExprIR> = s
                .properties
                .iter()
                .map(|(k, v)| {
                    let ir_val = match v {
                        StyleValue::Number(n) => {
                            // JSON does not support NaN or Infinity; fall back to 0
                            // for these edge cases (should not occur in well-formed source).
                            let num = if n.is_finite() {
                                serde_json::Number::from_f64(*n)
                                    .unwrap_or_else(|| serde_json::Number::from(0))
                            } else {
                                serde_json::Number::from(0)
                            };
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(num),
                            }
                        }
                        StyleValue::String(str_val) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(str_val.clone()),
                        },
                        StyleValue::Ident(id) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(id.clone()),
                        },
                        StyleValue::HexColor(h) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(h.clone()),
                        },
                        StyleValue::Dimension(val, unit) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(format!("{}{}", val, unit)),
                        },
                        StyleValue::List(parts) => {
                            let s: String = parts
                                .iter()
                                .map(|p| match p {
                                    StyleValuePart::Ident(id) => id.clone(),
                                    StyleValuePart::Number(n) => n.to_string(),
                                    StyleValuePart::Dimension(val, unit) => {
                                        format!("{}{}", val, unit)
                                    }
                                    StyleValuePart::HexColor(h) => h.clone(),
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(s),
                            }
                        }
                    };
                    (k.clone(), ir_val)
                })
                .collect();
            StyleRuleIR {
                class_name: s.class_name.clone(),
                properties,
                output_mode: "css".into(),
                is_tag_selector: s.is_tag_selector,
            }
        })
        .collect();

    let motions: Vec<MotionIntent> = comp
        .motions
        .iter()
        .map(|m| lower_motion_def(m, variant_map, semantics))
        .collect();

    let timelines: Vec<TimelineIntent> = comp
        .timelines
        .iter()
        .map(|t| lower_timeline_def(t, variant_map, semantics))
        .collect();

    let hero_defs: Vec<HeroIntent> = comp
        .hero_defs
        .iter()
        .map(|h| HeroIntent {
            name: h.name.clone(),
            over: lower_expr(&h.over, variant_map, None, semantics),
            ease: h.ease.as_ref().map(lower_easing),
            source_span: Some(h.span),
        })
        .collect();

    // ── Pass 2: Backfill used_fields for fn_ref event handlers ──
    if let Some(ref mut view_tree) = view {
        backfill_fn_ref_event_fields(view_tree, &fns);
    }

    ComponentIR {
        name: comp.name.clone(),
        source_file: None,
        generic_params: vec![],
        props,
        prop_defaults,
        signals: states,
        computeds,
        effects,
        lifecycles,
        fns,
        store_refs: comp.store_refs.clone(),
        refs: vec![],
        event_names: comp.events.iter().map(|e| e.name.clone()).collect(),
        view,
        styles,
        motions,
        timelines,
        if_mode_default: comp.if_mode_default.clone(),
        hero_defs,
        source_span: Some(comp.span),
    }
}

// ═══════════════════════════════════════════════════════
// Pass 2: Backfill used_fields for fn_ref event handlers
// ═══════════════════════════════════════════════════════

/// Pass 2: backfill used_fields for fn_ref event handlers by walking the
/// referenced function's IR body for `FieldAccessExpr { object: IdentifierExpr("e"), field }`.
pub(super) fn backfill_fn_ref_event_fields(view: &mut ViewTree, fns: &[FunctionIR]) {
    match view {
        ViewTree::Element(ref mut el) => {
            for (_event_name, handler) in el.events.iter_mut() {
                if let EventHandlerIR::FnRef {
                    ref mut used_fields,
                    r#ref: fn_name,
                    event_data: Some(kind),
                    ..
                } = handler
                {
                    if used_fields.is_some() {
                        continue;
                    }
                    let schema = event_data_schema(kind);
                    let valid_names: Vec<&str> = schema.iter().map(|f| f.name.as_str()).collect();

                    let func_ir = fns.iter().find(|f| f.name == *fn_name).or_else(|| {
                        fns.iter().find(|f| {
                            f.name == *fn_name || f.name.ends_with(&format!("_{}", fn_name))
                        })
                    });
                    if let Some(func) = func_ir {
                        let mut used = Vec::new();
                        collect_ir_event_fields(&func.body, &mut used);
                        let used: Vec<String> = used
                            .into_iter()
                            .filter(|f| valid_names.contains(&f.as_str()))
                            .collect();
                        *used_fields = Some(used);
                    }
                }
            }
            for child in &mut el.children {
                backfill_fn_ref_event_fields(child, fns);
            }
        }
        ViewTree::Conditional(ref mut cond) => {
            for branch in &mut cond.branches {
                if let Some(ref mut body) = branch.body {
                    backfill_fn_ref_event_fields(body, fns);
                }
            }
            if let Some(ref mut fallback) = cond.fallback {
                backfill_fn_ref_event_fields(fallback, fns);
            }
        }
        ViewTree::ListRendering(ref mut list) => {
            if let Some(ref mut body) = list.body {
                backfill_fn_ref_event_fields(body, fns);
            }
            if let Some(ref mut empty) = list.empty_body {
                backfill_fn_ref_event_fields(empty, fns);
            }
        }
        _ => {} // Interpolation, Text, Slot — no events
    }
}

/// Walk a BlockIR statement list for FieldAccessExpr where object is IdentifierExpr("e").
pub(super) fn collect_ir_event_fields(block: &BlockIR, fields: &mut Vec<String>) {
    for stmt in &block.statements {
        match stmt {
            StatementIR::ExprStmtIR { expr } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                walk_expr_event_fields(condition, fields);
                collect_ir_event_fields(then_body, fields);
                for (_, block) in else_ifs {
                    collect_ir_event_fields(block, fields);
                }
                if let Some(block) = else_body {
                    collect_ir_event_fields(block, fields);
                }
            }
            StatementIR::LetStmtIR {
                value: Some(expr), ..
            } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::ReturnStmtIR { value: Some(expr) } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::BlockStmtIR { body } => {
                collect_ir_event_fields(body, fields);
            }
            StatementIR::WhileStmtIR { condition, body } => {
                walk_expr_event_fields(condition, fields);
                collect_ir_event_fields(body, fields);
            }
            StatementIR::ForInStmtIR { iterable, body, .. } => {
                walk_expr_event_fields(iterable, fields);
                collect_ir_event_fields(body, fields);
            }
            StatementIR::ThrowStmtIR { expr } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                collect_ir_event_fields(try_body, fields);
                if let Some(body) = catch_body {
                    collect_ir_event_fields(body, fields);
                }
                if let Some(body) = finally_body {
                    collect_ir_event_fields(body, fields);
                }
            }
            StatementIR::EventEmitStmtIR { args, .. } => {
                for arg in args {
                    walk_expr_event_fields(arg, fields);
                }
            }
            StatementIR::MutationStmt {
                intent: Some(intent),
            } => match intent {
                MutationIntent::DirectAssign { value, .. } => {
                    if let Some(value) = value {
                        walk_expr_event_fields(value, fields);
                    }
                }
                MutationIntent::Increment { amount, .. }
                | MutationIntent::Decrement { amount, .. }
                | MutationIntent::Multiply { amount, .. }
                | MutationIntent::Divide { amount, .. }
                | MutationIntent::Modulo { amount, .. } => {
                    if let Some(amount) = amount {
                        walk_expr_event_fields(amount, fields);
                    }
                }
                MutationIntent::Push { item, .. } => {
                    if let Some(item) = item {
                        walk_expr_event_fields(item, fields);
                    }
                }
                MutationIntent::Pop { .. } => {}
            },
            _ => {}
        }
    }
}

/// Walk an ExprIR tree for `FieldAccessExpr { object: Box(IdentifierExpr("e")), field }`.
pub(super) fn walk_expr_event_fields(expr: &ExprIR, fields: &mut Vec<String>) {
    match expr {
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if name == "e" && !fields.contains(field) {
                    fields.push(field.clone());
                }
            }
            walk_expr_event_fields(object, fields);
        }
        ExprIR::OptionalChainExpr { object, .. } => {
            walk_expr_event_fields(object, fields);
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr_event_fields(object, fields);
            for arg in args {
                walk_expr_event_fields(arg, fields);
            }
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr_event_fields(arg, fields);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_event_fields(left, fields);
            walk_expr_event_fields(right, fields);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_event_fields(operand, fields),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_event_fields(cond, fields);
            walk_expr_event_fields(then_expr, fields);
            walk_expr_event_fields(else_expr, fields);
        }
        ExprIR::StructExpr {
            fields: map_fields, ..
        } => {
            for (_, v) in map_fields {
                walk_expr_event_fields(v, fields);
            }
        }
        ExprIR::ArrayExpr { elements, .. } => {
            for e in elements {
                walk_expr_event_fields(e, fields);
            }
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr_event_fields(object, fields);
            walk_expr_event_fields(index, fields);
        }
        ExprIR::ClosureExpr { body, .. } => {
            walk_expr_event_fields(body, fields);
        }
        ExprIR::BlockExpr { statements, tail } => {
            // Walk expressions in block expression statements
            for stmt in statements {
                match stmt {
                    StatementIR::ExprStmtIR { expr } => {
                        walk_expr_event_fields(expr, fields);
                    }
                    StatementIR::LetStmtIR {
                        value: Some(expr), ..
                    } => {
                        walk_expr_event_fields(expr, fields);
                    }
                    _ => {}
                }
            }
            walk_expr_event_fields(tail, fields);
        }
        ExprIR::AwaitExpr { operand } => walk_expr_event_fields(operand, fields),
        ExprIR::SpreadExpr { operand } => walk_expr_event_fields(operand, fields),
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr_event_fields(matched, fields);
            for arm in arms {
                collect_ir_event_fields(&arm.body, fields);
            }
        }
        ExprIR::VariantLiteral {
            fields: variant_fields,
            ..
        } => {
            for (_, v) in variant_fields {
                walk_expr_event_fields(v, fields);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                walk_expr_event_fields(arg, fields);
            }
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════
// Store lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_store(
    store: &StoreDef,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> StoreIR {
    let signals: FxHashMap<String, TypeIR> = store
        .signals
        .iter()
        .map(|s| (s.name.clone(), lower_type(&s.type_ann)))
        .collect();

    let signal_defaults: FxHashMap<String, serde_json::Value> = store
        .signals
        .iter()
        .map(|s| {
            let val = match &s.init {
                Expr::Literal(Literal::Int(n), _) => serde_json::Value::Number((*n).into()),
                Expr::Literal(Literal::Float(f), _) => {
                    serde_json::json!(f)
                }
                Expr::Literal(Literal::String(s), _) => serde_json::Value::String(s.clone()),
                Expr::Literal(Literal::Bool(b), _) => serde_json::Value::Bool(*b),
                Expr::Literal(Literal::Null, _) => serde_json::Value::Null,
                Expr::ArrayLiteral { elements, .. } => {
                    serde_json::Value::Array(elements.iter().map(expr_to_json_value).collect())
                }
                _ => {
                    // Non-literal initializer: serialize as a placeholder
                    // indicating the expression cannot be evaluated at compile time.
                    serde_json::Value::String("<runtime expr>".into())
                }
            };
            (s.name.clone(), val)
        })
        .collect();

    let getters: FxHashMap<String, ExprIR> = store
        .getters
        .iter()
        .map(|g| {
            (
                g.name.clone(),
                lower_expr(&g.body, variant_map, None, semantics),
            )
        })
        .collect();

    let actions: Vec<FunctionIR> = store
        .actions
        .iter()
        .map(|action| {
            semantics.symbols.push_scope(&format!("fn:{}", action.name));
            let result = FunctionIR {
                name: action.name.clone(),
                visibility: Visibility::Public,
                params: action
                    .params
                    .iter()
                    .map(|p| ParamIR {
                        name: p.name.clone(),
                        r#type: p.type_ann.as_ref().map(lower_type),
                    })
                    .collect(),
                return_type: action.return_type.as_ref().map(lower_type),
                body: lower_block(&action.body, variant_map, None, semantics),
                is_async: action.is_async,
                source_span: Some(action.span),
            };
            semantics.symbols.pop_scope();
            result
        })
        .collect();

    StoreIR {
        name: store.name.clone(),
        source_file: None,
        signals,
        signal_defaults,
        getters,
        actions,
        source_span: Some(store.span),
    }
}

/// Convert a simple AST expression to a JSON value (for store signal defaults).
pub(super) fn expr_to_json_value(expr: &Expr) -> serde_json::Value {
    match expr {
        Expr::Literal(Literal::Int(n), _) => serde_json::Value::Number((*n).into()),
        Expr::Literal(Literal::Float(f), _) => serde_json::json!(f),
        Expr::Literal(Literal::String(s), _) => serde_json::Value::String(s.clone()),
        Expr::Literal(Literal::Bool(b), _) => serde_json::Value::Bool(*b),
        Expr::Literal(Literal::Null, _) => serde_json::Value::Null,
        Expr::ArrayLiteral { elements, .. } => {
            serde_json::Value::Array(elements.iter().map(expr_to_json_value).collect())
        }
        _ => serde_json::Value::String(format!("{:?}", expr)),
    }
}

// ═══════════════════════════════════════════════════════
// Type lowering
// ═══════════════════════════════════════════════════════

pub(crate) fn lower_type(ty: &TypeAnnotation) -> TypeIR {
    match ty {
        TypeAnnotation::Named {
            name, type_args, ..
        } => {
            let args: Vec<TypeIR> = type_args.iter().map(lower_type).collect();
            let kind_args: Vec<TypeKind> = args.iter().map(|t| t.kind.clone()).collect();
            TypeIR {
                kind: TypeKind::Named {
                    name: name.clone(),
                    type_args: kind_args,
                },
                type_args: args,
                is_nullable: false,
            }
        }
        TypeAnnotation::Optional { base, .. } => {
            let mut inner = lower_type(base);
            inner.is_nullable = true;
            inner
        }
        TypeAnnotation::Union { types, .. } => {
            // Preserve all union variants by joining kind strings
            let kind_strs: Vec<String> = types
                .iter()
                .map(|t| {
                    let lowered = lower_type(t);
                    lowered.kind.as_str().to_string()
                })
                .collect();
            TypeIR {
                kind: TypeKind::Named {
                    name: kind_strs.join(" | "),
                    type_args: vec![],
                },
                type_args: vec![],
                is_nullable: false,
            }
        }
        TypeAnnotation::Fn { .. } => TypeIR {
            kind: TypeKind::Named {
                name: "function".into(),
                type_args: vec![],
            },
            type_args: vec![],
            is_nullable: false,
        },
    }
}
