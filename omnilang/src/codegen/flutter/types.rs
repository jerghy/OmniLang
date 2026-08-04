//! Flutter tagged-union type emission: `types.dart` (deferral #30).
//!
//! Tagged union types (`type Rating = Star { level: i32 } | Blank`) are
//! referenced by generated code as classes: variant literals emit
//! `Rating.Star(level: 5)` (`DartFormatExpr::format_variant_literal`) and
//! match arms dispatch on `_v._variant` / bind fields via `_v.level`
//! (`DartFormatExpr::format_match`). The class declarations themselves were
//! never emitted, so any component using a union produced uncompilable Dart
//! (`Undefined class 'Rating'` / `Undefined name 'level'`).
//!
//! This module emits every union referenced by generated code into a single
//! merged `types.dart` (mirroring how design blocks merge into
//! `design.dart`), and tells the component/store generators whether they
//! need the `import './types.dart';` line. Only unions actually referenced
//! are emitted, so the merged file never carries dangling declarations.

use crate::codegen::{CodeWriter, CompiledFile};
use crate::ir::*;
use rustc_hash::FxHashSet;

// ═══════════════════════════════════════════════════════════
// Referenced-union collection
// ═══════════════════════════════════════════════════════════

/// Union (enum) type names declared anywhere in the program.
fn enum_names(ir: &IRProgram) -> FxHashSet<&str> {
    ir.modules
        .values()
        .flat_map(|m| m.enums.keys().map(|k| k.as_str()))
        .collect()
}

/// Struct type names declared anywhere in the program (Task 5.8: structs
/// were never emitted — `List<Todo>` / `StatisticsData` annotations and
/// `Point { x: 0, y: 0 }` literals produced non_type_as_type_argument /
/// undefined_class / map-literal getter errors).
fn struct_names(ir: &IRProgram) -> FxHashSet<&str> {
    ir.modules
        .values()
        .flat_map(|m| m.structs.keys().map(|k| k.as_str()))
        .collect()
}

/// Record struct names mentioned by a `TypeIR` annotation (recursively).
fn collect_type_struct_names(ty: &TypeIR, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    if let TypeKind::Named { name } = &ty.kind {
        if names.contains(name.as_str()) {
            out.insert(name.clone());
        }
    }
    for t in &ty.type_args {
        collect_type_struct_names(t, names, out);
    }
}

/// Record struct references inside an expression: struct literals carry the
/// type name directly.
fn scan_expr_structs(expr: &ExprIR, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    match expr {
        ExprIR::StructExpr { type_name, fields } => {
            if names.contains(type_name.as_str()) {
                out.insert(type_name.clone());
            }
            for (_, v) in fields {
                scan_expr_structs(v, names, out);
            }
        }
        ExprIR::FieldAccessExpr { object, .. } => scan_expr_structs(object, names, out),
        ExprIR::OptionalChainExpr { object, .. } => scan_expr_structs(object, names, out),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            scan_expr_structs(left, names, out);
            scan_expr_structs(right, names, out);
        }
        ExprIR::UnaryOpExpr { operand, .. } => scan_expr_structs(operand, names, out),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            scan_expr_structs(cond, names, out);
            scan_expr_structs(then_expr, names, out);
            scan_expr_structs(else_expr, names, out);
        }
        ExprIR::CallExpr { args, .. } | ExprIR::ArrayExpr { elements: args } => {
            for a in args {
                scan_expr_structs(a, names, out);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for a in call_args {
                scan_expr_structs(a, names, out);
            }
        }
        ExprIR::SpreadExpr { operand } => scan_expr_structs(operand, names, out),
        ExprIR::MethodCallExpr { object, args, .. } => {
            scan_expr_structs(object, names, out);
            for a in args {
                scan_expr_structs(a, names, out);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for s in statements {
                scan_stmt_structs(s, names, out);
            }
            scan_expr_structs(tail, names, out);
        }
        ExprIR::IndexExpr { object, index } => {
            scan_expr_structs(object, names, out);
            scan_expr_structs(index, names, out);
        }
        ExprIR::AwaitExpr { operand } => scan_expr_structs(operand, names, out),
        ExprIR::ClosureExpr { body, .. } => scan_expr_structs(body, names, out),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, v) in fields {
                scan_expr_structs(v, names, out);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            scan_expr_structs(matched, names, out);
            for arm in arms {
                for s in &arm.body.statements {
                    scan_stmt_structs(s, names, out);
                }
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                scan_expr_structs(k, names, out);
                scan_expr_structs(v, names, out);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            scan_expr_structs(subject, names, out);
            scan_expr_structs(predicate, names, out);
        }
        _ => {}
    }
}

fn scan_stmt_structs(stmt: &StatementIR, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::LetStmtIR { value: Some(v), .. } => scan_expr_structs(v, names, out),
        StatementIR::ExprStmtIR { expr } => scan_expr_structs(expr, names, out),
        StatementIR::ReturnStmtIR { value: Some(v), .. } => scan_expr_structs(v, names, out),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            scan_expr_structs(condition, names, out);
            for s in &then_body.statements {
                scan_stmt_structs(s, names, out);
            }
            for (c, b) in else_ifs {
                scan_expr_structs(c, names, out);
                for s in &b.statements {
                    scan_stmt_structs(s, names, out);
                }
            }
            if let Some(b) = else_body {
                for s in &b.statements {
                    scan_stmt_structs(s, names, out);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            scan_expr_structs(condition, names, out);
            for s in &body.statements {
                scan_stmt_structs(s, names, out);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            scan_expr_structs(iterable, names, out);
            for s in &body.statements {
                scan_stmt_structs(s, names, out);
            }
        }
        StatementIR::MutationStmt { intent: Some(i) } => match i {
            MutationIntent::DirectAssign { value, .. }
            | MutationIntent::Increment { amount: value, .. }
            | MutationIntent::Decrement { amount: value, .. }
            | MutationIntent::Multiply { amount: value, .. }
            | MutationIntent::Divide { amount: value, .. }
            | MutationIntent::Modulo { amount: value, .. }
            | MutationIntent::Push { item: value, .. } => {
                if let Some(v) = value {
                    scan_expr_structs(v, names, out);
                }
            }
            MutationIntent::Pop { .. } => {}
            #[allow(unreachable_patterns)]
            _ => {}
        },
        StatementIR::EventEmitStmtIR { args, .. } => {
            for a in args {
                scan_expr_structs(a, names, out);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                scan_stmt_structs(s, names, out);
            }
        }
        StatementIR::ThrowStmtIR { expr } => scan_expr_structs(expr, names, out),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                scan_stmt_structs(s, names, out);
            }
            if let Some(b) = catch_body {
                for s in &b.statements {
                    scan_stmt_structs(s, names, out);
                }
            }
            if let Some(b) = finally_body {
                for s in &b.statements {
                    scan_stmt_structs(s, names, out);
                }
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Collect struct names referenced by a component's generated code.
fn collect_component_struct_names(
    comp: &ComponentIR,
    names: &FxHashSet<&str>,
    out: &mut FxHashSet<String>,
) {
    for ty in comp.props.values() {
        collect_type_struct_names(ty, names, out);
    }
    for ty in comp.prop_defaults.values() {
        scan_expr_structs(ty, names, out);
    }
    for s in &comp.signals {
        collect_type_struct_names(&s.r#type, names, out);
        scan_expr_structs(&s.initial_value, names, out);
    }
    for c in &comp.computeds {
        collect_type_struct_names(&c.r#type, names, out);
        scan_expr_structs(&c.body, names, out);
    }
    for e in &comp.effects {
        for s in &e.body.statements {
            scan_stmt_structs(s, names, out);
        }
    }
    for lc in &comp.lifecycles {
        for s in &lc.body.statements {
            scan_stmt_structs(s, names, out);
        }
    }
    for f in &comp.fns {
        for p in &f.params {
            if let Some(t) = &p.r#type {
                collect_type_struct_names(t, names, out);
            }
        }
        if let Some(t) = &f.return_type {
            collect_type_struct_names(t, names, out);
        }
        for s in &f.body.statements {
            scan_stmt_structs(s, names, out);
        }
    }
    if let Some(view) = &comp.view {
        scan_view_structs(view, names, out);
    }
}

/// Collect struct names referenced by a store's generated code.
fn collect_store_struct_names(
    store: &StoreIR,
    names: &FxHashSet<&str>,
    out: &mut FxHashSet<String>,
) {
    for ty in store.signals.values() {
        collect_type_struct_names(ty, names, out);
    }
    for ty in store.getter_types.values() {
        collect_type_struct_names(ty, names, out);
    }
    for expr in store.getters.values() {
        scan_expr_structs(expr, names, out);
    }
    for a in &store.actions {
        for p in &a.params {
            if let Some(t) = &p.r#type {
                collect_type_struct_names(t, names, out);
            }
        }
        if let Some(t) = &a.return_type {
            collect_type_struct_names(t, names, out);
        }
        for s in &a.body.statements {
            scan_stmt_structs(s, names, out);
        }
    }
}

fn scan_view_structs(node: &ViewTree, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for v in el.props.values() {
                scan_expr_structs(v, names, out);
            }
            for h in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = h {
                    for s in &body.statements {
                        scan_stmt_structs(s, names, out);
                    }
                }
            }
            for v in el.class_map_bindings.values() {
                scan_expr_structs(v, names, out);
            }
            for c in &el.children {
                scan_view_structs(c, names, out);
            }
        }
        ViewTree::Conditional(cond) => {
            for b in &cond.branches {
                scan_expr_structs(&b.condition, names, out);
                if let Some(body) = &b.body {
                    scan_view_structs(body, names, out);
                }
            }
            if let Some(fb) = &cond.fallback {
                scan_view_structs(fb, names, out);
            }
        }
        ViewTree::ListRendering(list) => {
            scan_expr_structs(&list.iterable, names, out);
            if let Some(key) = &list.key_expr {
                scan_expr_structs(key, names, out);
            }
            if let Some(b) = &list.body {
                scan_view_structs(b, names, out);
            }
            if let Some(b) = &list.empty_body {
                scan_view_structs(b, names, out);
            }
        }
        ViewTree::Interpolation(expr) => scan_expr_structs(expr, names, out),
        ViewTree::Fragment(children) => {
            for c in children {
                scan_view_structs(c, names, out);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                scan_view_structs(c, names, out);
            }
        }
        ViewTree::Text(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Record union names mentioned by a `TypeIR` annotation (recursively).
fn collect_type_union_names(ty: &TypeIR, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    if let TypeKind::Named { name } = &ty.kind {
        if names.contains(name.as_str()) {
            out.insert(name.clone());
        }
    }
    for t in &ty.type_args {
        collect_type_union_names(t, names, out);
    }
}

/// Record union references found inside an expression: variant literals
/// carry the union name directly; match arms carry it on the arm.
fn scan_expr_unions(expr: &ExprIR, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    match expr {
        ExprIR::VariantLiteral { union_type, .. } => {
            if names.contains(union_type.as_str()) {
                out.insert(union_type.clone());
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            scan_expr_unions(matched, names, out);
            for arm in arms {
                if let Some(u) = &arm.union_type {
                    if names.contains(u.as_str()) {
                        out.insert(u.clone());
                    }
                }
                for s in &arm.body.statements {
                    scan_stmt_unions(s, names, out);
                }
            }
        }
        ExprIR::FieldAccessExpr { object, .. } => scan_expr_unions(object, names, out),
        ExprIR::OptionalChainExpr { object, .. } => scan_expr_unions(object, names, out),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            scan_expr_unions(left, names, out);
            scan_expr_unions(right, names, out);
        }
        ExprIR::UnaryOpExpr { operand, .. } => scan_expr_unions(operand, names, out),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            scan_expr_unions(cond, names, out);
            scan_expr_unions(then_expr, names, out);
            scan_expr_unions(else_expr, names, out);
        }
        ExprIR::CallExpr { args, .. } | ExprIR::ArrayExpr { elements: args } => {
            for a in args {
                scan_expr_unions(a, names, out);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for a in call_args {
                scan_expr_unions(a, names, out);
            }
        }
        ExprIR::SpreadExpr { operand } => scan_expr_unions(operand, names, out),
        ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                scan_expr_unions(v, names, out);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            scan_expr_unions(object, names, out);
            for a in args {
                scan_expr_unions(a, names, out);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for s in statements {
                scan_stmt_unions(s, names, out);
            }
            scan_expr_unions(tail, names, out);
        }
        ExprIR::IndexExpr { object, index } => {
            scan_expr_unions(object, names, out);
            scan_expr_unions(index, names, out);
        }
        ExprIR::AwaitExpr { operand } => scan_expr_unions(operand, names, out),
        ExprIR::ClosureExpr { body, .. } => scan_expr_unions(body, names, out),
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                scan_expr_unions(k, names, out);
                scan_expr_unions(v, names, out);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            scan_expr_unions(subject, names, out);
            scan_expr_unions(predicate, names, out);
        }
        ExprIR::RefOpExpr { .. } | ExprIR::IdentifierExpr { .. } | ExprIR::LiteralExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Record union references inside a statement.
fn scan_stmt_unions(stmt: &StatementIR, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::LetStmtIR { value, .. } => scan_opt_expr_unions(value.as_ref(), names, out),
        StatementIR::ExprStmtIR { expr } => scan_expr_unions(expr, names, out),
        StatementIR::ReturnStmtIR { value } => scan_opt_expr_unions(value.as_ref(), names, out),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            scan_expr_unions(condition, names, out);
            for s in &then_body.statements {
                scan_stmt_unions(s, names, out);
            }
            for (c, b) in else_ifs {
                scan_expr_unions(c, names, out);
                for s in &b.statements {
                    scan_stmt_unions(s, names, out);
                }
            }
            if let Some(b) = else_body {
                for s in &b.statements {
                    scan_stmt_unions(s, names, out);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            scan_expr_unions(condition, names, out);
            for s in &body.statements {
                scan_stmt_unions(s, names, out);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            scan_expr_unions(iterable, names, out);
            for s in &body.statements {
                scan_stmt_unions(s, names, out);
            }
        }
        StatementIR::MutationStmt { intent: Some(i) } => scan_mutation_intent_unions(i, names, out),
        StatementIR::EventEmitStmtIR { args, .. } => {
            for a in args {
                scan_expr_unions(a, names, out);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                scan_stmt_unions(s, names, out);
            }
        }
        StatementIR::ThrowStmtIR { expr } => scan_expr_unions(expr, names, out),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                scan_stmt_unions(s, names, out);
            }
            if let Some(b) = catch_body {
                for s in &b.statements {
                    scan_stmt_unions(s, names, out);
                }
            }
            if let Some(b) = finally_body {
                for s in &b.statements {
                    scan_stmt_unions(s, names, out);
                }
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

fn scan_opt_expr_unions(
    expr: Option<&ExprIR>,
    names: &FxHashSet<&str>,
    out: &mut FxHashSet<String>,
) {
    if let Some(e) = expr {
        scan_expr_unions(e, names, out);
    }
}

fn scan_mutation_intent_unions(
    intent: &MutationIntent,
    names: &FxHashSet<&str>,
    out: &mut FxHashSet<String>,
) {
    match intent {
        MutationIntent::DirectAssign { value, .. } => {
            scan_opt_expr_unions(value.as_deref(), names, out)
        }
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. }
        | MutationIntent::Push { item: amount, .. } => {
            scan_opt_expr_unions(amount.as_deref(), names, out)
        }
        MutationIntent::Pop { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Record union references in the view tree (inline handler bodies, prop
/// values, class-map conditions, semantic bindings, interpolations).
fn scan_view_unions(node: &ViewTree, names: &FxHashSet<&str>, out: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for value in el.props.values() {
                scan_expr_unions(value, names, out);
            }
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    for s in &body.statements {
                        scan_stmt_unions(s, names, out);
                    }
                }
            }
            for value in el.class_map_bindings.values() {
                scan_expr_unions(value, names, out);
            }
            for expr in el.semantic_bindings.values() {
                scan_expr_unions(expr, names, out);
            }
            for child in &el.children {
                scan_view_unions(child, names, out);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                scan_expr_unions(&branch.condition, names, out);
                if let Some(ref body) = branch.body {
                    scan_view_unions(body, names, out);
                }
            }
            if let Some(ref fb) = cond.fallback {
                scan_view_unions(fb, names, out);
            }
        }
        ViewTree::ListRendering(list) => {
            scan_expr_unions(&list.iterable, names, out);
            if let Some(ref key) = list.key_expr {
                scan_expr_unions(key, names, out);
            }
            if let Some(ref body) = list.body {
                scan_view_unions(body, names, out);
            }
            if let Some(ref empty) = list.empty_body {
                scan_view_unions(empty, names, out);
            }
        }
        ViewTree::Interpolation(expr) => scan_expr_unions(expr, names, out),
        ViewTree::Fragment(children) => {
            for c in children {
                scan_view_unions(c, names, out);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                scan_view_unions(c, names, out);
            }
        }
        ViewTree::Text(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Whether a component's generated code references any declared union OR
/// struct (Task 5.8: structs were never emitted — `List<Todo>` annotations
/// and `StatisticsData { ... }` literals — so the types.dart import must
/// cover both). ALSO scans the public library fns the component INLINES
/// (their bodies may annotate struct types — `ApiResult` in lib/api.omni).
///
/// Task 5.8a: `inlined_lib_fns` limits the lib-fn scan to the functions
/// actually emitted into THIS component's file — scanning every program fn
/// made every component import types.dart (~50 unused_import warnings in
/// the showcase gate).
pub(crate) fn component_uses_types(
    comp: &ComponentIR,
    ir: &IRProgram,
    inlined_lib_fns: &FxHashSet<String>,
) -> bool {
    let union_names = enum_names(ir);
    let struct_names = struct_names(ir);
    let mut out: FxHashSet<String> = FxHashSet::default();
    if !union_names.is_empty() {
        collect_component_union_names(comp, &union_names, &mut out);
    }
    if !struct_names.is_empty() {
        collect_component_struct_names(comp, &struct_names, &mut out);
        for fun in ir.all_functions() {
            if !inlined_lib_fns.contains(&fun.name) {
                continue;
            }
            for stmt in &fun.body.statements {
                scan_stmt_structs(stmt, &struct_names, &mut out);
            }
            for p in &fun.params {
                if let Some(t) = &p.r#type {
                    collect_type_struct_names(t, &struct_names, &mut out);
                }
            }
            if let Some(t) = &fun.return_type {
                collect_type_struct_names(t, &struct_names, &mut out);
            }
        }
    }
    if !union_names.is_empty() {
        // Task 5.8: unions referenced by INLINED lib fns (lib/api.omni's
        // error_message(result: ApiResult) — the union class must exist in
        // types.dart for the inlined match bodies).
        for fun in ir.all_functions() {
            if !inlined_lib_fns.contains(&fun.name) {
                continue;
            }
            for stmt in &fun.body.statements {
                scan_stmt_unions(stmt, &union_names, &mut out);
            }
            for p in &fun.params {
                if let Some(t) = &p.r#type {
                    collect_type_union_names(t, &union_names, &mut out);
                }
            }
            if let Some(t) = &fun.return_type {
                collect_type_union_names(t, &union_names, &mut out);
            }
        }
    }
    !out.is_empty()
}

/// Whether a store's generated code references any declared union or struct.
pub(crate) fn store_uses_types(store: &StoreIR, ir: &IRProgram) -> bool {
    let union_names = enum_names(ir);
    let struct_names = struct_names(ir);
    let mut out: FxHashSet<String> = FxHashSet::default();
    if !union_names.is_empty() {
        collect_store_union_names(store, &union_names, &mut out);
    }
    if !struct_names.is_empty() {
        collect_store_struct_names(store, &struct_names, &mut out);
    }
    !out.is_empty()
}

/// Collect the specific union names referenced by a component.
fn collect_component_union_names(
    comp: &ComponentIR,
    names: &FxHashSet<&str>,
    out: &mut FxHashSet<String>,
) {
    for ty in comp.props.values() {
        collect_type_union_names(ty, names, out);
    }
    for ty in comp.prop_defaults.values() {
        scan_expr_unions(ty, names, out);
    }
    for s in &comp.signals {
        collect_type_union_names(&s.r#type, names, out);
        scan_expr_unions(&s.initial_value, names, out);
    }
    for c in &comp.computeds {
        collect_type_union_names(&c.r#type, names, out);
        scan_expr_unions(&c.body, names, out);
    }
    for e in &comp.effects {
        for s in &e.body.statements {
            scan_stmt_unions(s, names, out);
        }
    }
    for lc in &comp.lifecycles {
        for s in &lc.body.statements {
            scan_stmt_unions(s, names, out);
        }
    }
    for f in &comp.fns {
        for p in &f.params {
            if let Some(t) = &p.r#type {
                collect_type_union_names(t, names, out);
            }
        }
        if let Some(t) = &f.return_type {
            collect_type_union_names(t, names, out);
        }
        for s in &f.body.statements {
            scan_stmt_unions(s, names, out);
        }
    }
    if let Some(view) = &comp.view {
        scan_view_unions(view, names, out);
    }
}

/// Collect the specific union names referenced by a store.
fn collect_store_union_names(
    store: &StoreIR,
    names: &FxHashSet<&str>,
    out: &mut FxHashSet<String>,
) {
    for ty in store.signals.values() {
        collect_type_union_names(ty, names, out);
    }
    for ty in store.getter_types.values() {
        collect_type_union_names(ty, names, out);
    }
    for expr in store.getters.values() {
        scan_expr_unions(expr, names, out);
    }
    for a in &store.actions {
        for p in &a.params {
            if let Some(t) = &p.r#type {
                collect_type_union_names(t, names, out);
            }
        }
        if let Some(t) = &a.return_type {
            collect_type_union_names(t, names, out);
        }
        for s in &a.body.statements {
            scan_stmt_unions(s, names, out);
        }
    }
}

// ═══════════════════════════════════════════════════════════
// types.dart emission
// ═══════════════════════════════════════════════════════════

/// Emit `types.dart` with the tagged unions AND structs referenced by
/// generated code. Returns an empty vec when no generated file references
/// one (Task 5.8: structs are emitted as const classes — `List<Todo>`
/// annotations and `Point { x: 0, y: 0 }` literals need real declarations).
pub(crate) fn gen_types_files(ir: &IRProgram) -> Vec<CompiledFile> {
    let enum_names = enum_names(ir);
    let struct_names = struct_names(ir);
    if enum_names.is_empty() && struct_names.is_empty() {
        return Vec::new();
    }
    let mut referenced: FxHashSet<String> = FxHashSet::default();
    for (_, comp) in ir.all_components() {
        collect_component_union_names(comp, &enum_names, &mut referenced);
        collect_component_struct_names(comp, &struct_names, &mut referenced);
    }
    for (_, store) in ir.all_stores() {
        collect_store_union_names(store, &enum_names, &mut referenced);
        collect_store_struct_names(store, &struct_names, &mut referenced);
    }
    // Structs referenced by inlined library fns (Task 5.8: lib/api.omni's
    // `fetch_demo` returns `ApiResult` — the fn is inlined into components
    // that import its caller).
    for fun in ir.all_functions() {
        for stmt in &fun.body.statements {
            scan_stmt_structs(stmt, &struct_names, &mut referenced);
        }
        for p in &fun.params {
            if let Some(t) = &p.r#type {
                collect_type_struct_names(t, &struct_names, &mut referenced);
            }
        }
        if let Some(t) = &fun.return_type {
            collect_type_struct_names(t, &struct_names, &mut referenced);
        }
    }
    for fun in ir.all_functions() {
        for stmt in &fun.body.statements {
            scan_stmt_unions(stmt, &enum_names, &mut referenced);
        }
        for p in &fun.params {
            if let Some(t) = &p.r#type {
                collect_type_union_names(t, &enum_names, &mut referenced);
            }
        }
        if let Some(t) = &fun.return_type {
            collect_type_union_names(t, &enum_names, &mut referenced);
        }
    }
    if referenced.is_empty() {
        return Vec::new();
    }

    // Resolve each referenced type to its (first) declaration in
    // deterministic order (sorted module, sorted name).
    let mut enums: Vec<&EnumIR> = Vec::new();
    let mut structs: Vec<&StructIR> = Vec::new();
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    let mut module_keys: Vec<&String> = ir.modules.keys().collect();
    module_keys.sort();
    for module_key in module_keys {
        let module = &ir.modules[module_key];
        let mut enum_keys: Vec<&String> = module.enums.keys().collect();
        enum_keys.sort();
        for key in enum_keys {
            if referenced.contains(key) && seen.insert(key) {
                enums.push(&module.enums[key]);
            }
        }
        let mut struct_keys: Vec<&String> = module.structs.keys().collect();
        struct_keys.sort();
        for key in struct_keys {
            if referenced.contains(key) && seen.insert(key) {
                structs.push(&module.structs[key]);
            }
        }
    }

    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang \u{2014} Flutter target");
    w.emit("");
    w.emit("// Tagged union and struct types referenced by generated components/stores.");
    w.emit("");
    for e in &enums {
        emit_union_class(&mut w, e);
    }
    for s in &structs {
        emit_struct_class(&mut w, s);
    }
    vec![CompiledFile {
        path: "types.dart".to_string(),
        content: w.into_string(),
    }]
}

/// Emit one struct as a const class: `class Todo { final int id; ...;
/// const Todo({required this.id, ...}); }`. Struct literals emit
/// `Todo(id: ..., text: ..., done: ...)` (named params — the Dart family's
/// `format_struct` override) and field reads are plain `p.x`.
fn emit_struct_class(w: &mut CodeWriter, s: &StructIR) {
    let mut fields: Vec<(&str, &TypeIR)> = Vec::new();
    let mut seen_fields: FxHashSet<&str> = FxHashSet::default();
    for f in &s.fields {
        if seen_fields.insert(f.name.as_str()) {
            fields.push((f.name.as_str(), &f.r#type));
        }
    }
    w.emit_fmt(format_args!("/// Struct {}.", s.name));
    w.emit_fmt(format_args!("class {} {{", s.name));
    w.indent();
    for (name, ty) in &fields {
        let ann = super::super::dart::dart_type_ir(ty);
        w.emit_fmt(format_args!("final {} {};", ann, name));
    }
    if !fields.is_empty() {
        w.emit("");
        w.emit_fmt(format_args!("const {}({{", s.name));
        w.indent();
        for (name, _) in &fields {
            w.emit_fmt(format_args!("required this.{},", name));
        }
        w.dedent();
        w.emit("});");
    } else {
        w.emit("");
        w.emit_fmt(format_args!("const {}();", s.name));
    }
    w.dedent();
    w.emit("}");
    w.emit("");
}

/// Emit one union class. Variant literals call `Rating.Star(level: 5)`
/// (named params) and match arms read `_v.level` / `_v._variant`, so every
/// variant becomes a named-parameter factory constructor and the union of
/// variant fields becomes dynamic class members (null when a variant does
/// not carry the field — the dynamic type keeps `_v.<field>` legal).
fn emit_union_class(w: &mut CodeWriter, e: &EnumIR) {
    // `variant` is reserved for the discriminator member — a variant field
    // literally named `variant` is dropped defensively.
    let mut fields: Vec<&str> = Vec::new();
    for v in &e.variants {
        for f in &v.fields {
            if f.name != "variant" && !fields.contains(&f.name.as_str()) {
                fields.push(&f.name);
            }
        }
    }
    let summary: Vec<String> = e
        .variants
        .iter()
        .map(|v| {
            if v.fields.is_empty() {
                v.name.clone()
            } else {
                let fs: Vec<&str> = v.fields.iter().map(|f| f.name.as_str()).collect();
                format!("{} {{ {} }}", v.name, fs.join(", "))
            }
        })
        .collect();
    w.emit_fmt(format_args!(
        "/// Tagged union {}: {}",
        e.name,
        summary.join(" | ")
    ));
    w.emit_fmt(format_args!("class {} {{", e.name));
    w.indent();
    // The discriminator member is PUBLIC (`variant`, not `_variant`): Dart
    // `_`-prefixed members are library-private, and match arms in other
    // generated files read `_v.variant` across the types.dart boundary.
    w.emit("final String variant;");
    for f in &fields {
        w.emit_fmt(format_args!("final dynamic {};", f));
    }
    if !fields.is_empty() {
        w.emit("");
    }
    for v in &e.variants {
        let params: Vec<String> = v
            .fields
            .iter()
            .filter(|f| f.name != "variant")
            .map(|f| format!("required this.{}", f.name))
            .collect();
        let mut inits: Vec<String> = vec![format!("variant = '{}'", v.name)];
        for f in &fields {
            if !v.fields.iter().any(|vf| vf.name == *f) {
                inits.push(format!("{} = null", f));
            }
        }
        if params.is_empty() {
            w.emit_fmt(format_args!(
                "{}.{}() : {};",
                e.name,
                v.name,
                inits.join(", ")
            ));
        } else {
            w.emit_fmt(format_args!(
                "{}.{}({{ {} }}) : {};",
                e.name,
                v.name,
                params.join(", "),
                inits.join(", ")
            ));
        }
    }
    w.dedent();
    w.emit("}");
    w.emit("");
}
