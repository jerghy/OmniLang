//! Statement and function generation for Flutter codegen.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashSet;
use std::fmt::Write;

// ═══════════════════════════════════════════════════════════════
// Statement generation (Flutter/Dart)
// ═══════════════════════════════════════════════════════════════

/// Shared Dart-family `let` statement formatting.
fn format_dart_let_stmt<F: FormatExpr>(
    expr: &F,
    w: &mut CodeWriter,
    name: &str,
    value: &Option<ExprIR>,
    type_ann: &Option<TypeIR>,
) {
    let rhs = value
        .as_ref()
        .map(|v| expr.format_expr(v))
        .unwrap_or_else(|| "null".into());
    if let Some(ref ty) = type_ann {
        let ann = super::super::dart::dart_type(ty.kind.as_str(), &ty.type_args);
        w.emit_fmt(format_args!("{ann} {name} = {rhs};"));
    } else {
        w.emit_fmt(format_args!("var {name} = {rhs};"));
    }
}

/// Component-mode Flutter statement formatter — wraps mutations in `setState`.
pub(crate) struct FlutterStmtFormatter<F: FormatExpr> {
    pub(crate) expr: F,
}

impl<F: FormatExpr> FormatStmt for FlutterStmtFormatter<F> {
    type ExprFormatter = F;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "var"
    }

    fn for_in_uses_of(&self) -> bool {
        false
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
    ) {
        format_dart_let_stmt(&self.expr, w, name, value, type_ann);
    }

    /// Component-local mutations target the plain field name.
    fn mutation_target_expr(&self, target: &str) -> String {
        target.to_string()
    }

    /// Component-local mutations are wrapped in setState.
    fn mutation_stmt_prefix(&self) -> Option<&str> {
        Some("setState(() {")
    }

    fn mutation_stmt_suffix(&self) -> Option<&str> {
        Some("});")
    }
}

/// Store-action-mode Flutter statement formatter (Riverpod 3.0 Notifier
/// methods). Mutations use the `state = state.copyWith(...)` pattern with no
/// setState wrapping; bare signal/getter identifiers are rewritten to
/// `state.<name>` by `StoreActionExprFormatter`.
pub(crate) struct StoreActionStmtFormatter<'a> {
    pub(crate) expr: super::expr::StoreActionExprFormatter<'a>,
}

impl<'a> FormatStmt for StoreActionStmtFormatter<'a> {
    type ExprFormatter = super::expr::StoreActionExprFormatter<'a>;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "var"
    }

    fn for_in_uses_of(&self) -> bool {
        false
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
    ) {
        format_dart_let_stmt(&self.expr, w, name, value, type_ann);
    }

    /// Riverpod 3.0 store mutations: `state = state.copyWith(field: value)`.
    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        let fmt_expr = |e: &ExprIR| self.expr.format_expr(e);
        match intent {
            MutationIntent::DirectAssign { target, value, .. } => {
                let v = value
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "null".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: {});",
                    target.name, v
                ));
            }
            MutationIntent::Increment { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "1".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} + {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Decrement { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "1".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} - {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Multiply { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} * {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Divide { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} / {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Modulo { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} % {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Push { target, item } => {
                let it = item
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "null".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: [...state.{}]..add({}));",
                    target.name, target.name, it
                ));
            }
            MutationIntent::Pop { target } => {
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{}.sublist(0, state.{}.length - 1));",
                    target.name, target.name, target.name
                ));
            }
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
}

pub(crate) fn generate_statement_flutter(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = FlutterStmtFormatter {
        expr: super::r#ref::FlutterFormatExpr::new(None),
    };
    fmt.format_stmt(w, stmt);
}

/// Generate statements inside a store action body (Riverpod 3.0 Notifier
/// method — `state = state.copyWith(...)` mutations with bare signal
/// identifiers rewritten to `state.<name>`).
pub(crate) fn generate_store_action_stmt(w: &mut CodeWriter, stmt: &StatementIR, store: &StoreIR) {
    let store_names: FxHashSet<String> = store
        .signals
        .keys()
        .chain(store.getters.keys())
        .cloned()
        .collect();
    let fmt = StoreActionStmtFormatter {
        expr: super::expr::StoreActionExprFormatter {
            inner: DartFormatExpr {
                event_param_name: None,
            },
            store_names: &store_names,
        },
    };
    fmt.format_stmt(w, stmt);
}

/// Generate a function as a method on the State class.
pub(crate) fn generate_function_flutter(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = super::super::dart::dart_type(ty.kind.as_str(), &ty.type_args);
            let _ = write!(s, "{} ", ann);
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type
    // Wrap in Future<T> for async functions
    let return_prefix = if func.is_async {
        match func.return_type {
            Some(ref rt) if !matches!(rt.kind, TypeKind::Void) => {
                format!(
                    "Future<{}> ",
                    super::super::dart::dart_type(rt.kind.as_str(), &rt.type_args)
                )
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| !matches!(rt.kind, TypeKind::Void))
            .map(|rt| {
                format!(
                    "{} ",
                    super::super::dart::dart_type(rt.kind.as_str(), &rt.type_args)
                )
            })
            .unwrap_or_default()
    };

    let async_kw = if func.is_async { "async " } else { "" };
    let all_params = param_strs.join(", ");
    w.emit_fmt(format_args!(
        "{}{}({}) {}{{",
        return_prefix, func.name, all_params, async_kw
    ));
    w.indent();

    for stmt in &func.body.statements {
        generate_statement_flutter(w, stmt);
    }

    w.dedent();
    w.emit("}");
}
