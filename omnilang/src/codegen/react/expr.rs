// ═══════════════════════════════════════════════════════════
// Statement generation (TSX)
// ═══════════════════════════════════════════════════════════

use crate::codegen::shared::expr::{
    expr_is_typed_array, expr_is_typed_map, expr_is_typed_string, FormatExpr,
};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::CodeWriter;
use crate::codegen::{
    capitalize_first, try_format_stdlib_call, JsonFormatConfig, Target, JSON_REACT,
};
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::sync::Mutex;

/// React TSX expression formatter.
///
/// Uses JS-family syntax with `translate_builtin_call` for stdlib calls
/// and `translate_method_call` for method dispatch.
#[derive(Clone)]
pub(crate) struct ReactFormatExpr<'a> {
    pub ref_names: FxHashSet<String>,
    /// Identifier → replacement map (e.g. store signal `count` → `state.count`
    /// inside Zustand action bodies). Applied at identifier level only — string
    /// literals and object keys are never touched (C8 fix, HIGH #2).
    pub renames: FxHashMap<String, String>,
    /// Identifiers that shadow `renames` in every position — the store
    /// action's param names (fn_locals pattern, OmniView store.rs): an
    /// IdentifierExpr naming an action param is the param itself and must
    /// stay bare, never `state.<name>` (maintainability HIGH-1). Empty at
    /// component-codegen sites.
    pub param_names: FxHashSet<String>,
    /// Task 5.6 (deferral #61): names whose value may be undefined at an
    /// event-handler call site — event props are emitted as OPTIONAL props
    /// (`onX?: (value: string) => void`, the parent may not wire the event)
    /// and a signal initialized with the null literal may be null when
    /// invoked (`const f = null` hoisted shape). `format_call` emits the
    /// optional-call form `f?.(...)` for these; local fns/lib fns are
    /// always defined and stay bare `f(...)` (no noise).
    pub possibly_undefined: FxHashSet<String>,
    /// Review fix (Important #3): honest-degradation channel (ADR-054) for
    /// expression-position emissions that cannot be rendered faithfully —
    /// e.g. `navigate(dynamicPath, { query })` where the path is a runtime
    /// expression: the options are dropped (they can only be inlined into a
    /// literal path) and a warning is pushed instead of emitting broken TS.
    /// `None` at wrapper-function sites (no component context); component
    /// sites pass `Some(&self.warnings)`.
    pub warnings: Option<&'a Mutex<Vec<String>>>,
}

/// Extract a map-entry KEY as a plain string. After lowering, `{ params:
/// {...} }` option-object keys arrive as string-literal `ExprIR`s
/// (parser `key_to_string_literal`); identifier keys are kept as a
/// defensive fallback for hand-built IR. Anything else → None.
fn map_entry_key_str(k: &ExprIR) -> Option<String> {
    match k {
        ExprIR::LiteralExpr { value } => value.as_str().map(|s| s.to_string()),
        ExprIR::IdentifierExpr { name, .. } => Some(name.trim_start_matches('$').to_string()),
        _ => None,
    }
}

/// Build the `navigate(...)` argument from a template path plus the
/// params/query option maps (Task 6 — spec §2.8 navigate 内建):
///
/// ```text
/// navigate("/todos/:id", { params: { id: item } })  → "/todos/" + item
/// navigate("/todos", { query: { filter: "all" } })  → "/todos?filter=all"
/// navigate("/todos", { query: { filter: f } })      → "/todos?filter=" + f
/// navigate("/todos", { params: { id: item }, query: { filter: "all" } })
///                                                   → "/todos/" + item + "?filter=all"
/// ```
///
/// `:name` placeholders resolve against `params`; placeholders without a
/// params entry stay verbatim (the URL may fill them at runtime). Query
/// values that are string literals are inlined RAW (`filter=all`, never
/// `filter="all"`); any other value is concatenated as an expression.
/// Adjacent literal chunks merge so the emitted text stays a single
/// template where possible.
fn build_navigate_arg(
    path: &str,
    params: &FxHashMap<String, &ExprIR>,
    query: &[(String, &ExprIR)],
    fmt: &ReactFormatExpr<'_>,
) -> String {
    // 1) Path pieces: (is_literal, text) pairs — literal chunks and value
    //    expressions interleaved.
    let mut pieces: Vec<(bool, String)> = Vec::new();
    let mut lit = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ':' {
            let mut name = String::new();
            while let Some(&n) = chars.peek() {
                if n.is_ascii_alphanumeric() || n == '_' {
                    name.push(n);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Some(value) = params.get(&name) {
                pieces.push((true, std::mem::take(&mut lit)));
                pieces.push((false, fmt.format_expr(value)));
            } else {
                lit.push(':');
                lit.push_str(&name);
            }
        } else {
            lit.push(c);
        }
    }
    pieces.push((true, lit));
    // 2) Query pieces in declaration order (`?k=v&k2=v2`): string-literal
    //    values merge into the trailing literal chunk; expressions append
    //    a literal prefix then the value, preserving interleaving order.
    for (i, (k, v)) in query.iter().enumerate() {
        let sep = if i == 0 { '?' } else { '&' };
        match v {
            ExprIR::LiteralExpr { value } if value.is_string() => {
                if let Some((true, last)) = pieces.last_mut() {
                    last.push(sep);
                    last.push_str(k);
                    last.push('=');
                    last.push_str(value.as_str().unwrap_or_default());
                }
            }
            _ => {
                if let Some((true, last)) = pieces.last_mut() {
                    last.push_str(&format!("{sep}{k}="));
                } else {
                    pieces.push((true, format!("{sep}{k}=")));
                }
                pieces.push((false, fmt.format_expr(v)));
            }
        }
    }
    // 3) Quote literal chunks (JS double-quote convention), drop empty
    //    literals unless the whole expression is one, and join.
    let mut rendered: Vec<String> = pieces
        .iter()
        .map(|(is_lit, text)| {
            if *is_lit {
                format!("\"{text}\"")
            } else {
                text.clone()
            }
        })
        .collect();
    if rendered.len() > 1 {
        rendered.retain(|s| s != "\"\"");
    }
    if rendered.is_empty() {
        rendered.push("\"\"".to_string());
    }
    rendered.join(" + ")
}

impl<'a> FormatExpr for ReactFormatExpr<'a> {
    fn json_config(&self) -> &JsonFormatConfig {
        &JSON_REACT
    }

    /// Task 4 (@form 内建对象映射): 自由函数 `format_expr_ir_tsx_renamed_locals`
    /// 的 FieldAccess/OptionalChain 直写分支已映射 `form.valid` 等，但该映射
    /// 未覆盖语句上下文（fn 体经 `ReactStmtFormatter`）与复合表达式（UnaryOp/
    /// BinaryOp/Ternary 落入自由函数 fallback）。共享 trait 在 FieldAccessExpr
    /// 与 OptionalChainExpr 两个分支都先询问此钩子 —— 在此补上映射即修复
    /// 两处缺口（`if (!form.valid)`、class-map `!form.valid` 运行时 ReferenceError）。
    fn format_special_field_access(&self, object: &ExprIR, field: &str) -> Option<String> {
        if matches!(object, ExprIR::IdentifierExpr { name, .. } if name == "form") {
            if let Some(mapped) = form_builtin_field(field) {
                return Some(mapped.to_string());
            }
        }
        // `form.errors.<field>`（根为 `form`）→ `formErrors["<field>"]`
        form_errors_index_access(object, field)
    }

    /// Block expressions emit an IIFE so statements execute in expression position.
    /// Statements are formatted through the full React statement machinery
    /// (`const` lets, setter mutations, semicolons).
    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        if statements.is_empty() {
            return self.format_expr(tail);
        }
        let mut w = CodeWriter::new();
        let fmt = ReactStmtFormatter {
            expr: ReactFormatExpr {
                ref_names: self.ref_names.clone(),
                renames: self.renames.clone(),
                param_names: self.param_names.clone(),
                // Task 5.6: expression-position blocks have no component
                // context here — event-prop emits inside them stay bare
                // (no corpus instances; the computed-block body path uses
                // generate_statement_tsx, which carries the set).
                possibly_undefined: FxHashSet::default(),
                // Review fix (Important #3): keep the warning channel
                // flowing through expression-position blocks.
                warnings: self.warnings,
            },
            signal_names: FxHashSet::default(),
            store_signal_owners: FxHashMap::default(),
            possibly_undefined: FxHashSet::default(),
            // Expression-position blocks have no component context here —
            // no D1/D2 signals reachable (computed-block body path uses
            // generate_statement_tsx, which carries the sets).
            direct_write_signals: FxHashSet::default(),
            direct_write_applies: FxHashMap::default(),
        };
        for stmt in statements {
            fmt.format_stmt(&mut w, stmt);
        }
        let stmts = w.into_string();
        let stmts = stmts.trim_end().to_string();
        format!(
            "(() => {{ {} return {}; }})()",
            stmts,
            self.format_expr(tail)
        )
    }

    /// Strip `$` prefix from identifier names — `$` is an IR internal
    /// convention for store signal references and should not appear in
    /// generated TypeScript/React code.
    /// Renames (Zustand action bodies) apply first, then ref translation to
    /// `nameRef.current` for proper ref access in function bodies (ADR-036).
    fn format_identifier(&self, name: &str) -> String {
        let stripped = name.trim_start_matches('$');
        if self.param_names.contains(stripped) {
            return stripped.to_string();
        }
        if let Some(renamed) = self.renames.get(stripped) {
            return renamed.clone();
        }
        if self.ref_names.contains(stripped) {
            format!("{}Ref.current", stripped)
        } else {
            stripped.to_string()
        }
    }

    fn codegen_target(&self) -> Target {
        Target::React
    }

    fn strict_eq(&self) -> bool {
        true
    }

    fn null_fallback(&self) -> &str {
        "undefined"
    }

    fn format_closure(&self, params: &str, body: &str) -> String {
        if params.is_empty() {
            format!("(() => ({body}))")
        } else {
            // Prefix unused params with `_` to avoid ESLint no-unused-vars warnings.
            // Some IR-generated closures (e.g., sort comparators) include params
            // that aren't referenced in the closure body.
            let sanitized: Vec<String> = params
                .split(',')
                .map(|p| {
                    let p = p.trim();
                    if p.is_empty() {
                        String::new()
                    } else if body.contains(p) {
                        p.to_string()
                    } else {
                        format!("_{}", p)
                    }
                })
                .collect();
            let sanitized_params = sanitized.join(", ");
            format!("(({sanitized_params}) => ({body}))")
        }
    }

    fn format_call(&self, name: &str, args: &[String]) -> String {
        // Task 5.6 (deferral #61): a DIRECT call to a possibly-undefined
        // event handler (`onX(...)` as an expression statement — the
        // corpus form, vs. the `emit` keyword's EventEmitStmtIR handled in
        // `format_event_emit_stmt`) is a TS2722 possibly-undefined
        // invocation. Emit the optional-call form `onX?.(...)`; the name
        // can never collide with a stdlib/builtin (event props are
        // component-scoped identifiers), so the guard runs first.
        if self.possibly_undefined.contains(name) {
            return format!("{}?.({})", name, args.join(", "));
        }
        if let Some(native) = try_format_stdlib_call(name, args, Target::React) {
            return native;
        }
        translate_builtin_call(name, args)
    }

    /// Task 6 — `navigate(path, options)` / `back()` IR-aware translation
    /// (spec §2.8). The options object arrives as a `MapExpr` whose keys
    /// are RUNTIME string-literal ExprIRs — the string-level formatter
    /// would emit `{ params: {...} }` verbatim, which is both a TS error
    /// and wrong runtime semantics. Translate to flat template
    /// concatenations: `navigate("/todos/:id", { params: { id: item } })`
    /// → `navigate("/todos/" + item)`, `back()` → `navigate(-1)`.
    /// Dynamic (non-literal) paths with a params/query option map cannot be
    /// inlined (W1 makes dynamic paths legal) — emitting the MapExpr as a
    /// plain argument would be a TS2353 type error, so the options are
    /// dropped and a CodegenWarning pushed (honest degradation, ADR-054:
    /// never silent, never broken). A dynamic path WITHOUT options still
    /// falls through to the default emission — `navigate(expr)` is valid TS.
    fn format_call_with_ir(
        &self,
        name: &str,
        args: &[ExprIR],
        arg_strs: &[String],
    ) -> Option<String> {
        if name == "back" && args.is_empty() {
            return Some("navigate(-1)".to_string());
        }
        // bp("md") → `bp_768` hoisted useMediaQuery variable — the semantic
        // layer (validate_breakpoints) already rewrote the argument to a
        // numeric literal (batch 3, spec §2.2; three-target dispatch contract).
        if name == "bp" {
            if let Some(ExprIR::LiteralExpr { value }) = args.first() {
                if let Some(n) = value.as_u64() {
                    return Some(format!("bp_{n}"));
                }
            }
            return None;
        }
        if name != "navigate" || args.is_empty() {
            return None;
        }
        let path = match &args[0] {
            ExprIR::LiteralExpr { value } if value.is_string() => value.as_str()?,
            _ if matches!(args.get(1), Some(ExprIR::MapExpr { .. })) => {
                // Dynamic path + options — options cannot be inlined into a
                // runtime expression; drop them and warn (honest degradation).
                if let Some(warnings) = self.warnings {
                    warnings.lock().unwrap().push(
                        "navigate 的动态路径无法内联 params/query 选项——选项已丢弃，仅保留路径调用（诚实降级，ADR-054）"
                            .into(),
                    );
                }
                return Some(format!("navigate({})", arg_strs[0]));
            }
            _ => return None,
        };
        // Optional second arg: option MapExpr — `{ params: {...}, query:
        // {...} }` with string-literal keys.
        let mut params: FxHashMap<String, &ExprIR> = FxHashMap::default();
        let mut query: Vec<(String, &ExprIR)> = Vec::new();
        if let Some(ExprIR::MapExpr { entries }) = args.get(1) {
            for (k, v) in entries {
                match map_entry_key_str(k).as_deref() {
                    Some("params") => {
                        if let ExprIR::MapExpr { entries: p_entries } = v {
                            for (pk, pv) in p_entries {
                                if let Some(pk) = map_entry_key_str(pk) {
                                    params.insert(pk, pv);
                                }
                            }
                        }
                    }
                    Some("query") => {
                        if let ExprIR::MapExpr { entries: q_entries } = v {
                            for (qk, qv) in q_entries {
                                if let Some(qk) = map_entry_key_str(qk) {
                                    query.push((qk, qv));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // Plain literal path without options → passthrough as-is.
        if params.is_empty() && query.is_empty() {
            return Some(format!("navigate({})", arg_strs[0]));
        }
        let arg = build_navigate_arg(path, &params, &query, self);
        Some(format!("navigate({arg})"))
    }

    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        // Store action call through a slice selector: `TodoStore.add(x)` →
        // `add(x)` when the store-field rename map carries that field.
        // The obj string is the formatted receiver — for unrenamed store
        // refs it is exactly the store alias, matching the dotted rename key.
        if let Some(local) = store_field_rename(&self.renames, obj, method) {
            return format!("{}({})", local, args.join(", "));
        }
        translate_method_call(obj, method, args, None)
    }

    /// IR-aware method table (shared trait hook, same shape as the Dart
    /// override): the semantic symbol table's static type annotation on the
    /// receiver decides `.len()` emission — typed Array/String → direct
    /// `.length`; typed Map → `Object.keys(...).length` (`.length` is
    /// undefined on a plain object AND fails tsc on a typed Record receiver).
    /// Receivers without a static type fall through to the string-level
    /// table, which keeps the runtime dispatch (semantically identical).
    fn format_method_call_with_ir(
        &self,
        object: &ExprIR,
        method: &str,
        _args: &[ExprIR],
        is_optional: bool,
        obj_str: &str,
        arg_strs: &[String],
    ) -> String {
        if method == "len" && !is_optional {
            if expr_is_typed_array(object) || expr_is_typed_string(object) {
                return format!("{obj_str}.length");
            }
            if expr_is_typed_map(object) {
                return format!("Object.keys({obj_str}).length");
            }
        }
        if is_optional {
            self.format_optional_method_call(obj_str, method, arg_strs)
        } else {
            self.format_method_call(obj_str, method, arg_strs)
        }
    }

    /// Optional-chain method call (`obj?.method(args)`) — translate known
    /// stdlib methods exactly like plain calls while PRESERVING the `?.`
    /// so null-safety survives translation. The shared default emits
    /// `?.method()` verbatim, which fails tsc for translated names
    /// (`user?.len()` → TS2339 'len' on string — exposed by the fresh
    /// learn-omniview regen, task 2.1 verification).
    fn format_optional_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        match method {
            // len → length is a PROPERTY access — the optional chain lands
            // on the receiver: `user?.len()` → `user?.length`.
            "len" => format!("{obj}?.length"),
            // Everything else keeps the shared default shape.
            _ => format!("{obj}?.{method}({})", args.join(", ")),
        }
    }

    /// Store reads/actions formatted through the shared dispatch — when the
    /// store is subscribed per field, emit the slice local instead of the
    /// qualified `Store.field` name (Task 3.3, perf HIGH-1/H2).
    fn format_store_field_access(
        &self,
        store_alias: &str,
        field_name: &str,
        args: &[String],
    ) -> String {
        if let Some(local) = store_field_rename(&self.renames, store_alias, field_name) {
            if args.is_empty() {
                return local;
            }
            return format!("{}({})", local, args.join(", "));
        }
        if args.is_empty() {
            format!("{store_alias}.{field_name}")
        } else {
            format!("{store_alias}.{field_name}({})", args.join(", "))
        }
    }

    /// `Store.field` reads → slice local (`items` instead of `TodoStore.items`).
    fn format_field_access(&self, obj: &str, field: &str) -> String {
        if let Some(local) = store_field_rename(&self.renames, obj, field) {
            return local;
        }
        format!("{}.{}", obj, field)
    }

    /// `Store?.field` optional-chain reads → slice local too (the store hook
    /// object is never null, so the `?.` is redundant once sliced).
    fn format_optional_chain(&self, object: &str, field: &str) -> String {
        if let Some(local) = store_field_rename(&self.renames, object, field) {
            return local;
        }
        format!("{object}?.{field}")
    }

    /// Field access on a REF-OP RESULT (`rectBox.measure().y`, `sizeBox
    /// .measure().width`) — ref-op outputs carry a `?? fallback` (zero-rect
    /// literal, `0`, `''`), and member access binds TIGHTER than `??`, so a
    /// bare concat (`A ?? {...}.y`) would read the fallback literal's field
    /// (always `undefined`) instead of the measured result. Parenthesize the
    /// coalesced result before appending the field — same shape rewrite as
    /// Flutter's `format_ref_op_field_access` (Offset `dx`/`dy` parity),
    /// here for JS precedence parity.
    fn format_ref_op_field_access(&self, object: &ExprIR, field: &str) -> Option<String> {
        if let ExprIR::RefOpExpr { ref_name, op } = object {
            Some(format!("({}).{}", self.format_ref_op(ref_name, op), field))
        } else {
            None
        }
    }

    /// Semantic ref ops map to native DOM APIs on `nameRef.current`
    /// (ADR-036 Phase 2). All calls use optional chaining so a not-yet-mounted
    /// ref degrades to a no-op instead of throwing.
    fn format_ref_op(&self, ref_name: &str, op: &RefOpIR) -> String {
        let base = format!("{}Ref.current", ref_name);
        match op {
            RefOpIR::Focus => format!("{base}?.focus()"),
            RefOpIR::Blur => format!("{base}?.blur()"),
            RefOpIR::ScrollTo { x, y } => {
                format_ref_scroll(&base, self, x.as_deref(), y.as_deref())
            }
            RefOpIR::ScrollBy { dx, dy } => {
                format_ref_scroll_by(&base, self, dx.as_deref(), dy.as_deref())
            }
            RefOpIR::Measure { what } => match what {
                // Field-reads (`ref.width`) and nested ref-ops
                // (`scrollTo(0, rect.measure().y)`) flow straight into signal
                // setters / scroll args — the `?? 0` / zero-rect fallbacks
                // keep the result's type non-null so TS stays clean (same
                // `?? 0` semantics as the Flutter `_omni_measure` wrapper).
                MeasureWhat::Position => format!(
                    "{base}?.getBoundingClientRect() ?? {{ left: 0, top: 0, right: 0, bottom: 0, x: 0, y: 0, width: 0, height: 0 }}"
                ),
                MeasureWhat::Width => format!("{base}?.getBoundingClientRect().width ?? 0"),
                MeasureWhat::Height => format!("{base}?.getBoundingClientRect().height ?? 0"),
            },
            RefOpIR::Select => format!("{base}?.select()"),
            RefOpIR::SetSelection { start, end } => format!(
                "{base}?.setSelectionRange({}, {})",
                self.format_expr(start),
                self.format_expr(end)
            ),
            RefOpIR::Value => format!("{base}?.value ?? ''"),
            RefOpIR::CapturePointer => {
                // Parameterless DSL form (design D3) — capture the ACTIVE
                // pointer. Ref ops can be called from fns/effects/lifecycles
                // where no event is in scope, so the module-scope tracker
                // emitted by gen_component_tsx records the most recent
                // pointerdown's id (capture-phase listener).
                format!("{base}?.setPointerCapture(__omni_activePointerId!)")
            }
            RefOpIR::ReleasePointer => {
                format!("{base}?.releasePointerCapture(__omni_activePointerId!)")
            }
            // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
            #[allow(unreachable_patterns)]
            _ => format!("/* RefOp::{op:?} on {ref_name} */"),
        }
    }

    /// Variant literals (tagged-union constructors) emit
    /// `{ "_variant": "Star" as const, "level": 5 }` — the `as const` keeps
    /// the DISCRIMINANT a string-LITERAL type. The declared union type
    /// (types.rs) discriminates on `_variant: "Star"` and the match emitter
    /// narrows with `_v._variant === "Star"`, so a constructor that widens
    /// the discriminant to plain `string` is not assignable to the union
    /// (TS2345 — DataFetching's `fetch_demo` return into `error_message`).
    /// React-only override: the shared default stays for the JS-family
    /// targets whose output is not type-checked (`as const` is TS syntax).
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
        entries.insert(0, format!("\"_variant\": \"{variant}\" as const"));
        format!("{{{}}}", entries.join(", "))
    }

    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let cases: Vec<String> = arms
            .iter()
            .map(|arm| {
                let body = self.format_block_as_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, bindings } => {
                        let destructure = if bindings.is_empty() {
                            String::new()
                        } else {
                            // Only destructure the bindings the arm body
                            // actually uses. Unused bindings were previously
                            // emitted `_`-prefixed (`const { _data } = _v;`),
                            // which the `_`-prefix convention doesn't cover
                            // for destructured vars — @typescript-eslint/
                            // no-unused-vars still flagged them (task 2.11).
                            // A destructure with NO used binding is skipped
                            // entirely (the `_variant` guard needs no
                            // bindings).
                            let names: Vec<String> = bindings
                                .iter()
                                .filter(|b| body.contains(b.as_str()))
                                .cloned()
                                .collect();
                            if names.is_empty() {
                                String::new()
                            } else {
                                format!("const {{ {} }} = _v; ", names.join(", "))
                            }
                        };
                        format!(
                            "if (_v._variant === \"{}\") {{ {}return {}; }}",
                            name, destructure, body
                        )
                    }
                    MatchPatternKindIR::Wildcard => {
                        format!("return {};", body)
                    }
                    MatchPatternKindIR::Literal { value } => {
                        // Literal patterns match the subject value itself —
                        // primitives carry no `_variant` field (unlike tagged
                        // union values), so `_v._variant === 0` never fires.
                        let v = crate::codegen::format_json_value(value, &JSON_REACT);
                        format!("if (_v === {}) {{ return {}; }}", v, body)
                    }
                }
            })
            .collect();
        // A match WITHOUT a wildcard arm must not fall off the IIFE: tsc
        // treats an if-chain's implicit fall-through as reachable even when
        // every union member is consumed, so the IIFE's inferred return type
        // would include `undefined` (TS2345 on every consumer — the
        // DataFetching/MatchBreadthDemo cascade of deferral #49). The
        // throwing tail mirrors the Flutter target's `default:
        // throw StateError(...)` contract — unreachable at runtime for
        // exhaustive matches, and a loud failure instead of silent undefined
        // for genuinely non-exhaustive ones.
        let has_wildcard = arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, MatchPatternKindIR::Wildcard));
        if has_wildcard {
            format!("(() => {{ const _v = {matched}; {} }})()", cases.join(" "))
        } else {
            format!(
                "(() => {{ const _v = {matched}; {} throw new Error(\"match: no arm matched\"); }})()",
                cases.join(" ")
            )
        }
    }
}

/// Format `ref.scrollTo({ left: x, top: y })` — only axes provided in the
/// source call are included in the options object.
fn format_ref_scroll(
    base: &str,
    fmt: &ReactFormatExpr<'_>,
    x: Option<&ExprIR>,
    y: Option<&ExprIR>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(x) = x {
        parts.push(format!("left: {}", fmt.format_expr(x)));
    }
    if let Some(y) = y {
        parts.push(format!("top: {}", fmt.format_expr(y)));
    }
    format!("{base}?.scrollTo({{ {} }})", parts.join(", "))
}

/// Format `ref.scrollBy({ left: dx, top: dy })` — relative scroll delta.
fn format_ref_scroll_by(
    base: &str,
    fmt: &ReactFormatExpr<'_>,
    dx: Option<&ExprIR>,
    dy: Option<&ExprIR>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(dx) = dx {
        parts.push(format!("left: {}", fmt.format_expr(dx)));
    }
    if let Some(dy) = dy {
        parts.push(format!("top: {}", fmt.format_expr(dy)));
    }
    format!("{base}?.scrollBy({{ {} }})", parts.join(", "))
}

/// React TSX statement formatter.
///
/// Uses `signal_names` to determine mutation strategy (setter calls vs direct
/// assignment) and `store_signal_owners` (store signal name → store ref name)
/// so store-signal mutations emit `useStoreName.setState(...)` (C7).
pub(super) struct ReactStmtFormatter<'a> {
    pub expr: ReactFormatExpr<'a>,
    pub signal_names: FxHashSet<String>,
    /// Maps store signal names to their owning store ref (e.g. `count` → `AppStore`).
    pub store_signal_owners: FxHashMap<String, String>,
    /// Task 5.6 (deferral #61): names that may be undefined at an
    /// event-handler call site (event props, null-initial signals) — the
    /// `emit onX(...)` keyword form emits `onX?.(...)` for these. Mirrors
    /// `ReactFormatExpr.possibly_undefined` for the direct-call form.
    pub possibly_undefined: FxHashSet<String>,
    /// Phase 1 (D1/D2): signal names stored in `useRef` cells — mutations
    /// write `{name}Ref.current = ...` instead of a useState setter.
    pub direct_write_signals: FxHashSet<String>,
    /// Phase 1 (D1): signal → apply lines injected after its mutations
    /// (each apply re-reads the CURRENT ref values of all entry signals).
    pub direct_write_applies: FxHashMap<String, Vec<String>>,
}

impl<'a> FormatStmt for ReactStmtFormatter<'a> {
    type ExprFormatter = ReactFormatExpr<'a>;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn use_const_for_immutable(&self) -> bool {
        // Task 5.1 (deferral #36): LetStmtIR now carries `is_mut`, so
        // immutable bindings emit `const` while `let mut` bindings emit
        // `let` via the shared trait (`format_let_stmt` branches on the
        // flag). The harness eslint config previously disabled prefer-const
        // for generated output (BUG-A workaround, 2.1) — re-enabled with
        // this task.
        true
    }

    fn for_in_keyword(&self) -> &str {
        "const"
    }

    /// Override format_expr_stmt to ensure store action references are always called with ().
    /// Fixes ESLint `no-unused-expressions` when a store action like `NotificationStore.clear`
    /// is referenced without parentheses in an expression statement.
    fn format_expr_stmt(&self, w: &mut CodeWriter, expr: &ExprIR) {
        let line = self.expr_fmt().format_expr(expr);
        if line != self.null_value() {
            // Detect FieldAccessExpr on a store reference — always add () for actions
            let needs_parens = matches!(expr, ExprIR::FieldAccessExpr { object, .. }
                if matches!(object.as_ref(), ExprIR::IdentifierExpr { name, .. }
                    if self.signal_names.contains(name.as_str())));
            if self.needs_semicolon() {
                if needs_parens {
                    w.emit_fmt(format_args!("{line}();"));
                } else {
                    w.emit_fmt(format_args!("{line};"));
                }
            } else {
                w.emit(&line);
            }
        }
    }

    fn format_let_type_ann(&self, ty: &Option<TypeIR>) -> String {
        ty.as_ref()
            .map(|t| {
                let ann = JsTypeFormatter.format_type(t.kind.as_str(), &t.type_args);
                if ann.is_empty() {
                    String::new()
                } else {
                    format!(": {}", ann)
                }
            })
            .unwrap_or_default()
    }

    fn format_event_emit_stmt(&self, w: &mut CodeWriter, event_name: &str, args: &[ExprIR]) {
        let arg_str = if args.is_empty() {
            String::from("()")
        } else {
            let formatted: Vec<String> = args.iter().map(|a| self.expr.format_expr(a)).collect();
            format!("({})", formatted.join(", "))
        };
        // Task 5.6 (deferral #61): the `emit onX(...)` KEYWORD form. Event
        // props are optional props, so a bare `onX(...)` is a TS2722 —
        // emit `onX?.(...)` when the name may be undefined (local fn
        // targets stay bare `f(...)`).
        if self.possibly_undefined.contains(event_name) {
            w.emit_fmt(format_args!("{}?.{};", event_name, arg_str));
        } else {
            w.emit_fmt(format_args!("{}{};", event_name, arg_str));
        }
    }

    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        match intent {
            MutationIntent::DirectAssign { target, value, .. } => {
                let target_str = &target.name;
                // Store signal → Zustand setState on the imported store hook (C7)
                if let Some(owner) = self.store_signal_owners.get(target_str.as_str()) {
                    // M2: a mutation whose value reads the SAME store signal
                    // it writes must use the functional updater form —
                    // `setState({ count: AppStore.count + 1 })` closes over
                    // the slice/snapshot and loses updates under React
                    // batching / StrictMode double-invoke.
                    if let Some(v) = value {
                        if expr_reads_store_field(v, owner, target_str) {
                            let param = pick_updater_param("s", v);
                            let mut fmt = self.expr.clone();
                            fmt.renames.insert(
                                format!("{}.{}", owner, target_str),
                                format!("{}.{}", param, target_str),
                            );
                            let val = fmt.format_expr(v);
                            w.emit_fmt(format_args!(
                                "use{}.setState(({}) => ({{ {}: {} }}));",
                                owner, param, target_str, val
                            ));
                            return;
                        }
                    }
                    let amount = value
                        .as_ref()
                        .map(|v| self.expr.format_expr(v))
                        .unwrap_or_else(|| "0".into());
                    w.emit_fmt(format_args!(
                        "use{}.setState({{ {}: {} }});",
                        owner, target_str, amount
                    ));
                } else if self.direct_write_signals.contains(target_str.as_str()) {
                    // Phase 1 (D1/D2): 直写域信号存于 useRef 单元格——mutation
                    // 写 `{name}Ref.current`，D1 信号随后注入 apply 直写
                    // （读全部条目信号的当前 ref 值，与 DOM 保持同步）。
                    let amount = value
                        .as_ref()
                        .map(|v| self.expr.format_expr(v))
                        .unwrap_or_else(|| "0".into());
                    w.emit_fmt(format_args!("{}Ref.current = {};", target_str, amount));
                    if let Some(applies) = self.direct_write_applies.get(target_str.as_str()) {
                        for line in applies {
                            w.emit_fmt(format_args!("{}", line));
                        }
                    }
                } else if self.signal_names.contains(target_str.as_str()) {
                    let setter = format!("set{}", capitalize_first(target_str));
                    // M2: same-signal self-read (`count = count + 1`) emits
                    // the functional updater `setCount((c) => c + 1)` —
                    // `setCount(count + 1)` reads a stale closure value under
                    // batching (review M2). Other values keep the direct form.
                    if let Some(v) = value {
                        // Same-signal read detection: collect every identifier
                        // the value references (target's own occurrences are
                        // replaced by the param, so they are not collisions).
                        let mut read_ids = FxHashSet::default();
                        super::component::collect_expr_identifiers(v, &mut read_ids);
                        if read_ids.contains(target_str) {
                            let param = pick_updater_param(
                                &target_str
                                    .chars()
                                    .next()
                                    .map(|c| c.to_string())
                                    .unwrap_or_else(|| "v".into()),
                                v,
                            );
                            let mut fmt = self.expr.clone();
                            fmt.renames.insert(target_str.clone(), param.clone());
                            let val = fmt.format_expr(v);
                            // M2 (struct/map value): an object literal after
                            // `=>` would parse as an arrow block body — wrap
                            // it, mirroring the store path's `({{ ... }})`
                            // above (StructExpr emits `{ "k": v, ... }`).
                            let val = if val.starts_with('{') {
                                format!("({val})")
                            } else {
                                val
                            };
                            w.emit_fmt(format_args!("{}(({}) => {});", setter, param, val));
                            return;
                        }
                    }
                    let amount = value
                        .as_ref()
                        .map(|v| self.expr.format_expr(v))
                        .unwrap_or_else(|| "0".into());
                    w.emit_fmt(format_args!("{}({});", setter, amount));
                } else {
                    let amount = value
                        .as_ref()
                        .map(|v| self.expr.format_expr(v))
                        .unwrap_or_else(|| "0".into());
                    w.emit_fmt(format_args!("{} = {};", target_str, amount));
                }
            }
            _ => {
                // Increment / Decrement / Multiply / Divide / Modulo / Push / Pop
                self.format_compound_mutation(w, intent);
            }
        }
    }
}

/// Generate Zustand setState / setter / direct assignment for compound mutations.
macro_rules! compound_op {
    ($w:ident, $target_str:ident, $amt:ident, $owner:ident, $use_setter:ident, $op:tt) => {{
        if let Some(owner) = $owner {
            $w.emit_fmt(format_args!(
                "use{}.setState((s) => ({{ {}: s.{} {} {} }}));",
                owner,
                $target_str,
                $target_str,
                stringify!($op),
                $amt
            ));
        } else if $use_setter {
            let setter = format!("set{}", capitalize_first($target_str));
            $w.emit_fmt(format_args!(
                "{}(prev => prev {} {});",
                setter,
                stringify!($op),
                $amt
            ));
        } else {
            $w.emit_fmt(format_args!(
                "{} = {} {} {};",
                $target_str,
                $target_str,
                stringify!($op),
                $amt
            ));
        }
    }};
}

impl<'a> ReactStmtFormatter<'a> {
    /// Format compound mutations (Increment, Decrement, Push, Pop, etc.).
    /// Store signals (via `store_signal_owners`) emit `useStoreName.setState(...)`,
    /// component signals emit setter calls, everything else is a bare assignment.
    fn format_compound_mutation(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        let (target, amount_expr, item_expr) = match intent {
            MutationIntent::Increment { target, amount } => (target, amount.as_deref(), None),
            MutationIntent::Decrement { target, amount } => (target, amount.as_deref(), None),
            MutationIntent::Multiply { target, amount } => (target, amount.as_deref(), None),
            MutationIntent::Divide { target, amount } => (target, amount.as_deref(), None),
            MutationIntent::Modulo { target, amount } => (target, amount.as_deref(), None),
            MutationIntent::Push { target, item } => (target, None, item.as_deref()),
            MutationIntent::Pop { target } => (target, None, None),
            _ => return,
        };

        let target_str = &target.name;
        // Phase 1 (D1/D2): 直写域信号的 compound mutation 同样写 ref + 注入
        // apply——宏展开路径会发射 setter 调用，而 D1/D2 无 useState setter
        // （未声明名 TS2304）。
        if self.direct_write_signals.contains(target_str.as_str()) {
            let amt = amount_expr
                .map(|v| self.expr.format_expr(v))
                .unwrap_or_else(|| "0".into());
            match intent {
                MutationIntent::Increment { .. } => {
                    w.emit_fmt(format_args!("{}Ref.current += {};", target_str, amt));
                }
                MutationIntent::Decrement { .. } => {
                    w.emit_fmt(format_args!("{}Ref.current -= {};", target_str, amt));
                }
                MutationIntent::Multiply { .. } => {
                    w.emit_fmt(format_args!("{}Ref.current *= {};", target_str, amt));
                }
                MutationIntent::Divide { .. } => {
                    w.emit_fmt(format_args!("{}Ref.current /= {};", target_str, amt));
                }
                MutationIntent::Modulo { .. } => {
                    w.emit_fmt(format_args!("{}Ref.current %= {};", target_str, amt));
                }
                MutationIntent::Push { .. } => {
                    let it = item_expr
                        .map(|v| self.expr.format_expr(v))
                        .unwrap_or_else(|| "undefined".into());
                    w.emit_fmt(format_args!(
                        "{}Ref.current = [...{}Ref.current, {}];",
                        target_str, target_str, it
                    ));
                }
                MutationIntent::Pop { .. } => {
                    w.emit_fmt(format_args!(
                        "{}Ref.current = {}Ref.current.slice(0, -1);",
                        target_str, target_str
                    ));
                }
                _ => {}
            }
            if let Some(applies) = self.direct_write_applies.get(target_str.as_str()) {
                for line in applies {
                    w.emit_fmt(format_args!("{}", line));
                }
            }
            return;
        }
        let owner = self
            .store_signal_owners
            .get(target_str.as_str())
            .map(|s| s.as_str());
        let use_setter = self.signal_names.contains(target_str.as_str());

        let amt = amount_expr
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| "0".into());

        match intent {
            MutationIntent::Increment { .. } => {
                compound_op!(w, target_str, amt, owner, use_setter, +);
            }
            MutationIntent::Decrement { .. } => {
                compound_op!(w, target_str, amt, owner, use_setter, -);
            }
            MutationIntent::Multiply { .. } => {
                compound_op!(w, target_str, amt, owner, use_setter, *);
            }
            MutationIntent::Divide { .. } => {
                compound_op!(w, target_str, amt, owner, use_setter, /);
            }
            MutationIntent::Modulo { .. } => {
                compound_op!(w, target_str, amt, owner, use_setter, %);
            }
            MutationIntent::Push { .. } => {
                let it = item_expr
                    .map(|v| self.expr.format_expr(v))
                    .unwrap_or_else(|| "undefined".into());
                if let Some(owner) = owner {
                    w.emit_fmt(format_args!(
                        "use{}.setState((s) => ({{ {}: [...s.{}, {}] }}));",
                        owner, target_str, target_str, it
                    ));
                } else if use_setter {
                    let setter = format!("set{}", capitalize_first(target_str));
                    w.emit_fmt(format_args!("{}(prev => [...prev, {}]);", setter, it));
                } else {
                    w.emit_fmt(format_args!(
                        "{} = [...{}, {}];",
                        target_str, target_str, it
                    ));
                }
            }
            MutationIntent::Pop { .. } => {
                if let Some(owner) = owner {
                    w.emit_fmt(format_args!(
                        "use{}.setState((s) => ({{ {}: s.{}.slice(0, -1) }}));",
                        owner, target_str, target_str
                    ));
                } else if use_setter {
                    let setter = format!("set{}", capitalize_first(target_str));
                    w.emit_fmt(format_args!("{}(prev => prev.slice(0, -1));", setter));
                } else {
                    w.emit_fmt(format_args!(
                        "{} = {}.slice(0, -1);",
                        target_str, target_str
                    ));
                }
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Task 3.5 M2: functional-updater detection & param naming
// ═══════════════════════════════════════════════════════════

/// Whether the expression tree reads the store signal `store.field` (M2
/// store-signal detection — covers the property form and the semantic
/// field-access form).
fn expr_reads_store_field(expr: &ExprIR, store: &str, field: &str) -> bool {
    match expr {
        ExprIR::FieldAccessExpr {
            object, field: f, ..
        } => {
            (f == field
                && matches!(object.as_ref(), ExprIR::IdentifierExpr { name, .. } if name == store))
                || expr_reads_store_field(object, store, field)
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            (store_alias == store && field_name == field)
                || call_args
                    .iter()
                    .any(|a| expr_reads_store_field(a, store, field))
        }
        ExprIR::IdentifierExpr { .. } => false,
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_reads_store_field(left, store, field)
                || expr_reads_store_field(right, store, field)
        }
        ExprIR::UnaryOpExpr { operand, .. } | ExprIR::AwaitExpr { operand } => {
            expr_reads_store_field(operand, store, field)
        }
        ExprIR::CallExpr { args, .. } => {
            args.iter().any(|a| expr_reads_store_field(a, store, field))
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_reads_store_field(object, store, field)
                || args.iter().any(|a| expr_reads_store_field(a, store, field))
        }
        ExprIR::OptionalChainExpr { object, .. } => expr_reads_store_field(object, store, field),
        ExprIR::IndexExpr { object, index } => {
            expr_reads_store_field(object, store, field)
                || expr_reads_store_field(index, store, field)
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_reads_store_field(cond, store, field)
                || expr_reads_store_field(then_expr, store, field)
                || expr_reads_store_field(else_expr, store, field)
        }
        ExprIR::ArrayExpr { elements } => elements
            .iter()
            .any(|e| expr_reads_store_field(e, store, field)),
        ExprIR::MapExpr { entries } => entries.iter().any(|(k, v)| {
            expr_reads_store_field(k, store, field) || expr_reads_store_field(v, store, field)
        }),
        ExprIR::ClosureExpr { body, .. } => expr_reads_store_field(body, store, field),
        ExprIR::BlockExpr { statements, tail } => {
            statements
                .iter()
                .any(|s| stmts_read_store_field(s, store, field))
                || expr_reads_store_field(tail, store, field)
        }
        ExprIR::MatchExpr { matched, arms } => {
            expr_reads_store_field(matched, store, field)
                || arms.iter().any(|arm| {
                    arm.body
                        .statements
                        .iter()
                        .any(|s| stmts_read_store_field(s, store, field))
                })
        }
        ExprIR::VariantLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_reads_store_field(v, store, field)),
        ExprIR::StructExpr { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_reads_store_field(v, store, field)),
        ExprIR::SpreadExpr { operand } => expr_reads_store_field(operand, store, field),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            expr_reads_store_field(subject, store, field)
                || expr_reads_store_field(predicate, store, field)
        }
        ExprIR::RefOpExpr { op, .. } => expr_ref_op_reads_store_field(op, store, field),
        ExprIR::LiteralExpr { .. } => false,
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

fn stmts_read_store_field(stmt: &StatementIR, store: &str, field: &str) -> bool {
    match stmt {
        StatementIR::ExprStmtIR { expr } => expr_reads_store_field(expr, store, field),
        StatementIR::LetStmtIR { value, .. } => value
            .as_ref()
            .is_some_and(|v| expr_reads_store_field(v, store, field)),
        StatementIR::ReturnStmtIR { value } => value
            .as_ref()
            .is_some_and(|v| expr_reads_store_field(v, store, field)),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            expr_reads_store_field(condition, store, field)
                || then_body
                    .statements
                    .iter()
                    .any(|s| stmts_read_store_field(s, store, field))
                || else_ifs.iter().any(|(c, b)| {
                    expr_reads_store_field(c, store, field)
                        || b.statements
                            .iter()
                            .any(|s| stmts_read_store_field(s, store, field))
                })
                || else_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| stmts_read_store_field(s, store, field))
                })
        }
        StatementIR::WhileStmtIR { condition, body } => {
            expr_reads_store_field(condition, store, field)
                || body
                    .statements
                    .iter()
                    .any(|s| stmts_read_store_field(s, store, field))
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            expr_reads_store_field(iterable, store, field)
                || body
                    .statements
                    .iter()
                    .any(|s| stmts_read_store_field(s, store, field))
        }
        StatementIR::BlockStmtIR { body } => body
            .statements
            .iter()
            .any(|s| stmts_read_store_field(s, store, field)),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body
                .statements
                .iter()
                .any(|s| stmts_read_store_field(s, store, field))
                || catch_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| stmts_read_store_field(s, store, field))
                })
                || finally_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| stmts_read_store_field(s, store, field))
                })
        }
        StatementIR::ThrowStmtIR { expr } => expr_reads_store_field(expr, store, field),
        StatementIR::EventEmitStmtIR { args, .. } => {
            args.iter().any(|a| expr_reads_store_field(a, store, field))
        }
        StatementIR::MutationStmt { intent } => intent
            .as_ref()
            .is_some_and(|i| mutation_reads_store_field(i, store, field)),
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => false,
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

fn mutation_reads_store_field(intent: &MutationIntent, store: &str, field: &str) -> bool {
    match intent {
        MutationIntent::DirectAssign { value, .. } => value
            .as_ref()
            .is_some_and(|v| expr_reads_store_field(v, store, field)),
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => amount
            .as_ref()
            .is_some_and(|v| expr_reads_store_field(v, store, field)),
        MutationIntent::Push { item, .. } => item
            .as_ref()
            .is_some_and(|v| expr_reads_store_field(v, store, field)),
        MutationIntent::Pop { .. } => false,
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

fn expr_ref_op_reads_store_field(op: &RefOpIR, store: &str, field: &str) -> bool {
    match op {
        RefOpIR::Focus | RefOpIR::Blur | RefOpIR::CapturePointer | RefOpIR::ReleasePointer => false,
        RefOpIR::ScrollTo { x, y } | RefOpIR::ScrollBy { dx: x, dy: y } => {
            x.as_ref()
                .is_some_and(|e| expr_reads_store_field(e, store, field))
                || y.as_ref()
                    .is_some_and(|e| expr_reads_store_field(e, store, field))
        }
        RefOpIR::SetSelection { start, end } => {
            expr_reads_store_field(start, store, field) || expr_reads_store_field(end, store, field)
        }
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Pick the functional-updater parameter name: start from `base` and bump
/// (`c1`, `c2`, ...) while it collides with identifiers the value
/// expression keeps. The written signal itself is replaced by the param, so
/// its own occurrences never count as collisions.
fn pick_updater_param(base: &str, value: &ExprIR) -> String {
    let mut used = FxHashSet::default();
    super::component::collect_expr_identifiers(value, &mut used);
    if !used.contains(base) {
        return base.to_string();
    }
    for i in 1.. {
        let cand = format!("{}{}", base, i);
        if !used.contains(&cand) {
            return cand;
        }
    }
    unreachable!("updater param search always terminates")
}

/// The signal name a mutation writes (mirrors component/mod.rs's
/// `mutation_target_name`) — used by the task 2.11 write-only DCE.
fn mutation_target_name(intent: &MutationIntent) -> Option<&str> {
    match intent {
        MutationIntent::DirectAssign { target, .. }
        | MutationIntent::Increment { target, .. }
        | MutationIntent::Decrement { target, .. }
        | MutationIntent::Multiply { target, .. }
        | MutationIntent::Divide { target, .. }
        | MutationIntent::Modulo { target, .. }
        | MutationIntent::Push { target, .. }
        | MutationIntent::Pop { target } => Some(&target.name),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    }
}

/// The owner identifier of an `owner.field = value` write (the store alias
/// in `Store.field = v`, the ref name in `myRef.value = x`) — `None` for
/// plain signal/local writes and for intents that cannot carry an owner.
/// Discriminates component-signal mutations from store/ref field writes in
/// the write-only DCE (review I1).
fn mutation_field_object(intent: &MutationIntent) -> Option<&str> {
    match intent {
        MutationIntent::DirectAssign { field_object, .. } => field_object.as_deref(),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

/// The value expression a mutation writes — `None` for value-less intents
/// (`Pop`). `pub(super)`: also used by the component emitter's write-only
/// DCE restriction (collect_call_mutation_targets, task 2.11).
pub(super) fn mutation_value_expr(intent: &MutationIntent) -> Option<&ExprIR> {
    match intent {
        MutationIntent::DirectAssign { value, .. } => value.as_deref(),
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => amount.as_deref(),
        MutationIntent::Push { item, .. } => item.as_deref(),
        MutationIntent::Pop { .. } => None,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    }
}

/// Whether an expression subtree contains a call (or store action call, or
/// await) — a pruned mutation whose value carries calls must keep the value
/// as a bare expression statement, or the calls' side effects would be
/// silently discarded. `pub(super)`: also used by the component emitter's
/// write-only DCE restriction (collect_call_mutation_targets, task 2.11).
pub(super) fn expr_contains_call(expr: &ExprIR) -> bool {
    match expr {
        ExprIR::CallExpr { .. } | ExprIR::MethodCallExpr { .. } | ExprIR::AwaitExpr { .. } => true,
        ExprIR::StoreFieldAccessExpr { call_args, .. } => !call_args.is_empty(),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_contains_call(left) || expr_contains_call(right)
        }
        ExprIR::UnaryOpExpr { operand, .. } => expr_contains_call(operand),
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            expr_contains_call(object)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_contains_call(object) || expr_contains_call(index)
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_contains_call(cond)
                || expr_contains_call(then_expr)
                || expr_contains_call(else_expr)
        }
        ExprIR::ArrayExpr { elements } => elements.iter().any(expr_contains_call),
        ExprIR::MapExpr { entries } => entries
            .iter()
            .any(|(k, v)| expr_contains_call(k) || expr_contains_call(v)),
        ExprIR::ClosureExpr { body, .. } => expr_contains_call(body),
        ExprIR::BlockExpr { statements, tail } => {
            statements.iter().any(stmt_contains_call) || expr_contains_call(tail)
        }
        ExprIR::MatchExpr { matched, arms } => {
            expr_contains_call(matched)
                || arms
                    .iter()
                    .any(|arm| arm.body.statements.iter().any(stmt_contains_call))
        }
        ExprIR::VariantLiteral { fields, .. } | ExprIR::StructExpr { fields, .. } => {
            fields.iter().any(|(_, v)| expr_contains_call(v))
        }
        ExprIR::SpreadExpr { operand } => expr_contains_call(operand),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => expr_contains_call(subject) || expr_contains_call(predicate),
        ExprIR::RefOpExpr { .. } | ExprIR::IdentifierExpr { .. } | ExprIR::LiteralExpr { .. } => {
            false
        }
        // showToast lowers to a `toast(...)` call with visible side effects —
        // treat like any other call so bare-statement emission keeps it.
        ExprIR::BuiltinToastCall { msg, .. } => expr_contains_call(msg),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

/// Whether a statement subtree contains a call (used by
/// [`expr_contains_call`] for block/match bodies).
fn stmt_contains_call(stmt: &StatementIR) -> bool {
    match stmt {
        StatementIR::ExprStmtIR { expr } | StatementIR::ThrowStmtIR { expr } => {
            expr_contains_call(expr)
        }
        StatementIR::LetStmtIR { value, .. } | StatementIR::ReturnStmtIR { value } => {
            value.as_ref().is_some_and(expr_contains_call)
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            expr_contains_call(condition)
                || then_body.statements.iter().any(stmt_contains_call)
                || else_ifs.iter().any(|(c, b)| {
                    expr_contains_call(c) || b.statements.iter().any(stmt_contains_call)
                })
                || else_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(stmt_contains_call))
        }
        StatementIR::WhileStmtIR { condition, body }
        | StatementIR::ForInStmtIR {
            iterable: condition,
            body,
            ..
        } => expr_contains_call(condition) || body.statements.iter().any(stmt_contains_call),
        StatementIR::BlockStmtIR { body } => body.statements.iter().any(stmt_contains_call),
        StatementIR::MutationStmt { intent } => intent
            .as_ref()
            .and_then(mutation_value_expr)
            .is_some_and(expr_contains_call),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body.statements.iter().any(stmt_contains_call)
                || catch_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(stmt_contains_call))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(stmt_contains_call))
        }
        StatementIR::EventEmitStmtIR { args, .. } => args.iter().any(expr_contains_call),
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// v4 H1 (task 2.1) — write-then-read threading
// ═══════════════════════════════════════════════════════════

/// v4 H1: 处理器体内「写后读」链线程化。React 的 `setX` 是异步的——同一
/// 处理器内 `x = v; use(x)` 直接读 `x` 拿到的是陈旧闭包值。本重写把被写后
/// 又被读的信号的值线程化到局部：`const nextX = <下值>; setX(nextX);`，并把
/// 后续语句中对 x 的读取重写为 `nextX`（顺序语义）。
///
/// 仅当信号的「下值」真的被消费时才改发射形状——无后续读取的信号保持既有
/// 发射路径（functional updater 形态不变）。链式写（同信号多次写）按 DSL
/// 顺序语义逐次线程化，每次的「下值」读当前可见值（前一 local 若存在）：
/// `x = 1; x = x + 1; use(x)` →
/// `const nextX = 1; setX(nextX); const nextX1 = nextX + 1; setX(nextX1);
/// use(nextX1);`。mutation 自身 RHS 不重写（读旧值——正确）。
pub(super) fn rewrite_write_through(statements: &[StatementIR]) -> Vec<StatementIR> {
    // 收集「可表达下值」intent（DirectAssign/Increment/Decrement，与
    // mutation_next_expr 的匹配一致）的 mutation 语句位置，按目标分组。
    let mut targets: FxHashMap<String, Vec<usize>> = FxHashMap::default();
    for (i, stmt) in statements.iter().enumerate() {
        if let StatementIR::MutationStmt {
            intent: Some(intent),
        } = stmt
        {
            if matches!(
                intent,
                MutationIntent::DirectAssign { .. }
                    | MutationIntent::Increment { .. }
                    | MutationIntent::Decrement { .. }
            ) {
                if let Some(target) = mutation_target_name(intent) {
                    targets.entry(target.to_string()).or_default().push(i);
                }
            }
        }
    }
    if targets.is_empty() {
        return statements.to_vec();
    }
    // 分裂判定（基于原始语句序）：m_k 线程化 iff 其「下值」被消费——在
    // (m_k, m_{k+1}] 区间内（下一个同信号 mutation 的 RHS 含在内，自身
    // RHS 不含）存在对信号的读取。区间内的读取归 m_k 的 local，之后的
    // 归 m_{k+1} 的 local——顺序语义逐段成立。
    //
    // 位置敏感放弃（v4 评审修复轮）：rename 递归入块，会把「链断点之后」
    // 的读取也改写为 m_k 的 local——陈旧值；「断点之前」的读取 rename 到
    // local 在该程序点精确正确（写→读→条件重置形态常见，读在前必须保留
    // 线程化）。链断点 = 同信号嵌套写（if/while/闭包体内）或顶层非三变体
    // 写（Multiply/Push 等——其值不可局部化）。按语句序扫描区间：断点
    // 置位后再出现读取 → 放弃；同一语句同时含读与断点（块内次序不明）→
    // 保守放弃。放弃传播：前一写已放弃且本写 RHS 读 target 时，本写也读
    // 不到「当前可见值」（分裂会读陈旧闭包值，React 队列终态错误）——
    // 保持 functional updater（链安全）。首个写的 RHS 自读还要求「写前无
    // 同信号写」（断点在 m_0 之前执行后 closure 值陈旧，RHS 无值可读）。
    let mut split_flags: FxHashMap<String, Vec<bool>> = FxHashMap::default();
    for (target, indices) in &targets {
        let mut flags = Vec::with_capacity(indices.len());
        let mut prev_bailed = false;
        for (j, &i) in indices.iter().enumerate() {
            let last = statements.len() - 1;
            let end = indices.get(j + 1).copied().unwrap_or(last);
            let interval: Vec<&StatementIR> = statements.iter().skip(i + 1).take(end - i).collect();
            let has_read = interval.iter().any(|s| stmt_reads_identifier(s, target));
            // 读在链断点之后 → 区间模型失效（读在断点之前 → 精确正确）。
            let mut read_after_breaker = false;
            let mut seen_breaker = false;
            for s in &interval {
                let reads = stmt_reads_identifier(s, target);
                let breaker = stmt_contains_nested_target_mutation(s, target)
                    || stmt_is_unthreadable_target_write(s, target);
                if reads && (seen_breaker || breaker) {
                    read_after_breaker = true;
                    break;
                }
                if breaker {
                    seen_breaker = true;
                }
            }
            let self_read = match &statements[i] {
                StatementIR::MutationStmt {
                    intent: Some(intent),
                } => mutation_reads_target(intent, target),
                _ => false,
            };
            // 首个写之前已有同信号写（嵌套或非三变体顶层）——closure 值
            // 陈旧，自读 RHS 无法线程化（j > 0 时由 m_{k-1} 的区间扫描 +
            // 传播覆盖：m_k 的 RHS 在 m_{k-1} 的区间内，断点先行置位）。
            let breaker_before_first = j == 0
                && statements[..i].iter().any(|s| {
                    stmt_contains_nested_target_mutation(s, target)
                        || stmt_is_unthreadable_target_write(s, target)
                });
            let bailed = read_after_breaker
                || (prev_bailed && self_read)
                || (breaker_before_first && self_read);
            flags.push(has_read && !bailed);
            prev_bailed = bailed;
        }
        split_flags.insert(target.clone(), flags);
    }
    // 重写循环：按原始语句序处理（插入 setter 后索引后移，用 while 推进）。
    let mut rewritten = statements.to_vec();
    let mut seen: FxHashMap<String, usize> = FxHashMap::default();
    let mut last_local: FxHashMap<String, String> = FxHashMap::default();
    let mut local_counts: FxHashMap<String, usize> = FxHashMap::default();
    let mut i = 0;
    // 已插入的 setter 数——原索引 → 当前索引的位移（每个 split 插入 1 条）。
    let mut s = 0usize;
    while i < rewritten.len() {
        let mut plan: Option<(String, MutationIntent)> = None;
        if let StatementIR::MutationStmt {
            intent: Some(intent),
        } = &rewritten[i]
        {
            // 仅与 targets 收集同口径的 intent 推进序号（Push/Pop 等
            // 不入 local 表，序号偏移会造成 split_flags 错位）。
            if matches!(
                intent,
                MutationIntent::DirectAssign { .. }
                    | MutationIntent::Increment { .. }
                    | MutationIntent::Decrement { .. }
            ) {
                if let Some(target) = mutation_target_name(intent) {
                    let ordinal = seen.entry(target.to_string()).or_insert(0);
                    let threaded = split_flags
                        .get(target)
                        .and_then(|f| f.get(*ordinal))
                        .copied()
                        .unwrap_or(false);
                    if threaded {
                        plan = Some((target.to_string(), intent.clone()));
                    }
                    *ordinal += 1;
                }
            }
        }
        if let Some((target, intent)) = plan {
            let orig_i = i - s;
            // rename 范围 = (m_k, m_{k+1}]——本写之后、下一个同信号顶层
            // mutation（含其 RHS，链消费）为止；之后的读取归 m_{k+1} 的
            // rename（m_{k+1} 放弃时保持原状读旧值——无回归，v4 评审修复
            // 轮：full-suffix rename 会把 m_{k+1} 放弃后的读取改写为陈旧
            // local）。
            let next_orig = targets
                .get(&target)
                .and_then(|v| v.iter().find(|&&o| o > orig_i))
                .copied();
            let nth = local_counts.entry(target.clone()).or_insert(0);
            let local = if *nth == 0 {
                format!("next{}", capitalize_first(&target))
            } else {
                format!("next{}{}", capitalize_first(&target), nth)
            };
            *nth += 1;
            let prev = last_local.get(&target).cloned();
            // 下值 = mutation 的普通值表达式——读「当前可见值」（前一 local
            // 若存在；DirectAssign 的值 / Increment 的 amount 已被更早的
            // 线程化 rename 改写为前一 local 的引用）。
            let next_expr = mutation_next_expr(&intent, &target, prev.as_deref());
            // 合成 setter 的 field_object 恒为 None（v4 评审 Minor ①）：
            // 原 intent 若带 field_object（`Store.field = v` 类 owner 写），
            // 线程化后发射路径不读 field_object（仅写穿 DCE 守卫读它——
            // 而能线程化的 target 必有后续读取，绝不入 pruned 集），
            // 语义等价，None 不可达造成行为差异。
            let setter_stmt = StatementIR::MutationStmt {
                intent: Some(MutationIntent::DirectAssign {
                    target: SignalRefIR {
                        name: target.clone(),
                    },
                    value: Some(Box::new(ExprIR::IdentifierExpr {
                        name: local.clone(),
                        type_ann: None,
                    })),
                    field_object: None,
                }),
            };
            rewritten[i] = StatementIR::LetStmtIR {
                name: local.clone(),
                value: Some(next_expr),
                r#type: None,
                is_mut: false,
            };
            rewritten.insert(i + 1, setter_stmt);
            // 插入后 m_{k+1} 的当前索引 = orig + s + 1（s 个先前插入 + 本次
            // 插入均在它之前）；无后继 → 表尾。
            let end_current = next_orig.map(|o| o + s + 1).unwrap_or(rewritten.len() - 1);
            let rename_count = end_current.saturating_sub(i + 1);
            // 后续语句读取重写：跳过本 mutation 自身 RHS（读旧值——正确）。
            // 除目标名外还重写前一 local（防御——区间内由构造不含 prev）。
            for later in rewritten.iter_mut().skip(i + 2).take(rename_count) {
                rename_stmt_identifiers(later, &target, &local);
                if let Some(ref prev_name) = prev {
                    rename_stmt_identifiers(later, prev_name, &local);
                }
            }
            last_local.insert(target, local);
            i += 2;
            s += 1;
        } else {
            i += 1;
        }
    }
    rewritten
}

/// v4 H1: mutation 的「下值」表达式——DirectAssign 为值本身；
/// Increment/Decrement 组合二元表达式（当前可见值 + amount / - amount，
/// 当前可见值 = 前一 local 若存在否则信号本身——DSL 顺序语义
/// `x = x + 1` 读 x 的当前值）。必须与 rewrite_write_through 的 intent
/// 过滤一致（三变体，其余 intent 永不入 local 表，unreachable 为防御断言）。
fn mutation_next_expr(intent: &MutationIntent, target: &str, prev_local: Option<&str>) -> ExprIR {
    let read = |name: &str| ExprIR::IdentifierExpr {
        name: name.to_string(),
        type_ann: None,
    };
    let zero = || ExprIR::LiteralExpr {
        value: serde_json::Value::from(0),
    };
    match intent {
        MutationIntent::DirectAssign { value, .. } => match value {
            Some(v) => (**v).clone(),
            None => zero(),
        },
        MutationIntent::Increment { amount, .. } => ExprIR::BinaryOpExpr {
            op: BinaryOpIR::Add,
            left: Box::new(read(prev_local.unwrap_or(target))),
            right: Box::new(amount.clone().map(|b| *b).unwrap_or_else(zero)),
        },
        MutationIntent::Decrement { amount, .. } => ExprIR::BinaryOpExpr {
            op: BinaryOpIR::Sub,
            left: Box::new(read(prev_local.unwrap_or(target))),
            right: Box::new(amount.clone().map(|b| *b).unwrap_or_else(zero)),
        },
        _ => unreachable!("write-through 仅处理 DirectAssign/Increment/Decrement"),
    }
}

/// 该 mutation 的「下值」表达式是否读取 target——DirectAssign 看值表达式
/// 是否含 target；Increment/Decrement 恒为真（下值 = target ± amount，
/// 隐式读 target，与 mutation_next_expr 的 RHS 构造一致）。
fn mutation_reads_target(intent: &MutationIntent, target: &str) -> bool {
    matches!(
        intent,
        MutationIntent::Increment { .. } | MutationIntent::Decrement { .. }
    ) || mutation_value_expr(intent).is_some_and(|v| expr_reads_identifier(v, target))
}

/// 语句是否为「不可线程化」的顶层同信号写——非三变体 intent
/// （Multiply/Divide/Modulo/Push/Pop）的值无法局部化（mutation_next_expr
/// 契约只覆盖三变体），与嵌套写同级切断 local 链：其后的读取与自读写
/// 都读不到可局部化的当前值。
fn stmt_is_unthreadable_target_write(stmt: &StatementIR, target: &str) -> bool {
    matches!(
        stmt,
        StatementIR::MutationStmt { intent: Some(i) }
            if !matches!(
                i,
                MutationIntent::DirectAssign { .. }
                    | MutationIntent::Increment { .. }
                    | MutationIntent::Decrement { .. }
            ) && mutation_target_name(i) == Some(target)
    )
}

/// 语句子树是否含「嵌套」的同信号写——出现在 if/while/for/块语句/块
/// 表达式/闭包体内的 MutationStmt（顶层 mutation 不算——它是区间边界）。
/// 嵌套写切断 local 链：rename 递归入块，「嵌套写之后」的读取会被改写为
/// m_k 的 local（陈旧值）；「嵌套写之前」的读取 rename 到 local 精确正确。
/// 位置敏感判定见 rewrite_write_through 的 read_after_breaker 扫描
/// （v4 评审修复轮）。
fn stmt_contains_nested_target_mutation(stmt: &StatementIR, target: &str) -> bool {
    match stmt {
        StatementIR::ExprStmtIR { expr } | StatementIR::ThrowStmtIR { expr } => {
            expr_contains_nested_target_mutation(expr, target)
        }
        StatementIR::LetStmtIR { value, .. } | StatementIR::ReturnStmtIR { value } => value
            .as_ref()
            .is_some_and(|v| expr_contains_nested_target_mutation(v, target)),
        StatementIR::IfStmtIR {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            stmts_contains_nested_target_mutation(&then_body.statements, target)
                || else_ifs
                    .iter()
                    .any(|(_, b)| stmts_contains_nested_target_mutation(&b.statements, target))
                || else_body
                    .as_ref()
                    .is_some_and(|b| stmts_contains_nested_target_mutation(&b.statements, target))
        }
        StatementIR::WhileStmtIR { body, .. } | StatementIR::ForInStmtIR { body, .. } => {
            stmts_contains_nested_target_mutation(&body.statements, target)
        }
        StatementIR::BlockStmtIR { body } => {
            stmts_contains_nested_target_mutation(&body.statements, target)
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            stmts_contains_nested_target_mutation(&try_body.statements, target)
                || catch_body
                    .as_ref()
                    .is_some_and(|b| stmts_contains_nested_target_mutation(&b.statements, target))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| stmts_contains_nested_target_mutation(&b.statements, target))
        }
        StatementIR::MutationStmt { intent } => intent.as_ref().is_some_and(|i| {
            // 顶层 mutation 自身不算嵌套——但其值表达式（块表达式）可含嵌套写。
            mutation_value_expr(i).is_some_and(|v| expr_contains_nested_target_mutation(v, target))
        }),
        StatementIR::EventEmitStmtIR { args, .. } => args
            .iter()
            .any(|a| expr_contains_nested_target_mutation(a, target)),
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// 语句列表内是否含「嵌套」同信号写——列表内的顶层 MutationStmt 在
/// 调用语境中即为嵌套上下文中的写（相对处理器顶层而言）。
fn stmts_contains_nested_target_mutation(stmts: &[StatementIR], target: &str) -> bool {
    stmts.iter().any(|s| {
        matches!(
            s,
            StatementIR::MutationStmt { intent: Some(i) } if mutation_target_name(i) == Some(target)
        ) || stmt_contains_nested_target_mutation(s, target)
    })
}

/// 表达式子树是否含「嵌套」同信号写（块表达式语句 / match 臂体 / 闭包体）。
fn expr_contains_nested_target_mutation(expr: &ExprIR, target: &str) -> bool {
    match expr {
        ExprIR::BlockExpr { statements, tail } => {
            stmts_contains_nested_target_mutation(statements, target)
                || expr_contains_nested_target_mutation(tail, target)
        }
        ExprIR::MatchExpr { matched, arms } => {
            expr_contains_nested_target_mutation(matched, target)
                || arms
                    .iter()
                    .any(|arm| stmts_contains_nested_target_mutation(&arm.body.statements, target))
        }
        ExprIR::ClosureExpr { body, .. } => expr_contains_nested_target_mutation(body, target),
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            expr_contains_nested_target_mutation(object, target)
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_contains_nested_target_mutation(left, target)
                || expr_contains_nested_target_mutation(right, target)
        }
        ExprIR::UnaryOpExpr { operand, .. } => {
            expr_contains_nested_target_mutation(operand, target)
        }
        ExprIR::CallExpr { args, .. } => args
            .iter()
            .any(|a| expr_contains_nested_target_mutation(a, target)),
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_contains_nested_target_mutation(object, target)
                || args
                    .iter()
                    .any(|a| expr_contains_nested_target_mutation(a, target))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_contains_nested_target_mutation(cond, target)
                || expr_contains_nested_target_mutation(then_expr, target)
                || expr_contains_nested_target_mutation(else_expr, target)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_contains_nested_target_mutation(object, target)
                || expr_contains_nested_target_mutation(index, target)
        }
        ExprIR::ArrayExpr { elements } => elements
            .iter()
            .any(|e| expr_contains_nested_target_mutation(e, target)),
        ExprIR::SpreadExpr { operand } => expr_contains_nested_target_mutation(operand, target),
        ExprIR::StructExpr { fields, .. } | ExprIR::VariantLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_contains_nested_target_mutation(v, target)),
        ExprIR::StoreFieldAccessExpr { call_args, .. } => call_args
            .iter()
            .any(|a| expr_contains_nested_target_mutation(a, target)),
        ExprIR::AwaitExpr { operand } => expr_contains_nested_target_mutation(operand, target),
        ExprIR::MapExpr { entries } => entries.iter().any(|(k, v)| {
            expr_contains_nested_target_mutation(k, target)
                || expr_contains_nested_target_mutation(v, target)
        }),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            expr_contains_nested_target_mutation(subject, target)
                || expr_contains_nested_target_mutation(predicate, target)
        }
        ExprIR::RefOpExpr { op, .. } => ref_op_contains_nested_target_mutation(op, target),
        ExprIR::IdentifierExpr { .. } | ExprIR::LiteralExpr { .. } => false,
        ExprIR::BuiltinToastCall { msg, .. } => expr_contains_nested_target_mutation(msg, target),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

/// `RefOpIR` 操作数子树是否含「嵌套」同信号写。
fn ref_op_contains_nested_target_mutation(op: &RefOpIR, target: &str) -> bool {
    match op {
        RefOpIR::ScrollTo { x, y } => {
            x.as_ref()
                .is_some_and(|e| expr_contains_nested_target_mutation(e, target))
                || y.as_ref()
                    .is_some_and(|e| expr_contains_nested_target_mutation(e, target))
        }
        RefOpIR::ScrollBy { dx, dy } => {
            dx.as_ref()
                .is_some_and(|e| expr_contains_nested_target_mutation(e, target))
                || dy
                    .as_ref()
                    .is_some_and(|e| expr_contains_nested_target_mutation(e, target))
        }
        RefOpIR::SetSelection { start, end } => {
            expr_contains_nested_target_mutation(start, target)
                || expr_contains_nested_target_mutation(end, target)
        }
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Whether a statement subtree reads the identifier (recursive over nested
/// statements and expressions — mirrors `walk::walk_stmt`'s traversal).
fn stmt_reads_identifier(stmt: &StatementIR, target: &str) -> bool {
    match stmt {
        StatementIR::ExprStmtIR { expr } | StatementIR::ThrowStmtIR { expr } => {
            expr_reads_identifier(expr, target)
        }
        StatementIR::LetStmtIR { value, .. } | StatementIR::ReturnStmtIR { value } => value
            .as_ref()
            .is_some_and(|v| expr_reads_identifier(v, target)),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            expr_reads_identifier(condition, target)
                || then_body
                    .statements
                    .iter()
                    .any(|s| stmt_reads_identifier(s, target))
                || else_ifs.iter().any(|(c, b)| {
                    expr_reads_identifier(c, target)
                        || b.statements
                            .iter()
                            .any(|s| stmt_reads_identifier(s, target))
                })
                || else_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| stmt_reads_identifier(s, target))
                })
        }
        StatementIR::WhileStmtIR { condition, body }
        | StatementIR::ForInStmtIR {
            iterable: condition,
            body,
            ..
        } => {
            expr_reads_identifier(condition, target)
                || body
                    .statements
                    .iter()
                    .any(|s| stmt_reads_identifier(s, target))
        }
        StatementIR::BlockStmtIR { body } => body
            .statements
            .iter()
            .any(|s| stmt_reads_identifier(s, target)),
        StatementIR::MutationStmt { intent } => intent.as_ref().is_some_and(|i| {
            // Increment/Decrement 隐式读 target 自身（下值 = target ± amount，
            // 与 mutation_next_expr 的 RHS 构造一致）——mutation_value_expr
            // 只返回 amount，漏判会使 `+=` 链的上一写不线程化、下一写分裂
            // 读陈旧闭包值（v4 评审 Important 1）。
            (matches!(
                i,
                MutationIntent::Increment { .. } | MutationIntent::Decrement { .. }
            ) && mutation_target_name(i) == Some(target))
                || mutation_value_expr(i).is_some_and(|v| expr_reads_identifier(v, target))
        }),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body
                .statements
                .iter()
                .any(|s| stmt_reads_identifier(s, target))
                || catch_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| stmt_reads_identifier(s, target))
                })
                || finally_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| stmt_reads_identifier(s, target))
                })
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            args.iter().any(|a| expr_reads_identifier(a, target))
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Whether an expression subtree reads the identifier (recursive, mirrors
/// `walk::walk_expr`'s traversal). Call/method names and field names are
/// strings, not reads — only `IdentifierExpr` nodes count.
fn expr_reads_identifier(expr: &ExprIR, target: &str) -> bool {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => name == target,
        ExprIR::LiteralExpr { .. } => false,
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            expr_reads_identifier(object, target)
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_reads_identifier(left, target) || expr_reads_identifier(right, target)
        }
        ExprIR::UnaryOpExpr { operand, .. } => expr_reads_identifier(operand, target),
        ExprIR::CallExpr { args, .. } => args.iter().any(|a| expr_reads_identifier(a, target)),
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_reads_identifier(object, target)
                || args.iter().any(|a| expr_reads_identifier(a, target))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_reads_identifier(cond, target)
                || expr_reads_identifier(then_expr, target)
                || expr_reads_identifier(else_expr, target)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_reads_identifier(object, target) || expr_reads_identifier(index, target)
        }
        ExprIR::ArrayExpr { elements } => elements.iter().any(|e| expr_reads_identifier(e, target)),
        ExprIR::SpreadExpr { operand } => expr_reads_identifier(operand, target),
        ExprIR::StructExpr { fields, .. } | ExprIR::VariantLiteral { fields, .. } => {
            fields.iter().any(|(_, v)| expr_reads_identifier(v, target))
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            call_args.iter().any(|a| expr_reads_identifier(a, target))
        }
        ExprIR::BlockExpr { statements, tail } => {
            statements.iter().any(|s| stmt_reads_identifier(s, target))
                || expr_reads_identifier(tail, target)
        }
        ExprIR::ClosureExpr { body, .. } => expr_reads_identifier(body, target),
        ExprIR::AwaitExpr { operand } => expr_reads_identifier(operand, target),
        ExprIR::MatchExpr { matched, arms } => {
            expr_reads_identifier(matched, target)
                || arms.iter().any(|arm| {
                    arm.body
                        .statements
                        .iter()
                        .any(|s| stmt_reads_identifier(s, target))
                })
        }
        ExprIR::MapExpr { entries } => entries
            .iter()
            .any(|(k, v)| expr_reads_identifier(k, target) || expr_reads_identifier(v, target)),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => expr_reads_identifier(subject, target) || expr_reads_identifier(predicate, target),
        ExprIR::RefOpExpr { op, .. } => ref_op_reads_identifier(op, target),
        ExprIR::BuiltinToastCall { msg, .. } => expr_reads_identifier(msg, target),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

/// Whether a `RefOpIR` operand subtree reads the identifier.
fn ref_op_reads_identifier(op: &RefOpIR, target: &str) -> bool {
    match op {
        RefOpIR::Focus
        | RefOpIR::Blur
        | RefOpIR::Measure { .. }
        | RefOpIR::Select
        | RefOpIR::Value
        | RefOpIR::CapturePointer
        | RefOpIR::ReleasePointer => false,
        RefOpIR::ScrollTo { x, y } => {
            x.as_ref().is_some_and(|e| expr_reads_identifier(e, target))
                || y.as_ref().is_some_and(|e| expr_reads_identifier(e, target))
        }
        RefOpIR::ScrollBy { dx, dy } => {
            dx.as_ref()
                .is_some_and(|e| expr_reads_identifier(e, target))
                || dy
                    .as_ref()
                    .is_some_and(|e| expr_reads_identifier(e, target))
        }
        RefOpIR::SetSelection { start, end } => {
            expr_reads_identifier(start, target) || expr_reads_identifier(end, target)
        }
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Rename every read of `from` to `to` in a statement subtree (recursive
/// over nested statements and expressions — mirrors `stmt_reads_identifier`).
/// Only `IdentifierExpr` nodes are touched: call/method/field names and
/// statement-level names (`LetStmtIR.name`, `EventEmitStmtIR.event_name`)
/// are not reads and stay verbatim.
fn rename_stmt_identifiers(stmt: &mut StatementIR, from: &str, to: &str) {
    match stmt {
        StatementIR::ExprStmtIR { expr } | StatementIR::ThrowStmtIR { expr } => {
            rename_expr_identifiers(expr, from, to);
        }
        StatementIR::LetStmtIR { value, .. } | StatementIR::ReturnStmtIR { value } => {
            if let Some(v) = value {
                rename_expr_identifiers(v, from, to);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            rename_expr_identifiers(condition, from, to);
            for s in &mut then_body.statements {
                rename_stmt_identifiers(s, from, to);
            }
            for (cond, body) in else_ifs {
                rename_expr_identifiers(cond, from, to);
                for s in &mut body.statements {
                    rename_stmt_identifiers(s, from, to);
                }
            }
            if let Some(eb) = else_body {
                for s in &mut eb.statements {
                    rename_stmt_identifiers(s, from, to);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body }
        | StatementIR::ForInStmtIR {
            iterable: condition,
            body,
            ..
        } => {
            rename_expr_identifiers(condition, from, to);
            for s in &mut body.statements {
                rename_stmt_identifiers(s, from, to);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &mut body.statements {
                rename_stmt_identifiers(s, from, to);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                match intent {
                    MutationIntent::DirectAssign { value, .. } => {
                        if let Some(v) = value {
                            rename_expr_identifiers(v, from, to);
                        }
                    }
                    MutationIntent::Increment { amount, .. }
                    | MutationIntent::Decrement { amount, .. }
                    | MutationIntent::Multiply { amount, .. }
                    | MutationIntent::Divide { amount, .. }
                    | MutationIntent::Modulo { amount, .. } => {
                        if let Some(v) = amount {
                            rename_expr_identifiers(v, from, to);
                        }
                    }
                    MutationIntent::Push { item, .. } => {
                        if let Some(v) = item {
                            rename_expr_identifiers(v, from, to);
                        }
                    }
                    MutationIntent::Pop { .. } => {}
                    #[allow(unreachable_patterns)]
                    _ => unreachable!("Unknown MutationIntent variant — update codegen"),
                }
            }
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &mut try_body.statements {
                rename_stmt_identifiers(s, from, to);
            }
            if let Some(cb) = catch_body {
                for s in &mut cb.statements {
                    rename_stmt_identifiers(s, from, to);
                }
            }
            if let Some(fb) = finally_body {
                for s in &mut fb.statements {
                    rename_stmt_identifiers(s, from, to);
                }
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for a in args {
                rename_expr_identifiers(a, from, to);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Rename every read of `from` to `to` in an expression subtree (recursive,
/// mirrors `expr_reads_identifier`).
fn rename_expr_identifiers(expr: &mut ExprIR, from: &str, to: &str) {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => {
            if name == from {
                *name = to.to_string();
            }
        }
        ExprIR::LiteralExpr { .. } => {}
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            rename_expr_identifiers(object, from, to);
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            rename_expr_identifiers(left, from, to);
            rename_expr_identifiers(right, from, to);
        }
        ExprIR::UnaryOpExpr { operand, .. } => rename_expr_identifiers(operand, from, to),
        ExprIR::CallExpr { args, .. } => {
            for a in args {
                rename_expr_identifiers(a, from, to);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            rename_expr_identifiers(object, from, to);
            for a in args {
                rename_expr_identifiers(a, from, to);
            }
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            rename_expr_identifiers(cond, from, to);
            rename_expr_identifiers(then_expr, from, to);
            rename_expr_identifiers(else_expr, from, to);
        }
        ExprIR::IndexExpr { object, index } => {
            rename_expr_identifiers(object, from, to);
            rename_expr_identifiers(index, from, to);
        }
        ExprIR::ArrayExpr { elements } => {
            for e in elements {
                rename_expr_identifiers(e, from, to);
            }
        }
        ExprIR::SpreadExpr { operand } => rename_expr_identifiers(operand, from, to),
        ExprIR::StructExpr { fields, .. } | ExprIR::VariantLiteral { fields, .. } => {
            for (_, v) in fields {
                rename_expr_identifiers(v, from, to);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for a in call_args {
                rename_expr_identifiers(a, from, to);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for s in statements {
                rename_stmt_identifiers(s, from, to);
            }
            rename_expr_identifiers(tail, from, to);
        }
        ExprIR::ClosureExpr { body, .. } => rename_expr_identifiers(body, from, to),
        ExprIR::AwaitExpr { operand } => rename_expr_identifiers(operand, from, to),
        ExprIR::MatchExpr { matched, arms } => {
            rename_expr_identifiers(matched, from, to);
            for arm in arms {
                for s in &mut arm.body.statements {
                    rename_stmt_identifiers(s, from, to);
                }
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                rename_expr_identifiers(k, from, to);
                rename_expr_identifiers(v, from, to);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            rename_expr_identifiers(subject, from, to);
            rename_expr_identifiers(predicate, from, to);
        }
        ExprIR::RefOpExpr { op, .. } => rename_ref_op_identifiers(op, from, to),
        ExprIR::BuiltinToastCall { msg, .. } => rename_expr_identifiers(msg, from, to),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

/// Rename identifier reads in a `RefOpIR` operand subtree.
fn rename_ref_op_identifiers(op: &mut RefOpIR, from: &str, to: &str) {
    match op {
        RefOpIR::Focus
        | RefOpIR::Blur
        | RefOpIR::Measure { .. }
        | RefOpIR::Select
        | RefOpIR::Value
        | RefOpIR::CapturePointer
        | RefOpIR::ReleasePointer => {}
        RefOpIR::ScrollTo { x, y } => {
            if let Some(x) = x {
                rename_expr_identifiers(x, from, to);
            }
            if let Some(y) = y {
                rename_expr_identifiers(y, from, to);
            }
        }
        RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                rename_expr_identifiers(dx, from, to);
            }
            if let Some(dy) = dy {
                rename_expr_identifiers(dy, from, to);
            }
        }
        RefOpIR::SetSelection { start, end } => {
            rename_expr_identifiers(start, from, to);
            rename_expr_identifiers(end, from, to);
        }
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Statement generation with store-field renames (`"Store.field"` → slice
/// local, Task 3.3) applied at expression level (reads and action calls).
///
/// `pruned_signal_names` (task 2.11) — write-only component signals whose
/// state hook was skipped: mutations of them would emit an undefined setter
/// reference (TS2304). Such mutations are DCE'd; a mutation value carrying
/// calls is kept as a bare expression statement so the calls' side effects
/// survive.
// 8 cohesive context params (name/rename namespaces needed for statement
// emission); bundling into a context struct is a larger refactor than this
// lint warrants for an internal helper.
#[allow(clippy::too_many_arguments)]
pub(super) fn generate_statement_tsx(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    ref_names: &FxHashSet<String>,
    renames: &FxHashMap<String, String>,
    pruned_signal_names: &FxHashSet<String>,
    possibly_undefined: &FxHashSet<String>,
    direct_write_signals: &FxHashSet<String>,
    direct_write_applies: &FxHashMap<String, Vec<String>>,
    warnings: Option<&Mutex<Vec<String>>>,
) {
    // Task 2.11 — write-only signal mutation DCE: the signal's state hook
    // was skipped (its value is never consumed), so a setter call here
    // would reference an undeclared name. Skip the mutation; keep the
    // value when it carries calls (side effects).
    if let StatementIR::MutationStmt {
        intent: Some(intent),
    } = stmt
    {
        if let Some(target) = mutation_target_name(intent) {
            // Owner-field writes (`Store.field = v`, `myRef.field = v`)
            // lower with the owner in `field_object` — they are NOT
            // component-signal mutations and must never be pruned (review
            // I1: the prune matches by BARE target name, and a store field
            // can share a pruned component signal's name — pruning would
            // silently drop the store write). The `field_object` check is
            // the emission-side half of the two-layer discrimination: the
            // analysis half (pruned ∩ comp.signals, review N1) removes
            // NON-colliding store field names from the pruned set; this
            // check keeps the COLLIDING store write alive (its bare name IS
            // a pruned component signal). An owners-map check alone would
            // ALSO un-prune the colliding component signal's OWN write
            // (`mode = !mode` with a wired store field `mode`), which then
            // misroutes through the owners map to the store setter.
            let is_owner_field_write = mutation_field_object(intent).is_some();
            if pruned_signal_names.contains(target) && !is_owner_field_write {
                if let Some(v) = mutation_value_expr(intent) {
                    if expr_contains_call(v) {
                        let line = format_expr_ir_tsx_renamed_locals(
                            v,
                            ref_names,
                            renames,
                            &FxHashSet::default(),
                            warnings,
                        );
                        w.emit_fmt(format_args!("{};", line));
                    }
                }
                return;
            }
        }
    }
    // Intercept array_push / array_pop calls on signals — these should
    // be emitted as setter/setState-based mutations, not pure expressions.
    if let StatementIR::ExprStmtIR {
        expr: ExprIR::CallExpr { name, args },
    } = stmt
    {
        if name == "array_push" || name == "array_pop" {
            if let Some(ExprIR::IdentifierExpr {
                name: target_name, ..
            }) = args.first()
            {
                // A pruned (write-only) component signal has no setter —
                // skip the interception and emit nothing (task 2.11).
                // Review N1: NO owners guard here. The pruned set is
                // intersected with comp.signals at analysis time, so a bare
                // STORE field name never enters it — and a store field
                // sharing a pruned component signal's name must NOT un-prune
                // the component's own array_push (an owners guard would
                // misroute it to the store setState — a cross-write the
                // user never wrote). Store-field pushes with a bare name
                // are not in pruned and fall through to the owners branch.
                if pruned_signal_names.contains(target_name.as_str()) {
                    return;
                }
                // Phase 1 (D1/D2): ref 存储的信号无 setter——数组 push/pop
                // 拦截改写 ref current（未豁免则落入 setter 分支 → TS2304）。
                // Fix round F2: store-first convention (mirrors the
                // DirectAssign branch order below) — a store field colliding
                // with a D1/D2 component signal's bare name must route to the
                // store setState, not the ref write (which would silently
                // drop the store mutation).
                if direct_write_signals.contains(target_name.as_str())
                    && !store_signal_owners.contains_key(target_name.as_str())
                {
                    match name.as_str() {
                        "array_push" => {
                            let item = args
                                .get(1)
                                .map(|e| {
                                    format_expr_ir_tsx_renamed_locals(
                                        e,
                                        ref_names,
                                        renames,
                                        &FxHashSet::default(),
                                        warnings,
                                    )
                                })
                                .unwrap_or_else(|| "undefined".into());
                            w.emit_fmt(format_args!(
                                "{}Ref.current = [...{}Ref.current, {}];",
                                target_name, target_name, item
                            ));
                            return;
                        }
                        "array_pop" => {
                            w.emit_fmt(format_args!(
                                "{}Ref.current = {}Ref.current.slice(0, -1);",
                                target_name, target_name
                            ));
                            return;
                        }
                        _ => {}
                    }
                }
                if signal_names.contains(target_name.as_str())
                    || store_signal_owners.contains_key(target_name.as_str())
                {
                    match name.as_str() {
                        "array_push" => {
                            // The pushed item is an expression in the current
                            // scope — it must go through the store-field
                            // rename path too (`array_push(items,
                            // TodoStore.totalCount)` on a sliced store → the
                            // item becomes the slice local, review MEDIUM-1).
                            let item = args
                                .get(1)
                                .map(|e| {
                                    format_expr_ir_tsx_renamed_locals(
                                        e,
                                        ref_names,
                                        renames,
                                        &FxHashSet::default(),
                                        warnings,
                                    )
                                })
                                .unwrap_or_else(|| "undefined".into());
                            if let Some(owner) = store_signal_owners.get(target_name.as_str()) {
                                w.emit_fmt(format_args!(
                                    "use{}.setState((s) => ({{ {}: [...s.{}, {}] }}));",
                                    owner, target_name, target_name, item
                                ));
                            } else {
                                let setter = format!("set{}", capitalize_first(target_name));
                                w.emit_fmt(format_args!(
                                    "{}(prev => [...prev, {}]);",
                                    setter, item
                                ));
                            }
                            return;
                        }
                        "array_pop" => {
                            if let Some(owner) = store_signal_owners.get(target_name.as_str()) {
                                w.emit_fmt(format_args!(
                                    "use{}.setState((s) => ({{ {}: s.{}.slice(0, -1) }}));",
                                    owner, target_name, target_name
                                ));
                            } else {
                                let setter = format!("set{}", capitalize_first(target_name));
                                w.emit_fmt(format_args!("{}(prev => prev.slice(0, -1));", setter));
                            }
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    let fmt = ReactStmtFormatter {
        expr: ReactFormatExpr {
            ref_names: ref_names.clone(),
            renames: renames.clone(),
            param_names: FxHashSet::default(),
            possibly_undefined: possibly_undefined.clone(),
            warnings,
        },
        signal_names: signal_names.clone(),
        store_signal_owners: store_signal_owners.clone(),
        possibly_undefined: possibly_undefined.clone(),
        direct_write_signals: direct_write_signals.clone(),
        direct_write_applies: direct_write_applies.clone(),
    };
    fmt.format_stmt(w, stmt);
}

// ═══════════════════════════════════════════════════════════
// Expression formatting (TSX)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir_tsx(expr: &ExprIR, ref_names: &FxHashSet<String>) -> String {
    format_expr_ir_tsx_renamed(expr, ref_names, &FxHashMap::default())
}

/// Task 4 (@form 内建对象映射): `form.errors` / `form.valid` /
/// `form.submitting` → 生成的 React state 标识符。非内建字段返回 `None`。
fn form_builtin_field(field: &str) -> Option<&'static str> {
    match field {
        "errors" => Some("formErrors"),
        "valid" => Some("formValid"),
        "submitting" => Some("formSubmitting"),
        _ => None,
    }
}

/// Task 4 (H2/L1): `form.errors.<field>` / `form?.errors?.<field>` /
/// `form.errors?.<field>`（FieldAccess 或嵌套 OptionalChain，根对象必须
/// 是 `form`）→ 索引访问 `formErrors["<field>"]`（校验错误按字段名收集）。
/// 对象名通配（`name: _`）会误捕获任意 `x?.errors`，必须限定为 `form`。
fn form_errors_index_access(object: &ExprIR, field: &str) -> Option<String> {
    let (inner_obj, inner_field) = match object {
        ExprIR::FieldAccessExpr { object, field } | ExprIR::OptionalChainExpr { object, field } => {
            (object.as_ref(), field.as_str())
        }
        _ => return None,
    };
    if inner_field == "errors"
        && matches!(inner_obj, ExprIR::IdentifierExpr { name, .. } if name == "form")
    {
        Some(format!("formErrors[\"{}\"]", field))
    } else {
        None
    }
}

/// Look up a store-field rename: key `"{store}.{field}"` → slice local name.
/// Returns `Some(local)` when the store is subscribed per field (Task 3.3);
/// `None` keeps the qualified `Store.field` form (whole-store mode).
fn store_field_rename(
    renames: &FxHashMap<String, String>,
    store: &str,
    field: &str,
) -> Option<String> {
    renames.get(&format!("{store}.{field}")).cloned()
}

/// Format an expression with an identifier rename map (used inside Zustand
/// store action bodies and getter accessors — C8). Renaming happens at the
/// AST identifier level, so string literals and object keys are never
/// corrupted (HIGH #2) and list/object literals are covered (CRITICAL).
///
/// The same map carries store-field renames (dotted `"Store.field"` keys →
/// slice local names, Task 3.3) for component codegen.
pub(super) fn format_expr_ir_tsx_renamed(
    expr: &ExprIR,
    ref_names: &FxHashSet<String>,
    renames: &FxHashMap<String, String>,
) -> String {
    format_expr_ir_tsx_renamed_locals(
        expr,
        ref_names,
        renames,
        &FxHashSet::default(),
        None, // wrapper site — no component context, no warning channel
    )
}

/// Same as [`format_expr_ir_tsx_renamed`], plus a set of identifiers that
/// shadow the renames in every position — the store action's param names
/// (fn_locals pattern, OmniView store.rs): an IdentifierExpr naming an
/// action param is the param itself (the emitted function has that binding),
/// so it must stay bare instead of being rewritten to `state.<name>`.
/// `validate_all(items)` must iterate the ARGUMENT, never the store signal
/// (maintainability HIGH-1 — silent wrong-code class).
pub(super) fn format_expr_ir_tsx_renamed_locals(
    expr: &ExprIR,
    ref_names: &FxHashSet<String>,
    renames: &FxHashMap<String, String>,
    param_names: &FxHashSet<String>,
    warnings: Option<&Mutex<Vec<String>>>,
) -> String {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => {
            if param_names.contains(name.as_str()) {
                name.clone()
            } else {
                ReactFormatExpr {
                    ref_names: ref_names.clone(),
                    renames: renames.clone(),
                    param_names: param_names.clone(),
                    // Task 5.6: free-form expression formatting has no
                    // component context — event-prop direct calls here stay
                    // bare (no corpus instances; statement contexts carry
                    // the set via generate_statement_tsx).
                    possibly_undefined: FxHashSet::default(),
                    warnings,
                }
                .format_identifier(name)
            }
        }
        ExprIR::FieldAccessExpr { object, field } => {
            // Task 4 (@form 内建对象): `form.errors` / `form.valid` /
            // `form.submitting` → 生成的 React state 标识符；嵌套
            // `form.errors.<field>` → 索引访问 `formErrors["<field>"]`
            // （校验错误按字段名收集，用户显式渲染 @if 错误）。
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if name == "form" {
                    if let Some(builtin) = form_builtin_field(field) {
                        return builtin.to_string();
                    }
                }
            }
            if let Some(rendered) = form_errors_index_access(object, field) {
                return rendered;
            }
            // Store field read on a bare store ref (`TodoStore.items`) →
            // slice local, when subscribed per field.
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if let Some(local) = store_field_rename(renames, name, field) {
                    return local;
                }
            }
            let mut obj_str = format_expr_ir_tsx_renamed_locals(
                object,
                ref_names,
                renames,
                param_names,
                warnings,
            );
            // Ref-op result field access (`ref.measure().y`): ref-op outputs
            // carry `?? fallback` and member access binds tighter than `??` —
            // parenthesize or the field reads the fallback literal (same
            // precedence parity as format_ref_op_field_access above).
            if matches!(object.as_ref(), ExprIR::RefOpExpr { .. }) {
                obj_str = format!("({obj_str})");
            }
            format!("{}.{}", obj_str, field)
        }
        ExprIR::OptionalChainExpr { object, field } => {
            // Task 4 (@form 内建对象，可选链形态): `form?.errors` 等 →
            // 生成的 React state 标识符（与 FieldAccessExpr 分支同构）。
            // H2: 对象名必须限定为 `form`——`name: _` 通配会把任意
            // `x?.errors` 误渲染成 formErrors。
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if name == "form" {
                    if let Some(builtin) = form_builtin_field(field) {
                        return builtin.to_string();
                    }
                }
            }
            // L1: 嵌套可选链 `form?.errors?.name` → `formErrors["name"]`
            // （与 FieldAccessExpr 分支同构的索引访问）。
            if let Some(rendered) = form_errors_index_access(object, field) {
                return rendered;
            }
            // Store optional-chain read (`TodoStore?.items`) → slice local
            // (the `?.` is redundant once the field is sliced).
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if let Some(local) = store_field_rename(renames, name, field) {
                    return local;
                }
            }
            ReactFormatExpr {
                ref_names: ref_names.clone(),
                renames: renames.clone(),
                param_names: param_names.clone(),
                possibly_undefined: FxHashSet::default(),
                warnings,
            }
            .format_expr(expr)
        }
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            ..
        } => {
            // Store action call on a bare store ref (`TodoStore.add(x)`) →
            // action slice call `add(x)`, when subscribed per field.
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if let Some(local) = store_field_rename(renames, name, method) {
                    let args_str: Vec<String> = args
                        .iter()
                        .map(|a| {
                            format_expr_ir_tsx_renamed_locals(
                                a,
                                ref_names,
                                renames,
                                param_names,
                                warnings,
                            )
                        })
                        .collect();
                    return format!("{}({})", local, args_str.join(", "));
                }
            }
            let obj_str = format_expr_ir_tsx_renamed_locals(
                object,
                ref_names,
                renames,
                param_names,
                warnings,
            );
            let args_str: Vec<String> = args
                .iter()
                .map(|a| {
                    format_expr_ir_tsx_renamed_locals(a, ref_names, renames, param_names, warnings)
                })
                .collect();
            translate_method_call(&obj_str, method, &args_str, Some(object.as_ref()))
        }
        ExprIR::ArrayMethod {
            method,
            subject,
            predicate,
        } => {
            // Array language methods must apply the same identifier renames
            // (Zustand action bodies: store signals → state.<name>) inside
            // both the subject and the predicate before formatting.
            let subj_str = format_expr_ir_tsx_renamed_locals(
                subject,
                ref_names,
                renames,
                param_names,
                warnings,
            );
            let pred_str = format_expr_ir_tsx_renamed_locals(
                predicate,
                ref_names,
                renames,
                param_names,
                warnings,
            );
            ReactFormatExpr {
                ref_names: ref_names.clone(),
                renames: renames.clone(),
                param_names: param_names.clone(),
                possibly_undefined: FxHashSet::default(),
                warnings,
            }
            .format_array_method(method, &subj_str, &pred_str)
        }
        _ => ReactFormatExpr {
            ref_names: ref_names.clone(),
            renames: renames.clone(),
            param_names: param_names.clone(),
            possibly_undefined: FxHashSet::default(),
            warnings,
        }
        .format_expr(expr),
    }
}

// ═══════════════════════════════════════════════════════════
// OmniLang → JavaScript built-in translation
// ═══════════════════════════════════════════════════════════

fn translate_builtin_call(name: &str, args: &[String]) -> String {
    match name {
        // Console aliases (console_log itself is in stdlib, but println/print are not)
        "println" | "print" => format!("console.log({})", args.join(", ")),
        // Storage (storage_get/set are in stdlib; storage_remove is not)
        "storage_remove" => format!("localStorage.removeItem({})", args.join(", ")),
        // Array (most array_* are in stdlib; array_join is not)
        "array_join" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let sep = args.get(1).map(|s| s.as_str()).unwrap_or(",");
            format!("{}.join({})", a, sep)
        }
        // Math (most math_* are in stdlib; math_random_int and math_clamp are not)
        "math_random_int" => {
            let min = args.first().map(|s| s.as_str()).unwrap_or("0");
            let max = args.get(1).map(|s| s.as_str()).unwrap_or("100");
            format!(
                "Math.floor(Math.random() * ({} - {} + 1)) + {}",
                max, min, min
            )
        }
        "math_clamp" => {
            let val = args.first().map(|s| s.as_str()).unwrap_or("0");
            let lo = args.get(1).map(|s| s.as_str()).unwrap_or("0");
            let hi = args.get(2).map(|s| s.as_str()).unwrap_or("1");
            format!("Math.min(Math.max({}, {}), {})", val, lo, hi)
        }
        // Date/time
        "datetime_now" => "new Date().toISOString()".to_string(),
        "datetime_from_timestamp" => {
            let ts = args.first().map(|s| s.as_str()).unwrap_or("0");
            format!("new Date({})", ts)
        }
        "datetime_format" => {
            let dt = args.first().map(|s| s.as_str()).unwrap_or("new Date()");
            let _fmt = args.get(1).map(|s| s.as_str()).unwrap_or("");
            // Simple format using toLocaleDateString or manual formatting
            format!("new Date({}).toLocaleDateString()", dt)
        }
        // Regex
        "regex_replace" => {
            let text = args.first().map(|s| s.as_str()).unwrap_or("");
            let pattern = args.get(1).map(|s| s.as_str()).unwrap_or("");
            let replacement = args.get(2).map(|s| s.as_str()).unwrap_or("");
            format!(
                "({}).replace(new RegExp({}, 'g'), {})",
                text, pattern, replacement
            )
        }
        "regex_search" => {
            let text = args.first().map(|s| s.as_str()).unwrap_or("");
            let pattern = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("({}).search(new RegExp({}))", text, pattern)
        }
        "regex_match" => {
            let text = args.first().map(|s| s.as_str()).unwrap_or("");
            let pattern = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("({}).match(new RegExp({}))", text, pattern)
        }
        // Async
        "async_spawn" => {
            let body = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("setTimeout(() => {{ {}; }}, 0)", body)
        }
        "async_sleep" => {
            let ms = args.first().map(|s| s.as_str()).unwrap_or("0");
            format!("new Promise(_r => setTimeout(_r, {}))", ms)
        }
        "async_all" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("[]");
            format!("Promise.all({})", a)
        }
        "async_race" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("[]");
            format!("Promise.race({})", a)
        }
        "async_timeout" => {
            let promise = args.first().map(|s| s.as_str()).unwrap_or("");
            let ms = args.get(1).map(|s| s.as_str()).unwrap_or("5000");
            format!("Promise.race([{}, new Promise((_, _rej) => setTimeout(() => _rej(new Error('timeout')), {}))])", promise, ms)
        }
        // Encoding
        "encode_url_encode" => format!("encodeURIComponent({})", args.join(", ")),
        "encode_url_decode" => format!("decodeURIComponent({})", args.join(", ")),
        "encode_base64_encode" => format!("btoa({})", args.join(", ")),
        "encode_base64_decode" => format!("atob({})", args.join(", ")),
        "encode_hex_encode" => {
            let s = args.first().map(|s| s.as_str()).unwrap_or("");
            format!(
                "Array.from({}, c => c.charCodeAt(0).toString(16).padStart(2, '0')).join('')",
                s
            )
        }
        "encode_hex_decode" => {
            let s = args.first().map(|s| s.as_str()).unwrap_or("");
            format!(
                "({}).match(/.{{1,2}}/g).map(b => String.fromCharCode(parseInt(b, 16))).join('')",
                s
            )
        }
        _ => format!("{}({})", name, args.join(", ")),
    }
}

fn translate_method_call(
    obj: &str,
    method: &str,
    args: &[String],
    object: Option<&ExprIR>,
) -> String {
    match method {
        // Array element access: arr.get(i) → arr[i]
        "get" => {
            let idx = args.first().map(|s| s.as_str()).unwrap_or("0");
            format!("({})[{}]", obj, idx)
        }
        // Rust-style conversions → JS
        "to_string" => format!("{}.toString({})", obj, args.join(", ")),
        "to_float" => format!("parseFloat({})", obj),
        "len" => {
            // IR-aware static path: the semantic symbol table's type
            // annotation on an IdentifierExpr receiver decides statically —
            // typed Array/String → direct `.length` (zero noise);
            // typed Map → own-key count (`.length` is undefined on a plain
            // object AND fails tsc on a typed Record receiver — TS2339).
            // Receivers without a static type (store fields, arbitrary
            // expressions) fall back to the runtime dispatch, which is
            // semantically identical for arrays/strings.
            if let Some(obj_ir) = object {
                if expr_is_typed_array(obj_ir) || expr_is_typed_string(obj_ir) {
                    return format!("{obj}.length");
                }
                if expr_is_typed_map(obj_ir) {
                    return format!("Object.keys({obj}).length");
                }
            }
            format!(
                "((_c) => Array.isArray(_c) || typeof _c === \"string\" ? _c.length : (_c ? Object.keys(_c).length : 0))({obj})"
            )
        }
        // String methods
        "starts_with" => format!("{}.startsWith({})", obj, args.join(", ")),
        "ends_with" => format!("{}.endsWith({})", obj, args.join(", ")),
        "contains" => {
            // Map literals compile to plain JS objects (no native Map) —
            // `obj.includes(key)` is a runtime TypeError AND fails tsc on a
            // typed Record receiver (TS2349). Arrays and strings keep
            // `.includes` (both have it); objects fall back to
            // hasOwnProperty. Mirrors the OmniView backend's contains
            // translation, with the string branch preserved.
            let idx = args.first().map(|s| s.as_str()).unwrap_or("");
            format!(
                "((_c) => Array.isArray(_c) || typeof _c === \"string\" ? _c.includes({idx}) : Object.prototype.hasOwnProperty.call(_c, {idx}))({obj})"
            )
        }
        "substring" => format!("{}.substring({})", obj, args.join(", ")),
        "split" => format!("{}.split({})", obj, args.join(", ")),
        "replace" => format!("{}.replace({})", obj, args.join(", ")),
        "join" => format!("{}.join({})", obj, args.join(", ")),
        "trim" => format!("{}.trim()", obj),
        "concat" => format!("{}.concat({})", obj, args.join(", ")),
        // Array mutation methods → immutable equivalents
        "push" => {
            let items = args.join(", ");
            if items.is_empty() {
                format!("[...{}]", obj)
            } else {
                format!("[...{}, {}]", obj, items)
            }
        }
        "pop" => format!("{}.slice(0, -1)", obj),
        "reverse" => format!("[...{}].reverse()", obj),
        "sort" => {
            if args.is_empty() {
                format!("[...{}].sort()", obj)
            } else {
                format!("[...{}].sort({})", obj, args.join(", "))
            }
        }
        "splice" => {
            let start = args.first().map(|s| s.as_str()).unwrap_or("0");
            // Generate: obj.slice(0, start) + items + obj.slice(start + count)
            // Simplest: [...obj.slice(0,start), ...items, ...obj.slice(start+count)]
            let items_str = if args.len() > 2 {
                args[2..].join(", ")
            } else {
                String::new()
            };
            let delete_count = args.get(1).map(|s| s.as_str()).unwrap_or("0");
            if items_str.is_empty() {
                format!("[...{obj}.slice(0, {start}), ...{obj}.slice({start} + {delete_count})]")
            } else {
                format!("[...{obj}.slice(0, {start}), {items_str}, ...{obj}.slice({start} + {delete_count})]")
            }
        }
        // Default: call as-is
        _ => format!("{}.{}({})", obj, method, args.join(", ")),
    }
}
