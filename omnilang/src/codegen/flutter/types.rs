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
            for (_, expr) in &el.semantic_bindings {
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

/// Whether a component's generated code references any declared union
/// (expressions in fn/computed/effect/lifecycle bodies, signal
/// initializers, prop defaults, the view tree; type annotations on props,
/// signals, computeds, fn params/returns, slots).
pub(crate) fn component_uses_unions(comp: &ComponentIR, ir: &IRProgram) -> bool {
    let names = enum_names(ir);
    if names.is_empty() {
        return false;
    }
    let mut out: FxHashSet<String> = FxHashSet::default();
    collect_component_union_names(comp, &names, &mut out);
    !out.is_empty()
}

/// Whether a store's generated code references any declared union
/// (signal/getter types, action bodies).
pub(crate) fn store_uses_unions(store: &StoreIR, ir: &IRProgram) -> bool {
    let names = enum_names(ir);
    if names.is_empty() {
        return false;
    }
    let mut out: FxHashSet<String> = FxHashSet::default();
    collect_store_union_names(store, &names, &mut out);
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
    for s in &comp.slots {
        if let Some(t) = &s.type_constraint {
            collect_type_union_names(t, names, out);
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

/// Emit `types.dart` with the tagged unions referenced by generated code.
/// Returns an empty vec when no generated file references a union.
pub(crate) fn gen_types_files(ir: &IRProgram) -> Vec<CompiledFile> {
    let names = enum_names(ir);
    if names.is_empty() {
        return Vec::new();
    }
    let mut referenced: FxHashSet<String> = FxHashSet::default();
    for (_, comp) in ir.all_components() {
        collect_component_union_names(comp, &names, &mut referenced);
    }
    for (_, store) in ir.all_stores() {
        collect_store_union_names(store, &names, &mut referenced);
    }
    if referenced.is_empty() {
        return Vec::new();
    }

    // Resolve each referenced union to its (first) declaration in
    // deterministic order (sorted module, sorted name).
    let mut enums: Vec<&EnumIR> = Vec::new();
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
    }

    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang \u{2014} Flutter target");
    w.emit("");
    w.emit("// Tagged union types referenced by generated components/stores.");
    w.emit("");
    for e in &enums {
        emit_union_class(&mut w, e);
    }
    vec![CompiledFile {
        path: "types.dart".to_string(),
        content: w.into_string(),
    }]
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
