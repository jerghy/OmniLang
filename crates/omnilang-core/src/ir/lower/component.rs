//! Lowering for component and store definitions.
//!
//! Includes: `lower_component`, `lower_store`, `lower_type`.

use super::expr::lower_expr;
use super::stmt::lower_block;
use super::view::{lower_slot_common, lower_view_block, SlotDeclShape};
use crate::ir::logic::*;
use crate::ir::types::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;
use std::cell::Cell;

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
    let signals: Vec<SignalIntent> = comp
        .signals
        .iter()
        .map(|s| SignalIntent {
            name: s.name.clone(),
            r#type: lower_type(&s.type_ann),
            initial_value: lower_expr(&s.init, variant_map, None, semantics),
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

    // The input auto-binding counter is shared between body-slot fallbacks
    // and the view block, so `<input>` elements receive string-signal
    // auto-binding in declaration order across both (ADR-053 D5).
    let sig_idx = Cell::new(0usize);

    // Body-level slot declarations (ADR-053 D5) lower through the same
    // shared logic as view-level `@slot` directives.
    let slots: Vec<SlotDeclIR> = comp
        .slots
        .iter()
        .map(|s| {
            let shape = SlotDeclShape {
                name: &s.name,
                default_content: &s.default_content,
                type_constraint: &s.type_constraint,
            };
            lower_slot_common(
                &shape,
                &input_signals,
                Some(&sig_idx),
                variant_map,
                semantics,
                &fns,
            )
        })
        .collect();

    let view = comp
        .view
        .as_ref()
        .map(|v| lower_view_block(v, &input_signals, &sig_idx, variant_map, semantics, &fns));

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

    ComponentIR {
        name: comp.name.clone(),
        source_file: None,
        props,
        prop_defaults,
        signals,
        computeds,
        effects,
        lifecycles,
        fns,
        store_refs: comp.store_refs.clone(),
        refs: comp
            .refs
            .iter()
            .map(|r| RefDeclIR {
                name: r.name.clone(),
                element_tag: None,
            })
            .collect(),
        events: comp
            .events
            .iter()
            .map(|e| EventDeclIR {
                name: e.name.clone(),
                param_type: type_ann_to_ts(&e.type_ann),
            })
            .collect(),
        slots,
        view,
        styles,
        source_span: Some(comp.span),
    }
}

/// Convert an OmniLang TypeAnnotation to a TypeScript type string for React props.
fn type_ann_to_ts(ta: &TypeAnnotation) -> String {
    match ta {
        TypeAnnotation::Named {
            name, type_args, ..
        } => {
            let name_str = name.as_str();
            match name_str {
                "string" | "String" | "str" => "string".into(),
                "i32" | "i64" | "f32" | "f64" | "number" | "int" | "float" | "num" => {
                    "number".into()
                }
                "bool" | "Bool" | "boolean" => "boolean".into(),
                "void" | "Void" | "()" | "unit" => "void".into(),
                "any" | "auto" | "var" | "Any" => "any".into(),
                "Array" if type_args.len() == 1 => {
                    format!("{}[]", type_ann_to_ts(&type_args[0]))
                }
                _ => name_str.to_string(),
            }
        }
        TypeAnnotation::Optional { base, .. } => {
            format!("{} | undefined", type_ann_to_ts(base))
        }
        TypeAnnotation::Union { types, .. } => types
            .iter()
            .map(type_ann_to_ts)
            .collect::<Vec<_>>()
            .join(" | "),
        TypeAnnotation::Fn { .. } => "Function".into(),
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

    let getter_types: FxHashMap<String, TypeIR> = store
        .getters
        .iter()
        .map(|g| {
            let ty = lower_type(&g.type_ann);
            (g.name.clone(), ty)
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
        getter_types,
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
            TypeIR {
                kind: TypeKind::from(name.as_str()),
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
                },
                type_args: vec![],
                is_nullable: false,
            }
        }
        TypeAnnotation::Fn { .. } => TypeIR {
            kind: TypeKind::Named {
                name: "function".into(),
            },
            type_args: vec![],
            is_nullable: false,
        },
    }
}
