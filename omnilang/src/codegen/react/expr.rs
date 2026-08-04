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
#[derive(Clone)]
pub(crate) struct ReactFormatExpr {
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
                param_names: self.param_names.clone(),
                // Task 5.6: expression-position blocks have no component
                // context here — event-prop emits inside them stay bare
                // (no corpus instances; the computed-block body path uses
                // generate_statement_tsx, which carries the set).
                possibly_undefined: FxHashSet::default(),
            },
            signal_names: FxHashSet::default(),
            store_signal_owners: FxHashMap::default(),
            possibly_undefined: FxHashSet::default(),
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

    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        // Store action call through a slice selector: `TodoStore.add(x)` →
        // `add(x)` when the store-field rename map carries that field.
        // The obj string is the formatted receiver — for unrenamed store
        // refs it is exactly the store alias, matching the dotted rename key.
        if let Some(local) = store_field_rename(&self.renames, obj, method) {
            return format!("{}({})", local, args.join(", "));
        }
        translate_method_call(obj, method, args)
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
    /// Task 5.6 (deferral #61): names that may be undefined at an
    /// event-handler call site (event props, null-initial signals) — the
    /// `emit onX(...)` keyword form emits `onX?.(...)` for these. Mirrors
    /// `ReactFormatExpr.possibly_undefined` for the direct-call form.
    pub possibly_undefined: FxHashSet<String>,
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
                        let line = format_expr_ir_tsx_renamed(v, ref_names, renames);
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
                                .map(|e| format_expr_ir_tsx_renamed(e, ref_names, renames))
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
        },
        signal_names: signal_names.clone(),
        store_signal_owners: store_signal_owners.clone(),
        possibly_undefined: possibly_undefined.clone(),
    };
    fmt.format_stmt(w, stmt);
}

// ═══════════════════════════════════════════════════════════
// Expression formatting (TSX)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir_tsx(expr: &ExprIR, ref_names: &FxHashSet<String>) -> String {
    format_expr_ir_tsx_renamed(expr, ref_names, &FxHashMap::default())
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
    format_expr_ir_tsx_renamed_locals(expr, ref_names, renames, &FxHashSet::default())
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
                }
                .format_identifier(name)
            }
        }
        ExprIR::FieldAccessExpr { object, field } => {
            // Store field read on a bare store ref (`TodoStore.items`) →
            // slice local, when subscribed per field.
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if let Some(local) = store_field_rename(renames, name, field) {
                    return local;
                }
            }
            let obj_str =
                format_expr_ir_tsx_renamed_locals(object, ref_names, renames, param_names);
            format!("{}.{}", obj_str, field)
        }
        ExprIR::OptionalChainExpr { object, field } => {
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
                            format_expr_ir_tsx_renamed_locals(a, ref_names, renames, param_names)
                        })
                        .collect();
                    return format!("{}({})", local, args_str.join(", "));
                }
            }
            let obj_str =
                format_expr_ir_tsx_renamed_locals(object, ref_names, renames, param_names);
            let args_str: Vec<String> = args
                .iter()
                .map(|a| format_expr_ir_tsx_renamed_locals(a, ref_names, renames, param_names))
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
            let subj_str =
                format_expr_ir_tsx_renamed_locals(subject, ref_names, renames, param_names);
            let pred_str =
                format_expr_ir_tsx_renamed_locals(predicate, ref_names, renames, param_names);
            ReactFormatExpr {
                ref_names: ref_names.clone(),
                renames: renames.clone(),
                param_names: param_names.clone(),
                possibly_undefined: FxHashSet::default(),
            }
            .format_array_method(method, &subj_str, &pred_str)
        }
        _ => ReactFormatExpr {
            ref_names: ref_names.clone(),
            renames: renames.clone(),
            param_names: param_names.clone(),
            possibly_undefined: FxHashSet::default(),
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
