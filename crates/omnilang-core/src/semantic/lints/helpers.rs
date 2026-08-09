//! Shared helpers for lint rules: AST collectors, store/prop/computed validation.

use super::LintDiagnostic;
use crate::ir::{
    BlockIR, EventHandlerIR, ExprIR, MutationIntent, SignalRefIR, StatementIR, ViewTree,
};
use crate::lexer::Span;
use crate::parser::ast::{
    AssignTarget, BinaryOpKind, Block, ElementProp, EventHandler, Expr, ImportStmt, Stmt,
    TemplatePart, ViewNode,
};
use crate::parser::expr::span_of;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Recursively check if a block's statement tree contains a return.
pub(super) fn statements_contain_return(stmts: &[StatementIR], _in_loop: bool) -> bool {
    for stmt in stmts {
        match stmt {
            StatementIR::ReturnStmtIR { .. } => return true,
            StatementIR::IfStmtIR {
                then_body,
                else_body,
                ..
            } => {
                if statements_contain_return(&then_body.statements, _in_loop) {
                    return true;
                }
                if let Some(ref else_body) = else_body {
                    if statements_contain_return(&else_body.statements, _in_loop) {
                        return true;
                    }
                }
            }
            StatementIR::BlockStmtIR { body } => {
                if statements_contain_return(&body.statements, _in_loop) {
                    return true;
                }
            }
            StatementIR::WhileStmtIR { body, .. } => {
                if statements_contain_return(&body.statements, true) {
                    return true;
                }
            }
            StatementIR::ForInStmtIR { body, .. } => {
                if statements_contain_return(&body.statements, true) {
                    return true;
                }
            }
            StatementIR::LetStmtIR { .. }
            | StatementIR::MutationStmt { .. }
            | StatementIR::EventEmitStmtIR { .. }
            | StatementIR::ExprStmtIR { .. }
            | StatementIR::BreakStmtIR
            | StatementIR::ContinueStmtIR
            | StatementIR::ThrowStmtIR { .. } => {}
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                if statements_contain_return(&try_body.statements, _in_loop) {
                    if let Some(ref cb) = catch_body {
                        if statements_contain_return(&cb.statements, _in_loop) {
                            return true;
                        }
                    }
                }
                if let Some(ref fb) = finally_body {
                    if statements_contain_return(&fb.statements, _in_loop) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Get source line/col from an ImportStmt for diagnostic reporting.
pub(super) fn import_span_line_col(import: &ImportStmt) -> (u32, u32) {
    match import {
        ImportStmt::Named(n) => (n.span.line, n.span.col),
        ImportStmt::Namespace(ns) => (ns.span.line, ns.span.col),
        ImportStmt::TypeOnly(t) => (t.span.line, t.span.col),
        ImportStmt::Lib(_) => (0, 0),
    }
}

// ── Store / Prop / Computed validation helpers ──────

/// Recursively walk a BlockIR looking for store signal mutations.
///
/// `span` is the best-known source position of the enclosing context (the
/// owning function's / element's span) — diagnostics anchor there instead of
/// hardcoded 0:0 (Task 4.11).
pub(super) fn check_block_ir_for_store_mutations(
    block: &BlockIR,
    store_signals: &FxHashMap<&str, FxHashSet<&str>>,
    context_name: &str,
    file_path: &str,
    span: Option<Span>,
    diags: &mut Vec<LintDiagnostic>,
) {
    for stmt in &block.statements {
        match stmt {
            StatementIR::MutationStmt {
                intent: Some(ref mutation),
            } => {
                let _target = match mutation {
                    MutationIntent::DirectAssign { target, .. }
                    | MutationIntent::Increment { target, .. }
                    | MutationIntent::Decrement { target, .. }
                    | MutationIntent::Multiply { target, .. }
                    | MutationIntent::Divide { target, .. }
                    | MutationIntent::Modulo { target, .. }
                    | MutationIntent::Push { target, .. }
                    | MutationIntent::Pop { target } => target,
                };
                // Note: `owner` field removed from SignalRefIR — this lint
                // relied on it to detect store-field mutations. Revisit if
                // store-field-direct-mutation detection is needed.
            }
            StatementIR::IfStmtIR {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_block_ir_for_store_mutations(
                    then_body,
                    store_signals,
                    context_name,
                    file_path,
                    span,
                    diags,
                );
                for (_, body) in else_ifs {
                    check_block_ir_for_store_mutations(
                        body,
                        store_signals,
                        context_name,
                        file_path,
                        span,
                        diags,
                    );
                }
                if let Some(ref body) = else_body {
                    check_block_ir_for_store_mutations(
                        body,
                        store_signals,
                        context_name,
                        file_path,
                        span,
                        diags,
                    );
                }
            }
            StatementIR::WhileStmtIR { body, .. } | StatementIR::ForInStmtIR { body, .. } => {
                check_block_ir_for_store_mutations(
                    body,
                    store_signals,
                    context_name,
                    file_path,
                    span,
                    diags,
                );
            }
            StatementIR::BlockStmtIR { body } => {
                check_block_ir_for_store_mutations(
                    body,
                    store_signals,
                    context_name,
                    file_path,
                    span,
                    diags,
                );
            }
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                check_block_ir_for_store_mutations(
                    try_body,
                    store_signals,
                    context_name,
                    file_path,
                    span,
                    diags,
                );
                if let Some(ref body) = catch_body {
                    check_block_ir_for_store_mutations(
                        body,
                        store_signals,
                        context_name,
                        file_path,
                        span,
                        diags,
                    );
                }
                if let Some(ref body) = finally_body {
                    check_block_ir_for_store_mutations(
                        body,
                        store_signals,
                        context_name,
                        file_path,
                        span,
                        diags,
                    );
                }
            }
            StatementIR::ExprStmtIR { expr }
                if expr_ir_contains_store_mutation(expr, store_signals) =>
            {
                let (line, col) = span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
                diags.push(LintDiagnostic {
                    rule: "store-field-direct-mutation".into(),
                    severity: super::LintSeverity::Error,
                    message: format!(
                        "Expression in '{}' mutates store data — \
                             store data must only be changed through store actions",
                        context_name
                    ),
                    file: file_path.to_string(),
                    line,
                    col,
                });
            }
            _ => {}
        }
    }
}

/// Check if an ExprIR contains a store mutation (method calls like .set/.push on store fields).
pub(super) fn expr_ir_contains_store_mutation(
    expr: &ExprIR,
    store_signals: &FxHashMap<&str, FxHashSet<&str>>,
) -> bool {
    match expr {
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            call_args,
            ..
        } => {
            if !call_args.is_empty() {
                return store_signals.contains_key(store_alias.as_str());
            }
            false
        }
        ExprIR::MethodCallExpr { object, method, .. } => {
            if matches!(
                method.as_str(),
                "set" | "push" | "pop" | "inc" | "increment" | "dec" | "decrement"
            ) {
                if let ExprIR::StoreFieldAccessExpr { store_alias, .. } = object.as_ref() {
                    return store_signals.contains_key(store_alias.as_str());
                }
                if let ExprIR::FieldAccessExpr { object: inner, .. } = object.as_ref() {
                    if let ExprIR::IdentifierExpr { name, .. } = inner.as_ref() {
                        return store_signals.contains_key(name.as_str());
                    }
                }
            }
            expr_ir_contains_store_mutation(object, store_signals)
        }
        ExprIR::BinaryOpExpr {
            left, right, op, ..
        } => {
            if matches!(
                op.as_str(),
                "assign" | "addassign" | "subassign" | "mulassign" | "divassign"
            ) {
                if let ExprIR::StoreFieldAccessExpr { store_alias, .. } = left.as_ref() {
                    return store_signals.contains_key(store_alias.as_str());
                }
            }
            expr_ir_contains_store_mutation(left, store_signals)
                || expr_ir_contains_store_mutation(right, store_signals)
        }
        ExprIR::CallExpr { args, .. } => args
            .iter()
            .any(|a| expr_ir_contains_store_mutation(a, store_signals)),
        ExprIR::FieldAccessExpr { object, .. }
        | ExprIR::OptionalChainExpr { object, .. }
        | ExprIR::UnaryOpExpr {
            operand: object, ..
        }
        | ExprIR::AwaitExpr {
            operand: object, ..
        } => expr_ir_contains_store_mutation(object, store_signals),
        ExprIR::ArrayExpr { elements } => elements
            .iter()
            .any(|e| expr_ir_contains_store_mutation(e, store_signals)),
        ExprIR::StructExpr { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_ir_contains_store_mutation(v, store_signals)),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_ir_contains_store_mutation(cond, store_signals)
                || expr_ir_contains_store_mutation(then_expr, store_signals)
                || expr_ir_contains_store_mutation(else_expr, store_signals)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_ir_contains_store_mutation(object, store_signals)
                || expr_ir_contains_store_mutation(index, store_signals)
        }
        ExprIR::ClosureExpr { body, .. } => expr_ir_contains_store_mutation(body, store_signals),
        ExprIR::BlockExpr { statements, tail } => {
            statements.iter().any(|s| {
                if let StatementIR::MutationStmt {
                    intent: Some(ref _m),
                } = s
                {
                    // Note: `owner` field removed from SignalRefIR — store-field
                    // mutation detection is no longer available via target.owner.
                }
                false
            }) || expr_ir_contains_store_mutation(tail, store_signals)
        }
        ExprIR::MatchExpr { matched, arms } => {
            expr_ir_contains_store_mutation(matched, store_signals)
                || arms.iter().any(|arm| {
                    arm.body.statements.iter().any(|s| {
                        if let StatementIR::MutationStmt {
                            intent: Some(ref _m),
                        } = s
                        {
                            // Note: `owner` field removed from SignalRefIR — store-field
                            // mutation detection is no longer available via target.owner.
                        }
                        false
                    })
                })
        }
        ExprIR::VariantLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_ir_contains_store_mutation(v, store_signals)),
        ExprIR::SpreadExpr { operand } => expr_ir_contains_store_mutation(operand, store_signals),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            expr_ir_contains_store_mutation(subject, store_signals)
                || expr_ir_contains_store_mutation(predicate, store_signals)
        }
        _ => false,
    }
}

/// Walk a ViewTree recursively and check event handler bodies for store mutations.
///
/// `span` is the enclosing context's position; elements narrow it to their
/// own `source_span` (Task 4.11).
pub(super) fn check_view_tree_events_for_store_mutations(
    tree: &ViewTree,
    store_signals: &FxHashMap<&str, FxHashSet<&str>>,
    file_path: &str,
    span: Option<Span>,
    diags: &mut Vec<LintDiagnostic>,
) {
    match tree {
        ViewTree::Element(el) => {
            let el_span = el.source_span.or(span);
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    check_block_ir_for_store_mutations(
                        body,
                        store_signals,
                        "inline event handler",
                        file_path,
                        el_span,
                        diags,
                    );
                }
            }
            for child in &el.children {
                check_view_tree_events_for_store_mutations(
                    child,
                    store_signals,
                    file_path,
                    el_span,
                    diags,
                );
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    check_view_tree_events_for_store_mutations(
                        body,
                        store_signals,
                        file_path,
                        span,
                        diags,
                    );
                }
            }
            if let Some(ref fallback) = cond.fallback {
                check_view_tree_events_for_store_mutations(
                    fallback,
                    store_signals,
                    file_path,
                    span,
                    diags,
                );
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                check_view_tree_events_for_store_mutations(
                    body,
                    store_signals,
                    file_path,
                    span,
                    diags,
                );
            }
            if let Some(ref empty) = list.empty_body {
                check_view_tree_events_for_store_mutations(
                    empty,
                    store_signals,
                    file_path,
                    span,
                    diags,
                );
            }
        }
        _ => {}
    }
}

/// Check a Block for assignments to component props.
pub(super) fn check_block_for_prop_assign(
    block: &Block,
    prop_names: &FxHashSet<&str>,
    comp_name: &str,
    context_name: &str,
    file_path: &str,
    diags: &mut Vec<LintDiagnostic>,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Assign {
                target: AssignTarget::Name(name),
                span,
                ..
            } => {
                if prop_names.contains(name.as_str()) {
                    diags.push(LintDiagnostic {
                        rule: "prop-immutability".into(),
                        severity: super::LintSeverity::Error,
                        message: format!(
                            "Cannot assign to prop '{}' in '{}' of component '{}' — \
                             component props are read-only",
                            name, context_name, comp_name
                        ),
                        file: file_path.to_string(),
                        line: span.line,
                        col: span.col,
                    });
                }
            }
            Stmt::If {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_block_for_prop_assign(
                    then_body,
                    prop_names,
                    comp_name,
                    context_name,
                    file_path,
                    diags,
                );
                for (_, body) in else_ifs {
                    check_block_for_prop_assign(
                        body,
                        prop_names,
                        comp_name,
                        context_name,
                        file_path,
                        diags,
                    );
                }
                if let Some(ref body) = else_body {
                    check_block_for_prop_assign(
                        body,
                        prop_names,
                        comp_name,
                        context_name,
                        file_path,
                        diags,
                    );
                }
            }
            Stmt::While { body, .. } | Stmt::ForIn { body, .. } => {
                check_block_for_prop_assign(
                    body,
                    prop_names,
                    comp_name,
                    context_name,
                    file_path,
                    diags,
                );
            }
            Stmt::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                check_block_for_prop_assign(
                    try_body,
                    prop_names,
                    comp_name,
                    context_name,
                    file_path,
                    diags,
                );
                if let Some(ref body) = catch_body {
                    check_block_for_prop_assign(
                        body,
                        prop_names,
                        comp_name,
                        context_name,
                        file_path,
                        diags,
                    );
                }
                if let Some(ref body) = finally_body {
                    check_block_for_prop_assign(
                        body,
                        prop_names,
                        comp_name,
                        context_name,
                        file_path,
                        diags,
                    );
                }
            }
            _ => {}
        }
    }
    if let Some(ref expr) = block.trailing_expr {
        if expr_contains_prop_assign(expr, prop_names) {
            let span = span_of(expr);
            diags.push(LintDiagnostic {
                rule: "prop-immutability".into(),
                severity: super::LintSeverity::Error,
                message: format!(
                    "Cannot assign to prop in '{}' of component '{}' — \
                     component props are read-only",
                    context_name, comp_name
                ),
                file: file_path.to_string(),
                line: span.line,
                col: span.col,
            });
        }
    }
}

/// Check if an expression assigns to a prop.
pub(super) fn expr_contains_prop_assign(expr: &Expr, prop_names: &FxHashSet<&str>) -> bool {
    match expr {
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            if matches!(
                op,
                BinaryOpKind::Assign
                    | BinaryOpKind::AddAssign
                    | BinaryOpKind::SubAssign
                    | BinaryOpKind::MulAssign
                    | BinaryOpKind::DivAssign
                    | BinaryOpKind::ModAssign
            ) {
                if let Expr::VarRef(name, _) = left.as_ref() {
                    return prop_names.contains(name.as_str());
                }
            }
            expr_contains_prop_assign(left, prop_names)
                || expr_contains_prop_assign(right, prop_names)
        }
        Expr::Block { body, .. } => body.stmts.iter().any(|s| {
            if let Stmt::Assign {
                target: AssignTarget::Name(name),
                ..
            } = s
            {
                return prop_names.contains(name.as_str());
            }
            false
        }),
        _ => false,
    }
}

/// Walk view nodes to find prop assignments in inline event handlers.
pub(super) fn check_view_for_prop_assign(
    nodes: &[ViewNode],
    prop_names: &FxHashSet<&str>,
    comp_name: &str,
    file_path: &str,
    diags: &mut Vec<LintDiagnostic>,
) {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                for prop in &el.props {
                    if let ElementProp::Event {
                        handler: EventHandler::InlineFn { body, .. },
                        ..
                    } = prop
                    {
                        check_block_for_prop_assign(
                            body,
                            prop_names,
                            comp_name,
                            "inline event handler",
                            file_path,
                            diags,
                        );
                    }
                }
                check_view_for_prop_assign(&el.children, prop_names, comp_name, file_path, diags);
            }
            ViewNode::IfDirective(if_dir) => {
                check_view_for_prop_assign(&if_dir.body, prop_names, comp_name, file_path, diags);
                for (_, body) in &if_dir.else_ifs {
                    check_view_for_prop_assign(body, prop_names, comp_name, file_path, diags);
                }
                if let Some(ref else_body) = if_dir.else_body {
                    check_view_for_prop_assign(else_body, prop_names, comp_name, file_path, diags);
                }
            }
            ViewNode::ForDirective(for_dir) => {
                check_view_for_prop_assign(&for_dir.body, prop_names, comp_name, file_path, diags);
                if let Some(ref empty_body) = for_dir.empty_body {
                    check_view_for_prop_assign(empty_body, prop_names, comp_name, file_path, diags);
                }
            }
            _ => {}
        }
    }
}

/// Check if an AST expression contains side effects (mutations).
/// Used by ComputedPurity to ensure computed values are pure.
pub(super) fn expr_contains_side_effect(expr: &Expr, signal_names: &FxHashSet<&str>) -> bool {
    match expr {
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            if matches!(
                op,
                BinaryOpKind::Assign
                    | BinaryOpKind::AddAssign
                    | BinaryOpKind::SubAssign
                    | BinaryOpKind::MulAssign
                    | BinaryOpKind::DivAssign
                    | BinaryOpKind::ModAssign
            ) {
                if let Expr::VarRef(name, _) = left.as_ref() {
                    if signal_names.contains(name.as_str()) {
                        return true;
                    }
                }
            }
            expr_contains_side_effect(left, signal_names)
                || expr_contains_side_effect(right, signal_names)
        }
        Expr::MethodCall { object, method, .. } => {
            if matches!(
                method.as_str(),
                "set" | "setValue" | "push" | "pop" | "inc" | "increment" | "dec" | "decrement"
            ) {
                if let Expr::VarRef(name, _) = object.as_ref() {
                    if signal_names.contains(name.as_str()) {
                        return true;
                    }
                }
            }
            expr_contains_side_effect(object, signal_names)
        }
        Expr::FuncCall { func, args, .. } => {
            if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
                if matches!(
                    field.as_str(),
                    "set" | "setValue" | "push" | "pop" | "inc" | "increment" | "dec" | "decrement"
                ) {
                    if let Expr::VarRef(name, _) = object.as_ref() {
                        if signal_names.contains(name.as_str()) {
                            return true;
                        }
                    }
                }
            }
            expr_contains_side_effect(func, signal_names)
                || args
                    .iter()
                    .any(|a| expr_contains_side_effect(a, signal_names))
        }
        Expr::Block { body, .. } => {
            for stmt in &body.stmts {
                match stmt {
                    Stmt::Assign {
                        target: AssignTarget::Name(name),
                        ..
                    } => {
                        if signal_names.contains(name.as_str()) {
                            return true;
                        }
                    }
                    Stmt::ExprStmt { expr: e, .. } => {
                        if expr_contains_side_effect(e, signal_names) {
                            return true;
                        }
                    }
                    Stmt::Return { value: Some(e), .. } | Stmt::Print { arg: e, .. }
                        if expr_contains_side_effect(e, signal_names) =>
                    {
                        return true;
                    }
                    _ => {}
                }
            }
            if let Some(ref trailing) = body.trailing_expr {
                expr_contains_side_effect(trailing, signal_names)
            } else {
                false
            }
        }
        Expr::UnaryOp { operand, .. } => expr_contains_side_effect(operand, signal_names),
        Expr::FieldAccess { object, .. }
        | Expr::OptionalChain { object, .. }
        | Expr::IndexGet { object, .. } => expr_contains_side_effect(object, signal_names),
        Expr::ArrayLiteral { elements, .. } => elements
            .iter()
            .any(|e| expr_contains_side_effect(e, signal_names)),
        Expr::MapLiteral { entries, .. } => entries.iter().any(|(k, v)| {
            expr_contains_side_effect(k, signal_names) || expr_contains_side_effect(v, signal_names)
        }),
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
            expr_contains_side_effect(condition, signal_names)
                || expr_contains_side_effect(then_expr, signal_names)
                || expr_contains_side_effect(else_expr, signal_names)
        }
        Expr::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_contains_side_effect(v, signal_names)),
        Expr::TemplateString { parts, .. } => parts.iter().any(|p| {
            if let TemplatePart::Expr(e) = p {
                expr_contains_side_effect(e, signal_names)
            } else {
                false
            }
        }),
        Expr::ClosureExpr { body, .. } => expr_contains_side_effect(body, signal_names),
        Expr::AwaitExpr { operand, .. } => expr_contains_side_effect(operand, signal_names),
        Expr::MatchExpr { matched, arms, .. } => {
            expr_contains_side_effect(matched, signal_names)
                || arms.iter().any(|arm| {
                    arm.body.stmts.iter().any(|s| {
                        if let Stmt::Assign {
                            target: AssignTarget::Name(name),
                            ..
                        } = s
                        {
                            return signal_names.contains(name.as_str());
                        }
                        false
                    }) || arm
                        .body
                        .trailing_expr
                        .as_ref()
                        .is_some_and(|e| expr_contains_side_effect(e, signal_names))
                })
        }
        Expr::NotYetImplemented { .. } => false,
        Expr::IsCheck { .. } => false,
        Expr::StoreFieldAccess { .. } => false,
        Expr::SpreadExpr { operand, .. } => expr_contains_side_effect(operand, signal_names),
        Expr::Literal(..) | Expr::VarRef(..) => false,
        Expr::Error { .. } => false,
        // 批次 1 Task 1 占位——showToast 调用是副作用（纯函数净化应拒绝）
        Expr::BuiltinToastCall { .. } => true,
    }
}

/// Extract the target from any MutationIntent enum variant.
/// Note: currently unused since `owner` field was removed from SignalRefIR.
#[allow(dead_code)]
fn mutation_intent_target(intent: &MutationIntent) -> &SignalRefIR {
    match intent {
        MutationIntent::DirectAssign { target, .. }
        | MutationIntent::Increment { target, .. }
        | MutationIntent::Decrement { target, .. }
        | MutationIntent::Multiply { target, .. }
        | MutationIntent::Divide { target, .. }
        | MutationIntent::Modulo { target, .. }
        | MutationIntent::Push { target, .. }
        | MutationIntent::Pop { target } => target,
    }
}

// ── Test helpers ─────────────────────────────────────

#[cfg(test)]
pub(super) mod test_helpers {
    use crate::ir::{ComponentIR, IRProgram, ModuleIR, StoreIR};
    use rustc_hash::FxHashMap;

    /// Helper to construct an IRProgram with a single module for testing.
    pub fn make_test_ir(
        components: FxHashMap<String, ComponentIR>,
        stores: FxHashMap<String, StoreIR>,
    ) -> IRProgram {
        let mut modules = FxHashMap::default();
        modules.insert(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                stores,
                ..Default::default()
            },
        );
        IRProgram {
            version: "1.0".into(),
            modules,
            source_map: FxHashMap::default(),
            stdlib_imports: Default::default(),
            source_span: None,
            motion_index: FxHashMap::default(),
        }
    }
}
