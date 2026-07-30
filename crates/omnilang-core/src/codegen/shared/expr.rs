//! FormatExpr trait — eliminates expression formatting duplication across 8 backends.
//!
//! Provides a single `format_expr` dispatch method backed by overrideable helpers
//! for each `ExprIR` variant. Three concrete family implementations are provided:
//!
//! | Struct | Family | Key characteristics |
//! |--------|--------|-------------------|
//! | `JsFormatExpr` | JS (React, Vue, JavaScript) | `===`, `?.`, `undefined` fallback |
//! | `DartFormatExpr` | Dart (Dart, Flutter) | `==`, `.`, `null` fallback |
//! | `RustFormatExpr` | Rust (Rust, Dioxus) | `if/else`, `vec![]`, `.await`, `None` fallback |

use crate::codegen::{
    binary_op_to_str, format_block_as_expr, format_json_value, try_format_stdlib_call,
    JsonFormatConfig, Target,
};
use crate::ir::*;

// ═══════════════════════════════════════════════════════════
// Trait definition
// ═══════════════════════════════════════════════════════════

/// Trait for target-language expression formatting.
///
/// Default implementations produce JS-family syntax (React/Vue style).
/// Backends override only the methods that differ.
pub trait FormatExpr {
    /// JSON literal formatting config for this language (REQUIRED).
    fn json_config(&self) -> &JsonFormatConfig;

    /// Code generation target for this backend (REQUIRED).
    /// Used by `format_call` to route stdlib calls through the registry.
    fn codegen_target(&self) -> Target;

    /// Use strict equality (`===`/`!==`) — true for JS family.
    fn strict_eq(&self) -> bool {
        false
    }

    /// Fallback value for null/undefined expression positions.
    fn null_fallback(&self) -> &str {
        "null"
    }

    /// Format an identifier name (e.g., sanitize JS reserved words).
    fn format_identifier(&self, name: &str) -> String {
        name.to_string()
    }

    /// Whether TypeScript-style type annotations should be emitted.
    fn ts_mode(&self) -> bool {
        false
    }

    /// Use textual boolean operators (`and`/`or`) — true for Python.
    fn textual_bool_ops(&self) -> bool {
        false
    }

    /// When set, `FieldAccessExpr` on this identifier is redirected to a bare field name.
    /// Used by InlineFn handlers where event field extractions are bound as local variables.
    fn event_param_name(&self) -> Option<&str> {
        None
    }

    // ── Overridable helpers ──────────────────────────────

    /// Ternary / conditional expression.
    fn format_ternary(&self, cond: &str, t: &str, e: &str) -> String {
        format!("{cond} ? {t} : {e}")
    }

    /// Array literal.
    fn format_array(&self, items: &str) -> String {
        format!("[{items}]")
    }

    /// Await expression.
    fn format_await(&self, expr: &str) -> String {
        format!("await {expr}")
    }

    /// Index access: `obj[idx]`.
    fn format_index(&self, obj: &str, idx: &str) -> String {
        format!("({obj})[{idx}]")
    }

    /// Closure / lambda expression.
    fn format_closure(&self, params: &str, body: &str) -> String {
        if params.is_empty() {
            format!("(() => {body})")
        } else {
            format!("(({params}) => {body})")
        }
    }

    /// Struct literal wrapper (braces + content).
    /// `type_name` is the struct type name (used by Python: `TypeName(fields)`).
    fn format_struct(&self, _type_name: &str, fields: &str) -> String {
        format!("{{ {fields} }}")
    }

    /// Format a single struct field key-value pair.
    /// Keys that are already JSON string literals (start with `"`) are used as-is
    /// to avoid double-escaping (e.g., `""id""`).
    fn format_struct_field(&self, key: &str, value: &str) -> String {
        if key.starts_with('"') && key.ends_with('"') {
            format!("{key}: {value}")
        } else {
            format!("\"{key}\": {value}")
        }
    }

    /// Function call expression.
    fn format_call(&self, name: &str, args: &[String]) -> String {
        if let Some(native) = try_format_stdlib_call(name, args, self.codegen_target()) {
            return native;
        }
        format!("{}({})", name, args.join(", "))
    }

    /// Method call expression.
    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        format!("{obj}.{method}({})", args.join(", "))
    }

    /// Unary operator expression.
    fn format_unary_op(&self, op: &str, operand: &str) -> String {
        let op_str = match op {
            "not" => "!",
            "neg" => "-",
            _ => op,
        };
        format!("{op_str}{operand}")
    }

    /// Optional chain access: `obj?.field` (JS) vs `obj.field` (others).
    fn format_optional_chain(&self, object: &str, field: &str) -> String {
        format!("{object}.{field}")
    }

    /// Block expression (ExprIR::BlockExpr). Default: just the tail expression.
    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        debug_assert!(
            statements.is_empty(),
            "format_block_expr with non-empty statements — backend must override"
        );
        self.format_expr(tail)
    }

    /// Variant literal (tagged union constructor).
    fn format_variant_literal(
        &self,
        _union_type: &str,
        variant: &str,
        fields: &[(String, ExprIR)],
    ) -> String {
        let mut entries: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("\"{k}\": {}", self.format_expr(v)))
            .collect();
        entries.insert(0, format!("\"_variant\": \"{variant}\""));
        format!("{{{}}}", entries.join(", "))
    }

    /// Match expression (pattern matching dispatch).
    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let cases: Vec<String> = arms
            .iter()
            .map(|arm| {
                let body = self.format_block_as_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, .. } => {
                        format!("if (_v._variant === \"{name}\") {{ return {body}; }}")
                    }
                    MatchPatternKindIR::Wildcard => {
                        format!("return {body};")
                    }
                    MatchPatternKindIR::Literal { value } => {
                        let v = format_json_value(value, self.json_config());
                        format!("if (_v._variant === {v}) {{ return {body}; }}")
                    }
                }
            })
            .collect();
        format!("(() => {{ const _v = {matched}; {} }})()", cases.join(" "))
    }

    /// Format a `BlockIR`'s last statement as an expression (for match arms etc.).
    fn format_block_as_expr(&self, block: &BlockIR) -> String {
        let fmt = |e: &ExprIR| self.format_expr(e);
        format_block_as_expr(block, &fmt, self.null_fallback())
    }

    // ── Main dispatch ────────────────────────────────────

    /// Format any `ExprIR` node according to this language's syntax.
    fn format_expr(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => format_json_value(value, self.json_config()),
            ExprIR::IdentifierExpr { name, .. } => self.format_identifier(name),
            ExprIR::FieldAccessExpr { object, field } => {
                // Redirect: if object is the event param and field was extracted locally,
                // emit just the field name (not `param.field`).
                if let Some(param) = self.event_param_name() {
                    if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                        if name == param {
                            return self.format_identifier(field);
                        }
                    }
                }
                format!("{}.{field}", self.format_expr(object))
            }
            ExprIR::OptionalChainExpr { object, field, .. } => {
                self.format_optional_chain(&self.format_expr(object), field)
            }
            ExprIR::BinaryOpExpr { op, left, right } => {
                let op_str = binary_op_to_str(op, self.strict_eq(), self.textual_bool_ops());
                format!(
                    "{} {op_str} {}",
                    self.format_expr(left),
                    self.format_expr(right)
                )
            }
            ExprIR::UnaryOpExpr { op, operand } => {
                self.format_unary_op(op.as_str(), &self.format_expr(operand))
            }
            ExprIR::TernaryIR {
                cond,
                then_expr,
                else_expr,
            } => self.format_ternary(
                &self.format_expr(cond),
                &self.format_expr(then_expr),
                &self.format_expr(else_expr),
            ),
            ExprIR::CallExpr { name, args } => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.format_expr(a)).collect();
                self.format_call(name, &arg_strs)
            }
            ExprIR::StoreFieldAccessExpr {
                store_alias,
                field_name,
                call_args,
            } => {
                let args: Vec<String> = call_args.iter().map(|a| self.format_expr(a)).collect();
                if args.is_empty() {
                    // Field/getter access — no parens needed (Dart/Rust property syntax)
                    format!("{store_alias}.{field_name}")
                } else {
                    // Action call with arguments — emit parens with args
                    format!("{store_alias}.{field_name}({})", args.join(", "))
                }
            }
            ExprIR::ArrayExpr { elements } => {
                let items: Vec<String> = elements.iter().map(|e| self.format_expr(e)).collect();
                self.format_array(&items.join(", "))
            }
            ExprIR::SpreadExpr { operand } => format!("...{}", self.format_expr(operand)),
            ExprIR::StructExpr { type_name, fields } => {
                let f: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| self.format_struct_field(k, &self.format_expr(v)))
                    .collect();
                self.format_struct(type_name, &f.join(", "))
            }
            ExprIR::VariantLiteral {
                union_type,
                variant,
                fields,
            } => self.format_variant_literal(union_type, variant, fields),
            ExprIR::MethodCallExpr {
                object,
                method,
                args,
                ..
            } => {
                let obj = self.format_expr(object);
                let arg_strs: Vec<String> = args.iter().map(|a| self.format_expr(a)).collect();
                self.format_method_call(&obj, method, &arg_strs)
            }
            ExprIR::BlockExpr { statements, tail } => self.format_block_expr(statements, tail),
            ExprIR::IndexExpr { object, index } => {
                self.format_index(&self.format_expr(object), &self.format_expr(index))
            }
            ExprIR::AwaitExpr { operand } => self.format_await(&self.format_expr(operand)),
            ExprIR::ClosureExpr { params, body } => {
                let p = params.join(", ");
                let b = self.format_expr(body);
                self.format_closure(&p, &b)
            }
            ExprIR::MatchExpr { matched, arms } => {
                self.format_match(&self.format_expr(matched), arms)
            }
            ExprIR::MapExpr { entries } => {
                let fields: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| {
                        let key = self.format_expr(k);
                        let val = self.format_expr(v);
                        self.format_struct_field(&key, &val)
                    })
                    .collect();
                // Use empty type_name since MapExpr has no struct type
                self.format_struct("", &fields.join(", "))
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// JS family: React, Vue, JavaScript, TypeScript
// ═══════════════════════════════════════════════════════════

/// JS-family expression formatter.
///
/// Defaults: `===`, `?.`, `undefined` fallback.
pub struct JsFormatExpr;

impl FormatExpr for JsFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &crate::codegen::JSON_JS
    }

    fn codegen_target(&self) -> Target {
        Target::JavaScript
    }

    fn strict_eq(&self) -> bool {
        true
    }

    fn null_fallback(&self) -> &str {
        "undefined"
    }

    fn format_block_expr(&self, _statements: &[StatementIR], tail: &ExprIR) -> String {
        // Block statements not supported in expression context — drop them.
        self.format_expr(tail)
    }
}

// ═══════════════════════════════════════════════════════════
// Dart family: Dart, Flutter
// ═══════════════════════════════════════════════════════════

/// Dart-family expression formatter.
///
/// Defaults: `==`, `.`, `null` fallback, Dart string/number config.
pub struct DartFormatExpr {
    /// When set in an InlineFn handler context, FieldAccess on this identifier
    /// emits the bare field name (local variable) instead of `param.field`.
    pub event_param_name: Option<String>,
}

impl FormatExpr for DartFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &crate::codegen::JSON_DART
    }

    fn codegen_target(&self) -> Target {
        Target::Dart
    }

    fn event_param_name(&self) -> Option<&str> {
        self.event_param_name.as_deref()
    }

    // Dart uses `UnionType.variant(unquotedKey: value)` syntax.
    fn format_variant_literal(
        &self,
        union_type: &str,
        variant: &str,
        fields: &[(String, ExprIR)],
    ) -> String {
        let entries: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("{}: {}", k, self.format_expr(v)))
            .collect();
        format!("{}.{}({})", union_type, variant, entries.join(", "))
    }

    // Dart uses `switch/case` on `_variant` inside an IIFE.
    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let cases: Vec<String> = arms
            .iter()
            .map(|arm| {
                let body = self.format_block_as_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, .. } => {
                        format!("  case \"{name}\": return {body};")
                    }
                    MatchPatternKindIR::Wildcard => {
                        format!("  default: return {body};")
                    }
                    _ => String::new(),
                }
            })
            .collect();
        format!(
            "(() {{\nswitch ({matched}._variant) {{\n{}\n}}\n}})()",
            cases.join("\n")
        )
    }

    // Dart uses `{key: value}` without spaces inside braces.
    fn format_struct(&self, _type_name: &str, fields: &str) -> String {
        format!("{{{}}}", fields)
    }

    fn format_block_expr(&self, _statements: &[StatementIR], tail: &ExprIR) -> String {
        // Block statements not supported in expression context — drop them.
        self.format_expr(tail)
    }
}

// ═══════════════════════════════════════════════════════════
// Rust family: Rust, Dioxus
// ═══════════════════════════════════════════════════════════

/// Rust-family expression formatter.
///
/// Defaults: `if/else` ternary, `vec![]` arrays, `.await`, `None` fallback.
/// When `move_closures` is true (Dioxus), closures emit `move ||` syntax.
pub struct RustFormatExpr {
    pub move_closures: bool,
    /// When set in an InlineFn handler context, FieldAccess on this identifier
    /// emits the bare field name (local variable) instead of `param.field`.
    pub event_param_name: Option<String>,
}

impl FormatExpr for RustFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &crate::codegen::JSON_RUST
    }

    fn codegen_target(&self) -> Target {
        Target::Rust
    }

    fn null_fallback(&self) -> &str {
        "None"
    }

    fn event_param_name(&self) -> Option<&str> {
        self.event_param_name.as_deref()
    }

    fn format_ternary(&self, cond: &str, t: &str, e: &str) -> String {
        format!("if {cond} {{ {t} }} else {{ {e} }}")
    }

    fn format_array(&self, items: &str) -> String {
        format!("vec![{items}]")
    }

    fn format_await(&self, expr: &str) -> String {
        format!("{expr}.await")
    }

    fn format_index(&self, obj: &str, idx: &str) -> String {
        format!("({obj})[{idx} as usize]")
    }

    fn format_closure(&self, params: &str, body: &str) -> String {
        let move_kw = if self.move_closures { "move " } else { "" };
        if params.is_empty() {
            format!("({move_kw}|| {body})")
        } else {
            format!("({move_kw}|{params}| {body})")
        }
    }

    fn format_struct(&self, _type_name: &str, fields: &str) -> String {
        if fields.is_empty() {
            "{}".to_string()
        } else {
            format!("{{ {fields} }}")
        }
    }

    fn format_struct_field(&self, key: &str, value: &str) -> String {
        format!("{key}: {value}")
    }

    fn format_variant_literal(
        &self,
        union_type: &str,
        variant: &str,
        fields: &[(String, ExprIR)],
    ) -> String {
        if fields.is_empty() {
            format!("{union_type}::{variant}")
        } else {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{k}: {}", self.format_expr(v)))
                .collect();
            format!("{union_type}::{variant} {{ {} }}", entries.join(", "))
        }
    }

    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let cases: Vec<String> = arms
            .iter()
            .map(|arm| {
                let body = self.format_block_as_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, .. } => {
                        let type_prefix = arm.union_type.as_deref().unwrap_or("UnionType");
                        format!("    {type_prefix}::{name} {{ .. }} => {body},")
                    }
                    MatchPatternKindIR::Wildcard => {
                        format!("    _ => {body},")
                    }
                    _ => String::new(),
                }
            })
            .collect();
        format!("match &{matched} {{\n{}\n}}", cases.join("\n"))
    }

    fn format_block_expr(&self, _statements: &[StatementIR], tail: &ExprIR) -> String {
        // Block statements not supported in expression context — drop them.
        self.format_expr(tail)
    }
}
