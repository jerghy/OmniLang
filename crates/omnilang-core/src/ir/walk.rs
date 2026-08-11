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
        ExprIR::BuiltinToastCall { msg, .. } => walk_expr(msg, f),
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
        | RefOpIR::Value
        | RefOpIR::CapturePointer
        | RefOpIR::ReleasePointer => {}
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

// ═════════════════════════════════════════════════════════════════════
// Mutable variants (batch 3, T5) — for passes that rewrite ExprIR in
// place (e.g. validate_breakpoints rewriting bp("md") args to numbers).
// Mirrors the immutable walkers above exactly.
// ═════════════════════════════════════════════════════════════════════

/// Walk an `ExprIR` tree with mutable access, calling `f` for each node.
pub fn walk_expr_mut(expr: &mut ExprIR, f: &mut impl FnMut(&mut ExprIR)) {
    f(expr);
    match expr {
        ExprIR::LiteralExpr { .. } => {}
        ExprIR::IdentifierExpr { .. } => {}
        ExprIR::FieldAccessExpr { object, .. } => walk_expr_mut(object, f),
        ExprIR::OptionalChainExpr { object, .. } => walk_expr_mut(object, f),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_mut(left, f);
            walk_expr_mut(right, f);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_mut(operand, f),
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr_mut(arg, f);
            }
        }
        ExprIR::BuiltinToastCall { msg, .. } => walk_expr_mut(msg, f),
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr_mut(object, f);
            for arg in args {
                walk_expr_mut(arg, f);
            }
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_mut(cond, f);
            walk_expr_mut(then_expr, f);
            walk_expr_mut(else_expr, f);
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr_mut(object, f);
            walk_expr_mut(index, f);
        }
        ExprIR::ArrayExpr { elements } => {
            for elem in elements {
                walk_expr_mut(elem, f);
            }
        }
        ExprIR::SpreadExpr { operand } => walk_expr_mut(operand, f),
        ExprIR::StructExpr { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr_mut(field_expr, f);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                walk_expr_mut(arg, f);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                walk_stmt_mut(stmt, f);
            }
            walk_expr_mut(tail, f);
        }
        ExprIR::ClosureExpr { body, .. } => walk_expr_mut(body, f),
        ExprIR::AwaitExpr { operand } => walk_expr_mut(operand, f),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr_mut(field_expr, f);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr_mut(matched, f);
            for arm in arms {
                for stmt in &mut arm.body.statements {
                    walk_stmt_mut(stmt, f);
                }
            }
        }
        ExprIR::MapExpr { entries } => {
            for (key, value) in entries {
                walk_expr_mut(key, f);
                walk_expr_mut(value, f);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            walk_expr_mut(subject, f);
            walk_expr_mut(predicate, f);
        }
        ExprIR::RefOpExpr { op, .. } => walk_ref_op_mut(op, f),
    }
}

/// Walk the `ExprIR` children carried by a `RefOpIR` operation (mutable).
fn walk_ref_op_mut(op: &mut RefOpIR, f: &mut impl FnMut(&mut ExprIR)) {
    match op {
        RefOpIR::Focus
        | RefOpIR::Blur
        | RefOpIR::Measure { .. }
        | RefOpIR::Select
        | RefOpIR::Value
        | RefOpIR::CapturePointer
        | RefOpIR::ReleasePointer => {}
        RefOpIR::ScrollTo { x, y } => {
            if let Some(x) = x {
                walk_expr_mut(x, f);
            }
            if let Some(y) = y {
                walk_expr_mut(y, f);
            }
        }
        RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                walk_expr_mut(dx, f);
            }
            if let Some(dy) = dy {
                walk_expr_mut(dy, f);
            }
        }
        RefOpIR::SetSelection { start, end } => {
            walk_expr_mut(start, f);
            walk_expr_mut(end, f);
        }
    }
}

/// Walk a `StatementIR` with mutable access, calling `f` for every `ExprIR`.
pub fn walk_stmt_mut(stmt: &mut StatementIR, f: &mut impl FnMut(&mut ExprIR)) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => walk_expr_mut(expr, f),
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(v) = value {
                walk_expr_mut(v, f);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(v) = value {
                walk_expr_mut(v, f);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            walk_expr_mut(condition, f);
            walk_block_mut(then_body, f);
            for (cond, body) in else_ifs {
                walk_expr_mut(cond, f);
                walk_block_mut(body, f);
            }
            if let Some(b) = else_body {
                walk_block_mut(b, f);
            }
        }
        StatementIR::BlockStmtIR { body } => walk_block_mut(body, f),
        StatementIR::WhileStmtIR { condition, body } => {
            walk_expr_mut(condition, f);
            walk_block_mut(body, f);
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            walk_expr_mut(iterable, f);
            walk_block_mut(body, f);
        }
        StatementIR::ThrowStmtIR { expr } => walk_expr_mut(expr, f),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            walk_block_mut(try_body, f);
            if let Some(b) = catch_body {
                walk_block_mut(b, f);
            }
            if let Some(b) = finally_body {
                walk_block_mut(b, f);
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                walk_expr_mut(arg, f);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                walk_mutation_exprs_mut(intent, f);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
    }
}

/// Walk a `BlockIR` with mutable access.
fn walk_block_mut(block: &mut BlockIR, f: &mut impl FnMut(&mut ExprIR)) {
    for stmt in &mut block.statements {
        walk_stmt_mut(stmt, f);
    }
}

/// Walk a `MutationIntent` with mutable access.
fn walk_mutation_exprs_mut(intent: &mut MutationIntent, f: &mut impl FnMut(&mut ExprIR)) {
    match intent {
        MutationIntent::DirectAssign { value, .. } => {
            if let Some(v) = value {
                walk_expr_mut(v, f);
            }
        }
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => {
            if let Some(v) = amount {
                walk_expr_mut(v, f);
            }
        }
        MutationIntent::Push { item, .. } => {
            if let Some(v) = item {
                walk_expr_mut(v, f);
            }
        }
        MutationIntent::Pop { .. } => {}
    }
}

/// Walk every expression reachable from a view tree with mutable access —
/// element expression slots (props/class_map_bindings/semantic_bindings/
/// inline-event bodies/a11y/motion) plus the non-element slots
/// (Interpolation / Conditional conditions / ListRendering iterable+key).
/// Mirrors the immutable walk_view_exprs slot model and extends it to
/// cover motion trigger expressions (batch 3, T5).
///
/// NOTE: the two layers are DISJOINT by construction — `walk_view_mut`
/// already visits every element in the tree (including nested ones), so the
/// non-element recursion must NOT re-visit element slots (rewriting passes
/// like validate_breakpoints would otherwise see each element slot twice).
pub fn walk_view_exprs_mut(tree: &mut ViewTree, f: &mut impl FnMut(&mut ExprIR)) {
    walk_view_mut(tree, &mut |el| walk_element_exprs_mut(el, f));
    walk_non_element_slots_mut(tree, f);
}

/// Walk only the non-element expression slots of a view tree (recursing
/// through child trees without touching element slots again).
fn walk_non_element_slots_mut(tree: &mut ViewTree, f: &mut impl FnMut(&mut ExprIR)) {
    match tree {
        ViewTree::Interpolation(expr) => walk_expr_mut(expr, f),
        ViewTree::ListRendering(lr) => {
            walk_expr_mut(&mut lr.iterable, f);
            if let Some(ref mut key) = lr.key_expr {
                walk_expr_mut(key, f);
            }
            if let Some(ref mut body) = lr.body {
                walk_non_element_slots_mut(body, f);
            }
            if let Some(ref mut empty) = lr.empty_body {
                walk_non_element_slots_mut(empty, f);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &mut cond.branches {
                walk_expr_mut(&mut branch.condition, f);
                if let Some(ref mut body) = branch.body {
                    walk_non_element_slots_mut(body, f);
                }
            }
            if let Some(ref mut fallback) = cond.fallback {
                walk_non_element_slots_mut(fallback, f);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_non_element_slots_mut(child, f);
            }
        }
        ViewTree::Element(el) => {
            for child in &mut el.children {
                walk_non_element_slots_mut(child, f);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_non_element_slots_mut(child, f);
            }
        }
        ViewTree::Text(_) => {}
    }
}

/// Walk every `ExprIR` slot carried by an element (mutable) — mirrors the
/// immutable walk_element_exprs (semantic/validate_form_load.rs).
fn walk_element_exprs_mut<F: FnMut(&mut ExprIR)>(el: &mut ElementIR, f: &mut F) {
    for prop in el.props.values_mut() {
        walk_expr_mut(prop, f);
    }
    for expr in el.class_map_bindings.values_mut() {
        walk_expr_mut(expr, f);
    }
    for expr in el.semantic_bindings.values_mut() {
        walk_expr_mut(expr, f);
    }
    for handler in el.events.values_mut() {
        if let EventHandlerIR::InlineFn { body, .. } = handler {
            for stmt in &mut body.statements {
                walk_stmt_mut(stmt, f);
            }
        }
    }
    if let Some(ref mut a11y) = el.a11y {
        if let Some(ref mut expr) = a11y.hidden {
            walk_expr_mut(expr, f);
        }
        if let Some(ref mut expr) = a11y.alt {
            walk_expr_mut(expr, f);
        }
    }
    if let Some(ref mut motion) = el.motion {
        for expr in [
            motion.in_view_once.as_mut(),
            motion.in_view_repeat.as_mut(),
            motion.in_view_threshold.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            walk_expr_mut(expr, f);
        }
        for trigger in &mut motion.triggers {
            if let Some(ref mut body) = trigger.inline_motion {
                walk_expr_mut(&mut body.over, f);
                for stop in &mut body.stops {
                    for expr in stop.properties.values_mut() {
                        walk_expr_mut(expr, f);
                    }
                }
                if let Some(ref mut delay) = body.playback.delay {
                    walk_expr_mut(delay, f);
                }
            }
            match &mut trigger.kind {
                TriggerKind::Scroll {
                    start, end, scrub, ..
                } => {
                    for expr in [start.as_mut(), end.as_mut(), scrub.as_mut()]
                        .into_iter()
                        .flatten()
                    {
                        walk_expr_mut(expr, f);
                    }
                }
                TriggerKind::DragHorizontal {
                    constraints,
                    elastic,
                    threshold,
                    ..
                }
                | TriggerKind::DragVertical {
                    constraints,
                    elastic,
                    threshold,
                    ..
                } => {
                    if let Some((ref mut min, ref mut max)) = constraints {
                        walk_expr_mut(min, f);
                        walk_expr_mut(max, f);
                    }
                    for expr in [elastic.as_mut(), threshold.as_mut()].into_iter().flatten() {
                        walk_expr_mut(expr, f);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Walk a `ViewTree` with mutable access, calling `f` for every element.
fn walk_view_mut(tree: &mut ViewTree, f: &mut impl FnMut(&mut ElementIR)) {
    match tree {
        ViewTree::Element(el) => {
            f(el);
            for child in &mut el.children {
                walk_view_mut(child, f);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &mut cond.branches {
                if let Some(ref mut body) = branch.body {
                    walk_view_mut(body, f);
                }
            }
            if let Some(ref mut fallback) = cond.fallback {
                walk_view_mut(fallback, f);
            }
        }
        ViewTree::ListRendering(lr) => {
            if let Some(ref mut body) = lr.body {
                walk_view_mut(body, f);
            }
            if let Some(ref mut empty) = lr.empty_body {
                walk_view_mut(empty, f);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_mut(child, f);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_view_mut(child, f);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}
