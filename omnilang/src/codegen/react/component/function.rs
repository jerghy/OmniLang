// ═══════════════════════════════════════════════════════════
// Function codegen: fn / action → local function declarations
// ═══════════════════════════════════════════════════════════

use super::collect_expr_identifiers;
use super::effect;
use crate::codegen::capitalize_first;
use crate::codegen::react::expr;
use crate::codegen::react::ReactCodegen;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

impl ReactCodegen {
    /// Generate component-local function declarations (fn → const fn = (...) => {...}).
    /// Only generates functions that are actually called from the view/effects/lifecycles.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gen_functions(
        &self,
        w: &mut CodeWriter,
        comp: &ComponentIR,
        called_fns: &FxHashSet<String>,
        signal_names: &FxHashSet<String>,
        store_signal_owners: &FxHashMap<String, String>,
        fn_names: &FxHashSet<String>,
        ref_names: &FxHashSet<String>,
        store_field_renames: &FxHashMap<String, String>,
        pruned_signal_names: &FxHashSet<String>,
        possibly_undefined: &FxHashSet<String>,
        direct_write_signals: &FxHashSet<String>,
        direct_write_applies: &FxHashMap<String, Vec<String>>,
    ) {
        for fun in &comp.fns {
            if !called_fns.contains(&fun.name) {
                continue;
            }
            let param_names: Vec<(String, bool)> = fun
                .params
                .iter()
                .map(|p| {
                    let escaped = crate::codegen::react::escape_js_ident(&p.name);
                    let needs_rename = escaped != p.name;
                    (escaped, needs_rename)
                })
                .collect();
            let params_str = format_fn_params(fun);
            let is_async =
                fun.is_async || fun.body.statements.iter().any(effect::stmt_contains_await);
            let async_prefix = if is_async { "async " } else { "" };
            w.emit_fmt(format_args!(
                "const {} = {}({}) => {{",
                fun.name, async_prefix, params_str
            ));
            w.indent();
            // Emit body into temp writer for post-processing
            // (v4 H1, task 2.1: 写后读线程化——`x = v; use(x)` → const nextX
            // 线程化，无写后读时原样返回。)
            let body_statements = expr::rewrite_write_through(&fun.body.statements);
            let mut body_w = crate::codegen::CodeWriter::new();
            body_w.indent();
            let tail_idx = body_statements.len().saturating_sub(1);
            for (i, stmt) in body_statements.iter().enumerate() {
                if i == tail_idx {
                    if let StatementIR::ExprStmtIR { .. } = stmt {
                        // H1: the fn's trailing expression IS its return
                        // value — emit `return <expr>;` instead of the bare
                        // `{expr};` the generic machinery produces (which
                        // silently dropped match/block/identifier tail
                        // values — `error_message` returned undefined and
                        // DataFetching's success branch was unreachable).
                        emit_fn_tail_return(
                            &mut body_w,
                            stmt,
                            signal_names,
                            store_signal_owners,
                            ref_names,
                            store_field_renames,
                            pruned_signal_names,
                            possibly_undefined,
                            direct_write_signals,
                            direct_write_applies,
                        );
                        continue;
                    }
                }
                expr::generate_statement_tsx(
                    &mut body_w,
                    stmt,
                    signal_names,
                    store_signal_owners,
                    ref_names,
                    store_field_renames,
                    pruned_signal_names,
                    possibly_undefined,
                    direct_write_signals,
                    direct_write_applies,
                );
            }
            // Build setter renames for states whose setters conflict with function names
            let setter_renames: Vec<(String, String)> = comp
                .signals
                .iter()
                .filter_map(|s| {
                    let setter_name = format!("set{}", capitalize_first(&s.name));
                    if fn_names.contains(&setter_name) {
                        Some((setter_name.clone(), format!("__{}", setter_name)))
                    } else {
                        None
                    }
                })
                .collect();

            let mut body_str = body_w.into_string();
            // Apply param renames (reserved words)
            for (i, (new_name, needs_rename)) in param_names.iter().enumerate() {
                if *needs_rename {
                    let old_name = &fun.params[i].name;
                    body_str = body_str.replace(old_name, new_name);
                }
            }
            // Apply setter renames (conflict with function names)
            for (old_setter, new_setter) in &setter_renames {
                body_str = body_str.replace(old_setter, new_setter);
            }
            for line in body_str.lines() {
                w.emit(line);
            }
            w.dedent();
            w.emit("};");
        }
        if !comp.fns.is_empty() {
            w.emit("");
        }
    }
}

/// Emit a fn body's TAIL expression statement as `return <expr>;`.
///
/// The DSL's trailing expression IS the fn's value (H1, ts/react): before
/// this fix the tail `ExprStmtIR` was emitted as a bare `{expr};`
/// statement, so match/block/identifier tail values were silently dropped
/// and the fn returned undefined — DataFetching's `error_message` always
/// produced falsy and its success branch was unreachable.
///
/// The statement is formatted through the FULL statement machinery
/// (array_push/array_pop interception, store-field renames, setter
/// dispatch) into a temp writer, then re-emitted with `return` prepended —
/// statement emission always produces exactly one line. The param/setter
/// renames applied later in `gen_functions` are string replacements over
/// the whole body, so they cover this line too.
// 8 cohesive context params (mirrors generate_statement_tsx — same
// namespaces); struct-bundling is a larger refactor than this lint warrants
// for an internal helper.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_fn_tail_return(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    ref_names: &FxHashSet<String>,
    renames: &FxHashMap<String, String>,
    pruned_signal_names: &FxHashSet<String>,
    possibly_undefined: &FxHashSet<String>,
    direct_write_signals: &FxHashSet<String>,
    direct_write_applies: &FxHashMap<String, Vec<String>>,
) {
    let mut tmp = crate::codegen::CodeWriter::new();
    expr::generate_statement_tsx(
        &mut tmp,
        stmt,
        signal_names,
        store_signal_owners,
        ref_names,
        renames,
        pruned_signal_names,
        possibly_undefined,
        direct_write_signals,
        direct_write_applies,
    );
    let line = tmp.into_string();
    let line = line.trim_end_matches(['\n', '\r']);
    if line.is_empty() {
        // Formatted to nothing (null value) — nothing to return.
        return;
    }
    let core = line.strip_suffix(';').unwrap_or(line);
    w.emit_fmt(format_args!("return {};", core));
}

// ═══════════════════════════════════════════════════════════
// Called function tracking — determines which local functions
// are actually referenced from views/effects/lifecycles.
// ═══════════════════════════════════════════════════════════

/// Format a function's parameter list with TypeScript type annotations from
/// the IR param types (TS batch: `(name: string, score: number)` instead of
/// bare names — previously every generated fn param was implicit any).
/// Params without an IR type annotation stay bare (the source had none).
pub(super) fn format_fn_params(fun: &FunctionIR) -> String {
    fun.params
        .iter()
        .map(|p| {
            let escaped = crate::codegen::react::escape_js_ident(&p.name);
            match p.r#type.as_ref() {
                Some(t) => {
                    let ty = JsTypeFormatter.format_type(t.kind.as_str(), &t.type_args);
                    if ty.is_empty() {
                        escaped
                    } else {
                        format!("{}: {}", escaped, ty)
                    }
                }
                None => escaped,
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Collect names of local component functions that are called from the view
/// or from effects/lifecycles (transitively).
pub(super) fn collect_called_local_fns(comp: &ComponentIR) -> FxHashSet<String> {
    let mut acc = FxHashSet::default();

    // From view event handlers and props
    if let Some(ref view) = comp.view {
        collect_called_fns_from_view(view, &mut acc);
    }
    // From effects
    for effect in &comp.effects {
        for stmt in &effect.body.statements {
            collect_called_fns_from_stmt(stmt, &mut acc);
        }
    }
    // From lifecycles
    for lc in &comp.lifecycles {
        for stmt in &lc.body.statements {
            collect_called_fns_from_stmt(stmt, &mut acc);
        }
    }
    // From computeds
    for computed in &comp.computeds {
        collect_called_fns_from_expr(&computed.body, &mut acc);
    }

    // Transitive closure: scan bodies of called fns for calls to other local fns
    let mut prev_len = 0;
    while acc.len() > prev_len {
        prev_len = acc.len();
        for fun in &comp.fns {
            if acc.contains(&fun.name) {
                for stmt in &fun.body.statements {
                    collect_called_fns_from_stmt(stmt, &mut acc);
                }
            }
        }
        // Also scan fn bodies from view inline handlers for transitive calls
        if let Some(ref view) = comp.view {
            collect_called_fns_from_view_transitively(view, &mut acc);
        }
    }

    acc
}

fn collect_called_fns_from_view_transitively(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    for stmt in &body.statements {
                        collect_called_fns_from_stmt(stmt, acc);
                    }
                }
            }
            for child in &el.children {
                collect_called_fns_from_view_transitively(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_called_fns_from_view_transitively(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_called_fns_from_view_transitively(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_called_fns_from_view_transitively(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_called_fns_from_view_transitively(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_called_fns_from_view_transitively(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_called_fns_from_view_transitively(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

fn collect_called_fns_from_view(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        acc.insert(r#ref.clone());
                    }
                    EventHandlerIR::InlineFn { body, .. } => {
                        for stmt in &body.statements {
                            collect_called_fns_from_stmt(stmt, acc);
                        }
                    }
                    EventHandlerIR::StoreAction { .. } => {}
                }
            }
            // Also scan class_map_bindings for function calls (e.g. isActive())
            for binding in el.class_map_bindings.values() {
                collect_called_fns_from_expr(binding, acc);
            }
            // C1: prop values may CALL component fns (src={getTitle()}) or
            // reference them by VALUE (transform={shout}) — the emitted JSX
            // references both forms, so both must be collected. Identifier
            // names that are not component fns (signals, loop items, props)
            // are filtered by the consumer (gen_functions matches against
            // comp.fns), so the collection never over-emits.
            for prop in el.props.values() {
                collect_called_fns_from_expr(prop, acc);
                collect_expr_identifiers(prop, acc);
            }
            // Semantic ref bindings (focus={expr}, value={expr}, ...) emit
            // their expressions inside generated effects — calls there are
            // referenced by the emitted code and need declarations.
            for binding_expr in el.semantic_bindings.values() {
                collect_called_fns_from_expr(binding_expr, acc);
            }
            // C1: inline styles lower to JSON with `$call` markers rendered
            // as `fnName(args)` by format_style_value_expr — the called
            // function must be declared (getScoreColor(row.score)).
            if let Some(ref style_json) = el.inline_style {
                collect_called_fns_from_style_json(style_json, acc);
            }
            // el.bindings are SignalRefIR (name-only references) — they can
            // never carry function calls, so nothing to collect here (the
            // mutation side is handled by collect_signal_mutations).
            for child in &el.children {
                collect_called_fns_from_view(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_called_fns_from_expr(&branch.condition, acc);
                if let Some(ref body) = branch.body {
                    collect_called_fns_from_view(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_called_fns_from_view(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            collect_called_fns_from_expr(&list.iterable, acc);
            if let Some(ref body) = list.body {
                collect_called_fns_from_view(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_called_fns_from_view(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_called_fns_from_view(child, acc);
            }
        }
        // C1: template interpolations ({getScoreColor(row.score)}) emit the
        // expression verbatim — calls inside must be declared.
        ViewTree::Interpolation(expr) => {
            collect_called_fns_from_expr(expr, acc);
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_called_fns_from_view(child, acc);
            }
        }
        ViewTree::Text(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Collect function names from inline-style JSON values. Styles lower to
/// marker objects (`{"$call": {"name": "fn", "args": [...]}}` plus nested
/// `$signal` / `$ident` / `$field` / `$concat`), mirroring
/// `collect_inline_style_signals` in state.rs (which collects the signal
/// half of the same channel). Args nest arbitrarily, so the walk recurses
/// through the whole value.
fn collect_called_fns_from_style_json(val: &serde_json::Value, acc: &mut FxHashSet<String>) {
    match val {
        serde_json::Value::Object(obj) => {
            if let Some(call_obj) = obj.get("$call").and_then(|v| v.as_object()) {
                if let Some(fn_name) = call_obj.get("name").and_then(|v| v.as_str()) {
                    acc.insert(fn_name.to_string());
                }
            }
            for v in obj.values() {
                collect_called_fns_from_style_json(v, acc);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_called_fns_from_style_json(item, acc);
            }
        }
        _ => {}
    }
}

pub(super) fn collect_called_fns_from_stmt(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => {
            collect_called_fns_from_expr(expr, acc);
        }
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(ref v) = value {
                collect_called_fns_from_expr(v, acc);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(ref v) = value {
                collect_called_fns_from_expr(v, acc);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_called_fns_from_expr(condition, acc);
            for s in &then_body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
            for (cond, body) in else_ifs {
                collect_called_fns_from_expr(cond, acc);
                for s in &body.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_called_fns_from_expr(condition, acc);
            for s in &body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_called_fns_from_expr(iterable, acc);
            for s in &body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(MutationIntent::DirectAssign {
                value: Some(ref v), ..
            }) = intent
            {
                collect_called_fns_from_expr(v, acc);
            }
        }
        StatementIR::ThrowStmtIR { expr } => {
            collect_called_fns_from_expr(expr, acc);
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            // `emit onX(fnCall())` emits the argument expressions verbatim.
            for arg in args {
                collect_called_fns_from_expr(arg, acc);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

fn collect_called_fns_from_expr(expr: &ExprIR, acc: &mut FxHashSet<String>) {
    match expr {
        ExprIR::CallExpr { name, args, .. } => {
            acc.insert(name.clone());
            for arg in args {
                collect_called_fns_from_expr(arg, acc);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            collect_called_fns_from_expr(object, acc);
            for arg in args {
                collect_called_fns_from_expr(arg, acc);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_called_fns_from_expr(left, acc);
            collect_called_fns_from_expr(right, acc);
        }
        ExprIR::UnaryOpExpr { operand, .. } => {
            collect_called_fns_from_expr(operand, acc);
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_called_fns_from_expr(cond, acc);
            collect_called_fns_from_expr(then_expr, acc);
            collect_called_fns_from_expr(else_expr, acc);
        }
        ExprIR::ArrayExpr { elements } => {
            for elem in elements {
                collect_called_fns_from_expr(elem, acc);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                collect_called_fns_from_expr(k, acc);
                collect_called_fns_from_expr(v, acc);
            }
        }
        ExprIR::ClosureExpr { body, .. } => {
            collect_called_fns_from_expr(body, acc);
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                collect_called_fns_from_stmt(stmt, acc);
            }
            collect_called_fns_from_expr(tail, acc);
        }
        ExprIR::IndexExpr { object, index } => {
            collect_called_fns_from_expr(object, acc);
            collect_called_fns_from_expr(index, acc);
        }
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            collect_called_fns_from_expr(object, acc);
        }
        ExprIR::MatchExpr { matched, arms } => {
            collect_called_fns_from_expr(matched, acc);
            for arm in arms {
                for stmt in &arm.body.statements {
                    collect_called_fns_from_stmt(stmt, acc);
                }
            }
        }
        ExprIR::VariantLiteral { fields, .. } | ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                collect_called_fns_from_expr(v, acc);
            }
        }
        ExprIR::SpreadExpr { operand } => {
            collect_called_fns_from_expr(operand, acc);
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                collect_called_fns_from_expr(arg, acc);
            }
        }
        ExprIR::AwaitExpr { operand } => {
            collect_called_fns_from_expr(operand, acc);
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            collect_called_fns_from_expr(subject, acc);
            collect_called_fns_from_expr(predicate, acc);
        }
        ExprIR::IdentifierExpr { .. } | ExprIR::LiteralExpr { .. } => {}
        // Ref ops don't call functions — their args are value expressions.
        ExprIR::RefOpExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}
