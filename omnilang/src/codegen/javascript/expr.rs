//! JavaScript/TypeScript expression formatting utilities.
//!
//! Formats IR expressions, statements (as expressions), JSON values,
//! and binary operators into JavaScript/TypeScript source strings.

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::{
    format_json_value, sanitize_js_ident, try_format_stdlib_call, JsonFormatConfig, Target, JSON_JS,
};
use crate::ir::*;

/// JavaScript/TypeScript expression formatter.
///
/// Uses `===` strict equality, `?.` optional chaining, `undefined` fallback.
/// In TS mode, stdlib calls use `Target::TypeScript` (for type-aware codegen).
pub(crate) struct JavaScriptFormatExpr {
    pub ts_mode: bool,
}

impl FormatExpr for JavaScriptFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &JSON_JS
    }

    fn strict_eq(&self) -> bool {
        true
    }

    fn null_fallback(&self) -> &str {
        "undefined"
    }

    fn ts_mode(&self) -> bool {
        self.ts_mode
    }

    fn format_identifier(&self, name: &str) -> String {
        sanitize_js_ident(name)
    }

    fn format_unary_op(&self, op: &str, operand: &str) -> String {
        match op {
            "not" => format!("!{operand}"),
            "neg" => format!("-{operand}"),
            "await" => format!("await {operand}"),
            _ => format!("{op}{operand}"),
        }
    }

    fn format_optional_chain(&self, object: &str, field: &str) -> String {
        format!("{object}?.{field}")
    }

    fn format_closure(&self, params: &str, body: &str) -> String {
        format!("(({params}) => {{ return {body}; }})")
    }

    fn format_call(&self, name: &str, args: &[String]) -> String {
        let target = if self.ts_mode {
            Target::TypeScript
        } else {
            Target::JavaScript
        };
        if let Some(native) = try_format_stdlib_call(name, args, target) {
            return native;
        }
        match name {
            "println" => format!("console.log({})", args.join(", ")),
            "print" => format!("process.stdout.write(String({}))", args.join(", ")),
            _ => format!("{}({})", name, args.join(", ")),
        }
    }

    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        let stmts: Vec<String> = statements
            .iter()
            .map(|s| format!("  {};", format_stmt_ir(s, self)))
            .collect();
        let tail_str = self.format_expr(tail);
        format!(
            "(() => {{\n{}\n  return {};\n}})()",
            stmts.join("\n"),
            tail_str
        )
    }

    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let cases: Vec<String> = arms
            .iter()
            .map(|arm| {
                let body_expr = self.format_block_as_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, .. } => {
                        format!(
                            "  if (_v._variant === \"{}\") {{ return {}; }}",
                            name, body_expr
                        )
                    }
                    MatchPatternKindIR::Wildcard => {
                        format!("  return {};", body_expr)
                    }
                    MatchPatternKindIR::Literal { value } => {
                        let v = format_json_value(value, &JSON_JS);
                        format!("  if (_v._variant === {}) {{ return {}; }}", v, body_expr)
                    }
                }
            })
            .collect();
        format!(
            "(() => {{\n  const _v = {};\n{}\n}})()",
            matched,
            cases.join("\n  ")
        )
    }
}

/// Legacy wrapper for backward compatibility (preserves `ts_mode` parameter).
pub(super) fn format_expr_ir(expr: &ExprIR, ts_mode: bool) -> String {
    JavaScriptFormatExpr { ts_mode }.format_expr(expr)
}

/// Format a block's trailing expression as a JS value (for match arm bodies).
pub(super) fn format_block_as_js_expr(block: &BlockIR, ts_mode: bool) -> String {
    JavaScriptFormatExpr { ts_mode }.format_block_as_expr(block)
}

/// Format a single statement IR node as a JS expression string (for block exprs).
fn format_stmt_ir(stmt: &StatementIR, fmt: &impl FormatExpr) -> String {
    match stmt {
        StatementIR::LetStmtIR { name, value, .. } => {
            let safe_name = fmt.format_identifier(name);
            let val = value
                .as_ref()
                .map(|e| fmt.format_expr(e))
                .unwrap_or_else(|| fmt.null_fallback().into());
            format!("let {} = {}", safe_name, val)
        }
        StatementIR::ExprStmtIR { expr } => fmt.format_expr(expr),
        _ => "/* stmt */".into(),
    }
}
