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
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => {}
    }
}
