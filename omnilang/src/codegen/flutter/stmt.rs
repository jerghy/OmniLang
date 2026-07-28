//! Statement and function generation for Flutter codegen.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;

// ═══════════════════════════════════════════════════════════════
// Statement generation (Flutter/Dart)
// ═══════════════════════════════════════════════════════════════

/// Flutter statement formatter — wraps mutations in `setState`.
pub(crate) struct FlutterStmtFormatter {
    pub(crate) expr: DartFormatExpr,
}

impl FormatStmt for FlutterStmtFormatter {
    type ExprFormatter = DartFormatExpr;

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
        let rhs = value
            .as_ref()
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| "null".into());
        if let Some(ref ty) = type_ann {
            let ann = super::super::dart::dart_type(ty.kind.as_str(), &ty.type_args);
            w.emit_fmt(format_args!("{ann} {name} = {rhs};"));
        } else {
            w.emit_fmt(format_args!("var {name} = {rhs};"));
        }
    }

    fn mutation_target_expr(&self, target: &str) -> String {
        target.to_string()
    }

    fn mutation_stmt_prefix(&self) -> Option<&str> {
        Some("setState(() {")
    }

    fn mutation_stmt_suffix(&self) -> Option<&str> {
        Some("});")
    }
}

pub(crate) fn generate_statement_flutter(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = FlutterStmtFormatter {
        expr: DartFormatExpr { event_param_name: None },
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
            s.push_str(&format!("{} ", ann));
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
