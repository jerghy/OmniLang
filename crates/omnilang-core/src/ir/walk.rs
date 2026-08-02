//! Shared IR tree walkers with callback-based visitors.
//!
//! These walkers are building blocks for analysis passes (dead-code elimination,
//! event-field collection, etc.) that need to traverse IR trees.
//!
//! # Functions
//!
//! - `walk_expr`: visits every `ExprIR` node in an expression tree.
//! - `walk_stmt`: visits every `ExprIR` node reachable from a statement.
//! - `walk_view`: visits every `ElementIR` node in a view tree.

use crate::ir::*;

/// Walk an `ExprIR` tree and call `f` for each expression node (pre-order).
///
/// The callback is invoked on the root expression first, then on every
/// sub-expression reached via recursive descent. This includes expressions
/// nested inside statements (`BlockExpr`, `MatchExpr` arms, etc.).
pub fn walk_expr(expr: &ExprIR, f: &mut impl FnMut(&ExprIR)) {
    f(expr);
    match expr {
        ExprIR::LiteralExpr { .. } => {}
        ExprIR::IdentifierExpr { .. } => {}
        ExprIR::FieldAccessExpr { object, .. } => walk_expr(object, f),
        ExprIR::OptionalChainExpr { object, .. } => walk_expr(object, f),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr(left, f);
            walk_expr(right, f);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr(operand, f),
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr(arg, f);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr(object, f);
            for arg in args {
                walk_expr(arg, f);
            }
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr(cond, f);
            walk_expr(then_expr, f);
            walk_expr(else_expr, f);
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr(object, f);
            walk_expr(index, f);
        }
        ExprIR::ArrayExpr { elements } => {
            for elem in elements {
                walk_expr(elem, f);
            }
        }
        ExprIR::SpreadExpr { operand } => walk_expr(operand, f),
        ExprIR::StructExpr { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr(field_expr, f);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                walk_expr(arg, f);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                walk_stmt(stmt, f);
            }
            walk_expr(tail, f);
        }
        ExprIR::ClosureExpr { body, .. } => walk_expr(body, f),
        ExprIR::AwaitExpr { operand } => walk_expr(operand, f),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr(field_expr, f);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr(matched, f);
            for arm in arms {
                for stmt in &arm.body.statements {
                    walk_stmt(stmt, f);
                }
            }
        }
        ExprIR::MapExpr { entries } => {
            for (key, value) in entries {
                walk_expr(key, f);
                walk_expr(value, f);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            walk_expr(subject, f);
            walk_expr(predicate, f);
        }
        ExprIR::RefOpExpr { op, .. } => {
            walk_ref_op(op, f);
        }
    }
}

/// Walk the `ExprIR` children carried by a `RefOpIR` operation.
fn walk_ref_op(op: &RefOpIR, f: &mut impl FnMut(&ExprIR)) {
    match op {
        RefOpIR::Focus
        | RefOpIR::Blur
        | RefOpIR::Measure { .. }
        | RefOpIR::Select
        | RefOpIR::Value => {}
        RefOpIR::ScrollTo { x, y } => {
            if let Some(x) = x {
                walk_expr(x, f);
            }
            if let Some(y) = y {
                walk_expr(y, f);
            }
        }
        RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                walk_expr(dx, f);
            }
            if let Some(dy) = dy {
                walk_expr(dy, f);
            }
        }
        RefOpIR::SetSelection { start, end } => {
            walk_expr(start, f);
            walk_expr(end, f);
        }
        RefOpIR::SetValue { value } => walk_expr(value, f),
        RefOpIR::CapturePointer { id } => walk_expr(id, f),
        RefOpIR::ReleasePointer { id } => walk_expr(id, f),
    }
}

/// Walk a `StatementIR` and call `f` for every `ExprIR` node reachable
/// from the statement (including expressions inside nested blocks).
pub fn walk_stmt(stmt: &StatementIR, f: &mut impl FnMut(&ExprIR)) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => walk_expr(expr, f),
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, f);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(v) = value {
                walk_expr(v, f);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            walk_expr(condition, f);
            walk_block(then_body, f);
            for (cond, body) in else_ifs {
                walk_expr(cond, f);
                walk_block(body, f);
            }
            if let Some(b) = else_body {
                walk_block(b, f);
            }
        }
        StatementIR::BlockStmtIR { body } => walk_block(body, f),
        StatementIR::WhileStmtIR { condition, body } => {
            walk_expr(condition, f);
            walk_block(body, f);
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            walk_expr(iterable, f);
            walk_block(body, f);
        }
        StatementIR::ThrowStmtIR { expr } => walk_expr(expr, f),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            walk_block(try_body, f);
            if let Some(b) = catch_body {
                walk_block(b, f);
            }
            if let Some(b) = finally_body {
                walk_block(b, f);
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                walk_expr(arg, f);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                walk_mutation_exprs(intent, f);
            }
        }
        // Break and continue have no expressions
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
    }
}

/// Walk a `BlockIR` and call `f` for every `ExprIR` node reachable.
fn walk_block(block: &BlockIR, f: &mut impl FnMut(&ExprIR)) {
    for stmt in &block.statements {
        walk_stmt(stmt, f);
    }
}

/// Walk a `MutationIntent` and call `f` for every `ExprIR` node reachable.
fn walk_mutation_exprs(intent: &MutationIntent, f: &mut impl FnMut(&ExprIR)) {
    match intent {
        MutationIntent::DirectAssign { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, f);
            }
        }
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => {
            if let Some(v) = amount {
                walk_expr(v, f);
            }
        }
        MutationIntent::Push { item, .. } => {
            if let Some(v) = item {
                walk_expr(v, f);
            }
        }
        MutationIntent::Pop { .. } => {}
    }
}

/// Walk a `ViewTree` and call `f` for every `ElementIR` node (pre-order).
///
/// Recurses into all child view trees (conditional branches, list bodies,
/// fragments, etc.).
pub fn walk_view(tree: &ViewTree, f: &mut impl FnMut(&ElementIR)) {
    match tree {
        ViewTree::Element(el) => {
            f(el);
            for child in &el.children {
                walk_view(child, f);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    walk_view(body, f);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                walk_view(fallback, f);
            }
        }
        ViewTree::ListRendering(lr) => {
            if let Some(ref body) = lr.body {
                walk_view(body, f);
            }
            if let Some(ref empty) = lr.empty_body {
                walk_view(empty, f);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view(child, f);
            }
        }
        // Slot fallbacks are real view content — descend so elements inside
        // them (view-level `@slot name {...}`, component body slots, and
        // consumer slot projections) are visited like any other children.
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_view(child, f);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}
