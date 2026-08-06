//! Static-type re-annotation (Task 5.8a).
//!
//! The shared Dart method table (get/contains/concat shaping) reads
//! `IdentifierExpr.type_ann` — but the IR's annotation is never populated
//! in practice: `ir::lower` resolves identifiers against FRESH scopes (the
//! semantic pass's symbols live in scopes that were popped), so every
//! identifier arrives at codegen with `type_ann: None`. This pass
//! re-annotates fn bodies from the fn's params and the component's
//! signals/computeds, whose `TypeIR`s are fully populated.

use crate::ir::*;
use rustc_hash::FxHashMap;

/// The source-level type string for a `TypeIR` ("Array<RowItem>",
/// "Map<String, String>", "i32", "f64", "String?" ...) — the string shape
/// the shared `expr_is_typed_array` / `expr_is_typed_map` /
/// `expr_is_typed_string` helpers match against.
fn type_ir_to_lang_string(t: &TypeIR) -> String {
    let base = match &t.kind {
        TypeKind::String => "String".to_string(),
        TypeKind::Int => "i32".to_string(),
        TypeKind::Float => "f64".to_string(),
        TypeKind::Bool => "bool".to_string(),
        TypeKind::Void => "Void".to_string(),
        TypeKind::Any => "any".to_string(),
        TypeKind::Named { name } => {
            if t.type_args.is_empty() {
                name.clone()
            } else {
                let args: Vec<String> = t.type_args.iter().map(type_ir_to_lang_string).collect();
                format!("{}<{}>", name, args.join(", "))
            }
        }
    };
    if t.is_nullable {
        format!("{base}?")
    } else {
        base
    }
}

/// The name → language-type-string scope for a COMPONENT fn body: the fn's
/// own params first (they shadow), then the component's signals and
/// computeds.
pub(crate) fn build_fn_type_scope(
    comp: &ComponentIR,
    fun: &FunctionIR,
) -> FxHashMap<String, String> {
    let mut scope = FxHashMap::default();
    for p in &fun.params {
        if let Some(t) = &p.r#type {
            scope.insert(p.name.clone(), type_ir_to_lang_string(t));
        }
    }
    for s in &comp.signals {
        scope.insert(s.name.clone(), type_ir_to_lang_string(&s.r#type));
    }
    for c in &comp.computeds {
        scope.insert(c.name.clone(), type_ir_to_lang_string(&c.r#type));
    }
    scope
}

/// The component-level scope (signals + computeds) for effect/lifecycle
/// bodies and computed expressions.
pub(crate) fn build_comp_type_scope(comp: &ComponentIR) -> FxHashMap<String, String> {
    let mut scope = FxHashMap::default();
    for s in &comp.signals {
        scope.insert(s.name.clone(), type_ir_to_lang_string(&s.r#type));
    }
    for c in &comp.computeds {
        scope.insert(c.name.clone(), type_ir_to_lang_string(&c.r#type));
    }
    scope
}

/// The scope for a LIB fn body (inlined cross-file fn — params only).
pub(crate) fn build_lib_fn_type_scope(fun: &FunctionIR) -> FxHashMap<String, String> {
    let mut scope = FxHashMap::default();
    for p in &fun.params {
        if let Some(t) = &p.r#type {
            scope.insert(p.name.clone(), type_ir_to_lang_string(t));
        }
    }
    scope
}

/// Annotate `IdentifierExpr.type_ann` from `scope` (only when the current
/// annotation is None — manual annotations win).
pub(crate) fn annotate_expr_types(e: &ExprIR, scope: &FxHashMap<String, String>) -> ExprIR {
    let ann = |x: &ExprIR| annotate_expr_types(x, scope);
    match e {
        ExprIR::IdentifierExpr { name, type_ann } => {
            if type_ann.is_none() {
                if let Some(t) = scope.get(name) {
                    return ExprIR::IdentifierExpr {
                        name: name.clone(),
                        type_ann: Some(t.clone()),
                    };
                }
            }
            e.clone()
        }
        ExprIR::FieldAccessExpr { object, field } => ExprIR::FieldAccessExpr {
            object: Box::new(ann(object)),
            field: field.clone(),
        },
        ExprIR::OptionalChainExpr { object, field } => ExprIR::OptionalChainExpr {
            object: Box::new(ann(object)),
            field: field.clone(),
        },
        ExprIR::BinaryOpExpr { op, left, right } => ExprIR::BinaryOpExpr {
            op: *op,
            left: Box::new(ann(left)),
            right: Box::new(ann(right)),
        },
        ExprIR::UnaryOpExpr { op, operand } => ExprIR::UnaryOpExpr {
            op: *op,
            operand: Box::new(ann(operand)),
        },
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => ExprIR::TernaryIR {
            cond: Box::new(ann(cond)),
            then_expr: Box::new(ann(then_expr)),
            else_expr: Box::new(ann(else_expr)),
        },
        ExprIR::CallExpr { name, args } => ExprIR::CallExpr {
            name: name.clone(),
            args: args.iter().map(ann).collect(),
        },
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => ExprIR::StoreFieldAccessExpr {
            store_alias: store_alias.clone(),
            field_name: field_name.clone(),
            call_args: call_args.iter().map(ann).collect(),
        },
        ExprIR::ArrayExpr { elements } => ExprIR::ArrayExpr {
            elements: elements.iter().map(ann).collect(),
        },
        ExprIR::SpreadExpr { operand } => ExprIR::SpreadExpr {
            operand: Box::new(ann(operand)),
        },
        ExprIR::StructExpr { type_name, fields } => ExprIR::StructExpr {
            type_name: type_name.clone(),
            fields: fields.iter().map(|(k, v)| (k.clone(), ann(v))).collect(),
        },
        ExprIR::VariantLiteral {
            union_type,
            variant,
            fields,
        } => ExprIR::VariantLiteral {
            union_type: union_type.clone(),
            variant: variant.clone(),
            fields: fields.iter().map(|(k, v)| (k.clone(), ann(v))).collect(),
        },
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            is_optional,
        } => ExprIR::MethodCallExpr {
            object: Box::new(ann(object)),
            method: method.clone(),
            args: args.iter().map(ann).collect(),
            is_optional: *is_optional,
        },
        ExprIR::BlockExpr { statements, tail } => ExprIR::BlockExpr {
            statements: statements
                .iter()
                .map(|s| annotate_stmt_types(s, scope))
                .collect(),
            tail: Box::new(ann(tail)),
        },
        ExprIR::IndexExpr { object, index } => ExprIR::IndexExpr {
            object: Box::new(ann(object)),
            index: Box::new(ann(index)),
        },
        ExprIR::AwaitExpr { operand } => ExprIR::AwaitExpr {
            operand: Box::new(ann(operand)),
        },
        ExprIR::ClosureExpr { params, body } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(ann(body)),
        },
        ExprIR::MatchExpr { matched, arms } => ExprIR::MatchExpr {
            matched: Box::new(ann(matched)),
            arms: arms.clone(),
        },
        ExprIR::MapExpr { entries } => ExprIR::MapExpr {
            entries: entries.iter().map(|(k, v)| (ann(k), ann(v))).collect(),
        },
        ExprIR::ArrayMethod {
            method,
            subject,
            predicate,
        } => ExprIR::ArrayMethod {
            method: *method,
            subject: Box::new(ann(subject)),
            predicate: Box::new(ann(predicate)),
        },
        // v4 compiler HIGH-1: ref-op 参数内的标识符也必须注解类型——否则
        // Dart 方法表（get/contains/concat/len 形状）对 ref-op 参数内
        // 方法调用失明（此前空臂逃逸——同构纪律，经 expr::map_ref_op_args
        // 统一下钻）。
        ExprIR::RefOpExpr { ref_name, op } => ExprIR::RefOpExpr {
            ref_name: ref_name.clone(),
            op: super::expr::map_ref_op_args(op, &mut |x: &ExprIR| annotate_expr_types(x, scope)),
        },
        ExprIR::LiteralExpr { .. } => e.clone(),
        // ExprIR is #[non_exhaustive] — cross-crate matches need a wildcard.
        #[allow(unreachable_patterns)]
        _ => e.clone(),
    }
}

/// Annotate every expression inside a statement.
pub(crate) fn annotate_stmt_types(
    s: &StatementIR,
    scope: &FxHashMap<String, String>,
) -> StatementIR {
    let ann = |e: &ExprIR| annotate_expr_types(e, scope);
    match s {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
            is_mut,
        } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: value.as_ref().map(ann),
            r#type: r#type.clone(),
            is_mut: *is_mut,
        },
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => StatementIR::IfStmtIR {
            condition: ann(condition),
            then_body: annotate_block(then_body, scope),
            else_ifs: else_ifs
                .iter()
                .map(|(c, b)| (ann(c), annotate_block(b, scope)))
                .collect(),
            else_body: else_body.as_ref().map(|b| annotate_block(b, scope)),
        },
        StatementIR::ReturnStmtIR { value } => StatementIR::ReturnStmtIR {
            value: value.as_ref().map(ann),
        },
        StatementIR::MutationStmt { intent } => StatementIR::MutationStmt {
            intent: intent.as_ref().map(|i| annotate_mutation_intent(i, scope)),
        },
        StatementIR::EventEmitStmtIR { event_name, args } => StatementIR::EventEmitStmtIR {
            event_name: event_name.clone(),
            args: args.iter().map(ann).collect(),
        },
        StatementIR::ExprStmtIR { expr } => StatementIR::ExprStmtIR { expr: ann(expr) },
        StatementIR::BlockStmtIR { body } => StatementIR::BlockStmtIR {
            body: annotate_block(body, scope),
        },
        StatementIR::WhileStmtIR { condition, body } => StatementIR::WhileStmtIR {
            condition: ann(condition),
            body: annotate_block(body, scope),
        },
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => StatementIR::ForInStmtIR {
            var_name: var_name.clone(),
            iterable: ann(iterable),
            body: annotate_block(body, scope),
        },
        StatementIR::ThrowStmtIR { expr } => StatementIR::ThrowStmtIR { expr: ann(expr) },
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var,
            catch_body,
            finally_body,
        } => StatementIR::TryCatchStmtIR {
            try_body: annotate_block(try_body, scope),
            catch_var: catch_var.clone(),
            catch_body: catch_body.as_ref().map(|b| annotate_block(b, scope)),
            finally_body: finally_body.as_ref().map(|b| annotate_block(b, scope)),
        },
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => s.clone(),
        // StatementIR is #[non_exhaustive] — cross-crate matches need a
        // wildcard.
        #[allow(unreachable_patterns)]
        _ => s.clone(),
    }
}

fn annotate_block(b: &BlockIR, scope: &FxHashMap<String, String>) -> BlockIR {
    BlockIR {
        statements: b
            .statements
            .iter()
            .map(|s| annotate_stmt_types(s, scope))
            .collect(),
    }
}

fn annotate_mutation_intent(
    i: &MutationIntent,
    scope: &FxHashMap<String, String>,
) -> MutationIntent {
    let ann = |e: &ExprIR| annotate_expr_types(e, scope);
    match i {
        MutationIntent::DirectAssign {
            target,
            value,
            field_object,
        } => MutationIntent::DirectAssign {
            target: target.clone(),
            value: value.as_ref().map(|v| Box::new(ann(v))),
            field_object: field_object.clone(),
        },
        MutationIntent::Increment { target, amount } => MutationIntent::Increment {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(ann(v))),
        },
        MutationIntent::Decrement { target, amount } => MutationIntent::Decrement {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(ann(v))),
        },
        MutationIntent::Multiply { target, amount } => MutationIntent::Multiply {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(ann(v))),
        },
        MutationIntent::Divide { target, amount } => MutationIntent::Divide {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(ann(v))),
        },
        MutationIntent::Modulo { target, amount } => MutationIntent::Modulo {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(ann(v))),
        },
        MutationIntent::Push { target, item } => MutationIntent::Push {
            target: target.clone(),
            item: item.as_ref().map(|v| Box::new(ann(v))),
        },
        MutationIntent::Pop { target } => MutationIntent::Pop {
            target: target.clone(),
        },
        // MutationIntent is #[non_exhaustive] — cross-crate matches need a
        // wildcard.
        #[allow(unreachable_patterns)]
        _ => i.clone(),
    }
}
