// ═══════════════════════════════════════════════════════════
// Computed value codegen: computed → useMemo / useCallback
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::codegen::react::utils;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Generate useMemo / useCallback hooks for all @computed declarations.
pub(super) fn gen_computed_values(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    known_signal_names: &[String],
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
) {
    for computed in &comp.computeds {
        let mut deps_list: Vec<String> = Vec::new();
        let mut dep_names: FxHashSet<String> = FxHashSet::default();
        // Store-field reads inside the computed body dep on the SLICE local
        // (`[totalCount]`), never on the whole store object (T3.3): the
        // store hook object is recreated on every store write, so a
        // `[TodoStore]` dep would recompute on unrelated updates.
        let store_deps = collect_store_field_deps(&computed.body, store_field_renames);
        for sig_name in known_signal_names {
            let is_sliced_store_ref = store_deps
                .iter()
                .any(|(store, _)| store == sig_name.as_str());
            // Skip whole-store refs whose fields are all sliced — the slice
            // locals below are the real deps.
            if is_sliced_store_ref {
                continue;
            }
            if utils::getter_expr_contains_ident(&computed.body, sig_name) {
                deps_list.push(sig_name.clone());
                dep_names.insert(sig_name.clone());
            }
        }
        for (store, field) in &store_deps {
            if let Some(local) = store_field_renames.get(&format!("{}.{}", store, field)) {
                if dep_names.insert(local.clone()) {
                    deps_list.push(local.clone());
                }
            }
        }
        // Component props are memo deps too (review F1): a computed reading a
        // prop must re-derive when the parent re-renders with a new value.
        // Props were missing from the known-names set, so the memo silently
        // went stale — same bug class as perf HIGH-3. `comp.props` is
        // directly accessible here (no threading needed).
        for prop_name in comp.props.keys() {
            if utils::getter_expr_contains_ident(&computed.body, prop_name)
                && dep_names.insert(prop_name.clone())
            {
                deps_list.push(prop_name.clone());
            }
        }

        // Function-form computed: `computed name() -> Type { ... }` → useCallback
        // (detected by BlockExpr with ReturnStmtIR in statements)
        let is_fn_form = match &computed.body {
            ExprIR::BlockExpr { statements, .. } => statements
                .iter()
                .any(|s| matches!(s, StatementIR::ReturnStmtIR { .. })),
            _ => false,
        };

        if is_fn_form {
            w.emit_fmt(format_args!(
                "const {} = useCallback(() => {{",
                computed.name
            ));
        } else {
            w.emit_fmt(format_args!("const {} = useMemo(() => {{", computed.name));
        }
        w.indent();
        if let ExprIR::BlockExpr { statements, tail } = &computed.body {
            // Render preceding statements (let declarations, if/return, etc.)
            for stmt in statements {
                expr::generate_statement_tsx(
                    w,
                    stmt,
                    signal_names,
                    store_signal_owners,
                    ref_names,
                    store_field_renames,
                );
            }
            // Render tail as return value if non-null
            // (null tail means the block has explicit return statements)
            if !matches!(
                tail.as_ref(),
                ExprIR::LiteralExpr { value } if value == &serde_json::Value::Null
            ) {
                w.emit_fmt(format_args!(
                    "return {};",
                    expr::format_expr_ir_tsx_renamed(tail, ref_names, store_field_renames)
                ));
            }
        } else {
            w.emit_fmt(format_args!(
                "return {};",
                expr::format_expr_ir_tsx_renamed(
                    &computed.body,
                    ref_names,
                    store_field_renames
                )
            ));
        }
        w.dedent();
        w.emit_fmt(format_args!("}}, [{}]);", deps_list.join(", ")));
    }
}

/// Collect `(store_alias, field)` pairs referenced by a computed body —
/// both the `FieldAccessExpr { Ident(store), field }` and the semantic
/// `StoreFieldAccessExpr { store_alias, field_name }` forms. These become
/// memo deps on the slice locals.
fn collect_store_field_deps(
    expr: &ExprIR,
    store_field_renames: &FxHashMap<String, String>,
) -> Vec<(String, String)> {
    let mut acc: Vec<(String, String)> = Vec::new();
    let mut seen: FxHashSet<(String, String)> = FxHashSet::default();
    collect_store_field_deps_inner(expr, store_field_renames, &mut acc, &mut seen);
    acc
}

fn collect_store_field_deps_inner(
    expr: &ExprIR,
    store_field_renames: &FxHashMap<String, String>,
    acc: &mut Vec<(String, String)>,
    seen: &mut FxHashSet<(String, String)>,
) {
    match expr {
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if store_field_renames.contains_key(&format!("{}.{}", name, field))
                    && seen.insert((name.clone(), field.clone()))
                {
                    acc.push((name.clone(), field.clone()));
                }
            }
            collect_store_field_deps_inner(object, store_field_renames, acc, seen);
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            if store_field_renames.contains_key(&format!("{}.{}", store_alias, field_name))
                && seen.insert((store_alias.clone(), field_name.clone()))
            {
                acc.push((store_alias.clone(), field_name.clone()));
            }
            for arg in call_args {
                collect_store_field_deps_inner(arg, store_field_renames, acc, seen);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_store_field_deps_inner(left, store_field_renames, acc, seen);
            collect_store_field_deps_inner(right, store_field_renames, acc, seen);
        }
        ExprIR::UnaryOpExpr { operand, .. } => {
            collect_store_field_deps_inner(operand, store_field_renames, acc, seen);
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_store_field_deps_inner(cond, store_field_renames, acc, seen);
            collect_store_field_deps_inner(then_expr, store_field_renames, acc, seen);
            collect_store_field_deps_inner(else_expr, store_field_renames, acc, seen);
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                collect_store_field_deps_inner(arg, store_field_renames, acc, seen);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            collect_store_field_deps_inner(object, store_field_renames, acc, seen);
            for arg in args {
                collect_store_field_deps_inner(arg, store_field_renames, acc, seen);
            }
        }
        ExprIR::OptionalChainExpr { object, .. } | ExprIR::AwaitExpr { operand: object } => {
            collect_store_field_deps_inner(object, store_field_renames, acc, seen);
        }
        ExprIR::IndexExpr { object, index } => {
            collect_store_field_deps_inner(object, store_field_renames, acc, seen);
            collect_store_field_deps_inner(index, store_field_renames, acc, seen);
        }
        ExprIR::ArrayExpr { elements } => {
            for e in elements {
                collect_store_field_deps_inner(e, store_field_renames, acc, seen);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                collect_store_field_deps_inner(k, store_field_renames, acc, seen);
                collect_store_field_deps_inner(v, store_field_renames, acc, seen);
            }
        }
        ExprIR::ClosureExpr { body, .. } => {
            collect_store_field_deps_inner(body, store_field_renames, acc, seen);
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                collect_stmt_store_field_deps(stmt, store_field_renames, acc, seen);
            }
            collect_store_field_deps_inner(tail, store_field_renames, acc, seen);
        }
        ExprIR::MatchExpr { matched, arms } => {
            collect_store_field_deps_inner(matched, store_field_renames, acc, seen);
            for arm in arms {
                for stmt in &arm.body.statements {
                    collect_stmt_store_field_deps(stmt, store_field_renames, acc, seen);
                }
            }
        }
        ExprIR::VariantLiteral { fields, .. } | ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                collect_store_field_deps_inner(v, store_field_renames, acc, seen);
            }
        }
        ExprIR::SpreadExpr { operand } => {
            collect_store_field_deps_inner(operand, store_field_renames, acc, seen);
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            collect_store_field_deps_inner(subject, store_field_renames, acc, seen);
            collect_store_field_deps_inner(predicate, store_field_renames, acc, seen);
        }
        ExprIR::RefOpExpr { .. }
        | ExprIR::IdentifierExpr { .. }
        | ExprIR::LiteralExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

fn collect_stmt_store_field_deps(
    stmt: &StatementIR,
    store_field_renames: &FxHashMap<String, String>,
    acc: &mut Vec<(String, String)>,
    seen: &mut FxHashSet<(String, String)>,
) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => {
            collect_store_field_deps_inner(expr, store_field_renames, acc, seen);
        }
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(ref v) = value {
                collect_store_field_deps_inner(v, store_field_renames, acc, seen);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(ref v) = value {
                collect_store_field_deps_inner(v, store_field_renames, acc, seen);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_store_field_deps_inner(condition, store_field_renames, acc, seen);
            for s in &then_body.statements {
                collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
            }
            for (cond, body) in else_ifs {
                collect_store_field_deps_inner(cond, store_field_renames, acc, seen);
                for s in &body.statements {
                    collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_store_field_deps_inner(condition, store_field_renames, acc, seen);
            for s in &body.statements {
                collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_store_field_deps_inner(iterable, store_field_renames, acc, seen);
            for s in &body.statements {
                collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                collect_mutation_store_field_deps(intent, store_field_renames, acc, seen);
            }
        }
        StatementIR::ThrowStmtIR { expr } => {
            collect_store_field_deps_inner(expr, store_field_renames, acc, seen);
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_stmt_store_field_deps(s, store_field_renames, acc, seen);
                }
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                collect_store_field_deps_inner(arg, store_field_renames, acc, seen);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

fn collect_mutation_store_field_deps(
    intent: &MutationIntent,
    store_field_renames: &FxHashMap<String, String>,
    acc: &mut Vec<(String, String)>,
    seen: &mut FxHashSet<(String, String)>,
) {
    let expr = match intent {
        MutationIntent::DirectAssign { value, .. } => value.as_deref(),
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => amount.as_deref(),
        MutationIntent::Push { item, .. } => item.as_deref(),
        MutationIntent::Pop { .. } => None,
        #[allow(unreachable_patterns)]
        _ => None,
    };
    if let Some(e) = expr {
        collect_store_field_deps_inner(e, store_field_renames, acc, seen);
    }
}
