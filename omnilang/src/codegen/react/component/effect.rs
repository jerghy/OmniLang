// ═══════════════════════════════════════════════════════════
// Effect codegen: @effect/@mount/@unmount → useEffect
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::codegen::react::ReactCodegen;
use crate::codegen::CodeWriter;
use crate::ir::walk::walk_stmt;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

impl ReactCodegen {
    /// Generate useEffect calls for @effect declarations and lifecycle hooks
    /// (@mount, @unmount, @effect).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gen_effects(
        &self,
        w: &mut CodeWriter,
        comp: &ComponentIR,
        store_signal_map: &FxHashMap<String, String>,
        store_signal_owners: &FxHashMap<String, String>,
        signal_names: &FxHashSet<String>,
        ref_names: &FxHashSet<String>,
        store_field_renames: &FxHashMap<String, String>,
        store_fields: &FxHashMap<String, FxHashSet<String>>,
    ) {
        // Effects (useEffect) — wrap in async IIFE if body contains await
        for effect in &comp.effects {
            // Resolve store signal deps to their emitted variable names:
            // a slice-subscribed store field deps on the slice local
            // (`[notes]`, T3.3); otherwise the qualified name
            // (`NotificationStore.notifications`) or the bare name.
            let deps: Vec<String> = effect
                .deps
                .iter()
                .map(|d| {
                    let qualified = store_signal_map
                        .get(d)
                        .cloned()
                        .unwrap_or_else(|| d.clone());
                    store_field_renames
                        .get(&qualified)
                        .cloned()
                        .unwrap_or(qualified)
                })
                .collect();
            let deps_str = deps.join(", ");
            let body_has_await = effect.body.statements.iter().any(stmt_contains_await);
            if body_has_await {
                w.emit("useEffect(() => {");
                w.indent();
                w.emit("(async () => {");
                w.indent();
                for stmt in &effect.body.statements {
                    expr::generate_statement_tsx(
                        &mut *w,
                        stmt,
                        signal_names,
                        store_signal_owners,
                        ref_names,
                        store_field_renames,
                    );
                }
                w.dedent();
                w.emit("})();");
                w.dedent();
                w.emit_fmt(format_args!("}}, [{}]);", deps_str));
            } else {
                w.emit("useEffect(() => {");
                w.indent();
                for stmt in &effect.body.statements {
                    expr::generate_statement_tsx(
                        &mut *w,
                        stmt,
                        signal_names,
                        store_signal_owners,
                        ref_names,
                        store_field_renames,
                    );
                }
                w.dedent();
                w.emit_fmt(format_args!("}}, [{}]);", deps_str));
            }
        }

        // Lifecycles (mount/unmount)
        for lc in &comp.lifecycles {
            match lc.kind {
                LifecycleKind::Mount => {
                    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
                    if body_has_await {
                        w.emit("useEffect(() => {");
                        w.indent();
                        w.emit("(async () => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                            );
                        }
                        w.dedent();
                        w.emit("})();");
                        w.dedent();
                        w.emit("}, []);");
                    } else {
                        w.emit("useEffect(() => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                            );
                        }
                        w.dedent();
                        w.emit("}, []);");
                    }
                }
                LifecycleKind::Unmount => {
                    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
                    w.emit("useEffect(() => {");
                    w.indent();
                    w.emit("return () => {");
                    w.indent();
                    if body_has_await {
                        w.emit("(async () => {");
                        w.indent();
                    }
                    for stmt in &lc.body.statements {
                        expr::generate_statement_tsx(
                            &mut *w,
                            stmt,
                            signal_names,
                            store_signal_owners,
                            ref_names,
                            store_field_renames,
                        );
                    }
                    if body_has_await {
                        w.dedent();
                        w.emit("})();");
                    }
                    w.dedent();
                    w.emit("};");
                    w.dedent();
                    // Task 2.6 (ts/react M4): the cleanup closure reads must
                    // enter the deps — with `[]` the cleanup runs at unmount
                    // with the FIRST-render values (Timer's `final count`
                    // was always 0). Derived from the body reads: with
                    // `[count]` React re-registers the cleanup on every
                    // count change, so the unmount cleanup closes over the
                    // LATEST value.
                    //
                    // CONTRACT CONFLICT (mid-term item, review Important-1):
                    // derived deps re-run the WHOLE cleanup on every dep
                    // change — a state WRITE inside the cleanup executes
                    // early (Timer's `running = false` fired on every tick
                    // and killed the `if running` gate; the demo write was
                    // removed). Cleanups should only READ — a write + read
                    // shape needs a ref channel (latest-value ref, deps `[]`)
                    // or an explicit once-only contract. Tracked for the
                    // orchestrator (task 2.6 report concerns).
                    let unmount_deps = unmount_deps(
                        &lc.body,
                        comp,
                        store_signal_map,
                        store_field_renames,
                        store_fields,
                    );
                    let deps_str = unmount_deps.join(", ");
                    w.emit_fmt(format_args!("}}, [{}]);", deps_str));
                }
                LifecycleKind::Effect => {
                    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
                    if body_has_await {
                        w.emit("useEffect(() => {");
                        w.indent();
                        w.emit("(async () => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                            );
                        }
                        w.dedent();
                        w.emit("})();");
                        w.dedent();
                        w.emit("});");
                    } else {
                        w.emit("useEffect(() => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                            );
                        }
                        w.dedent();
                        w.emit("});");
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Unmount cleanup deps derivation (task 2.6, ts/react M4)
// ═══════════════════════════════════════════════════════════

/// Derive the useEffect deps for an `@unmount` cleanup from its body reads.
///
/// The cleanup closure runs at unmount with the values captured when the
/// effect was last registered — a read not in the deps closes over the
/// FIRST-render value (stale cleanup, Timer final-count bug). Every reactive
/// read enters the deps:
///   - component signals / computeds / props (bare names)
///   - store fields `Store.field` (resolved through the slice renames like
///     `@effect` deps — slice local, or the qualified whole-store read stays)
/// setState setters, store actions, function calls, and body locals never
/// enter the deps (stable or non-reactive — including them would re-register
/// the cleanup on every render and change the one-shot unmount semantics).
fn unmount_deps(
    body: &BlockIR,
    comp: &ComponentIR,
    store_signal_map: &FxHashMap<String, String>,
    store_field_renames: &FxHashMap<String, String>,
    store_fields: &FxHashMap<String, FxHashSet<String>>,
) -> Vec<String> {
    // Reactive bare names: component signals/computeds/props + store signal
    // names. Store ALIASES are excluded — a `Store.x` read resolves to the
    // qualified field (or slice local), never the whole-store alias.
    let mut reactive: FxHashSet<&str> = FxHashSet::default();
    reactive.extend(comp.signals.iter().map(|s| s.name.as_str()));
    reactive.extend(comp.computeds.iter().map(|c| c.name.as_str()));
    reactive.extend(comp.props.keys().map(String::as_str));
    reactive.extend(store_signal_map.keys().map(String::as_str));

    // Body locals (let-bound / for-in vars / catch vars) shadow reactive
    // names and never enter the deps.
    let mut locals: FxHashSet<&str> = FxHashSet::default();
    collect_body_locals(&body.statements, &mut locals);

    let mut reads: Vec<String> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for stmt in &body.statements {
        walk_stmt(stmt, &mut |expr| match expr {
            // `Store.field` / `Store?.field` — qualified read (the store
            // alias itself is never a bare dep).
            ExprIR::FieldAccessExpr { object, field }
            | ExprIR::OptionalChainExpr { object, field } => {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if store_fields
                        .get(name.as_str())
                        .is_some_and(|f| f.contains(field.as_str()))
                    {
                        push_read(format!("{}.{}", name, field), &mut seen, &mut reads);
                    }
                }
            }
            ExprIR::StoreFieldAccessExpr {
                store_alias,
                field_name,
                ..
            } => {
                if store_fields
                    .get(store_alias.as_str())
                    .is_some_and(|f| f.contains(field_name.as_str()))
                {
                    push_read(
                        format!("{}.{}", store_alias, field_name),
                        &mut seen,
                        &mut reads,
                    );
                }
            }
            // `Store.method(...)` — a store-field method call is a read
            // (e.g. `Store.items.len()` via its inner FieldAccessExpr arm
            // above); a store ACTION call (`Store.clear()`) never is.
            ExprIR::MethodCallExpr { object, method, .. } => {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if store_fields
                        .get(name.as_str())
                        .is_some_and(|f| f.contains(method.as_str()))
                    {
                        push_read(format!("{}.{}", name, method), &mut seen, &mut reads);
                    }
                }
            }
            // Bare identifier reads (signals/computeds/props/store signals),
            // excluding body locals.
            ExprIR::IdentifierExpr { name, .. } => {
                if reactive.contains(name.as_str()) && !locals.contains(name.as_str()) {
                    push_read(name.clone(), &mut seen, &mut reads);
                }
            }
            _ => {}
        });
    }

    // Resolve like `@effect` deps: bare store-signal names → `Store.field`,
    // then the slice renames → slice local (the qualified whole-store read
    // stays when no rename exists).
    reads
        .into_iter()
        .map(|read| {
            let qualified = store_signal_map.get(&read).cloned().unwrap_or(read);
            store_field_renames
                .get(&qualified)
                .cloned()
                .unwrap_or(qualified)
        })
        .collect()
}

/// Push a read unless already seen (dedupe — the walker visits nested
/// expressions like `count.to_string()`'s object twice).
fn push_read(read: String, seen: &mut FxHashSet<String>, reads: &mut Vec<String>) {
    if seen.insert(read.clone()) {
        reads.push(read);
    }
}

/// Collect locally-bound names in a statement list — they shadow reactive
/// names and never belong in the deps.
fn collect_body_locals<'a>(stmts: &'a [StatementIR], acc: &mut FxHashSet<&'a str>) {
    for stmt in stmts {
        match stmt {
            StatementIR::LetStmtIR { name, .. } => {
                acc.insert(name.as_str());
            }
            StatementIR::ForInStmtIR { var_name, body, .. } => {
                acc.insert(var_name.as_str());
                collect_body_locals(&body.statements, acc);
            }
            StatementIR::IfStmtIR {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_body_locals(&then_body.statements, acc);
                for (_, b) in else_ifs {
                    collect_body_locals(&b.statements, acc);
                }
                if let Some(b) = else_body {
                    collect_body_locals(&b.statements, acc);
                }
            }
            StatementIR::WhileStmtIR { body, .. } => collect_body_locals(&body.statements, acc),
            StatementIR::BlockStmtIR { body } => collect_body_locals(&body.statements, acc),
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_var,
                catch_body,
                finally_body,
                ..
            } => {
                collect_body_locals(&try_body.statements, acc);
                if let Some(cv) = catch_var {
                    acc.insert(cv);
                }
                if let Some(b) = catch_body {
                    collect_body_locals(&b.statements, acc);
                }
                if let Some(b) = finally_body {
                    collect_body_locals(&b.statements, acc);
                }
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Await detection helpers
// ═══════════════════════════════════════════════════════════

/// Check if a StatementIR or any nested statement contains an `await` expression.
pub(super) fn stmt_contains_await(stmt: &StatementIR) -> bool {
    match stmt {
        StatementIR::ExprStmtIR { expr } => expr_contains_await(expr),
        StatementIR::LetStmtIR { value, .. } => value.as_ref().is_some_and(expr_contains_await),
        StatementIR::ReturnStmtIR { value } => value.as_ref().is_some_and(expr_contains_await),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            if expr_contains_await(condition) {
                return true;
            }
            if then_body.statements.iter().any(stmt_contains_await) {
                return true;
            }
            for (_, elif_body) in else_ifs {
                if elif_body.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            if let Some(ref eb) = else_body {
                if eb.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            false
        }
        StatementIR::WhileStmtIR { body, .. } => body.statements.iter().any(stmt_contains_await),
        StatementIR::ForInStmtIR {
            var_name: _,
            iterable,
            body,
        } => expr_contains_await(iterable) || body.statements.iter().any(stmt_contains_await),
        StatementIR::BlockStmtIR { body } => body.statements.iter().any(stmt_contains_await),
        StatementIR::ThrowStmtIR { expr } => expr_contains_await(expr),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var: _,
            catch_body,
            finally_body,
        } => {
            if try_body.statements.iter().any(stmt_contains_await) {
                return true;
            }
            if let Some(ref cb) = catch_body {
                if cb.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            if let Some(ref fb) = finally_body {
                if fb.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            false
        }
        StatementIR::MutationStmt { .. }
        | StatementIR::BreakStmtIR
        | StatementIR::ContinueStmtIR
        | StatementIR::EventEmitStmtIR { .. } => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Check if an ExprIR contains an `await` expression anywhere in its tree.
pub(super) fn expr_contains_await(expr: &ExprIR) -> bool {
    match expr {
        ExprIR::AwaitExpr { .. } => true,
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_contains_await(left) || expr_contains_await(right)
        }
        ExprIR::UnaryOpExpr { operand, .. } => expr_contains_await(operand),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_contains_await(cond)
                || expr_contains_await(then_expr)
                || expr_contains_await(else_expr)
        }
        ExprIR::CallExpr { args, .. } => args.iter().any(expr_contains_await),
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_contains_await(object) || args.iter().any(expr_contains_await)
        }
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            expr_contains_await(object)
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => call_args.iter().any(expr_contains_await),
        ExprIR::ArrayExpr { elements } => elements.iter().any(expr_contains_await),
        ExprIR::SpreadExpr { operand } => expr_contains_await(operand),
        ExprIR::BlockExpr { statements, tail } => {
            statements.iter().any(stmt_contains_await) || expr_contains_await(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_contains_await(object) || expr_contains_await(index)
        }
        ExprIR::ClosureExpr { body, .. } => expr_contains_await(body),
        ExprIR::MatchExpr { matched, arms } => {
            if expr_contains_await(matched) {
                return true;
            }
            arms.iter()
                .any(|arm| arm.body.statements.iter().any(stmt_contains_await))
        }
        ExprIR::VariantLiteral { fields, .. } => fields.iter().any(|(_, v)| expr_contains_await(v)),
        ExprIR::StructExpr { fields, .. } => fields.iter().any(|(_, v)| expr_contains_await(v)),
        ExprIR::LiteralExpr { .. } | ExprIR::IdentifierExpr { .. } => false,
        ExprIR::MapExpr { entries } => entries
            .iter()
            .any(|(k, v)| expr_contains_await(k) || expr_contains_await(v)),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => expr_contains_await(subject) || expr_contains_await(predicate),
        // Ref ops are synchronous DOM operations — no await inside.
        ExprIR::RefOpExpr { .. } => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// Motion/effect detection in view tree
// ═══════════════════════════════════════════════════════════

/// Check if a view tree has motion triggers or effects that need useEffect.
/// All trigger types (Enter, Hover, Press, InView, Scroll, DragHorizontal,
/// DragVertical) generate useEffect hooks. Also checks in_view_once/in_view_repeat,
/// scroll, and drag fields.
pub(super) fn view_has_motion_or_effects(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            // Check for motion behaviors that generate useEffect
            if let Some(ref motion) = el.motion {
                // All TriggerKind variants generate useEffect hooks
                if motion.triggers.iter().any(|_| true) {
                    return true;
                }
                // in_view_once / in_view_repeat generates useEffect
                if motion.in_view_once.is_some() || motion.in_view_repeat.is_some() {
                    return true;
                }
            }
            // Note: @scroll / @drag-* control dimensions now live on the
            // trigger kinds (Task 5.1), so the trigger check above already
            // covers them — no separate scroll/drag field scan needed.
            // Check children
            el.children.iter().any(view_has_motion_or_effects)
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_has_motion_or_effects))
                || cond
                    .fallback
                    .as_ref()
                    .is_some_and(view_has_motion_or_effects)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_has_motion_or_effects)
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(view_has_motion_or_effects)
        }
        ViewTree::Fragment(children) => children.iter().any(view_has_motion_or_effects),
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_motion_or_effects),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}
