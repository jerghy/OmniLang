//! Lowering for expressions.
//!
//! Includes: `lower_expr`, `lower_expr_to_json_from_ir`.

use super::stmt::{is_store_symbol, lower_block};
use crate::ir::types::*;
use crate::parser::ast::*;
use crate::semantic::symbols::SymbolKind;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Convert an AST `BinaryOpKind` to its IR counterpart.
fn lower_binary_op(kind: &BinaryOpKind) -> BinaryOpIR {
    use BinaryOpKind::*;
    match kind {
        Add => BinaryOpIR::Add,
        Sub => BinaryOpIR::Sub,
        Mul => BinaryOpIR::Mul,
        Div => BinaryOpIR::Div,
        Mod => BinaryOpIR::Mod,
        Eq => BinaryOpIR::Eq,
        Neq => BinaryOpIR::Neq,
        Lt => BinaryOpIR::Lt,
        Gt => BinaryOpIR::Gt,
        LtEq => BinaryOpIR::LtEq,
        GtEq => BinaryOpIR::GtEq,
        And => BinaryOpIR::And,
        Or => BinaryOpIR::Or,
        NullishCoalesce => BinaryOpIR::NullishCoalesce,
        Assign => BinaryOpIR::Assign,
        AddAssign => BinaryOpIR::AddAssign,
        SubAssign => BinaryOpIR::SubAssign,
        MulAssign => BinaryOpIR::MulAssign,
        DivAssign => BinaryOpIR::DivAssign,
        ModAssign => BinaryOpIR::ModAssign,
    }
}

// ═══════════════════════════════════════════════════════
// Expression lowering
// ═══════════════════════════════════════════════════════

pub(crate) fn lower_expr(
    expr: &Expr,
    variant_map: &FxHashMap<String, String>,
    mut event_fields: Option<&mut FxHashSet<String>>,
    semantics: &mut SemanticResult,
) -> ExprIR {
    match expr {
        Expr::Literal(lit, _) => {
            let value = match lit {
                Literal::Int(n) => serde_json::Value::Number((*n).into()),
                Literal::Float(f) => serde_json::json!(f),
                Literal::String(s) => serde_json::Value::String(s.clone()),
                Literal::Bool(b) => serde_json::Value::Bool(*b),
                Literal::Null => serde_json::Value::Null,
            };
            ExprIR::LiteralExpr { value }
        }
        Expr::VarRef(name, _) => {
            // Check if this bare identifier is a tagged union variant (no fields)
            if let Some(union_name) = variant_map.get(name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: name.clone(),
                    fields: vec![],
                }
            } else {
                let type_ann = semantics
                    .symbols
                    .resolve(name)
                    .and_then(|s| s.type_ann.clone());
                ExprIR::IdentifierExpr {
                    name: name.clone(),
                    type_ann,
                }
            }
        }
        Expr::BinaryOp {
            left, op, right, ..
        } => ExprIR::BinaryOpExpr {
            op: lower_binary_op(op),
            left: Box::new(lower_expr(
                left,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
            right: Box::new(lower_expr(
                right,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::UnaryOp { op, operand, .. } => ExprIR::UnaryOpExpr {
            op: match op {
                UnaryOpKind::Not => UnaryOpIR::Not,
                UnaryOpKind::Neg => UnaryOpIR::Neg,
                UnaryOpKind::Await => UnaryOpIR::Await,
            },
            operand: Box::new(lower_expr(
                operand,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::FuncCall { func, args, .. } => {
            // Pattern: signal.get() → signal.value
            if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
                if let Expr::VarRef(obj_name, _) = object.as_ref() {
                    // Store actions named get/set/inc/dec are store field
                    // accesses, not signal accesses — skip the rewrites below.
                    let is_store_receiver = is_store_symbol(semantics, obj_name);
                    // Signal method calls
                    if !is_store_receiver && field == "get" {
                        return ExprIR::FieldAccessExpr {
                            object: Box::new(ExprIR::ident(obj_name)),
                            field: "value".into(),
                        };
                    }
                    if !is_store_receiver && (field == "set" || field == "inc" || field == "dec") {
                        // Multi-arg calls like map.set(key, val) are regular method calls,
                        // not signal mutations (which always have 0-1 args).
                        if field == "set" && args.len() > 1 {
                            return ExprIR::MethodCallExpr {
                                object: Box::new(ExprIR::ident(obj_name)),
                                method: field.clone(),
                                args: args
                                    .iter()
                                    .map(|e| {
                                        lower_expr(
                                            e,
                                            variant_map,
                                            event_fields.as_deref_mut(),
                                            semantics,
                                        )
                                    })
                                    .collect(),
                                is_optional: false,
                            };
                        }
                        // Single-arg mutation — handled at statement level by try_lower_mutation
                        return ExprIR::ident(obj_name);
                    }
                    // Store field access
                    return ExprIR::StoreFieldAccessExpr {
                        store_alias: obj_name.clone(),
                        field_name: field.clone(),
                        call_args: args
                            .iter()
                            .map(|e| {
                                lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics)
                            })
                            .collect(),
                    };
                }
            }
            // Regular function call
            if let Expr::VarRef(name, _) = func.as_ref() {
                ExprIR::CallExpr {
                    name: name.clone(),
                    args: args
                        .iter()
                        .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics))
                        .collect(),
                }
            } else if let Expr::OptionalChain { object, field, .. } = func.as_ref() {
                // user?.len() — treat as optional method call
                ExprIR::MethodCallExpr {
                    object: Box::new(lower_expr(
                        object,
                        variant_map,
                        event_fields.as_deref_mut(),
                        semantics,
                    )),
                    method: field.clone(),
                    args: args
                        .iter()
                        .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics))
                        .collect(),
                    is_optional: true,
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::FieldAccess { object, field, .. } => {
            // Collect event field names during lowering (e.field → push field).
            // FxHashSet ensures O(1) dedup.
            if let Some(ref mut fields) = event_fields {
                if let Expr::VarRef(name, _) = object.as_ref() {
                    let is_event_param = semantics.event_param_name.as_deref() == Some(name);
                    if is_event_param || name == "e" {
                        fields.insert(field.clone());
                    }
                }
            }
            ExprIR::FieldAccessExpr {
                object: Box::new(lower_expr(object, variant_map, event_fields, semantics)),
                field: field.clone(),
            }
        }
        Expr::OptionalChain { object, field, .. } => ExprIR::OptionalChainExpr {
            object: Box::new(lower_expr(
                object,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
            field: field.clone(),
        },
        Expr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            // Store action calls (CounterStore.get()/set()) are method calls,
            // not signal accesses — skip the signal-specific rewrites below.
            let is_store_receiver = matches!(
                object.as_ref(),
                Expr::VarRef(name, _) if is_store_symbol(semantics, name)
            );
            // get() with no args → signal access (.value)
            // get(i) with args → array/map index access
            if !is_store_receiver && method == "get" && args.is_empty() {
                ExprIR::FieldAccessExpr {
                    object: Box::new(lower_expr(
                        object,
                        variant_map,
                        event_fields.as_deref_mut(),
                        semantics,
                    )),
                    field: "value".into(),
                }
            } else if !is_store_receiver && method == "set" && args.len() <= 1 {
                // signal.set(value) with 0-1 args → handled at statement level by try_lower_mutation
                ExprIR::ident("set")
            } else {
                // General method call: to_string, len, get(i), push, pop, contains, insert, etc.
                ExprIR::MethodCallExpr {
                    object: Box::new(lower_expr(
                        object,
                        variant_map,
                        event_fields.as_deref_mut(),
                        semantics,
                    )),
                    method: method.clone(),
                    args: args
                        .iter()
                        .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics))
                        .collect(),
                    is_optional: false,
                }
            }
        }
        Expr::StoreFieldAccess {
            store_alias,
            field_name,
            ..
        } => ExprIR::StoreFieldAccessExpr {
            store_alias: store_alias.clone(),
            field_name: field_name.clone(),
            call_args: vec![],
        },
        Expr::ArrayLiteral { elements, .. } => ExprIR::ArrayExpr {
            elements: elements
                .iter()
                .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics))
                .collect(),
        },
        Expr::SpreadExpr { operand, .. } => ExprIR::SpreadExpr {
            operand: Box::new(lower_expr(
                operand,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::MapLiteral { entries, .. } => {
            let ir_entries: Vec<(ExprIR, ExprIR)> = entries
                .iter()
                .map(|(k, v)| {
                    let key = lower_expr(k, variant_map, event_fields.as_deref_mut(), semantics);
                    let val = lower_expr(v, variant_map, event_fields.as_deref_mut(), semantics);
                    (key, val)
                })
                .collect();
            ExprIR::MapExpr {
                entries: ir_entries,
            }
        }
        Expr::StructLiteral {
            type_name, fields, ..
        } => {
            let ir_fields: Vec<(String, ExprIR)> = fields
                .iter()
                .map(|(name, val)| {
                    (
                        name.clone(),
                        lower_expr(val, variant_map, event_fields.as_deref_mut(), semantics),
                    )
                })
                .collect();
            // Check if this is a tagged union variant construction
            if let Some(union_name) = variant_map.get(type_name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: type_name.clone(),
                    fields: ir_fields,
                }
            } else {
                ExprIR::StructExpr {
                    type_name: type_name.clone(),
                    fields: ir_fields,
                }
            }
        }
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(
                condition,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
            then_expr: Box::new(lower_expr(
                then_expr,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
            else_expr: Box::new(lower_expr(
                else_expr,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(
                condition,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
            then_expr: Box::new(lower_expr(
                then_expr,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
            else_expr: Box::new(lower_expr(
                else_expr,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::Block { body, .. } => {
            // Block in expression position: preserve ALL statements.
            // The last expression statement provides the value; preceding
            // statements (e.g. print, let, mutations) are kept as-is.
            let block_ir = lower_block(body, variant_map, event_fields.as_deref_mut(), semantics);
            if let Some(last) = block_ir.statements.last() {
                match last {
                    StatementIR::ExprStmtIR { expr } => {
                        if block_ir.statements.len() == 1 {
                            // Simple block { expr } — just the expression
                            expr.clone()
                        } else {
                            // Multi-statement block: wrap in BlockExpr
                            ExprIR::BlockExpr {
                                statements: block_ir.statements[..block_ir.statements.len() - 1]
                                    .to_vec(),
                                tail: Box::new(expr.clone()),
                            }
                        }
                    }
                    _ => {
                        // Last statement is not an expression (e.g. return, if)
                        // Preserve all statements; value is null
                        ExprIR::BlockExpr {
                            statements: block_ir.statements,
                            tail: Box::new(ExprIR::LiteralExpr {
                                value: serde_json::Value::Null,
                            }),
                        }
                    }
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::IndexGet { object, index, .. } => ExprIR::IndexExpr {
            object: Box::new(lower_expr(
                object,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
            index: Box::new(lower_expr(
                index,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::ClosureExpr { params, body, .. } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(lower_expr(
                body,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::AwaitExpr { operand, .. } => ExprIR::AwaitExpr {
            operand: Box::new(lower_expr(
                operand,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            )),
        },
        Expr::MatchExpr { matched, arms, .. } => {
            let matched_ir = Box::new(lower_expr(
                matched,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            ));
            let mut ir_arms = Vec::with_capacity(arms.len());
            for arm in arms {
                let pattern = match &arm.pattern.kind {
                    MatchPatternKind::Variant { name, bindings } => MatchPatternIR {
                        kind: MatchPatternKindIR::Variant {
                            name: name.clone(),
                            bindings: bindings.clone(),
                        },
                    },
                    MatchPatternKind::Wildcard => MatchPatternIR {
                        kind: MatchPatternKindIR::Wildcard,
                    },
                    MatchPatternKind::Literal(lit) => {
                        let value = match lit {
                            Literal::Int(n) => serde_json::Value::Number((*n).into()),
                            Literal::String(s) => serde_json::Value::String(s.clone()),
                            Literal::Bool(b) => serde_json::Value::Bool(*b),
                            _ => serde_json::Value::Null,
                        };
                        MatchPatternIR {
                            kind: MatchPatternKindIR::Literal { value },
                        }
                    }
                };
                let union_type = match &arm.pattern.kind {
                    MatchPatternKind::Variant { name, .. } => variant_map.get(name).cloned(),
                    _ => None,
                };
                ir_arms.push(MatchArmIR {
                    pattern,
                    body: lower_block(
                        &arm.body,
                        variant_map,
                        event_fields.as_deref_mut(),
                        semantics,
                    ),
                    union_type,
                });
            }
            ExprIR::MatchExpr {
                matched: matched_ir,
                arms: ir_arms,
            }
        }
        Expr::TemplateString { parts, .. } => {
            // Lower template string parts to a runtime concatenation chain.
            // Each part is either a literal string or an expression; they are
            // joined via BinaryOpExpr with Add so that existing codegen handles
            // JavaScript template literals automatically (string + expr + string).
            if parts.is_empty() {
                return ExprIR::LiteralExpr {
                    value: serde_json::Value::String(String::new()),
                };
            }

            let mut result: Option<ExprIR> = None;
            for part in parts {
                let part_ir = match part {
                    TemplatePart::Literal(s) => ExprIR::LiteralExpr {
                        value: serde_json::Value::String(s.clone()),
                    },
                    TemplatePart::Expr(e) => {
                        lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics)
                    }
                };
                result = Some(match result {
                    None => part_ir,
                    Some(left) => ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Add,
                        left: Box::new(left),
                        right: Box::new(part_ir),
                    },
                });
            }

            result.unwrap_or_else(|| ExprIR::LiteralExpr {
                value: serde_json::Value::String(String::new()),
            })
        }
        Expr::NotYetImplemented {
            feature: _feature, ..
        } => {
            // Emit a null literal for unimplemented features rather than
            // silently using a catch-all wildcard. This makes it explicit
            // that the feature gap is known.
            ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            }
        }
        Expr::IsCheck { .. } => {
            // Lower is-check to null (simplified; full codegen in next pass)
            ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            }
        }
        Expr::Error { .. } => {
            // Error nodes should be filtered upstream; emit null as fallback.
            ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
// Task 3.1: typed struct literals in store actions
// ═══════════════════════════════════════════════════════
//
// CRITICAL (flutter): `action add(...) { items = [...items, { id: nextId,
// text, done: false }] }` on a `Todo[]` signal used to lower the bare
// `{ ... }` literal to a `MapExpr` — Dart codegen then emitted
// `Map<String, Object>` (`{ 'id': ... }`), the list became `List<Object>`,
// and the state's `copyWith` cast (`items as List<Todo>`) threw at runtime —
// the TodoStore.add() TypeError that crashed the flagship demo's core loop.
//
// The expected type of a store action write is NOT resolvable from the symbol
// table at lowering time (lowering pushes fresh empty scopes; the
// analysis-time `store:X` scope holding the signal symbols is not on the
// resolve() parent chain — see the is_signal_like_symbol note above). Instead,
// `lower_store` has the store's own signal/param type map (`StoreIR.signals`,
// built from the AST annotations) — this pass runs right after each action
// body is lowered and rewrites bare map literals whose expected type is a
// struct into typed `StructExpr`s. Unknown/builtin/container types keep the
// map form (Dart-legal, and JS backends render both forms identically).

/// Rewrite every mutation in a store action body so bare `{ ... }` map
/// literals assigned to a typed signal/param (or pushed into a typed array)
/// become typed `StructExpr`s. `expected_types` maps names → declared types
/// for the store's signals AND the action's typed params.
pub(super) fn annotate_typed_struct_literals(
    block: &mut BlockIR,
    expected_types: &FxHashMap<String, TypeIR>,
) {
    for stmt in &mut block.statements {
        match stmt {
            StatementIR::MutationStmt {
                intent: Some(intent),
            } => {
                match intent {
                    MutationIntent::DirectAssign { target, value, .. } => {
                        if let Some(value) = value {
                            rewrite_expected_struct(value, expected_types.get(&target.name));
                        }
                    }
                    MutationIntent::Push { target, item } => {
                        if let Some(item) = item {
                            // `items.push({...})` — the item is expected to be
                            // the array's ELEMENT type.
                            let element = expected_types
                                .get(&target.name)
                                .and_then(array_element_type_ir);
                            rewrite_expected_struct(item, element);
                        }
                    }
                    _ => {}
                }
            }
            StatementIR::IfStmtIR {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                annotate_typed_struct_literals(then_body, expected_types);
                for (_, body) in else_ifs {
                    annotate_typed_struct_literals(body, expected_types);
                }
                if let Some(body) = else_body {
                    annotate_typed_struct_literals(body, expected_types);
                }
            }
            StatementIR::WhileStmtIR { body, .. } | StatementIR::ForInStmtIR { body, .. } => {
                annotate_typed_struct_literals(body, expected_types);
            }
            StatementIR::BlockStmtIR { body } => {
                annotate_typed_struct_literals(body, expected_types);
            }
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                annotate_typed_struct_literals(try_body, expected_types);
                if let Some(body) = catch_body {
                    annotate_typed_struct_literals(body, expected_types);
                }
                if let Some(body) = finally_body {
                    annotate_typed_struct_literals(body, expected_types);
                }
            }
            _ => {}
        }
    }
}

/// Rewrite an expression tree under a known expected type. Array types
/// descend into elements/spreads/branches; struct types convert bare
/// `MapExpr`s into `StructExpr`s (typed constructors); anything else is left
/// untouched.
fn rewrite_expected_struct(ir: &mut ExprIR, expected: Option<&TypeIR>) {
    let Some(expected) = expected else { return };
    if let Some(element) = array_element_type_ir(expected) {
        match ir {
            ExprIR::ArrayExpr { elements } => {
                for elem in elements {
                    rewrite_expected_struct(elem, Some(element));
                }
            }
            ExprIR::SpreadExpr { operand } => rewrite_expected_struct(operand, Some(expected)),
            ExprIR::TernaryIR {
                then_expr,
                else_expr,
                ..
            } => {
                rewrite_expected_struct(then_expr, Some(expected));
                rewrite_expected_struct(else_expr, Some(expected));
            }
            ExprIR::BinaryOpExpr { left, right, .. } => {
                rewrite_expected_struct(left, Some(expected));
                rewrite_expected_struct(right, Some(expected));
            }
            ExprIR::BlockExpr { tail, .. } => rewrite_expected_struct(tail, Some(expected)),
            _ => {}
        }
        return;
    }
    // Struct element context: a bare map literal becomes a typed constructor.
    if let TypeKind::Named { name } = &expected.kind {
        if expected.type_args.is_empty() && is_struct_like_name(name) {
            if let ExprIR::MapExpr { entries } = ir {
                let fields: Option<Vec<(String, ExprIR)>> = entries
                    .iter()
                    .map(|(k, v)| match k {
                        ExprIR::LiteralExpr {
                            value: serde_json::Value::String(s),
                        } => Some((s.clone(), v.clone())),
                        _ => None,
                    })
                    .collect();
                if let Some(fields) = fields {
                    *ir = ExprIR::StructExpr {
                        type_name: name.clone(),
                        fields,
                    };
                }
            }
        }
    }
}

/// Element type of an array `TypeIR` (`Array<T>`, `List<T>`, `Vec<T>`), or
/// `None` for non-array types.
fn array_element_type_ir(ty: &TypeIR) -> Option<&TypeIR> {
    match &ty.kind {
        TypeKind::Named { name } if matches!(name.as_str(), "Array" | "List" | "Vec") => {
            ty.type_args.first()
        }
        _ => None,
    }
}

/// A plain user type name that Dart emits as a named-parameter constructor
/// call — builtins and container types keep the map-literal form.
fn is_struct_like_name(name: &str) -> bool {
    !matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "int"
            | "Int"
            | "integer"
            | "isize"
            | "usize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f32"
            | "f64"
            | "float"
            | "Float"
            | "num"
            | "number"
            | "double"
            | "Double"
            | "String"
            | "str"
            | "string"
            | "bool"
            | "Bool"
            | "boolean"
            | "void"
            | "Void"
            | "()"
            | "unit"
            | "auto"
            | "Auto"
            | "var"
            | "Var"
            | "dynamic"
            | "Dynamic"
            | "Any"
            | "null"
            | "Null"
            | "nil"
            | "None"
            | "Map"
            | "Array"
            | "List"
            | "Vec"
            | "Set"
            | "Dict"
            | "HashMap"
            | "function"
            | "Fn"
            | "fn"
    )
}

/// Whether a name resolves to a signal-like symbol — a value with its own
/// reactive JS binding (signal, computed, prop, getter, ref). Such values
/// are emitted as `.value`-backed objects (OmniView) or bare variables
/// (React) and carry no `name_field` alias; field access on them is REAL
/// field access on the value.
///
/// `signal_like_names` carries the current component's signal-like names:
/// the symbol table alone cannot answer this at lowering time (ir::lower
/// pushes fresh empty component scopes, so the analysis-time component
/// scope holding these symbols is not on the resolve() parent chain).
fn is_signal_like_symbol(
    semantics: &SemanticResult,
    signal_like_names: &FxHashSet<String>,
    name: &str,
) -> bool {
    signal_like_names.contains(name)
        || matches!(
            semantics.symbols.resolve(name).map(|s| &s.kind),
            Some(
                SymbolKind::Signal
                    | SymbolKind::Computed
                    | SymbolKind::Prop
                    | SymbolKind::Getter
                    | SymbolKind::Ref
            )
        )
}

/// Convert an ExprIR back to a JSON value for inline_style serialization.
///
/// Channel contract (设计文档 域 8, review C2 root cause):
/// - `Store.field` — the object resolves to a store symbol, so `Store_field`
///   IS a real emitted signal variable in every backend — keep the `$signal`
///   underscore form.
/// - `obj.field` on a signal-like object → `$field` with a `$signal` object
///   (React emits `obj.field`; OmniView emits `obj.value.field`).
/// - `obj.field` on any other object (loop item, param, const, …) →
///   `$field` with an `$ident` object (both targets emit `obj.field`).
///   Never mangle `row.score` into the nonexistent identifier `row_score`.
pub(super) fn lower_expr_to_json_from_ir(
    expr: &ExprIR,
    semantics: &SemanticResult,
    signal_like_names: &FxHashSet<String>,
) -> serde_json::Value {
    match expr {
        ExprIR::LiteralExpr { value } => value.clone(),
        ExprIR::IdentifierExpr { name, .. } => serde_json::json!({"$signal": name}),
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name: obj_name, .. } = object.as_ref() {
                // Genuine store signal alias (ThemeStore.currentBg →
                // ThemeStore_currentBg): the `$signal` underscore form.
                if is_store_symbol(semantics, obj_name) {
                    return serde_json::json!({"$signal": format!("{}_{}", obj_name, field)});
                }
                let obj_marker = if is_signal_like_symbol(semantics, signal_like_names, obj_name) {
                    // Signal-like value: backends read through `.value` /
                    // bare variable — field access lands on the value.
                    serde_json::json!({"$signal": obj_name})
                } else {
                    // Plain local (loop item, param, const): bare identifier.
                    serde_json::json!({"$ident": obj_name})
                };
                return serde_json::json!({"$field": [obj_marker, field.as_str()]});
            }
            // Complex object (nested field access, call result, index):
            // the object is itself an expression — recurse.
            let obj = lower_expr_to_json_from_ir(object, semantics, signal_like_names);
            serde_json::json!({"$field": [obj, field.as_str()]})
        }
        ExprIR::BinaryOpExpr { op: _, left, right } => {
            serde_json::json!({
                "$concat": [
                    lower_expr_to_json_from_ir(left, semantics, signal_like_names),
                    lower_expr_to_json_from_ir(right, semantics, signal_like_names)
                ]
            })
        }
        ExprIR::CallExpr { name, args } => {
            serde_json::json!({
                "$call": {
                    "name": name,
                    "args": args
                        .iter()
                        .map(|a| lower_expr_to_json_from_ir(a, semantics, signal_like_names))
                        .collect::<Vec<_>>()
                }
            })
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            ..
        } => {
            // Use underscore format that matches JS variable naming
            serde_json::json!({"$signal": format!("{}_{}", store_alias, field_name)})
        }
        ExprIR::MapExpr { entries } => {
            let mut map = serde_json::Map::new();
            for (k, v) in entries {
                let key = match k {
                    ExprIR::LiteralExpr { value } => match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => format!("{}", other),
                    },
                    ExprIR::IdentifierExpr { name, .. } => name.clone(),
                    _ => format!("{:?}", k),
                };
                map.insert(
                    key,
                    lower_expr_to_json_from_ir(v, semantics, signal_like_names),
                );
            }
            serde_json::Value::Object(map)
        }
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}
