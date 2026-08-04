//! Expression formatting for Flutter codegen.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::JsonFormatConfig;
use crate::codegen::Target;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

// ═══════════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════════

pub(crate) fn format_expr_ir_flutter(expr: &ExprIR) -> String {
    super::r#ref::FlutterFormatExpr::new(None).format_expr(expr)
}

// ═══════════════════════════════════════════════════════════════
// Store access rewriting (C4)
// ═══════════════════════════════════════════════════════════════

/// Rewrite `StoreFieldAccessExpr` nodes into Riverpod 3.0 access expressions
/// (C4). Maps:
///
/// - signal/getter read:  `TodoStore.totalCount` →
///   `ref.watch(todoStoreProvider.select((s) => s.totalCount))`
/// - action call:         `TodoStore.setFilter("all")` →
///   `ref.read(todoStoreProvider.notifier).setFilter("all")`
///
/// The rewrite is performed at IR level (not on formatted text) so nested
/// store accesses inside any expression (binary ops, array elements, call
/// args, ...) are caught by the recursive walk. Stores that are not in
/// `store_provider_map` fall back to the plain `alias.field` form.
///
/// INLINE stores (`store Name { ... }` inside a component — flattened into
/// the component's signals/fns at lowering, never emitted as a provider)
/// map to an EMPTY provider string: the alias access rewrites to the bare
/// field/action name (`CartStore.addItem(x)` → `addItem(x)`), which the
/// State class resolves to its own method/field (Task 5.8: InlineStoreDemo
/// emitted `CartStore.addItem(...)` with no such class → undefined
/// identifier).
#[must_use]
pub(crate) fn rewrite_store_accesses(
    expr: &ExprIR,
    store_provider_map: &FxHashMap<String, String>,
    ir: &IRProgram,
) -> ExprIR {
    let rw = |e: &ExprIR| rewrite_store_accesses(e, store_provider_map, ir);
    match expr {
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let Some(provider) = store_provider_map.get(store_alias) else {
                return expr.clone();
            };
            let is_action = ir
                .modules
                .values()
                .flat_map(|m| m.stores.values())
                .find(|s| s.name == *store_alias)
                .is_some_and(|s| s.actions.iter().any(|a| a.name == *field_name));
            let args: Vec<ExprIR> = call_args.iter().map(rw).collect();
            if is_action {
                store_action_call_expr(provider, field_name, args)
            } else if provider.is_empty() {
                // Inline store action — the flattened component method
                // (empty provider string is the inline-store marker).
                ExprIR::CallExpr {
                    name: field_name.clone(),
                    args,
                }
            } else {
                store_read_expr(store_alias, provider, field_name)
            }
        }
        ExprIR::FieldAccessExpr { object, field } => {
            // Store property read in property form: `TodoStore.totalCount`
            // lowers to FieldAccessExpr on the store identifier (only the call
            // form `TodoStore.action(...)` becomes StoreFieldAccessExpr).
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if let Some(provider) = store_provider_map.get(name) {
                    let is_action = ir
                        .modules
                        .values()
                        .flat_map(|m| m.stores.values())
                        .find(|s| s.name == *name)
                        .is_some_and(|s| s.actions.iter().any(|a| a.name == *field));
                    if is_action {
                        // Bare `TodoStore.action` reference → invoke the action
                        return store_action_call_expr(provider, field, vec![]);
                    }
                    if provider.is_empty() {
                        // Inline store getter read — the flattened field.
                        return ExprIR::IdentifierExpr {
                            name: field.clone(),
                            type_ann: None,
                        };
                    }
                    return store_read_expr(name, provider, field);
                }
            }
            ExprIR::FieldAccessExpr {
                object: Box::new(rw(object)),
                field: field.clone(),
            }
        }
        ExprIR::OptionalChainExpr { object, field } => ExprIR::OptionalChainExpr {
            object: Box::new(rw(object)),
            field: field.clone(),
        },
        ExprIR::BinaryOpExpr { op, left, right } => ExprIR::BinaryOpExpr {
            op: *op,
            left: Box::new(rw(left)),
            right: Box::new(rw(right)),
        },
        ExprIR::UnaryOpExpr { op, operand } => ExprIR::UnaryOpExpr {
            op: *op,
            operand: Box::new(rw(operand)),
        },
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => ExprIR::TernaryIR {
            cond: Box::new(rw(cond)),
            then_expr: Box::new(rw(then_expr)),
            else_expr: Box::new(rw(else_expr)),
        },
        ExprIR::CallExpr { name, args } => ExprIR::CallExpr {
            name: name.clone(),
            args: args.iter().map(rw).collect(),
        },
        ExprIR::ArrayExpr { elements } => ExprIR::ArrayExpr {
            elements: elements.iter().map(rw).collect(),
        },
        ExprIR::SpreadExpr { operand } => ExprIR::SpreadExpr {
            operand: Box::new(rw(operand)),
        },
        ExprIR::StructExpr { type_name, fields } => ExprIR::StructExpr {
            type_name: type_name.clone(),
            fields: fields.iter().map(|(k, v)| (k.clone(), rw(v))).collect(),
        },
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            is_optional,
        } => {
            // Store action call in method-call form: `TodoStore.clear()`
            // lowers to MethodCallExpr on the store identifier inside inline
            // handlers (only `TodoStore.action()` at statement level in fn
            // bodies becomes StoreFieldAccessExpr).
            if !is_optional {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if let Some(provider) = store_provider_map.get(name) {
                        let is_action = ir
                            .modules
                            .values()
                            .flat_map(|m| m.stores.values())
                            .find(|s| s.name == *name)
                            .is_some_and(|s| s.actions.iter().any(|a| a.name == *method));
                        if is_action {
                            let rewritten_args: Vec<ExprIR> = args.iter().map(rw).collect();
                            return store_action_call_expr(provider, method, rewritten_args);
                        }
                        if provider.is_empty() {
                            // Inline store action in method-call form
                            // (`CartStore.clear()` inside inline handlers) —
                            // bare call against the flattened component fn.
                            let rewritten_args: Vec<ExprIR> = args.iter().map(rw).collect();
                            return ExprIR::CallExpr {
                                name: method.clone(),
                                args: rewritten_args,
                            };
                        }
                    }
                }
            }
            ExprIR::MethodCallExpr {
                object: Box::new(rw(object)),
                method: method.clone(),
                args: args.iter().map(rw).collect(),
                is_optional: *is_optional,
            }
        }
        ExprIR::BlockExpr { statements, tail } => ExprIR::BlockExpr {
            statements: statements.clone(),
            tail: Box::new(rw(tail)),
        },
        ExprIR::IndexExpr { object, index } => ExprIR::IndexExpr {
            object: Box::new(rw(object)),
            index: Box::new(rw(index)),
        },
        ExprIR::AwaitExpr { operand } => ExprIR::AwaitExpr {
            operand: Box::new(rw(operand)),
        },
        ExprIR::ClosureExpr { params, body } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(rw(body)),
        },
        ExprIR::VariantLiteral {
            union_type,
            variant,
            fields,
        } => ExprIR::VariantLiteral {
            union_type: union_type.clone(),
            variant: variant.clone(),
            fields: fields.iter().map(|(k, v)| (k.clone(), rw(v))).collect(),
        },
        ExprIR::MatchExpr { matched, arms } => ExprIR::MatchExpr {
            matched: Box::new(rw(matched)),
            arms: arms.clone(),
        },
        ExprIR::MapExpr { entries } => ExprIR::MapExpr {
            entries: entries.iter().map(|(k, v)| (rw(k), rw(v))).collect(),
        },
        ExprIR::ArrayMethod {
            method,
            subject,
            predicate,
        } => ExprIR::ArrayMethod {
            method: *method,
            subject: Box::new(rw(subject)),
            predicate: Box::new(rw(predicate)),
        },
        leaf => leaf.clone(),
    }
}

/// Rewrite store accesses inside a statement (used for inline event handler
/// bodies in view context). Mutations targets are left untouched — store
/// signal mutations from components are handled at statement level elsewhere.
#[must_use]
pub(crate) fn rewrite_stmt_store_accesses(
    stmt: &StatementIR,
    store_provider_map: &FxHashMap<String, String>,
    ir: &IRProgram,
) -> StatementIR {
    let rw_expr = |e: &ExprIR| rewrite_store_accesses(e, store_provider_map, ir);
    let rw_stmt = |s: &StatementIR| rewrite_stmt_store_accesses(s, store_provider_map, ir);
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: value.as_ref().map(rw_expr),
            r#type: r#type.clone(),
        },
        StatementIR::ExprStmtIR { expr } => StatementIR::ExprStmtIR {
            expr: rw_expr(expr),
        },
        StatementIR::ReturnStmtIR { value } => StatementIR::ReturnStmtIR {
            value: value.as_ref().map(rw_expr),
        },
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => StatementIR::IfStmtIR {
            condition: rw_expr(condition),
            then_body: BlockIR {
                statements: then_body.statements.iter().map(rw_stmt).collect(),
            },
            else_ifs: else_ifs
                .iter()
                .map(|(c, b)| {
                    (
                        rw_expr(c),
                        BlockIR {
                            statements: b.statements.iter().map(rw_stmt).collect(),
                        },
                    )
                })
                .collect(),
            else_body: else_body.as_ref().map(|b| BlockIR {
                statements: b.statements.iter().map(rw_stmt).collect(),
            }),
        },
        StatementIR::WhileStmtIR { condition, body } => StatementIR::WhileStmtIR {
            condition: rw_expr(condition),
            body: BlockIR {
                statements: body.statements.iter().map(rw_stmt).collect(),
            },
        },
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => StatementIR::ForInStmtIR {
            var_name: var_name.clone(),
            iterable: rw_expr(iterable),
            body: BlockIR {
                statements: body.statements.iter().map(rw_stmt).collect(),
            },
        },
        StatementIR::MutationStmt { intent } => StatementIR::MutationStmt {
            intent: intent
                .as_ref()
                .map(|i| rewrite_mutation_intent(i, &rw_expr)),
        },
        StatementIR::EventEmitStmtIR { event_name, args } => StatementIR::EventEmitStmtIR {
            event_name: event_name.clone(),
            args: args.iter().map(rw_expr).collect(),
        },
        StatementIR::BlockStmtIR { body } => StatementIR::BlockStmtIR {
            body: BlockIR {
                statements: body.statements.iter().map(rw_stmt).collect(),
            },
        },
        StatementIR::ThrowStmtIR { expr } => StatementIR::ThrowStmtIR {
            expr: rw_expr(expr),
        },
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var,
            catch_body,
            finally_body,
        } => StatementIR::TryCatchStmtIR {
            try_body: BlockIR {
                statements: try_body.statements.iter().map(rw_stmt).collect(),
            },
            catch_var: catch_var.clone(),
            catch_body: catch_body.as_ref().map(|b| BlockIR {
                statements: b.statements.iter().map(rw_stmt).collect(),
            }),
            finally_body: finally_body.as_ref().map(|b| BlockIR {
                statements: b.statements.iter().map(rw_stmt).collect(),
            }),
        },
        leaf => leaf.clone(),
    }
}

/// Rewrite component prop / event / slot identifiers into `widget.<name>`
/// field accesses (Task 5.8). Props and event callbacks live on the
/// StatefulWidget; the State class previously emitted bare `icon` / `label`
/// / `onOptionalAck` references — undefined identifiers. Also applies
/// component-fn renames (`double` → `double_` when the fn name shadows a
/// Dart builtin type used in the same class).
///
/// Mutation targets are NOT rewritten (they are handled at statement level
/// by the mutation formatter, which keeps plain field names).
#[must_use]
pub(crate) fn rewrite_widget_refs(
    expr: &ExprIR,
    widget_names: &FxHashSet<String>,
    fn_renames: &FxHashMap<String, String>,
) -> ExprIR {
    let rw = |e: &ExprIR| rewrite_widget_refs(e, widget_names, fn_renames);
    match expr {
        ExprIR::IdentifierExpr { name, type_ann } => {
            if widget_names.contains(name) {
                return ExprIR::FieldAccessExpr {
                    object: Box::new(ExprIR::ident("widget")),
                    field: name.clone(),
                };
            }
            if let Some(renamed) = fn_renames.get(name) {
                return ExprIR::IdentifierExpr {
                    name: renamed.clone(),
                    type_ann: type_ann.clone(),
                };
            }
            expr.clone()
        }
        ExprIR::CallExpr { name, args } => {
            // Bare event/prop-fn calls (`onSwitch('overview')` inside fn
            // bodies and inline handlers) resolve to the widget's callback
            // field (Task 5.8: previously emitted bare → undefined method on
            // the State class). The call is null-aware ON THE FIELD:
            // `widget.onSwitch?.call('overview')` — the receiver is the
            // (non-null) widget, the nullable member is the callback.
            if widget_names.contains(name) {
                return ExprIR::MethodCallExpr {
                    object: Box::new(ExprIR::FieldAccessExpr {
                        object: Box::new(ExprIR::ident("widget")),
                        field: name.clone(),
                    }),
                    method: "call".into(),
                    args: args.iter().map(rw).collect(),
                    is_optional: true,
                };
            }
            let renamed = fn_renames
                .get(name)
                .cloned()
                .unwrap_or_else(|| name.clone());
            ExprIR::CallExpr {
                name: renamed,
                args: args.iter().map(rw).collect(),
            }
        }
        ExprIR::FieldAccessExpr { object, field } => ExprIR::FieldAccessExpr {
            object: Box::new(rw(object)),
            field: field.clone(),
        },
        ExprIR::OptionalChainExpr { object, field } => ExprIR::OptionalChainExpr {
            object: Box::new(rw(object)),
            field: field.clone(),
        },
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            is_optional,
        } => ExprIR::MethodCallExpr {
            object: Box::new(rw(object)),
            method: method.clone(),
            args: args.iter().map(rw).collect(),
            is_optional: *is_optional,
        },
        ExprIR::BinaryOpExpr { op, left, right } => ExprIR::BinaryOpExpr {
            op: *op,
            left: Box::new(rw(left)),
            right: Box::new(rw(right)),
        },
        ExprIR::UnaryOpExpr { op, operand } => ExprIR::UnaryOpExpr {
            op: *op,
            operand: Box::new(rw(operand)),
        },
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => ExprIR::TernaryIR {
            cond: Box::new(rw(cond)),
            then_expr: Box::new(rw(then_expr)),
            else_expr: Box::new(rw(else_expr)),
        },
        ExprIR::ArrayExpr { elements } => ExprIR::ArrayExpr {
            elements: elements.iter().map(rw).collect(),
        },
        ExprIR::SpreadExpr { operand } => ExprIR::SpreadExpr {
            operand: Box::new(rw(operand)),
        },
        ExprIR::StructExpr { type_name, fields } => ExprIR::StructExpr {
            type_name: type_name.clone(),
            fields: fields.iter().map(|(k, v)| (k.clone(), rw(v))).collect(),
        },
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => ExprIR::StoreFieldAccessExpr {
            store_alias: store_alias.clone(),
            field_name: field_name.clone(),
            call_args: call_args.iter().map(rw).collect(),
        },
        ExprIR::BlockExpr { statements, tail } => ExprIR::BlockExpr {
            statements: statements.clone(),
            tail: Box::new(rw(tail)),
        },
        ExprIR::IndexExpr { object, index } => ExprIR::IndexExpr {
            object: Box::new(rw(object)),
            index: Box::new(rw(index)),
        },
        ExprIR::AwaitExpr { operand } => ExprIR::AwaitExpr {
            operand: Box::new(rw(operand)),
        },
        ExprIR::ClosureExpr { params, body } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(rw(body)),
        },
        ExprIR::VariantLiteral {
            union_type,
            variant,
            fields,
        } => ExprIR::VariantLiteral {
            union_type: union_type.clone(),
            variant: variant.clone(),
            fields: fields.iter().map(|(k, v)| (k.clone(), rw(v))).collect(),
        },
        ExprIR::MatchExpr { matched, arms } => ExprIR::MatchExpr {
            matched: Box::new(rw(matched)),
            arms: arms.clone(),
        },
        ExprIR::MapExpr { entries } => ExprIR::MapExpr {
            entries: entries.iter().map(|(k, v)| (rw(k), rw(v))).collect(),
        },
        ExprIR::ArrayMethod {
            method,
            subject,
            predicate,
        } => ExprIR::ArrayMethod {
            method: *method,
            subject: Box::new(rw(subject)),
            predicate: Box::new(rw(predicate)),
        },
        leaf => leaf.clone(),
    }
}

/// Rewrite widget refs + fn renames inside a statement block.
#[must_use]
pub(crate) fn rewrite_stmt_widget_refs(
    stmt: &StatementIR,
    widget_names: &FxHashSet<String>,
    fn_renames: &FxHashMap<String, String>,
) -> StatementIR {
    let rw_expr = |e: &ExprIR| rewrite_widget_refs(e, widget_names, fn_renames);
    let rw_stmt = |s: &StatementIR| rewrite_stmt_widget_refs(s, widget_names, fn_renames);
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: value.as_ref().map(rw_expr),
            r#type: r#type.clone(),
        },
        StatementIR::ExprStmtIR { expr } => StatementIR::ExprStmtIR {
            expr: rw_expr(expr),
        },
        StatementIR::ReturnStmtIR { value } => StatementIR::ReturnStmtIR {
            value: value.as_ref().map(rw_expr),
        },
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => StatementIR::IfStmtIR {
            condition: rw_expr(condition),
            then_body: BlockIR {
                statements: then_body.statements.iter().map(rw_stmt).collect(),
            },
            else_ifs: else_ifs
                .iter()
                .map(|(c, b)| {
                    (
                        rw_expr(c),
                        BlockIR {
                            statements: b.statements.iter().map(rw_stmt).collect(),
                        },
                    )
                })
                .collect(),
            else_body: else_body.as_ref().map(|b| BlockIR {
                statements: b.statements.iter().map(rw_stmt).collect(),
            }),
        },
        StatementIR::WhileStmtIR { condition, body } => StatementIR::WhileStmtIR {
            condition: rw_expr(condition),
            body: BlockIR {
                statements: body.statements.iter().map(rw_stmt).collect(),
            },
        },
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => StatementIR::ForInStmtIR {
            var_name: var_name.clone(),
            iterable: rw_expr(iterable),
            body: BlockIR {
                statements: body.statements.iter().map(rw_stmt).collect(),
            },
        },
        StatementIR::MutationStmt { intent } => StatementIR::MutationStmt {
            intent: intent
                .as_ref()
                .map(|i| rewrite_mutation_intent(i, &rw_expr)),
        },
        StatementIR::EventEmitStmtIR { event_name, args } => StatementIR::EventEmitStmtIR {
            event_name: event_name.clone(),
            args: args.iter().map(rw_expr).collect(),
        },
        StatementIR::BlockStmtIR { body } => StatementIR::BlockStmtIR {
            body: BlockIR {
                statements: body.statements.iter().map(rw_stmt).collect(),
            },
        },
        StatementIR::ThrowStmtIR { expr } => StatementIR::ThrowStmtIR {
            expr: rw_expr(expr),
        },
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var,
            catch_body,
            finally_body,
        } => StatementIR::TryCatchStmtIR {
            try_body: BlockIR {
                statements: try_body.statements.iter().map(rw_stmt).collect(),
            },
            catch_var: catch_var.clone(),
            catch_body: catch_body.as_ref().map(|b| BlockIR {
                statements: b.statements.iter().map(rw_stmt).collect(),
            }),
            finally_body: finally_body.as_ref().map(|b| BlockIR {
                statements: b.statements.iter().map(rw_stmt).collect(),
            }),
        },
        leaf => leaf.clone(),
    }
}

/// Rewrite store accesses inside mutation intent expressions.
fn rewrite_mutation_intent(
    intent: &MutationIntent,
    rw_expr: &impl Fn(&ExprIR) -> ExprIR,
) -> MutationIntent {
    match intent {
        MutationIntent::DirectAssign {
            target,
            value,
            field_object,
        } => MutationIntent::DirectAssign {
            target: target.clone(),
            value: value.as_ref().map(|v| Box::new(rw_expr(v))),
            field_object: field_object.clone(),
        },
        MutationIntent::Increment { target, amount } => MutationIntent::Increment {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(rw_expr(v))),
        },
        MutationIntent::Decrement { target, amount } => MutationIntent::Decrement {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(rw_expr(v))),
        },
        MutationIntent::Multiply { target, amount } => MutationIntent::Multiply {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(rw_expr(v))),
        },
        MutationIntent::Divide { target, amount } => MutationIntent::Divide {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(rw_expr(v))),
        },
        MutationIntent::Modulo { target, amount } => MutationIntent::Modulo {
            target: target.clone(),
            amount: amount.as_ref().map(|v| Box::new(rw_expr(v))),
        },
        MutationIntent::Push { target, item } => MutationIntent::Push {
            target: target.clone(),
            item: item.as_ref().map(|v| Box::new(rw_expr(v))),
        },
        MutationIntent::Pop { target } => MutationIntent::Pop {
            target: target.clone(),
        },
        #[allow(unreachable_patterns)]
        _ => intent.clone(),
    }
}

/// Build the Riverpod 3.0 action call expression:
/// `ref.read(storeProvider.notifier).action(args)`.
fn store_action_call_expr(provider: &str, action: &str, args: Vec<ExprIR>) -> ExprIR {
    ExprIR::MethodCallExpr {
        object: Box::new(ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::ident("ref")),
            method: "read".into(),
            args: vec![ExprIR::FieldAccessExpr {
                object: Box::new(ExprIR::ident(provider)),
                field: "notifier".into(),
            }],
            is_optional: false,
        }),
        method: action.to_string(),
        args,
        is_optional: false,
    }
}

/// Build the Riverpod 3.0 property read expression:
/// `ref.watch(storeProvider.select((s) => s.field))`.
///
/// Task 5.8a: the select closure's param is TYPED with the store's state
/// class (`(TodoStoreState s) => s.totalCount`) — the shared closure
/// formatter otherwise annotates every untyped param with `dynamic`, and a
/// `dynamic` select result made a collection-`for` over it flag the next
/// list element as dead_code in dart analyze (NotificationsPanel).
fn store_read_expr(store_name: &str, provider: &str, field: &str) -> ExprIR {
    ExprIR::MethodCallExpr {
        object: Box::new(ExprIR::ident("ref")),
        method: "watch".into(),
        args: vec![ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::ident(provider)),
            method: "select".into(),
            args: vec![ExprIR::ClosureExpr {
                params: vec![format!("{}State s", store_name)],
                body: Box::new(ExprIR::FieldAccessExpr {
                    object: Box::new(ExprIR::ident("s")),
                    field: field.to_string(),
                }),
            }],
            is_optional: false,
        }],
        is_optional: false,
    }
}

// ═══════════════════════════════════════════════════════════════
// Store action expression formatting
// ═══════════════════════════════════════════════════════════════

/// Expression formatter used inside store action bodies (Riverpod 3.0
/// Notifier methods). Rewrites bare identifiers that name a store signal or
/// getter to `state.<name>` — e.g. `count + 1` → `state.count + 1`.
pub(crate) struct StoreActionExprFormatter<'a> {
    pub(crate) inner: DartFormatExpr,
    pub(crate) store_names: &'a FxHashSet<String>,
}

impl FormatExpr for StoreActionExprFormatter<'_> {
    fn json_config(&self) -> &JsonFormatConfig {
        self.inner.json_config()
    }

    fn codegen_target(&self) -> Target {
        // Flutter, not Dart: stdlib modules that are Flutter-only (storage —
        // SharedPreferences) must translate here too (Task 5.8: store
        // actions calling storage_set previously emitted the bare call).
        Target::Flutter
    }

    fn event_param_name(&self) -> Option<&str> {
        self.inner.event_param_name()
    }

    // Delegate to the Dart-family tables (`.to_string()` → `.toString()`,
    // `.len()` → `.length`, string `+` coercion) — store action bodies
    // previously fell through to the JS-default verbatim form.
    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        self.inner.format_method_call(obj, method, args)
    }

    fn format_optional_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        self.inner.format_optional_method_call(obj, method, args)
    }

    fn format_binary_op(
        &self,
        op: &str,
        left: &ExprIR,
        right: &ExprIR,
        left_str: &str,
        right_str: &str,
    ) -> String {
        self.inner
            .format_binary_op(op, left, right, left_str, right_str)
    }

    /// Task 3.1: typed struct literals in store actions (`Todo(id: ...)`)
    /// need the Dart constructor form — without this override the default
    /// JS-style `{ fields }` wrapper (which ignores `type_name`) would emit
    /// `{ id: nextId }` — bare identifier keys, invalid Dart.
    fn format_struct(&self, type_name: &str, fields: &str) -> String {
        self.inner.format_struct(type_name, fields)
    }

    fn format_struct_field_named(&self, key: &str, value: &str) -> String {
        self.inner.format_struct_field_named(key, value)
    }

    fn format_identifier(&self, name: &str) -> String {
        if self.store_names.contains(name) {
            format!("state.{name}")
        } else {
            name.to_string()
        }
    }
}
