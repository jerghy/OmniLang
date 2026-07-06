//! Statement and function generation for Flutter codegen.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;

// ═══════════════════════════════════════════════════════════════
// Statement generation (Flutter/Dart)
// ═══════════════════════════════════════════════════════════════

/// Flutter statement formatter — wraps mutations in `setState`.
struct FlutterStmtFormatter {
    expr: DartFormatExpr,
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
            let ann = super::super::dart::dart_type(&ty.kind, &ty.type_args);
            w.emit_fmt(format_args!("{ann} {name} = {rhs};"));
        } else {
            w.emit_fmt(format_args!("var {name} = {rhs};"));
        }
    }

    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        let target = &intent.target.name;
        let amount = intent
            .amount
            .as_ref()
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| "0".into());
        w.emit("setState(() {");
        w.indent();
        match &intent.operation {
            MutationOp::DirectAssign => {
                w.emit_fmt(format_args!("{} = {};", target, amount));
            }
            MutationOp::Increment => {
                w.emit_fmt(format_args!("{} += {};", target, amount));
            }
            MutationOp::Decrement => {
                w.emit_fmt(format_args!("{} -= {};", target, amount));
            }
            MutationOp::Multiply => {
                w.emit_fmt(format_args!("{} *= {};", target, amount));
            }
            MutationOp::Divide => {
                w.emit_fmt(format_args!("{} /= {};", target, amount));
            }
            MutationOp::Modulo => {
                w.emit_fmt(format_args!("{} %= {};", target, amount));
            }
            other => {
                w.emit_fmt(format_args!(
                    "// TODO: mutation operation '{}' on '{}'",
                    other, target
                ));
            }
        }
        w.dedent();
        w.emit("});");
    }
}

pub(crate) fn generate_statement_flutter(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = FlutterStmtFormatter {
        expr: DartFormatExpr,
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
            let ann = super::super::dart::dart_type(&ty.kind, &ty.type_args);
            s.push_str(&format!("{} ", ann));
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type
    // Wrap in Future<T> for async functions
    let return_prefix = if func.is_async {
        match func.return_type {
            Some(ref rt) if rt.kind != "void" && rt.kind != "()" => {
                format!(
                    "Future<{}> ",
                    super::super::dart::dart_type(&rt.kind, &rt.type_args)
                )
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| rt.kind != "void" && rt.kind != "()")
            .map(|rt| {
                format!(
                    "{} ",
                    super::super::dart::dart_type(&rt.kind, &rt.type_args)
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
