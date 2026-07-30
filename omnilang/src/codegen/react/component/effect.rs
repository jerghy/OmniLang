// ═══════════════════════════════════════════════════════════
// Effect codegen: @effect/@mount/@unmount → useEffect
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::codegen::react::ReactCodegen;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

impl ReactCodegen {
    /// Generate useEffect calls for @effect declarations and lifecycle hooks
    /// (@mount, @unmount, @effect).
    pub(super) fn gen_effects(
        &self,
        w: &mut CodeWriter,
        comp: &ComponentIR,
        store_signal_map: &FxHashMap<String, String>,
        signal_names: &FxHashSet<String>,
        ref_names: &FxHashSet<String>,
    ) {
        // Effects (useEffect) — wrap in async IIFE if body contains await
        for effect in &comp.effects {
            // Resolve store signal deps to qualified names (e.g., "notifications" → "NotificationStore.notifications")
            let deps: Vec<String> = effect
                .deps
                .iter()
                .map(|d| {
                    store_signal_map
                        .get(d)
                        .cloned()
                        .unwrap_or_else(|| d.clone())
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
                    expr::generate_statement_tsx_with_mode(
                        &mut *w,
                        stmt,
                        signal_names,
                        self.store_mode,
                        ref_names,
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
                    expr::generate_statement_tsx_with_mode(
                        &mut *w,
                        stmt,
                        signal_names,
                        self.store_mode,
                        ref_names,
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
                            expr::generate_statement_tsx_with_mode(
                                &mut *w,
                                stmt,
                                signal_names,
                                self.store_mode,
                                ref_names,
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
                            expr::generate_statement_tsx_with_mode(
                                &mut *w,
                                stmt,
                                signal_names,
                                self.store_mode,
                                ref_names,
                            );
                        }
                        w.dedent();
                        w.emit("}, []);");
                    }
                }
                LifecycleKind::Unmount => {
                    w.emit("useEffect(() => {");
                    w.indent();
                    w.emit("return () => {");
                    w.indent();
                    for stmt in &lc.body.statements {
                        expr::generate_statement_tsx_with_mode(
                            &mut *w,
                            stmt,
                            signal_names,
                            self.store_mode,
                            ref_names,
                        );
                    }
                    w.dedent();
                    w.emit("};");
                    w.dedent();
                    w.emit("}, []);");
                }
                LifecycleKind::Effect => {
                    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
                    if body_has_await {
                        w.emit("useEffect(() => {");
                        w.indent();
                        w.emit("(async () => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx_with_mode(
                                &mut *w,
                                stmt,
                                signal_names,
                                self.store_mode,
                                ref_names,
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
                            expr::generate_statement_tsx_with_mode(
                                &mut *w,
                                stmt,
                                signal_names,
                                self.store_mode,
                                ref_names,
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
    }
}

// ═══════════════════════════════════════════════════════════
// Motion/effect detection in view tree
// ═══════════════════════════════════════════════════════════

/// Check if a view tree has motion triggers or effects that need useEffect.
/// Only returns true for motion types that actually generate useEffect hooks:
/// @enter, @hover/@press/@focus triggers, driven_by, in_view, scroll, drag.
/// Bare @timeline references do NOT need useEffect.
pub(super) fn view_has_motion_or_effects(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            // Check for motion behaviors that generate useEffect
            if let Some(ref motion) = el.motion {
                // Triggers like @enter, @hover, @press, @focus generate useEffect
                // @timeline and @animate triggers do NOT generate useEffect (they're WAAPI-only)
                if motion.triggers.iter().any(|_| true) {
                    return true;
                }
                // in_view_once / in_view_repeat generates useEffect
                if motion.in_view_once.is_some() || motion.in_view_repeat.is_some() {
                    return true;
                }
            }
            // scroll and drag always generate useEffect
            if el.scroll.is_some() || el.drag.is_some() {
                return true;
            }
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
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => false,
    }
}
