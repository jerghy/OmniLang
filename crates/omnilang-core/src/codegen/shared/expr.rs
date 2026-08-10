//! FormatExpr trait — eliminates expression formatting duplication across 8 backends.
//!
//! Provides a single `format_expr` dispatch method backed by overrideable helpers
//! for each `ExprIR` variant. Three concrete family implementations are provided:
//!
//! | Struct | Family | Key characteristics |
//! |--------|--------|-------------------|
//! | `JsFormatExpr` | JS (React, Vue, JavaScript) | `===`, `?.`, `undefined` fallback |
//! | `DartFormatExpr` | Dart (Dart, Flutter) | `==`, `?.`, `null` fallback |
//! | `RustFormatExpr` | Rust (Rust, Dioxus) | `if/else`, `vec![]`, `.await`, `None` fallback |

use crate::codegen::{
    binary_op_to_str, format_block_as_expr, format_json_value, try_format_stdlib_call,
    JsonFormatConfig, Target,
};
use crate::error::{Diagnostic, DiagnosticKind, Severity};
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

    /// Closure-body pre-processing hook — the body's IR is available so
    /// backends can shape the body before `format_closure` renders it.
    /// Default: pass through. Dart-family narrows int-literal arithmetic
    /// closure bodies (`|s| s + 5` → `(s + 5) as int`) so the closure's
    /// result is statically int — the language's closures are Any-typed,
    /// and an un-narrowed `int + dynamic` result is `num` in Dart
    /// (invalid_assignment at the call site, Task 5.8a DataTable).
    fn format_closure_body(&self, _body: &ExprIR, body_str: &str) -> String {
        body_str.to_string()
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

    /// Format a struct-literal field for a TYPED struct (a real `Name { ... }`
    /// literal, not a map). Default: same as [`format_struct_field`] (JS
    /// backends render structs as map literals). Dart-family backends
    /// override to emit NAMED-CONSTRUCTOR args — `name: value` unquoted
    /// (Task 5.8: `StatisticsData { name: "demo" }` → `StatisticsData(name: 'demo')`).
    fn format_struct_field_named(&self, key: &str, value: &str) -> String {
        self.format_struct_field(key, value)
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

    /// Method call WITH IR context (object/args shapes and static type
    /// annotations still available). The default delegates to the
    /// string-level formatters; backends whose method tables need shape
    /// info (Dart `get`/`contains`/`concat` — Task 5.8a) override this.
    fn format_method_call_with_ir(
        &self,
        _object: &ExprIR,
        method: &str,
        _args: &[ExprIR],
        is_optional: bool,
        obj_str: &str,
        arg_strs: &[String],
    ) -> String {
        if is_optional {
            self.format_optional_method_call(obj_str, method, arg_strs)
        } else {
            self.format_method_call(obj_str, method, arg_strs)
        }
    }

    /// Optional-chain method call (`obj?.method(args)`).
    fn format_optional_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        format!("{obj}?.{method}({})", args.join(", "))
    }

    /// Binary operator expression — hook for backends that need operand-level
    /// rewriting (e.g. Dart string `+` coercion of non-String operands).
    fn format_binary_op(
        &self,
        op: &str,
        _left: &ExprIR,
        _right: &ExprIR,
        left_str: &str,
        right_str: &str,
    ) -> String {
        format!("{left_str} {op} {right_str}")
    }

    /// Field access expression: `obj.field`.
    /// Separate hook so backends can redirect store field reads (`Store.x`)
    /// to slice-selector locals (React T3.3) without touching the dispatch.
    fn format_field_access(&self, obj: &str, field: &str) -> String {
        format!("{}.{}", obj, field)
    }

    /// Optional hook for FIELD ACCESS ON A REF-OP RESULT (`rect.x` where
    /// `rect = refBox.measure()`). Default: no rewrite — backends that model
    /// measure results with different property names (Flutter's Offset has
    /// `dx`/`dy`, DOMRect parity wants `x`/`y`) override this.
    fn format_ref_op_field_access(&self, _object: &ExprIR, _field: &str) -> Option<String> {
        None
    }

    /// Optional hook for LANGUAGE-LEVEL BUILTIN FIELD ACCESS — a field read
    /// on a semantic builtin object (Task 5: `form.valid` / `form.errors`
    /// / `form.submitting`). Backends whose runtime model maps these to
    /// different shapes (React: signal accessors; Flutter: plain locals or
    /// map lookups) override this to emit the mapped accessor. Default: no
    /// rewrite, so the object is formatted by the ordinary path.
    /// Fired in BOTH `FieldAccessExpr` and `OptionalChainExpr` arms.
    fn format_special_field_access(&self, _object: &ExprIR, _field: &str) -> Option<String> {
        None
    }

    /// Store field access: read (`Store.items`) or action call
    /// (`Store.add(args)`). Backends that subscribe per field (React T3.3)
    /// override this to emit the slice local instead of the qualified name.
    fn format_store_field_access(
        &self,
        store_alias: &str,
        field_name: &str,
        args: &[String],
    ) -> String {
        if args.is_empty() {
            format!("{store_alias}.{field_name}")
        } else {
            format!("{store_alias}.{field_name}({})", args.join(", "))
        }
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

    /// Optional chain access: `obj?.field` (JS/Dart) vs `obj.field` (Rust/Python).
    /// The default is JS-family syntax; Rust and Python override to plain `.`.
    fn format_optional_chain(&self, object: &str, field: &str) -> String {
        format!("{object}?.{field}")
    }

    /// Block expression (ExprIR::BlockExpr). Default: just the tail expression.
    ///
    /// Statement-bearing block expressions are dropped by targets that
    /// cannot represent statements in expression position. Debug builds
    /// assert (backends must override); release builds log a warning so the
    /// drop is never fully invisible. Backends with a codegen-diagnostics
    /// channel (Python, Vue) hard-diagnose instead of reaching this default.
    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        if !statements.is_empty() {
            log::warn!(
                "format_block_expr: dropping {} statement(s) in expression position \
                 on target {:?} — backend must override format_block_expr",
                statements.len(),
                self.codegen_target()
            );
            debug_assert!(
                false,
                "format_block_expr with non-empty statements — backend must override"
            );
        }
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
                        // Literal patterns compare the subject value itself —
                        // primitives carry no `_variant` field.
                        let v = format_json_value(value, self.json_config());
                        format!("if (_v === {v}) {{ return {body}; }}")
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

    /// Format a semantic ref operation (`ref.focus()`, `ref.scrollTo(...)`, …).
    ///
    /// The default is a placeholder comment — backends that support ref ops
    /// (React) override this with native DOM API calls. `ref_name` is the
    /// declared ref name (e.g. `box` for `box.focus()`); backends append the
    /// runtime ref accessor (e.g. `boxRef.current`).
    fn format_ref_op(&self, ref_name: &str, op: &RefOpIR) -> String {
        format!("/* RefOp::{op:?} on {ref_name} */")
    }

    /// Format an array language method (`items.filter(...)`, `items.map(...)`,
    /// `items.count(...)`) — the cross-target mapping table for
    /// `ExprIR::ArrayMethod` (design doc 域 8).
    ///
    /// JS family (React/Vue/JavaScript/TypeScript): `filter`/`map` map
    /// verbatim (native JS array methods); `count` expands to
    /// `subject.filter(pred).length` (no `count` method exists on JS arrays).
    /// Dart family (Dart/Flutter): arrays are `List`s, whose functional
    /// methods are `Iterable`-returning — `filter` → `where(...).toList()`,
    /// `map` → `map(...).toList()`, `count` → `where(...).length` (no
    /// `toList` needed — length is available on the lazy Iterable).
    /// Unmapped targets (Python/Rust/Dioxus) are floor-gated: the compile
    /// pipeline hard-diagnoses them with E_CODEGEN
    /// (`validate_array_method_floor`) BEFORE formatting; the sentinel
    /// below keeps DIRECT formatter use loud too — a verbatim
    /// `items.filter(..)` would be a nonexistent stdlib call (compiler
    /// MEDIUM: runtime failure with zero diagnostics).
    fn format_array_method(
        &self,
        method: &ArrayMethodIR,
        subject: &str,
        predicate: &str,
    ) -> String {
        match self.codegen_target() {
            Target::JavaScript | Target::TypeScript | Target::React | Target::Vue => match method {
                ArrayMethodIR::Filter => format!("{subject}.filter({predicate})"),
                ArrayMethodIR::Map => format!("{subject}.map({predicate})"),
                ArrayMethodIR::Count => format!("{subject}.filter({predicate}).length"),
            },
            Target::Dart | Target::Flutter => match method {
                ArrayMethodIR::Filter => format!("{subject}.where({predicate}).toList()"),
                ArrayMethodIR::Map => format!("{subject}.map({predicate}).toList()"),
                ArrayMethodIR::Count => format!("{subject}.where({predicate}).length"),
            },
            _ => {
                let name = match method {
                    ArrayMethodIR::Filter => "filter",
                    ArrayMethodIR::Map => "map",
                    ArrayMethodIR::Count => "count",
                };
                if self.codegen_target() == Target::OmniView {
                    // Defensive fallback: the OmniView generator routes
                    // ArrayMethod through its own machinery (omniview/expr.rs)
                    // and never reaches this trait default — keep the
                    // historical verbatim form for the JS runtime.
                    return self.format_method_call(subject, name, &[predicate.to_string()]);
                }
                let hint = match method {
                    ArrayMethodIR::Filter => {
                        "rewrite with an explicit loop (stdlib 'array_filter' is deprecated)"
                    }
                    ArrayMethodIR::Map => {
                        "rewrite with an explicit loop (stdlib 'array_map' is deprecated)"
                    }
                    ArrayMethodIR::Count => {
                        "rewrite with an explicit loop (there is no stdlib counterpart)"
                    }
                };
                format!(
                    "compile_error!(\"language array method '{name}' is not supported on this target — {hint}\")"
                )
            }
        }
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
                // Ref-op result shape rewrite (measure-position parity).
                if let Some(mapped) = self.format_ref_op_field_access(object, field) {
                    return mapped;
                }
                // Language-level builtin field access (`form.valid` — Task 5).
                if let Some(mapped) = self.format_special_field_access(object, field) {
                    return mapped;
                }
                self.format_field_access(&self.format_expr(object), field)
            }
            ExprIR::OptionalChainExpr { object, field, .. } => {
                if let Some(mapped) = self.format_special_field_access(object, field) {
                    return mapped;
                }
                self.format_optional_chain(&self.format_expr(object), field)
            }
            ExprIR::BinaryOpExpr { op, left, right } => {
                let op_str = binary_op_to_str(op, self.strict_eq(), self.textual_bool_ops());
                let left_str = self.format_expr(left);
                let right_str = self.format_expr(right);
                self.format_binary_op(op_str, left, right, &left_str, &right_str)
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
                self.format_store_field_access(store_alias, field_name, &args)
            }
            ExprIR::ArrayExpr { elements } => {
                let items: Vec<String> = elements.iter().map(|e| self.format_expr(e)).collect();
                self.format_array(&items.join(", "))
            }
            ExprIR::SpreadExpr { operand } => format!("...{}", self.format_expr(operand)),
            ExprIR::StructExpr { type_name, fields } => {
                let f: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| {
                        if type_name.is_empty() {
                            self.format_struct_field(k, &self.format_expr(v))
                        } else {
                            self.format_struct_field_named(k, &self.format_expr(v))
                        }
                    })
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
                is_optional,
            } => {
                let obj = self.format_expr(object);
                let arg_strs: Vec<String> = args.iter().map(|a| self.format_expr(a)).collect();
                self.format_method_call_with_ir(object, method, args, *is_optional, &obj, &arg_strs)
            }
            ExprIR::BlockExpr { statements, tail } => self.format_block_expr(statements, tail),
            ExprIR::IndexExpr { object, index } => {
                self.format_index(&self.format_expr(object), &self.format_expr(index))
            }
            ExprIR::AwaitExpr { operand } => self.format_await(&self.format_expr(operand)),
            ExprIR::ClosureExpr { params, body } => {
                let p = params.join(", ");
                let b = self.format_closure_body(body, &self.format_expr(body));
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
            ExprIR::ArrayMethod {
                method,
                subject,
                predicate,
            } => self.format_array_method(
                method,
                &self.format_expr(subject),
                &self.format_expr(predicate),
            ),
            ExprIR::RefOpExpr { ref_name, op } => self.format_ref_op(ref_name, op),
            // showToast 目标分发（批次 1 Task 4）：React 生成真实的
            // `toast({ message, kind, duration })` 调用（配对的
            // ToastProvider 由组件代码生成器产出，见 react/component/
            // mod.rs）；其他目标保持 Task 2 显式占位——Flutter（Task 5）
            // 与 OmniView（Task 6）后续替换各自分支，JS/Dart/Rust 族
            // 没有 Provider 机制，占位符保证"响亮失败"而非运行时裸奔。
            ExprIR::BuiltinToastCall {
                msg,
                kind,
                duration,
            } => match self.codegen_target() {
                Target::React => {
                    let mut parts = vec![format!("message: {}", self.format_expr(msg))];
                    if let Some(k) = kind {
                        let kind_str = match k {
                            ToastKindIR::Info => "info",
                            ToastKindIR::Success => "success",
                            ToastKindIR::Error => "error",
                            // V6 已硬错误拒绝 Unknown——到达 codegen 即语义
                            // 违约；原样透传保持诚实（与枚举注释一致）。
                            ToastKindIR::Unknown(raw) => raw.as_str(),
                        };
                        parts.push(format!("kind: \"{kind_str}\""));
                    }
                    if let Some(d) = duration {
                        parts.push(format!("duration: {d}"));
                    }
                    format!("toast({{ {} }})", parts.join(", "))
                }
                // showToast Flutter 分支（批次 1 Task 5 F2）：ScaffoldMessenger
                // SnackBar 语义调用。kind → 三色板背景色（error/success/info）；
                // duration: 0 → 超长时长 + "关闭" Action（不自动消失），其余 →
                // Duration(milliseconds: N)。注记：SnackBar 平台语义为单条队列
                // ——连续触发替换当前条，不做合并/排队扩展。context 依赖组件
                // State 方法体（State.context 在场）；lib 层 fn 无 context，
                // 产物编译期"响亮失败"（诚实降级，与占位符同义）。
                Target::Flutter => {
                    let mut parts = vec![format!("content: Text({})", self.format_expr(msg))];
                    if let Some(k) = kind {
                        let color = match k {
                            ToastKindIR::Info => Some("Color(0xFF1976D2)"),
                            ToastKindIR::Success => Some("Color(0xFF388E3C)"),
                            ToastKindIR::Error => Some("Color(0xFFD32F2F)"),
                            // V6 已硬错误拒绝 Unknown——防御分支不映射颜色
                            //（普通 SnackBar 诚实降级）。
                            ToastKindIR::Unknown(_) => None,
                        };
                        if let Some(c) = color {
                            parts.push(format!("backgroundColor: {c}"));
                        }
                    }
                    match duration {
                        Some(0) => {
                            parts.push("duration: const Duration(days: 1)".to_string());
                            parts.push(
                                "action: SnackBarAction(label: \"关闭\", onPressed: () => \
                                 ScaffoldMessenger.of(context).hideCurrentSnackBar())"
                                    .to_string(),
                            );
                        }
                        Some(n) => parts.push(format!("duration: Duration(milliseconds: {n})")),
                        None => {}
                    }
                    format!(
                        "ScaffoldMessenger.of(context).showSnackBar(/* 注记：SnackBar 单条队列——连续触发替换当前条 */ SnackBar({}))",
                        parts.join(", ")
                    )
                }
                _ => "<builtin:showToast>".to_string(),
            },
        }
    }
}

// ═══════════════════════════════════════════════════════════
// ArrayMethod floor gate (compiler MEDIUM, task 4.8)
// ═══════════════════════════════════════════════════════════

/// Whether the IR contains any `ExprIR::ArrayMethod` node.
///
/// Scans the FFI serialization shape (`#[serde(tag = "_kind")]` on `ExprIR`
/// and `StatementIR` — the only two tagged enums; no statement variant
/// collides): the exact tag+value pair `"_kind":"ArrayMethod"` is
/// unambiguous — a user identifier named `ArrayMethod` serializes under
/// `"_kind":"IdentifierExpr"` with the name in a different key. This
/// mirrors the DCE pass's JSON-walk precedent (`ir/optimize.rs`:
/// `walk_json_for_signal_refs`) and is complete by construction — every IR
/// node is scanned, so no per-surface walker can drift and silently miss a
/// position (the exact class of hole this floor exists to close).
fn program_has_array_methods(ir: &IRProgram) -> bool {
    serde_json::to_value(ir).is_ok_and(|v| v.to_string().contains("\"_kind\":\"ArrayMethod\""))
}

/// Codegen floor gate: language array methods (`items.filter(..)` etc.) on
/// targets with no cross-target mapping.
///
/// The mapping table ([`FormatExpr::format_array_method`]) covers the JS
/// family (React/Vue/JavaScript/TypeScript) and the Dart family
/// (Dart/Flutter); OmniView has its own generator (omniview/expr.rs).
/// Python/Rust/Dioxus have NO mapping — a verbatim emission would be a
/// nonexistent stdlib call (compiler MEDIUM: wrong program, runtime
/// failure, zero diagnostics). This gate hard-diagnoses E_CODEGEN for
/// those targets, mirroring the design-block gate in the facade
/// (`omnilang/src/codegen/mod.rs`), which calls this before dispatching to
/// the backend.
pub fn validate_array_method_floor(ir: &IRProgram, target: Target) -> Result<(), Vec<Diagnostic>> {
    if !matches!(target, Target::Python | Target::Rust | Target::Dioxus) {
        return Ok(());
    }
    if !program_has_array_methods(ir) {
        return Ok(());
    }
    Err(vec![Diagnostic {
        severity: Severity::Error,
        kind: DiagnosticKind::ECodegen,
        message: format!(
            "language array methods (filter/map/count) are not supported on the {:?} target — \
             they are mapped only on the JS family (filter/map/filter().length) and Dart family \
             (where(...).toList()/where(...).length); rewrite with an explicit loop \
             (stdlib 'array_filter'/'array_map' are deprecated)",
            target
        ),
        file: None,
        line: 0,
        col: 0,
        end_line: None,
        end_col: None,
        suggestion: None,
    }])
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

    // Task 5.8a: closure params carry NO types in the IR — the adjudicated
    // `dynamic` annotation makes the closure's result statically usable in
    // arithmetic (`int + dynamic` is `num` in Dart; the body narrowing
    // below makes int-literal arithmetic closures return int). Params that
    // ALREADY carry a type (`"TodoStoreState s"` — the codegen's typed
    // store-select closures) are left untouched, so context-typed closures
    // keep their real inference (a `dynamic` select result made a
    // collection-`for` over it flag the next list element as dead_code).
    // Empty closures stay `() => body`.
    fn format_closure(&self, params: &str, body: &str) -> String {
        if params.is_empty() {
            format!("(() => {body})")
        } else {
            let typed: Vec<String> = params
                .split(", ")
                .map(|p| {
                    if p.contains(' ') {
                        p.to_string()
                    } else {
                        format!("dynamic {p}")
                    }
                })
                .collect();
            format!("(({}) => {body})", typed.join(", "))
        }
    }

    // Task 5.8a: the closure body's IR is available here. An arithmetic
    // closure over an INT literal (`|s| s + 5`, `|x| x * 2`) is an
    // int-computation in practice — narrow the result `(body) as int` so
    // `int + closureCall(...)` stays `int` (Dart's `int + dynamic` is
    // `num`). The narrowing is a LOUD runtime error (`as int` throws) when
    // the arg is not int-coercible — the language's closures are Any-typed,
    // so this is an honest degradation, and closures over floats/strings
    // (no int literal in the arithmetic body) are left untouched.
    fn format_closure_body(&self, body: &ExprIR, body_str: &str) -> String {
        if is_int_arithmetic_closure_body(body) {
            format!("({body_str}) as int")
        } else {
            body_str.to_string()
        }
    }

    // Method calls WITH IR context — shape/static-type aware arms for
    // `get`/`contains`/`concat` (Task 5.8a), delegating the rest to the
    // string-level table below.
    fn format_method_call_with_ir(
        &self,
        object: &ExprIR,
        method: &str,
        args: &[ExprIR],
        is_optional: bool,
        obj_str: &str,
        arg_strs: &[String],
    ) -> String {
        match method {
            // `arr.get(i)` — the `!` is needed only when the indexer is
            // nullable (Map indexers); typed NON-nullable Array receivers
            // make it unnecessary_non_null_assertion (DataTable:94), and
            // String receivers (typed identifiers or literals — the doc's
            // `"abc".get(1)`) index to a non-nullable single-character
            // String (review MEDIUM/LOW, fix round 1/5).
            "get" if !is_optional => {
                let idx = arg_strs.first().map(|s| s.as_str()).unwrap_or("0");
                if expr_is_typed_array(object) || expr_is_string_shape(object) {
                    format!("({obj_str})[{idx}]")
                } else {
                    format!("({obj_str})[{idx}]!")
                }
            }
            // `x.contains(y)` — Dart's Map has no `contains` (String/List
            // do). Statically-known receivers emit the right method
            // directly — the runtime `is Map` dispatch on a typed Map
            // param was unnecessary_type_check (always true,
            // ErrorHandlingDemo's get_or_throw). Unknown receivers keep
            // the runtime dispatch (Task 5.8).
            "contains" if !is_optional => {
                let arg = arg_strs.first().map(|s| s.as_str()).unwrap_or("null");
                if expr_is_typed_map(object) {
                    format!("({obj_str}).containsKey({arg})")
                } else if expr_is_typed_string(object) || expr_is_typed_array(object) {
                    format!("({obj_str}).contains({arg})")
                } else {
                    format!(
                        "({obj_str} is Map ? ({obj_str} as Map).containsKey({arg}) : ({obj_str} as dynamic).contains({arg}))"
                    )
                }
            }
            // `a.concat(b)` — array args SPREAD their elements (the
            // language's JS semantics; Dart has no List.concat). The
            // string-shape heuristic (literal `[..]` args) is extended to
            // TYPED array variable references (Task 5.8a MEDIUM-2); scalar
            // args append as items.
            "concat" if !is_optional => {
                if arg_strs.is_empty() {
                    format!("[...{obj_str}]")
                } else {
                    let items: Vec<String> = args
                        .iter()
                        .zip(arg_strs.iter())
                        .map(|(a, s)| {
                            if expr_is_array_shape(a) || s.starts_with('[') {
                                format!("...{s}")
                            } else {
                                s.clone()
                            }
                        })
                        .collect();
                    format!("[...{obj_str}, {}]", items.join(", "))
                }
            }
            _ => {
                if is_optional {
                    self.format_optional_method_call(obj_str, method, arg_strs)
                } else {
                    self.format_method_call(obj_str, method, arg_strs)
                }
            }
        }
    }

    // Rust-style method names → Dart (mirrors the React backend's
    // translate_method_call, with Dart-native targets):
    //   `x.to_string()` → `x.toString()`, `list.len()` → `list.length`,
    //   `arr.get(i)` → `arr[i]`, `arr.push(x)` → `arr.add(x)`, etc.
    // Task 5.8: the showcase gate — the Flutter backend previously emitted
    // every method verbatim, so `.len()`/`.to_string()`/`.push()` produced
    // undefined_method errors.
    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        match method {
            "to_string" => format!("{obj}.toString({})", args.join(", ")),
            "len" => format!("{}.length", obj),
            // Array element access: `arr.get(i)` → `arr[i]` — the
            // language guarantees the element exists (the JS family indexes
            // directly), so the Dart indexer result is asserted non-null
            // (Map indexers are nullable — get_or_throw returned String?
            // from a String fn).
            "get" => {
                let idx = args.first().map(|s| s.as_str()).unwrap_or("0");
                format!("({obj})[{idx}]!")
            }
            // Array mutation methods → Dart equivalents. `push`/`pop` are
            // the language's mutation-returning forms; the emitted Dart
            // matches the method-call shape.
            "push" => format!("{obj}.add({})", args.join(", ")),
            "pop" => format!("{obj}.removeLast()"),
            "concat" => {
                // `list.concat([val])` — array args spread their elements;
                // scalar args are appended as items (Dart List has no
                // `concat` — the language's form is spread + literal).
                if args.is_empty() {
                    format!("[...{obj}]")
                } else if args[0].starts_with('[') && args[0].ends_with(']') {
                    format!("[...{obj}, ...{}]", args[0])
                } else {
                    format!("[...{obj}, {}]", args.join(", "))
                }
            }
            "slice" => {
                let start = args.first().map(|s| s.as_str()).unwrap_or("0");
                match args.get(1) {
                    Some(end) => format!("{obj}.sublist({start}, {end})"),
                    None => format!("{obj}.sublist({start})"),
                }
            }
            "splice" => {
                // Task 4.10 review I-1: `.splice(start, deleteCount, ...items)`
                // was React-only — the Dart table fell through to the verbatim
                // fallback (Dart List has no splice → compile error). Mirror
                // the React backend's slice-combination with sublist so
                // `splice` is a real cross-backend language method (React:
                // slice-combination; OmniView: native Array.prototype.splice;
                // Dart: sublist-combination).
                let start = args.first().map(|s| s.as_str()).unwrap_or("0");
                let delete_count = args.get(1).map(|s| s.as_str()).unwrap_or("0");
                let items: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();
                if items.is_empty() {
                    format!(
                        "[...{obj}.sublist(0, {start}), ...{obj}.sublist({start} + {delete_count})]"
                    )
                } else {
                    format!(
                        "[...{obj}.sublist(0, {start}), {}, ...{obj}.sublist({start} + {delete_count})]",
                        items.join(", ")
                    )
                }
            }
            "reverse" => format!("{obj}.reversed.toList()"),
            "sort" => {
                if args.is_empty() {
                    format!("([...{obj}]..sort())")
                } else {
                    format!("([...{obj}]..sort({}))", args.join(", "))
                }
            }
            // String methods
            "starts_with" => format!("{}.startsWith({})", obj, args.join(", ")),
            "ends_with" => format!("{}.endsWith({})", obj, args.join(", ")),
            "contains" => {
                // Dart's Map has no `contains` (String/List do) — dispatch on
                // the receiver shape at runtime. The analyzer type-checks
                // BOTH branches, so the non-Map branch casts to dynamic
                // (Task 5.8: ErrorHandlingDemo's `map.contains(key)` was
                // undefined_method on Map).
                let arg = args.first().map(|s| s.as_str()).unwrap_or("null");
                format!(
                    "({obj} is Map ? ({obj} as Map).containsKey({arg}) : ({obj} as dynamic).contains({arg}))"
                )
            }
            "substring" => format!("{}.substring({})", obj, args.join(", ")),
            "split" => format!("{}.split({})", obj, args.join(", ")),
            "replace" => format!("{}.replaceAll({})", obj, args.join(", ")),
            "join" => format!("{}.join({})", obj, args.join(", ")),
            "trim" => format!("{}.trim()", obj),
            "to_float" => format!("double.parse({})", obj),
            // Task 5.8a (MEDIUM-1): the built-in method table (design doc
            // 12-standard-library.md) — `.to_uppercase()`/`.to_lowercase()`
            // (String) and `.is_empty()` (Array) fell through to the
            // verbatim fallback → undefined methods on Flutter.
            "to_uppercase" => format!("{}.toUpperCase()", obj),
            "to_lowercase" => format!("{}.toLowerCase()", obj),
            "is_empty" => format!("{}.isEmpty", obj),
            _ => format!("{obj}.{method}({})", args.join(", ")),
        }
    }

    /// Optional-chain method call — same table as [`format_method_call`],
    /// with the null-aware receiver (`user?.len()` → `user?.length`).
    fn format_optional_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        match method {
            "to_string" => format!("{obj}?.toString({})", args.join(", ")),
            "len" => format!("{}?.length", obj),
            "get" => {
                let idx = args.first().map(|s| s.as_str()).unwrap_or("0");
                format!("({obj})?[{idx}]")
            }
            "push" => format!("{obj}?.add({})", args.join(", ")),
            "pop" => format!("{obj}?.removeLast()"),
            "slice" => {
                let start = args.first().map(|s| s.as_str()).unwrap_or("0");
                match args.get(1) {
                    Some(end) => format!("{obj}?.sublist({start}, {end})"),
                    None => format!("{obj}?.sublist({start})"),
                }
            }
            "starts_with" => format!("{}?.startsWith({})", obj, args.join(", ")),
            "ends_with" => format!("{}?.endsWith({})", obj, args.join(", ")),
            "contains" => {
                let arg = args.first().map(|s| s.as_str()).unwrap_or("null");
                format!(
                    "({obj} is Map ? ({obj} as Map)?.containsKey({arg}) : ({obj} as dynamic)?.contains({arg}))"
                )
            }
            "substring" => format!("{}?.substring({})", obj, args.join(", ")),
            "split" => format!("{}?.split({})", obj, args.join(", ")),
            "replace" => format!("{}?.replaceAll({})", obj, args.join(", ")),
            "join" => format!("{}?.join({})", obj, args.join(", ")),
            "trim" => format!("{}?.trim()", obj),
            "to_float" => format!("double.parse({}!)", obj),
            "to_uppercase" => format!("{}?.toUpperCase()", obj),
            "to_lowercase" => format!("{}?.toLowerCase()", obj),
            "is_empty" => format!("{}?.isEmpty", obj),
            _ => format!("{obj}?.{method}({})", args.join(", ")),
        }
    }

    /// Dart string `+` requires both operands to be Strings. The language
    /// (JS-semantics) allows `'x' + 5` / `'...' + err` — coerce the
    /// non-literal side with `.toString()` so the emitted Dart type-checks
    /// (Task 5.8: StringLiteralsDemo / StdLibShowcase concat errors).
    /// An operand that is ALREADY a `.to_string()` call is left alone
    /// (`'star Lv.' + level.toString()` must not become
    /// `(level.toString()).toString()`).
    fn format_binary_op(
        &self,
        op: &str,
        left: &ExprIR,
        right: &ExprIR,
        left_str: &str,
        right_str: &str,
    ) -> String {
        if op == "+" {
            // "String-ish" = a string literal, an already-coerced toString()
            // call, or a NESTED `+` chain (`'Hello ' + userName + ...` — the
            // top-level op's left side is the whole chain, so the literal
            // check alone missed `... + version` at the tail, Task 5.8).
            let is_stringish = |e: &ExprIR| -> bool {
                match e {
                    ExprIR::LiteralExpr { value } => value.is_string(),
                    ExprIR::MethodCallExpr { method, .. } => {
                        method == "to_string" || method == "toString"
                    }
                    ExprIR::BinaryOpExpr { op, .. } => matches!(op, BinaryOpIR::Add),
                    _ => false,
                }
            };
            if is_stringish(left) && !is_stringish(right) {
                return format!("{left_str} + ({right_str}).toString()");
            }
            if is_stringish(right) && !is_stringish(left) {
                return format!("({left_str}).toString() + {right_str}");
            }
        }
        format!("{left_str} {op} {right_str}")
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

    // Dart uses `switch/case` inside an IIFE. Variant patterns dispatch on
    // `._variant`; literal patterns (match on primitives) switch on the
    // subject value itself — `n._variant` on a raw int is null and never
    // matches, so a switch on the value is required when any literal arm
    // exists (variant dispatch stays when all arms are variants).
    //
    // Variant arms bind their pattern fields (`Star { level }`) from the
    // subject's union-class members — `final level = _v.level;` — and are
    // wrapped in a block so the declarations stay scoped to their case
    // (Dart switch cases share the enclosing scope). The union class itself
    // exposes every variant field as a dynamic member, so the access
    // type-checks; the emitting backend is responsible for declaring it
    // (Flutter emits referenced unions into types.dart — deferral #30).
    // Matches without a wildcard arm get a `default: throw StateError(...)`
    // fallback so the IIFE's return type is never nullable — otherwise
    // `return (() { switch ... })();` reports return_of_invalid_type.
    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let has_variant = arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, MatchPatternKindIR::Variant { .. }));
        let has_wildcard = arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, MatchPatternKindIR::Wildcard));
        let subject = if has_variant {
            // `variant` (not `_variant`): the union class lives in the shared
            // types.dart library, and `_`-prefixed members are library-private
            // in Dart — the match arm's `_v.variant` access must cross files.
            "_v.variant".to_string()
        } else {
            "_v".to_string()
        };
        let mut cases: Vec<String> = arms
            .iter()
            .map(|arm| {
                let body = self.format_block_as_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, bindings } => {
                        // Bind pattern fields from the subject's union-class
                        // members. UNUSED bindings are skipped entirely —
                        // the old `_`-prefix trick does NOT silence Dart's
                        // unused_local_variable lint for `_data`
                        // (DataFetching:14, Task 5.8a). The discriminator
                        // member is `variant` — public, so the access
                        // crosses the types.dart library boundary (a
                        // `_`-prefixed member would be library-private).
                        let binds: Vec<String> = bindings
                            .iter()
                            .filter(|b| b.as_str() != "variant")
                            .filter(|b| body.contains(b.as_str()))
                            .map(|b| format!("final {b} = _v.{b};"))
                            .collect();
                        if binds.is_empty() {
                            format!("  case \"{name}\": return {body};")
                        } else {
                            format!(
                                "  case \"{name}\": {{\n{}\n    return {body};\n  }}",
                                binds.join("\n    ")
                            )
                        }
                    }
                    MatchPatternKindIR::Wildcard => {
                        format!("  default: return {body};")
                    }
                    MatchPatternKindIR::Literal { value } => {
                        let v = format_json_value(value, self.json_config());
                        format!("  case {v}: return {body};")
                    }
                }
            })
            .collect();
        if !has_wildcard {
            cases.push("  default: throw StateError('Unmatched variant');".to_string());
        }
        format!(
            "(() {{\nfinal _v = {matched};\nswitch ({subject}) {{\n{}\n}}\n}})()",
            cases.join("\n")
        )
    }

    // Dart uses `{key: value}` without spaces inside braces for MAP
    // literals; STRUCT literals (`StatisticsData { name: "demo" }`) become
    // named-parameter constructor calls (`StatisticsData(name: 'demo')`) —
    // the struct class is emitted into types.dart (Task 5.8). Keys arrive
    // unquoted via `format_struct_field_named`.
    fn format_struct(&self, type_name: &str, fields: &str) -> String {
        if type_name.is_empty() {
            return format!("{{{fields}}}");
        }
        format!("{type_name}({fields})")
    }

    /// Map-literal keys keep their formatted expression verbatim: a string
    /// key already carries Dart quotes (`'dev'`), numbers/bools are unquoted
    /// (`90`, `true`). The default `format_struct_field` wraps non-`"`-quoted
    /// keys in double quotes (JSON style), which double-quotes Dart strings
    /// into `"'dev'"` and breaks `map["dev"]` lookups at runtime.
    fn format_struct_field(&self, key: &str, value: &str) -> String {
        format!("{key}: {value}")
    }

    /// Struct-literal keys become Dart named-constructor parameters —
    /// unquoted `name: value` (map literals keep the formatted form).
    fn format_struct_field_named(&self, key: &str, value: &str) -> String {
        let key = key.trim_matches('"');
        format!("{key}: {value}")
    }

    // Dart supports block-body closures — emit an immediately-invoked closure
    // so statements inside the block execute in expression position.
    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        if statements.is_empty() {
            return self.format_expr(tail);
        }
        let stmts: Vec<String> = statements
            .iter()
            .map(|s| format!("  {};", format_block_stmt_dart(s, self)))
            .collect();
        format!(
            "(() {{\n{}\n  return {};\n}})()",
            stmts.join("\n"),
            self.format_expr(tail)
        )
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

    // Rust has no `?.` operator — optional chain degrades to plain field access.
    fn format_optional_chain(&self, object: &str, field: &str) -> String {
        format!("{object}.{field}")
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

    // Rust natively supports block expressions — statements in expression
    // position emit as a real `{ stmts; tail }` block (was: silent drop of
    // the statements — rust MEDIUM, wrong programs with zero diagnostics).
    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        if statements.is_empty() {
            return self.format_expr(tail);
        }
        let stmts: Vec<String> = statements
            .iter()
            .map(|s| format!("  {};", format_block_stmt_rust(s, self)))
            .collect();
        format!("{{\n{}\n  {}\n}}", stmts.join("\n"), self.format_expr(tail))
    }
}

// ═══════════════════════════════════════════════════════════
// Static-type shape helpers (Task 5.8a)
// ═══════════════════════════════════════════════════════════
//
// IdentifierExpr.type_ann carries the semantic symbol table's annotation
// (source-level type strings: "Array<RowItem>", "Map<String, String>",
// "String", ...) — the Dart method table uses these to shape emissions
// (`get` bang, `contains` dispatch, `concat` spread) without a runtime
// dispatch or an analyzer warning.

/// Whether the expression's static type is a NON-NULLABLE array.
pub fn expr_is_typed_array(e: &ExprIR) -> bool {
    match e {
        ExprIR::IdentifierExpr {
            type_ann: Some(t), ..
        } => (t == "Array" || t.starts_with("Array<")) && !t.ends_with('?'),
        _ => false,
    }
}

/// Whether the expression's static type is a non-nullable Map.
pub fn expr_is_typed_map(e: &ExprIR) -> bool {
    match e {
        ExprIR::IdentifierExpr {
            type_ann: Some(t), ..
        } => (t == "Map" || t.starts_with("Map<")) && !t.ends_with('?'),
        _ => false,
    }
}

/// Whether the expression's static type is the non-nullable String.
pub fn expr_is_typed_string(e: &ExprIR) -> bool {
    matches!(
        e,
        ExprIR::IdentifierExpr {
            type_ann: Some(t),
            ..
        } if t == "String"
    )
}

/// STRING-SHAPE check for the `get` arm (review MEDIUM/LOW, fix round 1/5):
/// a typed String identifier OR a string literal — Dart's String indexer
/// returns a NON-nullable single-character String, so the `!` is
/// unnecessary for both (12-standard-library.md's `"abc".get(1)` → `"b"`).
fn expr_is_string_shape(e: &ExprIR) -> bool {
    expr_is_typed_string(e) || matches!(e, ExprIR::LiteralExpr { value } if value.is_string())
}

/// Array-SHAPE check for `concat` args: a typed array variable OR an array
/// literal — the string-shape heuristic extended to typed variable
/// references (Task 5.8a MEDIUM-2).
fn expr_is_array_shape(e: &ExprIR) -> bool {
    expr_is_typed_array(e) || matches!(e, ExprIR::ArrayExpr { .. })
}

/// Whether an integer literal appears in the expression.
fn has_int_literal_operand(e: &ExprIR) -> bool {
    match e {
        ExprIR::LiteralExpr { value } => value.is_i64(),
        _ => false,
    }
}

/// Task 5.8a closure-body narrowing rule: the body is a binary ARITHMETIC
/// expression (`+ - * / %`) with at least one INT literal operand — the
/// `|s| s + 5` / `|x| x * 2` shapes. String concat closures (`|a, b| a + b`),
/// float arithmetic (`|v| v * 1.5`), comparisons, ternaries, and member
/// accesses are NOT narrowed (no int-literal operand / non-arithmetic op).
fn is_int_arithmetic_closure_body(e: &ExprIR) -> bool {
    match e {
        ExprIR::BinaryOpExpr {
            op, left, right, ..
        } => {
            // DIV is excluded (review MEDIUM, fix round 1/5): Dart's
            // `int / int` returns DOUBLE — `(x / 2) as int` throws for every
            // int argument, so the language's `int/int -> Int` semantic rule
            // is NOT sound on the Dart target. Add/Sub/Mul/Mod are
            // int-closed in Dart and stay narrowed.
            matches!(
                op,
                BinaryOpIR::Add | BinaryOpIR::Sub | BinaryOpIR::Mul | BinaryOpIR::Mod
            ) && (has_int_literal_operand(left) || has_int_literal_operand(right))
        }
        _ => false,
    }
}

/// Format a single statement as a Dart string for use inside block-expression IIFEs.
/// Mirrors `DartStmtFormatter` conventions (`var` bindings, `;` appended by caller).
fn format_block_stmt_dart(stmt: &StatementIR, fmt: &impl FormatExpr) -> String {
    match stmt {
        StatementIR::LetStmtIR { name, value, .. } => {
            let val = value
                .as_ref()
                .map(|e| fmt.format_expr(e))
                .unwrap_or_else(|| fmt.null_fallback().into());
            format!("var {} = {}", fmt.format_identifier(name), val)
        }
        StatementIR::ExprStmtIR { expr } => fmt.format_expr(expr),
        StatementIR::ReturnStmtIR { value } => {
            let val = value
                .as_ref()
                .map(|e| fmt.format_expr(e))
                .unwrap_or_else(|| fmt.null_fallback().into());
            format!("return {}", val)
        }
        _ => "/* stmt */".into(),
    }
}

/// Format a single statement as a Rust string for use inside block
/// expressions (`RustFormatExpr::format_block_expr`). Mirrors the Dart
/// helper's conventions (`let` bindings; `;` appended by the caller; the
/// final statement of the block is the tail expression without `;`).
///
/// Only expression-returning statements (let/expr/return) are
/// string-representable in this helper's signature. Control-flow and
/// side-effect statements (if/while/for/try/mutation/emit — 8 of the 11
/// `StatementIR` variants) cannot be formatted here and must fail LOUDLY
/// via a `compile_error!` sentinel (review I1): a silent `/* stmt */`
/// placeholder would drop the statement with zero diagnostics — the exact
/// silent-wrong-program shape this task exists to close.
fn format_block_stmt_rust(stmt: &StatementIR, fmt: &impl FormatExpr) -> String {
    match stmt {
        StatementIR::LetStmtIR { name, value, .. } => {
            let val = value
                .as_ref()
                .map(|e| fmt.format_expr(e))
                .unwrap_or_else(|| fmt.null_fallback().into());
            format!("let {} = {}", fmt.format_identifier(name), val)
        }
        StatementIR::ExprStmtIR { expr } => fmt.format_expr(expr),
        StatementIR::ReturnStmtIR { value } => {
            let val = value
                .as_ref()
                .map(|e| fmt.format_expr(e))
                .unwrap_or_else(|| fmt.null_fallback().into());
            format!("return {}", val)
        }
        _ => {
            // Uncovered statement variant in an expression-position block —
            // never emit a silent placeholder. The target build fails with
            // the variant name (variant names are identifiers — safe inside
            // the string literal, no escaping needed).
            let debug = format!("{stmt:?}");
            let name = debug.split(['(', '{']).next().unwrap_or("Statement");
            format!(
                "compile_error!(\"statement '{name}' in an expression-position block is not supported on this target\")"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt_chain(user: &str) -> ExprIR {
        ExprIR::OptionalChainExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: user.into(),
                type_ann: None,
            }),
            field: "name".into(),
        }
    }

    fn block_expr_with_stmt() -> ExprIR {
        ExprIR::BlockExpr {
            statements: vec![StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(5),
                }),
                r#type: None,
                is_mut: false,
            }],
            tail: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
                type_ann: None,
            }),
        }
    }

    #[test]
    fn optional_chain_default_preserves_question_dot() {
        // JS-family default must keep `?.` (was silently dropped as `.`).
        assert_eq!(JsFormatExpr.format_expr(&opt_chain("user")), "user?.name");
    }

    #[test]
    fn dart_optional_chain_keeps_question_dot() {
        // Dart supports null-aware access — `?.` is valid and semantic-preserving.
        let fmt = DartFormatExpr {
            event_param_name: None,
        };
        assert_eq!(fmt.format_expr(&opt_chain("user")), "user?.name");
    }

    #[test]
    fn rust_optional_chain_degrades_to_field_access() {
        // Rust has no `?.` — must degrade to plain field access.
        let fmt = RustFormatExpr {
            move_closures: false,
            event_param_name: None,
        };
        assert_eq!(fmt.format_expr(&opt_chain("user")), "user.name");
    }

    #[test]
    fn dart_block_expr_emits_iife() {
        let fmt = DartFormatExpr {
            event_param_name: None,
        };
        let out = fmt.format_expr(&block_expr_with_stmt());
        assert!(
            out.contains("(() {") && out.contains("var x = 5;") && out.contains("return x;"),
            "dart block expr: {out}"
        );
    }

    #[test]
    fn rust_block_expr_emits_block_expression() {
        // rust MEDIUM: `RustFormatExpr::format_block_expr` used to drop the
        // statements and emit only the tail — a silent wrong program (the
        // trait default's debug_assert guard was overridden away). Rust
        // natively supports block expressions, so the statements must be
        // emitted as `{ stmts; tail }`.
        let fmt = RustFormatExpr {
            move_closures: false,
            event_param_name: None,
        };
        let out = fmt.format_expr(&block_expr_with_stmt());
        assert!(out.contains("let x = 5;"), "silent drop: {out}");
    }

    #[test]
    fn rust_block_expr_uncovered_stmt_fails_loudly() {
        // Review I1: `{ if c { } y }` — control-flow statements cannot be
        // string-formatted by the block helper; the statement must fail
        // LOUDLY (compile_error! sentinel) instead of a silent `/* stmt */`
        // placeholder (8/11 StatementIR variants were dropped with zero
        // diagnostics).
        let expr = ExprIR::BlockExpr {
            statements: vec![StatementIR::IfStmtIR {
                condition: ident("c", None),
                then_body: BlockIR { statements: vec![] },
                else_ifs: vec![],
                else_body: None,
            }],
            tail: Box::new(ident("y", None)),
        };
        let fmt = RustFormatExpr {
            move_closures: false,
            event_param_name: None,
        };
        let out = fmt.format_expr(&expr);
        assert!(
            out.contains("compile_error!") && !out.contains("/* stmt */"),
            "uncovered statement must fail loudly, got: {out}"
        );
    }

    /// Helper: an `items.filter(|i| i.done)`-shaped ArrayMethod IR node.
    fn array_method(method: ArrayMethodIR, predicate_body: &str) -> ExprIR {
        ExprIR::ArrayMethod {
            method,
            subject: Box::new(ExprIR::IdentifierExpr {
                name: "items".into(),
                type_ann: Some("Array<Todo>".into()),
            }),
            predicate: Box::new(ExprIR::ClosureExpr {
                params: vec!["i".into()],
                body: Box::new(ExprIR::IdentifierExpr {
                    name: predicate_body.into(),
                    type_ann: None,
                }),
            }),
        }
    }

    fn filter_expr() -> ExprIR {
        array_method(ArrayMethodIR::Filter, "i.done")
    }

    fn map_expr() -> ExprIR {
        array_method(ArrayMethodIR::Map, "i * 2")
    }

    fn count_expr() -> ExprIR {
        array_method(ArrayMethodIR::Count, "i.done")
    }

    #[test]
    fn js_family_filter_maps_to_js_filter() {
        // items.filter(|i| i.done) → items.filter(((i) => i.done)) — the
        // closure keeps the shared `((params) => body)` emission convention.
        assert_eq!(
            JsFormatExpr.format_expr(&filter_expr()),
            "items.filter(((i) => i.done))"
        );
    }

    #[test]
    fn js_family_map_maps_to_js_map() {
        assert_eq!(
            JsFormatExpr.format_expr(&map_expr()),
            "items.map(((i) => i * 2))"
        );
    }

    #[test]
    fn js_family_count_expands_to_filter_length() {
        // count → filter(pred).length — JS arrays have no native count.
        assert_eq!(
            JsFormatExpr.format_expr(&count_expr()),
            "items.filter(((i) => i.done)).length"
        );
    }

    #[test]
    fn dart_family_filter_maps_to_where_tolist() {
        // items.filter(|i| i.done) → items.where(((i) => i.done)).toList()
        // (Dart List has no `filter` method — the cross-target mapping that
        // makes `.filter()` compile on Flutter.) The predicate body is a
        // member access — no int-narrowing (Task 5.8a).
        let fmt = DartFormatExpr {
            event_param_name: None,
        };
        assert_eq!(
            fmt.format_expr(&filter_expr()),
            "items.where(((dynamic i) => i.done)).toList()"
        );
    }

    #[test]
    fn dart_family_map_maps_to_dart_map_tolist() {
        // `|x| x * 2` (a REAL arithmetic closure body) narrows so the map
        // result is List<int> — not List<dynamic> →
        // return_of_invalid_type (Task 5.8a StdLibShowcase's demo_map).
        let fmt = DartFormatExpr {
            event_param_name: None,
        };
        let arithmetic_map = ExprIR::ArrayMethod {
            method: ArrayMethodIR::Map,
            subject: Box::new(ExprIR::IdentifierExpr {
                name: "items".into(),
                type_ann: Some("Array<i32>".into()),
            }),
            predicate: Box::new(ExprIR::ClosureExpr {
                params: vec!["x".into()],
                body: Box::new(ExprIR::BinaryOpExpr {
                    op: BinaryOpIR::Mul,
                    left: Box::new(ident("x", None)),
                    right: Box::new(ExprIR::LiteralExpr {
                        value: serde_json::json!(2),
                    }),
                }),
            }),
        };
        assert_eq!(
            fmt.format_expr(&arithmetic_map),
            "items.map(((dynamic x) => (x * 2) as int)).toList()"
        );
    }

    #[test]
    fn dart_family_count_maps_to_where_length() {
        // count → where(pred).length — the lazy Iterable's length is the
        // count; no toList() needed.
        let fmt = DartFormatExpr {
            event_param_name: None,
        };
        assert_eq!(
            fmt.format_expr(&count_expr()),
            "items.where(((dynamic i) => i.done)).length"
        );
    }

    #[test]
    fn rust_family_floors_unmapped_array_methods() {
        // compiler MEDIUM: unmapped targets (Python/Rust/Dioxus) used to
        // emit the verbatim `items.filter(..)` — a nonexistent stdlib call
        // (runtime failure, zero diagnostics). The floor emits a hard
        // `compile_error!` sentinel so even direct formatter use stays
        // loud; the compile pipeline additionally gates these targets with
        // E_CODEGEN before formatting (validate_array_method_floor).
        let fmt = RustFormatExpr {
            move_closures: false,
            event_param_name: None,
        };
        let out = fmt.format_expr(&filter_expr());
        assert!(
            out.contains("compile_error!") && out.contains("not supported"),
            "unmapped targets must floor-gate array methods, got: {out}"
        );
    }

    /// Compile .omni source through the real pipeline, then run the
    /// production Python-target floor gate — the same function the compile
    /// pipeline calls for unmapped targets. Returns (output, diagnostics)
    /// with diagnostics in the FFI envelope shape (`DiagnosticJson`, whose
    /// `.code` field carries the kind string like "E_CODEGEN").
    fn compile_python_with_diags(source: &str) -> (String, Vec<crate::error::DiagnosticJson>) {
        let out = crate::run_pipeline(source);
        let mut diags: Vec<crate::error::Diagnostic> = out.diagnostics;
        if let Err(errors) = validate_array_method_floor(&out.ir, Target::Python) {
            diags.extend(errors);
        }
        let json: Vec<crate::error::DiagnosticJson> = diags
            .iter()
            .map(crate::error::DiagnosticJson::from)
            .collect();
        (String::new(), json)
    }

    #[test]
    fn array_method_python_hard_diagnosed() {
        // compiler MEDIUM: `items.filter(..)` used to emit verbatim on the
        // Python target — the generated `items.filter(...)` is a nonexistent
        // stdlib call (runtime failure, zero compiler diagnostics). The
        // floor gate must hard-diagnose E_CODEGEN.
        let (_, diags) = compile_python_with_diags(
            r#"component V { view { @for item in items.filter(|x| x > 0) { <p>{item}</p> } } }"#,
        );
        assert!(
            diags.iter().any(|d| d.code == "E_CODEGEN"),
            "verbatim emission must be gated, got: {:?}",
            diags
        );
    }

    // ═══════════════════════════════════════════════════════════
    // Task 5.8a: closure param typing, stdlib method table completion,
    // concat spread, match binding skip, get/contains static-type shaping
    // ═══════════════════════════════════════════════════════════

    fn dart_fmt() -> DartFormatExpr {
        DartFormatExpr {
            event_param_name: None,
        }
    }

    fn ident(name: &str, type_ann: Option<&str>) -> ExprIR {
        ExprIR::IdentifierExpr {
            name: name.into(),
            type_ann: type_ann.map(str::to_string),
        }
    }

    fn method_call(object: ExprIR, method: &str, args: Vec<ExprIR>) -> ExprIR {
        ExprIR::MethodCallExpr {
            object: Box::new(object),
            method: method.into(),
            args,
            is_optional: false,
        }
    }

    /// `|s| s + 5` — the DataTable residual: untyped closure params made
    /// Dart infer `num Function(...)` (invalid_assignment at the call site).
    fn untyped_closure_expr() -> ExprIR {
        ExprIR::ClosureExpr {
            params: vec!["s".into()],
            body: Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Add,
                left: Box::new(ident("s", None)),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(5),
                }),
            }),
        }
    }

    #[test]
    fn dart_closure_int_arithmetic_body_narrowed_as_int() {
        // Controller adjudication (a): untyped closure params emit
        // `(dynamic s) => ...`, and the int-literal arithmetic body narrows
        // `(s + 5) as int` — `bonus` becomes `int Function(dynamic)`, so
        // `total + bonus(...)` stays int (Dart's `int + dynamic` would be
        // `num` — the DataTable invalid_assignment).
        let out = dart_fmt().format_expr(&untyped_closure_expr());
        assert_eq!(out, "((dynamic s) => (s + 5) as int)");
    }

    #[test]
    fn dart_closure_multiple_params_annotated_dynamic() {
        let expr = ExprIR::ClosureExpr {
            params: vec!["a".into(), "b".into()],
            body: Box::new(ident("a", None)),
        };
        assert_eq!(
            dart_fmt().format_expr(&expr),
            "((dynamic a, dynamic b) => a)"
        );
    }

    #[test]
    fn dart_closure_typed_param_passthrough() {
        // The codegen's store-select closures carry a TYPED param
        // ("TodoStoreState s") — the `dynamic` annotation must not double
        // up: a `dynamic` select result made a collection-`for` over it
        // flag the next list element as dead_code (NotificationsPanel).
        let expr = ExprIR::ClosureExpr {
            params: vec!["TodoStoreState s".into()],
            body: Box::new(ExprIR::FieldAccessExpr {
                object: Box::new(ident("s", None)),
                field: "totalCount".into(),
            }),
        };
        assert_eq!(
            dart_fmt().format_expr(&expr),
            "((TodoStoreState s) => s.totalCount)"
        );
    }

    #[test]
    fn dart_closure_non_arithmetic_body_keeps_plain_form() {
        // Comparisons, member accesses, ternaries — no narrowing.
        let expr = ExprIR::ClosureExpr {
            params: vec!["r".into()],
            body: Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Eq,
                left: Box::new(ExprIR::FieldAccessExpr {
                    object: Box::new(ident("r", None)),
                    field: "status".into(),
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!("Active"),
                }),
            }),
        };
        assert_eq!(
            dart_fmt().format_expr(&expr),
            "((dynamic r) => r.status == 'Active')"
        );
    }

    #[test]
    fn dart_closure_float_arithmetic_body_not_narrowed() {
        // Float-literal arithmetic (`|v| v * 1.5`) is NOT narrowed — the
        // rule targets int-literal arithmetic closures only.
        let expr = ExprIR::ClosureExpr {
            params: vec!["v".into()],
            body: Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Mul,
                left: Box::new(ident("v", None)),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(1.5),
                }),
            }),
        };
        assert_eq!(dart_fmt().format_expr(&expr), "((dynamic v) => v * 1.5)");
    }

    #[test]
    fn dart_closure_div_body_not_narrowed() {
        // Review MEDIUM (fix round 1/5): DIV is excluded from the narrowing
        // set — Dart's `int / int` returns DOUBLE, so `(x / 2) as int`
        // throws at runtime for every int argument (the language's
        // `int/int -> Int` semantic rule does NOT hold on the Dart target).
        // Add/Sub/Mul/Mod are int-closed and stay narrowed.
        let expr = ExprIR::ClosureExpr {
            params: vec!["x".into()],
            body: Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Div,
                left: Box::new(ident("x", None)),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
            }),
        };
        assert_eq!(dart_fmt().format_expr(&expr), "((dynamic x) => x / 2)");
    }

    #[test]
    fn dart_closure_mod_body_stays_narrowed() {
        // `int % int` is int-closed in Dart — Mod stays in the narrowing set.
        let expr = ExprIR::ClosureExpr {
            params: vec!["x".into()],
            body: Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Mod,
                left: Box::new(ident("x", None)),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(3),
                }),
            }),
        };
        assert_eq!(
            dart_fmt().format_expr(&expr),
            "((dynamic x) => (x % 3) as int)"
        );
    }

    #[test]
    fn dart_closure_string_concat_body_not_narrowed() {
        // `|a, b| a + b` (string concat closure) is NOT narrowed — no int
        // literal operand.
        let expr = ExprIR::ClosureExpr {
            params: vec!["a".into(), "b".into()],
            body: Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Add,
                left: Box::new(ident("a", None)),
                right: Box::new(ident("b", None)),
            }),
        };
        assert_eq!(
            dart_fmt().format_expr(&expr),
            "((dynamic a, dynamic b) => a + b)"
        );
    }

    #[test]
    fn dart_closure_empty_params_unchanged() {
        let expr = ExprIR::ClosureExpr {
            params: vec![],
            body: Box::new(ident("x", None)),
        };
        assert_eq!(dart_fmt().format_expr(&expr), "(() => x)");
    }

    #[test]
    fn js_closure_params_stay_untyped() {
        // JS-family closures keep the shared `(s) => ...` form — the
        // `dynamic` annotation is Dart-only.
        let expr = ExprIR::ClosureExpr {
            params: vec!["s".into()],
            body: Box::new(ident("s", None)),
        };
        assert_eq!(JsFormatExpr.format_expr(&expr), "((s) => s)");
    }

    #[test]
    fn dart_method_table_covers_to_uppercase_lowercase_is_empty() {
        // MEDIUM-1: `.to_uppercase()`/`.to_lowercase()`/`.is_empty()` were
        // missing from the Dart method table — the verbatim fallback emitted
        // undefined methods.
        let upper = method_call(ident("s", Some("String")), "to_uppercase", vec![]);
        assert_eq!(dart_fmt().format_expr(&upper), "s.toUpperCase()");
        let lower = method_call(ident("s", Some("String")), "to_lowercase", vec![]);
        assert_eq!(dart_fmt().format_expr(&lower), "s.toLowerCase()");
        let empty = method_call(ident("arr", Some("Array<i32>")), "is_empty", vec![]);
        assert_eq!(dart_fmt().format_expr(&empty), "arr.isEmpty");
    }

    #[test]
    fn dart_method_table_covers_splice() {
        // Task 4.10 review I-1: `.splice(start, deleteCount, ...items)` was
        // React-only — the Dart table fell through to the verbatim fallback
        // (Dart List has no splice → compile error). Mirrors the React
        // backend's slice-combination with sublist so `splice` is a real
        // cross-backend language method.
        let splice = method_call(
            ident("items", Some("Array<i32>")),
            "splice",
            vec![
                ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                },
                ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                },
            ],
        );
        assert_eq!(
            dart_fmt().format_expr(&splice),
            "[...items.sublist(0, 1), ...items.sublist(1 + 2)]"
        );
        let splice_with_items = method_call(
            ident("items", Some("Array<i32>")),
            "splice",
            vec![
                ExprIR::LiteralExpr {
                    value: serde_json::json!(0),
                },
                ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                },
                ident("x", None),
            ],
        );
        assert_eq!(
            dart_fmt().format_expr(&splice_with_items),
            "[...items.sublist(0, 0), x, ...items.sublist(0 + 1)]"
        );
    }

    #[test]
    fn dart_optional_method_table_covers_to_uppercase_lowercase_is_empty() {
        let fmt = dart_fmt();
        let upper = ExprIR::MethodCallExpr {
            object: Box::new(ident("s", None)),
            method: "to_uppercase".into(),
            args: vec![],
            is_optional: true,
        };
        assert_eq!(fmt.format_expr(&upper), "s?.toUpperCase()");
        let empty = ExprIR::MethodCallExpr {
            object: Box::new(ident("arr", None)),
            method: "is_empty".into(),
            args: vec![],
            is_optional: true,
        };
        assert_eq!(fmt.format_expr(&empty), "arr?.isEmpty");
    }

    #[test]
    fn dart_concat_spreads_typed_array_variable() {
        // MEDIUM-2: `a.concat(b)` where `b` is a TYPED array variable must
        // spread the elements (`[...a, ...b]`) — element-append (`[...a, b]`)
        // was a JS-semantics fork.
        let expr = method_call(
            ident("a", Some("Array<i32>")),
            "concat",
            vec![ident("b", Some("Array<i32>"))],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "[...a, ...b]");
    }

    #[test]
    fn dart_concat_spreads_literal_array_arg() {
        let expr = method_call(
            ident("a", Some("Array<i32>")),
            "concat",
            vec![ExprIR::ArrayExpr {
                elements: vec![ident("v", None)],
            }],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "[...a, ...[v]]");
    }

    #[test]
    fn dart_concat_keeps_element_append_for_untyped_variable() {
        // Untyped variable args cannot be proven arrays — keep the current
        // element-append (documented degradation, no false spread).
        let expr = method_call(
            ident("a", Some("Array<i32>")),
            "concat",
            vec![ident("b", None)],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "[...a, b]");
    }

    #[test]
    fn dart_concat_no_args_keeps_copy() {
        let expr = method_call(ident("a", Some("Array<i32>")), "concat", vec![]);
        assert_eq!(dart_fmt().format_expr(&expr), "[...a]");
    }

    #[test]
    fn dart_get_on_typed_array_omits_bang() {
        // `rows.get(i)` on a typed non-nullable Array: the indexer is
        // non-nullable — the unconditional `!` was
        // unnecessary_non_null_assertion in dart analyze.
        let expr = method_call(
            ident("rows", Some("Array<RowItem>")),
            "get",
            vec![ident("i", Some("i32"))],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "(rows)[i]");
    }

    #[test]
    fn dart_get_on_untyped_receiver_keeps_bang() {
        // Untyped receivers (map literals etc.) keep the `!` — their
        // indexers are nullable.
        let expr = method_call(
            ident("cfg", None),
            "get",
            vec![ExprIR::LiteralExpr {
                value: serde_json::json!("90"),
            }],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "(cfg)['90']!");
    }

    #[test]
    fn dart_get_on_typed_string_omits_bang() {
        // Review MEDIUM/LOW (fix round 1/5): the doc's built-in `.get` on
        // String (`"abc".get(1)` → `"b"`, 12-standard-library.md) — Dart's
        // String indexer returns a NON-nullable single-character String, so
        // the `!` was unnecessary_non_null_assertion. Typed String
        // receivers emit the plain index.
        let expr = method_call(
            ident("s", Some("String")),
            "get",
            vec![ident("i", Some("i32"))],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "(s)[i]");
    }

    #[test]
    fn dart_get_on_string_literal_omits_bang() {
        // The doc example `"abc".get(1)` — a string LITERAL receiver also
        // has a non-nullable indexer.
        let expr = method_call(
            ExprIR::LiteralExpr {
                value: serde_json::json!("abc"),
            },
            "get",
            vec![ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "('abc')[1]");
    }

    #[test]
    fn dart_get_on_nullable_array_keeps_bang() {
        let expr = method_call(
            ident("rows", Some("Array<RowItem>?")),
            "get",
            vec![ident("i", Some("i32"))],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "(rows)[i]!");
    }

    #[test]
    fn dart_contains_on_typed_map_uses_contains_key() {
        // `map.contains(key)` on a typed Map receiver: emit containsKey
        // directly — the runtime `is Map` dispatch was
        // unnecessary_type_check (always true).
        let expr = method_call(
            ident("map", Some("Map<String, String>")),
            "contains",
            vec![ident("key", Some("String"))],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "(map).containsKey(key)");
    }

    #[test]
    fn dart_contains_on_typed_string_keeps_contains() {
        let expr = method_call(
            ident("s", Some("String")),
            "contains",
            vec![ident("sub", Some("String"))],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "(s).contains(sub)");
    }

    #[test]
    fn dart_contains_on_typed_array_keeps_contains() {
        let expr = method_call(
            ident("items", Some("Array<String>")),
            "contains",
            vec![ident("x", Some("String"))],
        );
        assert_eq!(dart_fmt().format_expr(&expr), "(items).contains(x)");
    }

    #[test]
    fn dart_contains_on_unknown_receiver_keeps_runtime_dispatch() {
        let expr = method_call(ident("x", None), "contains", vec![ident("y", None)]);
        assert_eq!(
            dart_fmt().format_expr(&expr),
            "(x is Map ? (x as Map).containsKey(y) : (x as dynamic).contains(y))"
        );
    }

    #[test]
    fn dart_match_skips_unused_bindings_entirely() {
        // The `_`-prefixed unused binding trick does not silence Dart's
        // unused_local_variable lint for `_data` — an unused pattern
        // binding must not be emitted at all.
        let expr = ExprIR::MatchExpr {
            matched: Box::new(ident("v", Some("ApiResult"))),
            arms: vec![MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Ok".into(),
                        bindings: vec!["data".into(), "extra".into()],
                    },
                },
                body: BlockIR {
                    statements: Vec::new(),
                },
                union_type: Some("ApiResult".into()),
            }],
        };
        let out = dart_fmt().format_expr(&expr);
        assert!(
            !out.contains("final data =") && !out.contains("final _data ="),
            "unused bindings must not be emitted: {out}"
        );
        assert!(
            out.contains("case \"Ok\": return"),
            "arm body must survive: {out}"
        );
    }

    #[test]
    fn dart_match_keeps_used_bindings() {
        // The arm body references the binding (the shared formatter renders
        // it inside the arm), so it must be emitted — not skipped.
        let expr = ExprIR::MatchExpr {
            matched: Box::new(ident("v", Some("ApiResult"))),
            arms: vec![MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Err".into(),
                        bindings: vec!["error".into()],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ExprStmtIR {
                        expr: ident("error", None),
                    }],
                },
                union_type: Some("ApiResult".into()),
            }],
        };
        let out = dart_fmt().format_expr(&expr);
        assert!(
            out.contains("final error = _v.error;"),
            "used bindings must still be emitted: {out}"
        );
    }

    #[test]
    fn dart_match_literal_arm_emits_case() {
        let fmt = DartFormatExpr {
            event_param_name: None,
        };
        let expr = ExprIR::MatchExpr {
            matched: Box::new(ExprIR::IdentifierExpr {
                name: "v".into(),
                type_ann: None,
            }),
            arms: vec![MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Literal {
                        value: serde_json::json!("red"),
                    },
                },
                body: BlockIR {
                    statements: Vec::new(),
                },
                union_type: None,
            }],
        };
        let out = fmt.format_expr(&expr);
        // JSON_DART formats strings with single quotes.
        assert!(
            out.contains("case 'red': return"),
            "dart match literal arm: {out}"
        );
    }
}
