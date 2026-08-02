// ═══════════════════════════════════════════════════════════
// Statement generation (TSX)
// ═══════════════════════════════════════════════════════════

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::CodeWriter;
use crate::codegen::{
    capitalize_first, try_format_stdlib_call, JsonFormatConfig, Target, JSON_REACT,
};
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// React TSX expression formatter.
///
/// Uses JS-family syntax with `translate_builtin_call` for stdlib calls
/// and `translate_method_call` for method dispatch.
pub(crate) struct ReactFormatExpr {
    pub ref_names: FxHashSet<String>,
    /// Identifier → replacement map (e.g. store signal `count` → `state.count`
    /// inside Zustand action bodies). Applied at identifier level only — string
    /// literals and object keys are never touched (C8 fix, HIGH #2).
    pub renames: FxHashMap<String, String>,
}

impl FormatExpr for ReactFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &JSON_REACT
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
            },
            signal_names: FxHashSet::default(),
            store_signal_owners: FxHashMap::default(),
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
        if let Some(native) = try_format_stdlib_call(name, args, Target::React) {
            return native;
        }
        translate_builtin_call(name, args)
    }

    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        translate_method_call(obj, method, args)
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
                MeasureWhat::Position => format!("{base}?.getBoundingClientRect()"),
                MeasureWhat::Width => format!("{base}?.getBoundingClientRect().width"),
                MeasureWhat::Height => format!("{base}?.getBoundingClientRect().height"),
            },
            RefOpIR::Select => format!("{base}?.select()"),
            RefOpIR::SetSelection { start, end } => format!(
                "{base}?.setSelectionRange({}, {})",
                self.format_expr(start),
                self.format_expr(end)
            ),
            RefOpIR::Value => format!("{base}?.value"),
            // Assignment needs an explicit guard — `?.` is illegal on an
            // assignment LHS, and a setValue from @mount runs before the
            // ref attaches (would otherwise throw TypeError).
            RefOpIR::SetValue { value } => {
                format!(
                    "if ({base}) {{ {base}.value = {}; }}",
                    self.format_expr(value)
                )
            }
            RefOpIR::CapturePointer { id } => {
                format!("{base}?.setPointerCapture({})", self.format_expr(id))
            }
            RefOpIR::ReleasePointer { id } => {
                format!("{base}?.releasePointerCapture({})", self.format_expr(id))
            }
            #[allow(unreachable_patterns)]
            _ => format!("/* RefOp::{op:?} on {ref_name} */"),
        }
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
                            // Prefix unused bindings with _ to suppress ESLint no-unused-vars
                            let names: Vec<String> = bindings
                                .iter()
                                .map(|b| {
                                    if body.contains(b) {
                                        b.clone()
                                    } else {
                                        format!("_{}", b)
                                    }
                                })
                                .collect();
                            format!("const {{ {} }} = _v; ", names.join(", "))
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
        format!("(() => {{ const _v = {matched}; {} }})()", cases.join(" "))
    }
}

/// Format `ref.scrollTo({ left: x, top: y })` — only axes provided in the
/// source call are included in the options object.
fn format_ref_scroll(
    base: &str,
    fmt: &ReactFormatExpr,
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
    fmt: &ReactFormatExpr,
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
pub(super) struct ReactStmtFormatter {
    pub expr: ReactFormatExpr,
    pub signal_names: FxHashSet<String>,
    /// Maps store signal names to their owning store ref (e.g. `count` → `AppStore`).
    pub store_signal_owners: FxHashMap<String, String>,
}

impl FormatStmt for ReactStmtFormatter {
    type ExprFormatter = ReactFormatExpr;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn use_const_for_immutable(&self) -> bool {
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
        w.emit_fmt(format_args!("{}{};", event_name, arg_str));
    }

    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        match intent {
            MutationIntent::DirectAssign { target, value, .. } => {
                let target_str = &target.name;
                let amount = value
                    .as_ref()
                    .map(|v| self.expr.format_expr(v))
                    .unwrap_or_else(|| "0".into());
                // Store signal → Zustand setState on the imported store hook (C7)
                if let Some(owner) = self.store_signal_owners.get(target_str.as_str()) {
                    w.emit_fmt(format_args!(
                        "use{}.setState({{ {}: {} }});",
                        owner, target_str, amount
                    ));
                } else if self.signal_names.contains(target_str.as_str()) {
                    let setter = format!("set{}", capitalize_first(target_str));
                    w.emit_fmt(format_args!("{}({});", setter, amount));
                } else {
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

impl ReactStmtFormatter {
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

pub(super) fn generate_statement_tsx(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    ref_names: &FxHashSet<String>,
) {
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
                if signal_names.contains(target_name.as_str())
                    || store_signal_owners.contains_key(target_name.as_str())
                {
                    match name.as_str() {
                        "array_push" => {
                            let item = args
                                .get(1)
                                .map(|e| format_expr_ir_tsx(e, ref_names))
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
            renames: FxHashMap::default(),
        },
        signal_names: signal_names.clone(),
        store_signal_owners: store_signal_owners.clone(),
    };
    fmt.format_stmt(w, stmt);
}

// ═══════════════════════════════════════════════════════════
// Expression formatting (TSX)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir_tsx(expr: &ExprIR, ref_names: &FxHashSet<String>) -> String {
    format_expr_ir_tsx_renamed(expr, ref_names, &FxHashMap::default())
}

/// Format an expression with an identifier rename map (used inside Zustand
/// store action bodies and getter accessors — C8). Renaming happens at the
/// AST identifier level, so string literals and object keys are never
/// corrupted (HIGH #2) and list/object literals are covered (CRITICAL).
pub(super) fn format_expr_ir_tsx_renamed(
    expr: &ExprIR,
    ref_names: &FxHashSet<String>,
    renames: &FxHashMap<String, String>,
) -> String {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => ReactFormatExpr {
            ref_names: ref_names.clone(),
            renames: renames.clone(),
        }
        .format_identifier(name),
        ExprIR::FieldAccessExpr { object, field } => {
            let obj_str = format_expr_ir_tsx_renamed(object, ref_names, renames);
            format!("{}.{}", obj_str, field)
        }
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            ..
        } => {
            let obj_str = format_expr_ir_tsx_renamed(object, ref_names, renames);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| format_expr_ir_tsx_renamed(a, ref_names, renames))
                .collect();
            translate_method_call(&obj_str, method, &args_str)
        }
        ExprIR::ArrayMethod {
            method,
            subject,
            predicate,
        } => {
            // Array language methods must apply the same identifier renames
            // (Zustand action bodies: store signals → state.<name>) inside
            // both the subject and the predicate before formatting.
            let subj_str = format_expr_ir_tsx_renamed(subject, ref_names, renames);
            let pred_str = format_expr_ir_tsx_renamed(predicate, ref_names, renames);
            ReactFormatExpr {
                ref_names: ref_names.clone(),
                renames: renames.clone(),
            }
            .format_array_method(method, &subj_str, &pred_str)
        }
        _ => ReactFormatExpr {
            ref_names: ref_names.clone(),
            renames: renames.clone(),
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

fn translate_method_call(obj: &str, method: &str, args: &[String]) -> String {
    match method {
        // Array element access: arr.get(i) → arr[i]
        "get" => {
            let idx = args.first().map(|s| s.as_str()).unwrap_or("0");
            format!("({})[{}]", obj, idx)
        }
        // Rust-style conversions → JS
        "to_string" => format!("{}.toString({})", obj, args.join(", ")),
        "to_float" => format!("parseFloat({})", obj),
        "len" => format!("{}.length", obj),
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
