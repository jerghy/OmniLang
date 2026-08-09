//! Lowering for component and store definitions.
//!
//! Includes: `lower_component`, `lower_store`, `lower_type`.

use super::expr::{annotate_typed_struct_literals, lower_expr};
use super::stmt::lower_block;
use super::view::{lower_slot_common, lower_view_block, SlotDeclShape};
use crate::ir::logic::*;
use crate::ir::types::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

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

    // Names of this component's signal-like symbols (signals, computeds,
    // props, getters, refs). The style-channel lowering consults this set
    // to decide whether `obj.field` is a genuine signal read (OmniView
    // emits `obj.value.field`) or a plain local field access — the symbol
    // table cannot answer this at lowering time: ir::lower pushes fresh
    // empty `component:<name>` scopes, so the analysis-time component scope
    // (which holds these symbols) is not on the resolve() parent chain.
    let signal_like_names: FxHashSet<String> = comp
        .signals
        .iter()
        .map(|s| s.name.clone())
        .chain(comp.computeds.iter().map(|c| c.name.clone()))
        .chain(comp.props.iter().map(|p| p.name.clone()))
        .chain(comp.getters.iter().map(|g| g.name.clone()))
        .chain(comp.refs.iter().map(|r| r.name.clone()))
        .collect();

    // Body-level slot declarations (ADR-053 D5) lower through the same
    // shared logic as view-level `@slot` directives.
    let slots: Vec<SlotDeclIR> = comp
        .slots
        .iter()
        .map(|s| {
            let shape = SlotDeclShape {
                name: &s.name,
                default_content: &s.default_content,
            };
            lower_slot_common(&shape, variant_map, semantics, &fns, &signal_like_names)
        })
        .collect();

    let view = comp
        .view
        .as_ref()
        .map(|v| lower_view_block(v, variant_map, semantics, &fns, &signal_like_names));

    // 表单/数据三态意图（spec 2026-08-08 §4.1/§4.2）——AST 预扫描收集。
    // 内建信号（errors/submitting/loading/data）不在此生成信号声明：由语义
    // pass 校验保留名冲突、codegen 生成对应实现（三目标各自惯用形态）。
    let mut form_intent: Option<FormIntentIR> = None;
    let mut load_intent: Option<LoadIntentIR> = None;
    // V-F6 重复上报闸门：发现第二个即报，报完即止（恰好 1 条）。
    let mut form_dup_reported = false;
    let mut load_dup_reported = false;
    if let Some(v) = &comp.view {
        scan_view_intents(
            &v.children,
            variant_map,
            semantics,
            &mut form_intent,
            &mut load_intent,
            &mut form_dup_reported,
            &mut load_dup_reported,
        );
    }

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
        inline_stores: comp.inline_stores.clone(),
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
        form_intent,
        load_intent,
        source_span: Some(comp.span),
    }
}

/// 遍历组件视图收集 @form / @load 意图（spec 2026-08-08 §4.1/§4.2）。
/// V-F6（每组件至多一个 @form/@load）在此扫描处上报：发现第二个即报，
/// 报完即止（`*_dup_reported` 闸门保证恰好 1 条）；V-F1（@form 仅限
/// form 元素）在 collect_form_intent 内上报（首槽位），重复分支若落在
/// 非 form 元素上同样报 V-F1（R2：V-F1 优先于 V-F6）。
fn scan_view_intents(
    nodes: &[ViewNode],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    form_intent: &mut Option<FormIntentIR>,
    load_intent: &mut Option<LoadIntentIR>,
    form_dup_reported: &mut bool,
    load_dup_reported: &mut bool,
) {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                if el.props.iter().any(|p| matches!(p, ElementProp::Form)) {
                    if form_intent.is_none() {
                        if let Some(form) = collect_form_intent(el, variant_map, semantics) {
                            *form_intent = Some(form);
                        }
                    } else if !*form_dup_reported {
                        // R2：V-F1 优先于 V-F6——第二个 @form 落在非 form 元素上
                        // 报「@form 只能用于 form 元素」，否则报 V-F6 重复；两者
                        // 均置闸门（恰好 1 条）。
                        if el.tag != "form" {
                            semantics.diagnostics.push(crate::error::Diagnostic::error(
                                crate::error::DiagnosticKind::E0006,
                                "@form 只能用于 form 元素",
                                &el.span,
                            ));
                        } else {
                            semantics.diagnostics.push(crate::error::Diagnostic::error(
                                crate::error::DiagnosticKind::E0006,
                                "组件内 @form/@load 至多一个",
                                &el.span,
                            ));
                        }
                        *form_dup_reported = true;
                    }
                }
                scan_view_intents(
                    &el.children,
                    variant_map,
                    semantics,
                    form_intent,
                    load_intent,
                    form_dup_reported,
                    load_dup_reported,
                );
            }
            ViewNode::IfDirective(ifd) => {
                scan_view_intents(
                    &ifd.body,
                    variant_map,
                    semantics,
                    form_intent,
                    load_intent,
                    form_dup_reported,
                    load_dup_reported,
                );
                for (_, branch) in &ifd.else_ifs {
                    scan_view_intents(
                        branch,
                        variant_map,
                        semantics,
                        form_intent,
                        load_intent,
                        form_dup_reported,
                        load_dup_reported,
                    );
                }
                if let Some(else_body) = &ifd.else_body {
                    scan_view_intents(
                        else_body,
                        variant_map,
                        semantics,
                        form_intent,
                        load_intent,
                        form_dup_reported,
                        load_dup_reported,
                    );
                }
            }
            ViewNode::ForDirective(fd) => {
                scan_view_intents(
                    &fd.body,
                    variant_map,
                    semantics,
                    form_intent,
                    load_intent,
                    form_dup_reported,
                    load_dup_reported,
                );
                if let Some(empty) = &fd.empty_body {
                    scan_view_intents(
                        empty,
                        variant_map,
                        semantics,
                        form_intent,
                        load_intent,
                        form_dup_reported,
                        load_dup_reported,
                    );
                }
            }
            ViewNode::SlotDirective {
                default_content, ..
            } => {
                if let Some(fallback) = default_content {
                    scan_view_intents(
                        fallback,
                        variant_map,
                        semantics,
                        form_intent,
                        load_intent,
                        form_dup_reported,
                        load_dup_reported,
                    );
                }
            }
            ViewNode::LoadDirective { fn_name, span } => {
                if load_intent.is_none() {
                    *load_intent = Some(LoadIntentIR {
                        fn_name: fn_name.clone(),
                    });
                } else if !*load_dup_reported {
                    // V-F6：组件内第二个 @load
                    semantics.diagnostics.push(crate::error::Diagnostic::error(
                        crate::error::DiagnosticKind::E0006,
                        "组件内 @form/@load 至多一个",
                        span,
                    ));
                    *load_dup_reported = true;
                }
            }
            ViewNode::Text(_) | ViewNode::Interpolation(_) => {}
        }
    }
}

/// 从 @form 元素构造表单意图：onSubmit 的 FnRef 为 submit_handler
/// （InlineFn/StoreAction 不接受，缺失诊断属语义 pass V-F2），
/// 子树内带 validate: 的元素（+ 同元素 bind: 信号路径末段）为字段。
fn collect_form_intent(
    el: &Element,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> Option<FormIntentIR> {
    if !el.props.iter().any(|p| matches!(p, ElementProp::Form)) {
        return None;
    }
    // V-F1：@form 只能在 <form> 元素上（非法 tag 报错且不占用组件唯一的
    // @form 槽位——V-F6 重复检测不因非法元素误报）。
    if el.tag != "form" {
        semantics.diagnostics.push(crate::error::Diagnostic::error(
            crate::error::DiagnosticKind::E0006,
            "@form 只能用于 form 元素",
            &el.span,
        ));
        return None;
    }
    let submit_handler = el
        .props
        .iter()
        .find_map(|p| match p {
            ElementProp::Event {
                name,
                handler: EventHandler::FnRef(f),
            } if name == "onSubmit" => Some(f.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let mut fields = Vec::new();
    collect_validated_fields(&el.children, variant_map, semantics, &mut fields);
    Some(FormIntentIR {
        fields,
        submit_handler,
    })
}

/// 收集子树内所有带 validate: 的字段元素：字段名为同元素 bind: 信号路径
/// 末段（validate 无 bind 的缺失诊断属语义 pass V-F3）。
fn collect_validated_fields(
    nodes: &[ViewNode],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    out: &mut Vec<FormFieldIR>,
) {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                let rules: Vec<ValidateRuleIR> = el
                    .props
                    .iter()
                    .filter_map(|p| match p {
                        ElementProp::Validate { rules } => Some(rules.clone()),
                        _ => None,
                    })
                    .flatten()
                    .map(|r| ValidateRuleIR {
                        name: r.name.clone(),
                        value: lower_expr(&r.value, variant_map, None, semantics),
                    })
                    .collect();
                if !rules.is_empty() {
                    let name = el
                        .props
                        .iter()
                        .find_map(|p| match p {
                            ElementProp::Bind { signal_path, .. } => {
                                signal_path.rsplit('.').next().map(str::to_string)
                            }
                            _ => None,
                        })
                        .unwrap_or_default();
                    out.push(FormFieldIR { name, rules });
                }
                collect_validated_fields(&el.children, variant_map, semantics, out);
            }
            ViewNode::IfDirective(ifd) => {
                collect_validated_fields(&ifd.body, variant_map, semantics, out);
                for (_, branch) in &ifd.else_ifs {
                    collect_validated_fields(branch, variant_map, semantics, out);
                }
                if let Some(else_body) = &ifd.else_body {
                    collect_validated_fields(else_body, variant_map, semantics, out);
                }
            }
            ViewNode::ForDirective(fd) => {
                collect_validated_fields(&fd.body, variant_map, semantics, out);
                if let Some(empty) = &fd.empty_body {
                    collect_validated_fields(empty, variant_map, semantics, out);
                }
            }
            ViewNode::SlotDirective {
                default_content, ..
            } => {
                if let Some(fallback) = default_content {
                    collect_validated_fields(fallback, variant_map, semantics, out);
                }
            }
            ViewNode::LoadDirective { .. } | ViewNode::Text(_) | ViewNode::Interpolation(_) => {}
        }
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

    // v2 域 5 (design D1): the @theme-marked signal is THE current theme mode.
    // At most one per store by construction (single Option field); the
    // program-level validate_theme pass enforces at most one per program.
    // Same-store duplicates are invisible to that pass (the Option only
    // carries one name), so they hard-error here during lowering.
    let theme_signals: Vec<&SignalDecl> = store.signals.iter().filter(|s| s.is_theme).collect();
    for dup in theme_signals.iter().skip(1) {
        semantics.diagnostics.push(crate::error::Diagnostic::error(
            crate::error::DiagnosticKind::E0006,
            &format!(
                "重复 @theme 信号：'{}.{}'（一个程序只能有一个当前主题；删除多余的 @theme 标记）",
                store.name, dup.name
            ),
            &store.span,
        ));
    }
    let theme_signal = theme_signals.first().map(|s| s.name.clone());

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
            let mut result = FunctionIR {
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
            // Task 3.1 (CRITICAL flutter): rewrite bare struct literals
            // assigned to typed signals/params into typed StructExprs —
            // `{ id: nextId, text, done: false }` → `Todo(id: ...)` so Dart
            // emits constructors instead of Map<String, Object> literals
            // (the TodoStore.add() copyWith cast TypeError).
            let mut expected_types: FxHashMap<String, TypeIR> = signals.clone();
            for p in &result.params {
                if let Some(ty) = &p.r#type {
                    expected_types.insert(p.name.clone(), ty.clone());
                }
            }
            annotate_typed_struct_literals(&mut result.body, &expected_types);
            result
        })
        .collect();

    StoreIR {
        name: store.name.clone(),
        source_file: None,
        theme_signal,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::analyze;

    /// Lower the first component of a source snippet and clone it out of the IR program.
    fn lower_component_src(src: &str) -> ComponentIR {
        let tokens = tokenize(src);
        let (program, _) = parse(tokens, src);
        let mut semantics = analyze(&program);
        let ir = crate::ir::lower(&program, &mut semantics, None);
        let module = ir.modules.get("input.omni").expect("should have module");
        let (_, comp) = module
            .components
            .iter()
            .next()
            .expect("should have a component");
        comp.clone()
    }

    /// Like `lower_component_src`, but also returns the lowering diagnostics —
    /// V-F1/V-F6 errors are emitted during scan_view_intents (semantic-diagnostic
    /// sink), which the plain helper discards.
    fn lower_component_diags(src: &str) -> (ComponentIR, Vec<crate::error::Diagnostic>) {
        let tokens = tokenize(src);
        let (program, _) = parse(tokens, src);
        let mut semantics = analyze(&program);
        let ir = crate::ir::lower(&program, &mut semantics, None);
        let module = ir.modules.get("input.omni").expect("should have module");
        let (_, comp) = module
            .components
            .iter()
            .next()
            .expect("should have a component");
        (comp.clone(), semantics.diagnostics)
    }

    #[test]
    fn lower_form_intent_collects_validated_fields() {
        let ir = lower_component_src(
            r#"component App {
                fn submit() {}
                signal name: String = ""
                view {
                    <form @form @onSubmit={submit}>
                        <input bind:value={name} validate:{required:true}/>
                    </form>
                }
            }"#,
        );
        let form = ir.form_intent.expect("form intent");
        assert_eq!(form.submit_handler, "submit");
        assert_eq!(form.fields.len(), 1);
        assert_eq!(form.fields[0].name, "name");
        assert_eq!(form.fields[0].rules.len(), 1);
    }

    #[test]
    fn lower_load_directive_carries_fn_name() {
        let ir = lower_component_src("component App { fn f() {} view { @load={f} <div/> } }");
        assert_eq!(ir.load_intent.expect("load intent").fn_name, "f");
    }

    /// V-F1：@form 只能在 <form> 元素上 → error「@form 只能用于 form 元素」，
    /// 且非法元素不占用组件唯一的 @form 槽位（form_intent 保持 None）。
    #[test]
    fn lower_form_rejects_non_form_tag() {
        let (ir, diags) = lower_component_diags(
            r#"component App {
                fn submit() {}
                view { <div @form @onSubmit={submit} /> }
            }"#,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("@form 只能用于 form 元素")),
            "V-F1 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            ir.form_intent.is_none(),
            "invalid @form must not occupy the slot"
        );
    }

    /// V-F6：组件内第二个 @form → error「组件内 @form/@load 至多一个」，
    /// 报完即止（恰好 1 条）。
    #[test]
    fn lower_form_duplicate_reports_once() {
        let (_, diags) = lower_component_diags(
            r#"component App {
                fn submit() {}
                view {
                    <form @form @onSubmit={submit} />
                    <form @form @onSubmit={submit} />
                }
            }"#,
        );
        let count = diags
            .iter()
            .filter(|d| d.message.contains("组件内 @form/@load 至多一个"))
            .count();
        assert_eq!(count, 1, "V-F6 must report exactly once, got: {:?}", diags);
    }

    /// R2：V-F1 优先于 V-F6——第一个 @form 占槽后，第二个 @form 落在非
    /// form 元素上 → error「@form 只能用于 form 元素」（而非重复报 V-F6）。
    #[test]
    fn lower_form_dup_on_non_form_reports_vf1() {
        let (_, diags) = lower_component_diags(
            r#"component App {
                fn submit() {}
                view {
                    <form @form @onSubmit={submit} />
                    <div @form @onSubmit={submit} />
                }
            }"#,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("@form 只能用于 form 元素")),
            "R2 V-F1 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        let vf6_count = diags
            .iter()
            .filter(|d| d.message.contains("组件内 @form/@load 至多一个"))
            .count();
        assert_eq!(
            vf6_count, 0,
            "R2 must not report V-F6 here, got: {:?}",
            diags
        );
    }

    /// R2：两个 @form 均在合法 form 元素上 → 仍报 V-F6（V-F1 不介入），
    /// 报完即止（恰好 1 条）。
    #[test]
    fn lower_form_dup_on_form_reports_vf6() {
        let (_, diags) = lower_component_diags(
            r#"component App {
                fn submit() {}
                view {
                    <form @form @onSubmit={submit} />
                    <form @form @onSubmit={submit} />
                }
            }"#,
        );
        let vf6_count = diags
            .iter()
            .filter(|d| d.message.contains("组件内 @form/@load 至多一个"))
            .count();
        assert_eq!(
            vf6_count, 1,
            "V-F6 must report exactly once, got: {:?}",
            diags
        );
        let vf1_count = diags
            .iter()
            .filter(|d| d.message.contains("@form 只能用于 form 元素"))
            .count();
        assert_eq!(vf1_count, 0, "V-F1 must not fire here, got: {:?}", diags);
    }

    /// V-F6：组件内第二个 @load → error「组件内 @form/@load 至多一个」，
    /// 报完即止（恰好 1 条）。
    #[test]
    fn lower_load_duplicate_reports_once() {
        let (_, diags) = lower_component_diags(
            "component App { fn f() {} view { @load={f} <div/> @load={f} <div/> } }",
        );
        let count = diags
            .iter()
            .filter(|d| d.message.contains("组件内 @form/@load 至多一个"))
            .count();
        assert_eq!(count, 1, "V-F6 must report exactly once, got: {:?}", diags);
    }
}
