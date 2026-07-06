//! Expression formatting for Python codegen.
//!
//! Implements the `FormatExpr` trait for Python, providing Python-specific
//! syntax for all expression types (ternary, closures, method calls, etc.).

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::types::{FormatType, PythonTypeFormatter};
use crate::codegen::{try_format_stdlib_call, JsonFormatConfig, Target, JSON_PYTHON};
use crate::ir::*;

/// Python expression formatter.
///
/// Key differences from JS/Rust families:
/// - `and`/`or` for boolean operators
/// - `{t} if {cond} else {e}` for ternary
/// - `lambda {params}: {body}` for closures
/// - `len(obj)` / `x in obj` for method calls
/// - `TypeName(fields)` for struct construction
pub(crate) struct PythonFormatExpr;

impl FormatExpr for PythonFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &JSON_PYTHON
    }

    fn codegen_target(&self) -> Target {
        Target::Python
    }

    fn null_fallback(&self) -> &str {
        "None"
    }

    fn textual_bool_ops(&self) -> bool {
        true
    }

    // ── Ternary ───────────────────────────────────────────

    fn format_ternary(&self, cond: &str, t: &str, e: &str) -> String {
        format!("{t} if {cond} else {e}")
    }

    // ── Closure ───────────────────────────────────────────

    fn format_closure(&self, params: &str, body: &str) -> String {
        if params.is_empty() {
            format!("lambda: {body}")
        } else {
            format!("lambda {params}: {body}")
        }
    }

    // ── Unary operators ───────────────────────────────────

    fn format_unary_op(&self, op: &str, operand: &str) -> String {
        match op {
            "not" => format!("not {operand}"),
            "neg" => format!("-{operand}"),
            _ => format!("{op}{operand}"),
        }
    }

    // ── Index access (no outer parens) ────────────────────

    fn format_index(&self, obj: &str, idx: &str) -> String {
        format!("{obj}[{idx}]")
    }

    // ── Struct / named-argument construction ──────────────

    fn format_struct_field(&self, key: &str, value: &str) -> String {
        format!("{key}={value}")
    }

    fn format_struct(&self, type_name: &str, fields: &str) -> String {
        format!("{type_name}({fields})")
    }

    // ── Call expression ───────────────────────────────────

    fn format_call(&self, name: &str, args: &[String]) -> String {
        if let Some(native) = try_format_stdlib_call(name, args, Target::Python) {
            return native;
        }
        format!("{}({})", name, args.join(", "))
    }

    // ── Method call ───────────────────────────────────────

    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        match method {
            "len" | "length" => format!("len({obj})"),
            "is_empty" => format!("len({obj}) == 0"),
            "push" => format!("{obj}.append({})", args.join(", ")),
            "pop" => format!("{obj}.pop()"),
            "contains" => format!("({} in {obj})", args.join(", ")),
            _ => format!("{obj}.{method}({})", args.join(", ")),
        }
    }

    // ── Match expression (if/elif/else chain) ─────────────

    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let mut parts = Vec::new();
        for (i, arm) in arms.iter().enumerate() {
            let body = self.format_block_as_expr(&arm.body);
            match &arm.pattern.kind {
                MatchPatternKindIR::Variant { name, .. } => {
                    let kw = if i == 0 { "if" } else { "elif" };
                    parts.push(format!(
                        "({body}) {kw} {matched}.get(\"_variant\") == \"{name}\""
                    ));
                }
                MatchPatternKindIR::Wildcard => {
                    parts.push(format!("({body})"));
                }
                _ => {}
            }
        }
        if parts.is_empty() {
            "None".into()
        } else {
            parts.join(" else ")
        }
    }
}

/// Format a TypeIR as a Python type annotation string.
pub(super) fn format_type_ir(ty: &TypeIR) -> String {
    PythonTypeFormatter.format_type(&ty.kind, &ty.type_args)
}
