//! Flutter ref codegen: GlobalKey + on-demand controllers.
//!
//! Each declared `@ref={name}` becomes a `GlobalKey` attached to the bound
//! widget (`key: _nameKey`). Controllers are declared on demand, only when
//! the component actually uses the corresponding capability:
//!
//! | Capability | Controller | Trigger |
//! |------------|------------|---------|
//! | focus/blur | `FocusNode _nameFocus` | `ref.focus()`/`ref.blur()` or `focus={sig}` |
//! | scroll     | `ScrollController _nameScroll` | `ref.scrollTo()`/`ref.scrollBy()` or `scroll-x/y={sig}` |
//! | text       | `TextEditingController _nameText` | `ref.value`/`ref.select()`/`ref.setSelection()` or `value={sig}` |
//! | measure    | (none — inline `RenderBox` reads) | `ref.measure()` / `ref.width` / `ref.height` / `ref.position` |
//! | pointer capture | (none — hard error) | `ref.capturePointer()`/`ref.releasePointer()` — unsupported on Flutter; compile error + generated `throw UnsupportedError` (design D3 / spec §10.3) |
//!
//! Semantic bindings (`focus={sig}`, `scroll-x/y={sig}`) are applied by a
//! `_syncSemanticBindings()` State method registered as a post-frame
//! callback from `build` — dep-scoped (Task 5.6): each binding snapshots its
//! value and is re-applied only when the value CHANGED, giving reactive
//! parity with React's `useEffect(..., [deps])` without re-yanking focus or
//! replaying scroll animations on unrelated rebuilds. `bind:value` (design
//! D2) additionally installs a controller listener so user edits flow back
//! into the bound signal through `setState`, guarded by a flag so the
//! programmatic sync never fights (or echoes) the user's typing.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::{CodeWriter, JsonFormatConfig, Target};
use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Compile-error hint for pointer capture on Flutter — verbatim from the v2
/// design doc 域 1 诊断表 / spec §10.3. Pointer capture is floor-outside on
/// Flutter; the generated code additionally throws `UnsupportedError` so
/// failure is loud even if the diagnostic is bypassed.
const FLUTTER_POINTER_CAPTURE_ERROR: &str = "Flutter 不支持指针捕获，用 GestureDetector";

use super::capitalize_first;
use super::expr::rewrite_store_accesses;
use super::format_expr_ir_flutter;

// ═══════════════════════════════════════════════════════════
// Per-ref usage analysis
// ═══════════════════════════════════════════════════════════

/// Which capabilities a ref actually uses — drives on-demand controller
/// declarations (Step 1 of the ref codegen plan).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RefUsage {
    /// `ref.focus()`/`ref.blur()` or `focus={sig}` — needs a `FocusNode`.
    pub needs_focus: bool,
    /// `ref.scrollTo()`/`ref.scrollBy()` or `scroll-x/y={sig}` — needs a
    /// `ScrollController`.
    pub needs_scroll: bool,
    /// `ref.value`/`ref.setValue()`/`ref.select()`/`ref.setSelection()` or
    /// `value={sig}` — needs a `TextEditingController`.
    pub needs_text: bool,
    /// `ref.measure()`-family reads — resolved inline via `RenderBox` at call
    /// time; no controller required.
    pub needs_measure: bool,
    /// `value={sig}` binding present — installs a controller listener and a
    /// guard flag field.
    pub has_value_binding: bool,
    /// `ref.capturePointer()`/`ref.releasePointer()` present — unsupported on
    /// Flutter: drives the hard compile error (design D3 / spec §10.3).
    pub needs_pointer_capture: bool,
}

impl RefUsage {
    /// Whether any controller needs to be disposed.
    fn has_controller(self) -> bool {
        self.needs_focus || self.needs_scroll || self.needs_text
    }
}

/// Collect per-ref usage flags by scanning every expression the component
/// owns: fn bodies, @effect bodies, lifecycle bodies, computed getters, and
/// the view tree (inline handler bodies, prop values, semantic bindings).
///
/// Every declared ref is seeded with an empty `RefUsage` so its GlobalKey is
/// always emitted even when the ref is never exercised.
///
/// The second return value carries hard compile errors for ops the Flutter
/// target does not support (`capturePointer()`/`releasePointer()`, design
/// D3 / spec §10.3) — one error per ref that uses them.
pub(crate) fn collect_ref_usage(
    comp: &ComponentIR,
) -> (FxHashMap<String, RefUsage>, Vec<Diagnostic>) {
    let mut usage: FxHashMap<String, RefUsage> = FxHashMap::default();
    for r in &comp.refs {
        usage.entry(r.name.clone()).or_default();
    }
    for f in &comp.fns {
        for s in &f.body.statements {
            scan_stmt(s, &mut usage);
        }
    }
    for e in &comp.effects {
        for s in &e.body.statements {
            scan_stmt(s, &mut usage);
        }
    }
    for lc in &comp.lifecycles {
        for s in &lc.body.statements {
            scan_stmt(s, &mut usage);
        }
    }
    for c in &comp.computeds {
        scan_expr(&c.body, &mut usage);
    }
    if let Some(view) = &comp.view {
        scan_view(view, &mut usage);
    }
    let (line, col) = comp.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
    let errors: Vec<Diagnostic> = usage
        .iter()
        .filter(|(_, u)| u.needs_pointer_capture)
        .map(|_| Diagnostic {
            severity: Severity::Error,
            kind: DiagnosticKind::ECodegen,
            message: FLUTTER_POINTER_CAPTURE_ERROR.to_string(),
            file: comp.source_file.clone(),
            line,
            col,
            end_line: None,
            end_col: None,
            suggestion: None,
        })
        .collect();
    (usage, errors)
}

/// Record a `RefOpExpr`'s capability flags.
fn record_ref_op(ref_name: &str, op: &RefOpIR, usage: &mut FxHashMap<String, RefUsage>) {
    let u = usage.entry(ref_name.to_string()).or_default();
    match op {
        RefOpIR::Focus | RefOpIR::Blur => u.needs_focus = true,
        RefOpIR::ScrollTo { .. } | RefOpIR::ScrollBy { .. } => u.needs_scroll = true,
        RefOpIR::Value | RefOpIR::Select | RefOpIR::SetSelection { .. } => u.needs_text = true,
        RefOpIR::Measure { .. } => u.needs_measure = true,
        RefOpIR::CapturePointer | RefOpIR::ReleasePointer => {
            // Pointer capture is unsupported on Flutter (design D3): flagged
            // here so collect_ref_usage emits the hard error diagnostic.
            u.needs_pointer_capture = true;
        }
        // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Scan `RefOpIR` child expressions (args like x/y/dx/dy/start/end/value)
/// for nested ref ops.
fn scan_ref_op_args(op: &RefOpIR, usage: &mut FxHashMap<String, RefUsage>) {
    match op {
        RefOpIR::ScrollTo { x, y } => {
            scan_opt_expr(x.as_deref(), usage);
            scan_opt_expr(y.as_deref(), usage);
        }
        RefOpIR::ScrollBy { dx, dy } => {
            scan_opt_expr(dx.as_deref(), usage);
            scan_opt_expr(dy.as_deref(), usage);
        }
        RefOpIR::SetSelection { start, end } => {
            scan_expr(start, usage);
            scan_expr(end, usage);
        }
        // No-arg ops (Focus/Blur/Measure/Select/Value/CapturePointer/
        // ReleasePointer) carry no child expressions.
        _ => {}
    }
}

fn scan_opt_expr(expr: Option<&ExprIR>, usage: &mut FxHashMap<String, RefUsage>) {
    if let Some(e) = expr {
        scan_expr(e, usage);
    }
}

/// Recursively scan an expression for `RefOpExpr` nodes.
fn scan_expr(expr: &ExprIR, usage: &mut FxHashMap<String, RefUsage>) {
    match expr {
        ExprIR::RefOpExpr { ref_name, op } => {
            record_ref_op(ref_name, op, usage);
            scan_ref_op_args(op, usage);
        }
        ExprIR::FieldAccessExpr { object, .. } => scan_expr(object, usage),
        ExprIR::OptionalChainExpr { object, .. } => scan_expr(object, usage),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            scan_expr(left, usage);
            scan_expr(right, usage);
        }
        ExprIR::UnaryOpExpr { operand, .. } => scan_expr(operand, usage),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            scan_expr(cond, usage);
            scan_expr(then_expr, usage);
            scan_expr(else_expr, usage);
        }
        ExprIR::CallExpr { args, .. } | ExprIR::ArrayExpr { elements: args } => {
            for a in args {
                scan_expr(a, usage);
            }
        }
        ExprIR::SpreadExpr { operand } => scan_expr(operand, usage),
        ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                scan_expr(v, usage);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for a in call_args {
                scan_expr(a, usage);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            scan_expr(object, usage);
            for a in args {
                scan_expr(a, usage);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for s in statements {
                scan_stmt(s, usage);
            }
            scan_expr(tail, usage);
        }
        ExprIR::IndexExpr { object, index } => {
            scan_expr(object, usage);
            scan_expr(index, usage);
        }
        ExprIR::AwaitExpr { operand } => scan_expr(operand, usage),
        ExprIR::ClosureExpr { body, .. } => scan_expr(body, usage),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, v) in fields {
                scan_expr(v, usage);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            scan_expr(matched, usage);
            for arm in arms {
                for s in &arm.body.statements {
                    scan_stmt(s, usage);
                }
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                scan_expr(k, usage);
                scan_expr(v, usage);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            scan_expr(subject, usage);
            scan_expr(predicate, usage);
        }
        ExprIR::LiteralExpr { .. } | ExprIR::IdentifierExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Recursively scan a statement for `RefOpExpr` nodes.
fn scan_stmt(stmt: &StatementIR, usage: &mut FxHashMap<String, RefUsage>) {
    match stmt {
        StatementIR::LetStmtIR { value, .. } => scan_opt_expr(value.as_ref(), usage),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            scan_expr(condition, usage);
            for s in &then_body.statements {
                scan_stmt(s, usage);
            }
            for (c, b) in else_ifs {
                scan_expr(c, usage);
                for s in &b.statements {
                    scan_stmt(s, usage);
                }
            }
            if let Some(b) = else_body {
                for s in &b.statements {
                    scan_stmt(s, usage);
                }
            }
        }
        StatementIR::ReturnStmtIR { value } => scan_opt_expr(value.as_ref(), usage),
        StatementIR::MutationStmt { intent: Some(i) } => scan_mutation_intent(i, usage),
        StatementIR::MutationStmt { intent: None } => {}
        StatementIR::EventEmitStmtIR { args, .. } => {
            for a in args {
                scan_expr(a, usage);
            }
        }
        StatementIR::ExprStmtIR { expr } => scan_expr(expr, usage),
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                scan_stmt(s, usage);
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            scan_expr(condition, usage);
            for s in &body.statements {
                scan_stmt(s, usage);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            scan_expr(iterable, usage);
            for s in &body.statements {
                scan_stmt(s, usage);
            }
        }
        StatementIR::ThrowStmtIR { expr } => scan_expr(expr, usage),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                scan_stmt(s, usage);
            }
            if let Some(b) = catch_body {
                for s in &b.statements {
                    scan_stmt(s, usage);
                }
            }
            if let Some(b) = finally_body {
                for s in &b.statements {
                    scan_stmt(s, usage);
                }
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Scan a mutation intent's expression fields.
fn scan_mutation_intent(intent: &MutationIntent, usage: &mut FxHashMap<String, RefUsage>) {
    match intent {
        MutationIntent::DirectAssign { value, .. } => scan_opt_expr(value.as_deref(), usage),
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. }
        | MutationIntent::Push { item: amount, .. } => scan_opt_expr(amount.as_deref(), usage),
        MutationIntent::Pop { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Scan the view tree for ref ops (inline handler bodies, prop values, class
/// map conditions) and record semantic-binding capability flags.
fn scan_view(node: &ViewTree, usage: &mut FxHashMap<String, RefUsage>) {
    match node {
        ViewTree::Element(el) => {
            for value in el.props.values() {
                scan_expr(value, usage);
            }
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    for s in &body.statements {
                        scan_stmt(s, usage);
                    }
                }
            }
            for value in el.class_map_bindings.values() {
                scan_expr(value, usage);
            }
            // Semantic bindings on @ref-bound elements (Step 1):
            // focus={sig} → needs_focus, scroll-y/x={sig} → needs_scroll.
            // (The value={sig} form is removed — design D2 — bind:value below
            // is the sole two-way form and drives the text controller.)
            if let Some(ref_name) = &el.ref_binding {
                for (kind, expr) in &el.semantic_bindings {
                    let u = usage.entry(ref_name.clone()).or_default();
                    match kind.as_str() {
                        "focus" => u.needs_focus = true,
                        "scroll-x" | "scroll-y" => u.needs_scroll = true,
                        _ => {}
                    }
                    scan_expr(expr, usage);
                }
                // bind:value on a ref'd element → needs_text (+ listener):
                // the text controller carries the value and the guarded
                // listener writes user edits back into the signal.
                if el.bindings.contains_key("value") {
                    let u = usage.entry(ref_name.clone()).or_default();
                    u.needs_text = true;
                    u.has_value_binding = true;
                }
            }
            for child in &el.children {
                scan_view(child, usage);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                scan_expr(&branch.condition, usage);
                if let Some(ref body) = branch.body {
                    scan_view(body, usage);
                }
            }
            if let Some(ref fb) = cond.fallback {
                scan_view(fb, usage);
            }
        }
        ViewTree::ListRendering(list) => {
            scan_expr(&list.iterable, usage);
            if let Some(ref key) = list.key_expr {
                scan_expr(key, usage);
            }
            if let Some(ref body) = list.body {
                scan_view(body, usage);
            }
            if let Some(ref empty) = list.empty_body {
                scan_view(empty, usage);
            }
        }
        ViewTree::Interpolation(expr) => scan_expr(expr, usage),
        ViewTree::Fragment(children) => {
            for c in children {
                scan_view(c, usage);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                scan_view(c, usage);
            }
        }
        ViewTree::Text(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════
// State field declarations
// ═══════════════════════════════════════════════════════════

/// GlobalKey field name for a ref (`box` → `_boxKey`).
pub(crate) fn ref_key_name(ref_name: &str) -> String {
    format!("_{}Key", ref_name)
}

/// FocusNode field name for a ref (`box` → `_boxFocus`).
pub(crate) fn ref_focus_name(ref_name: &str) -> String {
    format!("_{}Focus", ref_name)
}

/// ScrollController field name for a ref (`box` → `_boxScroll`).
pub(crate) fn ref_scroll_name(ref_name: &str) -> String {
    format!("_{}Scroll", ref_name)
}

/// TextEditingController field name for a ref (`box` → `_boxText`).
pub(crate) fn ref_text_name(ref_name: &str) -> String {
    format!("_{}Text", ref_name)
}

/// Guard flag name for a ref's value binding (`box` → `_syncingBoxText`).
fn ref_text_guard_name(ref_name: &str) -> String {
    format!("_syncing{}Text", capitalize_first(ref_name))
}

/// Emit per-ref field declarations in the State class (Step 2):
/// `final GlobalKey _nameKey = GlobalKey();` plus on-demand controllers and
/// value-binding guard flags.
pub(crate) fn emit_ref_declarations(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    usage: &FxHashMap<String, RefUsage>,
) {
    for r in &comp.refs {
        let u = usage.get(&r.name).copied().unwrap_or_default();
        w.emit_fmt(format_args!(
            "final GlobalKey {} = GlobalKey();",
            ref_key_name(&r.name)
        ));
        if u.needs_focus {
            w.emit_fmt(format_args!(
                "final FocusNode {} = FocusNode();",
                ref_focus_name(&r.name)
            ));
        }
        if u.needs_scroll {
            w.emit_fmt(format_args!(
                "final ScrollController {} = ScrollController();",
                ref_scroll_name(&r.name)
            ));
        }
        if u.needs_text {
            w.emit_fmt(format_args!(
                "final TextEditingController {} = TextEditingController();",
                ref_text_name(&r.name)
            ));
        }
        if u.has_value_binding {
            w.emit_fmt(format_args!(
                "bool {} = false;",
                ref_text_guard_name(&r.name)
            ));
        }
    }
}

// ═══════════════════════════════════════════════════════════
// initState semantic-binding setup (Steps 3 & 5)
// ═══════════════════════════════════════════════════════════

/// Whether any element in the component's view carries semantic ref bindings
/// (`focus={sig}` / `scroll-x/y={sig}` / `value={sig}`) — gates the initState
/// post-frame setup.
pub(crate) fn has_semantic_bindings(comp: &ComponentIR) -> bool {
    let Some(view) = &comp.view else {
        return false;
    };
    let mut found = false;
    scan_semantic_bindings(view, &mut found);
    found
}

fn scan_semantic_bindings(node: &ViewTree, found: &mut bool) {
    if *found {
        return;
    }
    match node {
        ViewTree::Element(el) => {
            if !el.semantic_bindings.is_empty() {
                *found = true;
                return;
            }
            for child in &el.children {
                scan_semantic_bindings(child, found);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    scan_semantic_bindings(body, found);
                }
            }
            if let Some(ref fb) = cond.fallback {
                scan_semantic_bindings(fb, found);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                scan_semantic_bindings(body, found);
            }
            if let Some(ref empty) = list.empty_body {
                scan_semantic_bindings(empty, found);
            }
        }
        ViewTree::Fragment(children) => {
            for c in children {
                scan_semantic_bindings(c, found);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                scan_semantic_bindings(c, found);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// A ref + its semantic bindings, collected in view-tree order.
type RefBinding<'a> = (&'a str, Vec<(&'a str, &'a ExprIR)>);

/// Whether any `bind:value` binding targets a bare component signal — this
/// installs a guarded controller listener in `initState` (user edits flow
/// back into the signal through `setState`). The `value={sig}` semantic
/// form is removed (design D2); `bind:value` is the sole two-way binding.
pub(crate) fn has_text_listeners(comp: &ComponentIR) -> bool {
    let Some(view) = &comp.view else {
        return false;
    };
    let mut bindings: Vec<(&str, &str)> = Vec::new();
    collect_binding_refs(view, &mut bindings);
    let signal_names: Vec<&str> = comp.signals.iter().map(|s| s.name.as_str()).collect();
    bindings.iter().any(|(_, name)| signal_names.contains(name))
}

/// Emit the guarded `bind:value` controller listeners inside `initState`
/// (Step 5): user edits flow back into the bound signal through `setState` —
/// a bare field write would leave every other consumer of the signal stale.
/// The guard flag stops the programmatic sync from echoing into the signal.
/// Only bare component signals get a listener (store-qualified bindings are
/// re-applied by `_syncSemanticBindings` from the provider instead).
pub(crate) fn emit_semantic_binding_listeners(w: &mut CodeWriter, comp: &ComponentIR) {
    let Some(view) = &comp.view else {
        return;
    };
    let mut bindings: Vec<(&str, &str)> = Vec::new();
    collect_binding_refs(view, &mut bindings);
    let signal_names: Vec<&str> = comp.signals.iter().map(|s| s.name.as_str()).collect();
    let mut first = true;
    for (ref_name, name) in &bindings {
        if !signal_names.contains(name) {
            continue;
        }
        if !first {
            w.emit("");
        }
        first = false;
        let text = ref_text_name(ref_name);
        let guard = ref_text_guard_name(ref_name);
        w.emit_fmt(format_args!("{}.addListener(() {{", text));
        w.indent();
        w.emit_fmt(format_args!("if ({}) {{", guard));
        w.indent();
        w.emit("return;");
        w.dedent();
        w.emit("}");
        w.emit("setState(() {");
        w.indent();
        w.emit_fmt(format_args!("{} = {}.text;", name, text));
        w.dedent();
        w.emit("});");
        w.dedent();
        w.emit("});");
    }
}

/// Snapshot field name for a semantic binding kind on a ref
/// (`focus` + `inputBox` → `_prevFocusInputBox`).
fn binding_prev_field(kind: &str, ref_name: &str) -> String {
    let cap = capitalize_first(ref_name);
    match kind {
        "focus" => format!("_prevFocus{cap}"),
        "scroll-x" => format!("_prevScrollX{cap}"),
        "scroll-y" => format!("_prevScrollY{cap}"),
        _ => format!("_prevValue{cap}"),
    }
}

/// Local name for the snapshotted value inside `_syncSemanticBindings`
/// (`focus` + `inputBox` → `syncFocusInputBox`).
fn binding_sync_local(kind: &str, ref_name: &str) -> String {
    let cap = capitalize_first(ref_name);
    let kind_cap = match kind {
        "focus" => "Focus",
        "scroll-x" => "ScrollX",
        "scroll-y" => "ScrollY",
        _ => "Value",
    };
    format!("sync{kind_cap}{cap}")
}

/// Resolve the `TypeIR` of a binding expression when it is a plain
/// identifier (component signal/computed/prop) or a store field access
/// (`Store.sig`) — the two shapes the binding forms produce. Anything else
/// falls back to `None` (the caller uses the kind's default type).
fn binding_expr_type<'a>(
    comp: &'a ComponentIR,
    ir: &'a IRProgram,
    expr: &ExprIR,
) -> Option<&'a TypeIR> {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => comp
            .signals
            .iter()
            .find(|s| s.name == *name)
            .map(|s| &s.r#type)
            .or_else(|| {
                comp.computeds
                    .iter()
                    .find(|c| c.name == *name)
                    .map(|c| &c.r#type)
            })
            .or_else(|| comp.props.get(name)),
        ExprIR::FieldAccessExpr { object, field } => match object.as_ref() {
            ExprIR::IdentifierExpr { name: store, .. } => ir
                .all_stores()
                .get(store.as_str())
                .and_then(|s| s.signals.get(field.as_str())),
            _ => None,
        },
        _ => None,
    }
}

/// Dart annotation for a binding's snapshot field: the bound expression's
/// type when resolvable (else the kind's default — bool for focus, double
/// for scroll, String for value), widened to nullable so the field can be
/// read before the first sync (a null previous value means "apply").
fn binding_prev_ann(kind: &str, expr: &ExprIR, comp: &ComponentIR, ir: &IRProgram) -> String {
    let default = match kind {
        "focus" => "bool",
        "scroll-x" | "scroll-y" => "double",
        _ => "String",
    };
    let ty = binding_expr_type(comp, ir, expr);
    let ann = ty
        .map(|t| crate::codegen::dart::dart_type(t.kind.as_str(), &t.type_args))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string());
    if ty.is_some_and(|t| t.is_nullable) {
        ann
    } else {
        format!("{ann}?")
    }
}

/// Emit the previous-value snapshot fields for semantic bindings (Task 5.6):
/// `_prevFocus{Ref}` / `_prevScrollY{Ref}` / `_prevScrollX{Ref}` /
/// `_prevValue{Ref}`. `_syncSemanticBindings` compares each binding against
/// its field so a binding is re-applied ONLY when its value CHANGED — an
/// unrelated setState-driven rebuild must not re-request focus (focus
/// stealing, review HIGH-2) or replay the scroll animation (parity with
/// React's deps-scoped effect, unlike the previous unconditional re-apply).
///
/// Returns whether any field was emitted (the caller appends the separating
/// blank line).
pub(crate) fn emit_semantic_binding_prev_fields(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    ir: &IRProgram,
) -> bool {
    let Some(view) = &comp.view else {
        return false;
    };
    let mut bindings: Vec<RefBinding> = Vec::new();
    collect_ref_bindings(view, &mut bindings);
    let mut value_bindings: Vec<(&str, &str)> = Vec::new();
    collect_binding_refs(view, &mut value_bindings);
    let mut emitted = false;
    // Dedupe by field name: duplicate @ref bindings (a warning-level
    // semantic violation) must never emit duplicate field declarations —
    // that would be a generated-code compile error.
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for (ref_name, ref_bindings) in &bindings {
        // Deterministic per-ref order: focus, scroll-y, scroll-x.
        for kind in ["focus", "scroll-y", "scroll-x"] {
            for (k, expr) in ref_bindings {
                if *k != kind {
                    continue;
                }
                let field = binding_prev_field(kind, ref_name);
                if !seen.insert(field.clone()) {
                    continue;
                }
                // binding_prev_ann already widens to nullable (a null
                // previous value means "first sync — apply").
                let ann = binding_prev_ann(kind, expr, comp, ir);
                w.emit_fmt(format_args!("{} {};", ann, field));
                emitted = true;
            }
        }
    }
    for (ref_name, name) in &value_bindings {
        let field = binding_prev_field("value", ref_name);
        if !seen.insert(field.clone()) {
            continue;
        }
        let expr = binding_name_to_expr(name);
        let ann = binding_prev_ann("value", &expr, comp, ir);
        w.emit_fmt(format_args!("{} {};", ann, field));
        emitted = true;
    }
    emitted
}

/// Emit the `_syncSemanticBindings()` State method — re-applies every
/// semantic binding (`focus={sig}`, `scroll-x/y={sig}`) and the `bind:value`
/// two-way value bindings.
///
/// The method is registered as a post-frame callback from `build`, so it
/// re-runs after every setState-driven rebuild. Dep-scoped (Task 5.6, review
/// HIGH-2): each binding's value is snapshotted into a `_prev{Kind}{Ref}`
/// field and re-applied ONLY when it CHANGED — parity with React's
/// `useEffect(..., [deps])`. Previously every rebuild re-requested focus
/// (typing in a sibling field through `bind:value` yanked the focus back)
/// and replayed the 300ms scroll animation. Flipping `active` re-focuses,
/// moving `pos` re-scrolls, and programmatic `text` writes still reach the
/// TextField after mount — but only when the bound value actually changed.
///
/// The `bind:value` re-apply additionally skips the controller write when
/// the controller already holds the bound text, so typing in the field does
/// not reset the caret on every rebuild.
///
/// Store-backed binding expressions are rewritten to `ref.read(...)` — the
/// callback-legal Riverpod 3.0 form of the view-context `ref.watch(...)`
/// rewrite.
pub(crate) fn emit_semantic_binding_sync(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    store_provider_map: &FxHashMap<String, String>,
    ir: &IRProgram,
) {
    let Some(view) = &comp.view else {
        return;
    };
    let mut bindings: Vec<RefBinding> = Vec::new();
    collect_ref_bindings(view, &mut bindings);
    // bind:value pairs contribute the "value" re-apply (design D2).
    let mut value_bindings: Vec<(&str, &str)> = Vec::new();
    collect_binding_refs(view, &mut value_bindings);
    if bindings.is_empty() && value_bindings.is_empty() {
        return;
    }

    w.emit("void _syncSemanticBindings() {");
    w.indent();
    // Dedupe by (kind, ref): duplicate @ref bindings (a warning-level
    // semantic violation) must not emit duplicate locals (compile error).
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for (ref_name, ref_bindings) in &bindings {
        // Deterministic per-ref order: focus, scroll-y, scroll-x.
        for kind in ["focus", "scroll-y", "scroll-x"] {
            for (k, expr) in ref_bindings {
                if *k != kind {
                    continue;
                }
                let key = format!("{kind}:{ref_name}");
                if !seen.insert(key) {
                    continue;
                }
                let val = format_binding_expr(expr, store_provider_map, ir);
                let local = binding_sync_local(kind, ref_name);
                let prev = binding_prev_field(kind, ref_name);
                match kind {
                    "focus" => {
                        let focus = ref_focus_name(ref_name);
                        w.emit_fmt(format_args!("final {} = {};", local, val));
                        w.emit_fmt(format_args!("if ({} != {}) {{", local, prev));
                        w.indent();
                        w.emit_fmt(format_args!("{} = {};", prev, local));
                        w.emit_fmt(format_args!("if ({}) {{", local));
                        w.indent();
                        w.emit_fmt(format_args!("{}.requestFocus();", focus));
                        w.dedent();
                        w.emit("} else {");
                        w.indent();
                        w.emit_fmt(format_args!("{}.unfocus();", focus));
                        w.dedent();
                        w.emit("}");
                        w.dedent();
                        w.emit("}");
                    }
                    "scroll-y" | "scroll-x" => {
                        // Axis is determined by the attached Scrollable — a
                        // single ScrollController covers whichever axis the
                        // bound element scrolls in.
                        let scroll = ref_scroll_name(ref_name);
                        w.emit_fmt(format_args!("final {} = {};", local, val));
                        w.emit_fmt(format_args!("if ({} != {}) {{", local, prev));
                        w.indent();
                        w.emit_fmt(format_args!("{} = {};", prev, local));
                        w.emit_fmt(format_args!("if ({}.hasClients) {{", scroll));
                        w.indent();
                        w.emit_fmt(format_args!(
                            "{}.animateTo({}, duration: Duration(milliseconds: 300), curve: Curves.easeInOut);",
                            scroll, local
                        ));
                        w.dedent();
                        w.emit("}");
                        w.dedent();
                        w.emit("}");
                    }
                    _ => {}
                }
            }
        }
    }
    // bind:value re-apply: signal → controller text, guarded against echo
    // AND against re-applying the identical text (typing in the field must
    // not reset the caret on every rebuild).
    for (ref_name, name) in &value_bindings {
        let key = format!("value:{ref_name}");
        if !seen.insert(key) {
            continue;
        }
        let expr = binding_name_to_expr(name);
        let val = format_binding_expr(&expr, store_provider_map, ir);
        let text = ref_text_name(ref_name);
        let guard = ref_text_guard_name(ref_name);
        let local = binding_sync_local("value", ref_name);
        let prev = binding_prev_field("value", ref_name);
        w.emit_fmt(format_args!("final {} = {};", local, val));
        w.emit_fmt(format_args!("if ({} != {}) {{", local, prev));
        w.indent();
        w.emit_fmt(format_args!("{} = {};", prev, local));
        w.emit_fmt(format_args!("if ({}.text != {}) {{", text, local));
        w.indent();
        w.emit_fmt(format_args!("{} = true;", guard));
        w.emit_fmt(format_args!("{}.text = {};", text, local));
        w.emit_fmt(format_args!("{} = false;", guard));
        w.dedent();
        w.emit("}");
        w.dedent();
        w.emit("}");
    }
    w.dedent();
    w.emit("}");
}

/// Collect `(ref_name, sorted semantic bindings)` pairs in view-tree order.
fn collect_ref_bindings<'a>(node: &'a ViewTree, acc: &mut Vec<RefBinding<'a>>) {
    match node {
        ViewTree::Element(el) => {
            if let Some(ref ref_name) = el.ref_binding {
                if !el.semantic_bindings.is_empty() {
                    let mut pairs: Vec<(&str, &ExprIR)> = el
                        .semantic_bindings
                        .iter()
                        .map(|(k, v)| (k.as_str(), v))
                        .collect();
                    pairs.sort_by(|a, b| a.0.cmp(b.0));
                    acc.push((ref_name.as_str(), pairs));
                }
            }
            for child in &el.children {
                collect_ref_bindings(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_ref_bindings(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_ref_bindings(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_ref_bindings(body, acc);
            }
            if let Some(ref empty) = list.empty_body {
                collect_ref_bindings(empty, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for c in children {
                collect_ref_bindings(c, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                collect_ref_bindings(c, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Collect `(ref_name, bound signal name)` pairs for `bind:value` on
/// @ref-bound elements, in view-tree order (design D2: `bind:value` is the
/// sole two-way form; the removed `value={sig}` never reaches codegen).
fn collect_binding_refs<'a>(node: &'a ViewTree, acc: &mut Vec<(&'a str, &'a str)>) {
    match node {
        ViewTree::Element(el) => {
            if let Some(ref ref_name) = el.ref_binding {
                for (bind_name, signal_ref) in &el.bindings {
                    if bind_name == "value" {
                        acc.push((ref_name.as_str(), signal_ref.name.as_str()));
                    }
                }
            }
            for child in &el.children {
                collect_binding_refs(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_binding_refs(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_binding_refs(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_binding_refs(body, acc);
            }
            if let Some(ref empty) = list.empty_body {
                collect_binding_refs(empty, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for c in children {
                collect_binding_refs(c, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                collect_binding_refs(c, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Build an `ExprIR` for a binding's signal name — `Store.sig` becomes a
/// field access so the store rewrite (`ref.read(storeProvider.sig)`) applies.
fn binding_name_to_expr(name: &str) -> ExprIR {
    if let Some((store, field)) = name.split_once('.') {
        ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: store.to_string(),
                type_ann: None,
            }),
            field: field.to_string(),
        }
    } else {
        ExprIR::IdentifierExpr {
            name: name.to_string(),
            type_ann: None,
        }
    }
}

/// Format a semantic-binding expression for use inside the post-frame
/// callback: the standard store rewrite, then `ref.watch(...)` reads become
/// `ref.read(...)` (Riverpod 3.0 forbids `watch` outside build).
fn format_binding_expr(
    expr: &ExprIR,
    store_provider_map: &FxHashMap<String, String>,
    ir: &IRProgram,
) -> String {
    let rewritten = rewrite_store_accesses(expr, store_provider_map, ir);
    let callback_legal = watch_to_read(&rewritten);
    format_expr_ir_flutter(&callback_legal)
}

/// Convert `ref.watch(...)` reads into `ref.read(...)`.
fn watch_to_read(expr: &ExprIR) -> ExprIR {
    match expr {
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            is_optional,
        } if method == "watch" => ExprIR::MethodCallExpr {
            object: object.clone(),
            method: "read".into(),
            args: args.iter().map(watch_to_read).collect(),
            is_optional: *is_optional,
        },
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            is_optional,
        } => ExprIR::MethodCallExpr {
            object: Box::new(watch_to_read(object)),
            method: method.clone(),
            args: args.iter().map(watch_to_read).collect(),
            is_optional: *is_optional,
        },
        ExprIR::FieldAccessExpr { object, field } => ExprIR::FieldAccessExpr {
            object: Box::new(watch_to_read(object)),
            field: field.clone(),
        },
        ExprIR::OptionalChainExpr { object, field } => ExprIR::OptionalChainExpr {
            object: Box::new(watch_to_read(object)),
            field: field.clone(),
        },
        ExprIR::BinaryOpExpr { op, left, right } => ExprIR::BinaryOpExpr {
            op: *op,
            left: Box::new(watch_to_read(left)),
            right: Box::new(watch_to_read(right)),
        },
        ExprIR::UnaryOpExpr { op, operand } => ExprIR::UnaryOpExpr {
            op: *op,
            operand: Box::new(watch_to_read(operand)),
        },
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => ExprIR::TernaryIR {
            cond: Box::new(watch_to_read(cond)),
            then_expr: Box::new(watch_to_read(then_expr)),
            else_expr: Box::new(watch_to_read(else_expr)),
        },
        ExprIR::CallExpr { name, args } => ExprIR::CallExpr {
            name: name.clone(),
            args: args.iter().map(watch_to_read).collect(),
        },
        ExprIR::ArrayExpr { elements } => ExprIR::ArrayExpr {
            elements: elements.iter().map(watch_to_read).collect(),
        },
        ExprIR::IndexExpr { object, index } => ExprIR::IndexExpr {
            object: Box::new(watch_to_read(object)),
            index: Box::new(watch_to_read(index)),
        },
        ExprIR::AwaitExpr { operand } => ExprIR::AwaitExpr {
            operand: Box::new(watch_to_read(operand)),
        },
        ExprIR::ClosureExpr { params, body } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(watch_to_read(body)),
        },
        ExprIR::ArrayMethod {
            method,
            subject,
            predicate,
        } => ExprIR::ArrayMethod {
            method: *method,
            subject: Box::new(watch_to_read(subject)),
            predicate: Box::new(watch_to_read(predicate)),
        },
        leaf => leaf.clone(),
    }
}

// ═══════════════════════════════════════════════════════════
// dispose (Step 4)
// ═══════════════════════════════════════════════════════════

/// Whether any ref has a controller that must be disposed.
pub(crate) fn has_ref_dispose(comp: &ComponentIR, usage: &FxHashMap<String, RefUsage>) -> bool {
    comp.refs.iter().any(|r| {
        usage
            .get(&r.name)
            .copied()
            .unwrap_or_default()
            .has_controller()
    })
}

/// Emit `dispose()` calls for all on-demand controllers (Step 4).
pub(crate) fn emit_ref_dispose(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    usage: &FxHashMap<String, RefUsage>,
) {
    for r in &comp.refs {
        let u = usage.get(&r.name).copied().unwrap_or_default();
        if u.needs_focus {
            w.emit_fmt(format_args!("{}.dispose();", ref_focus_name(&r.name)));
        }
        if u.needs_scroll {
            w.emit_fmt(format_args!("{}.dispose();", ref_scroll_name(&r.name)));
        }
        if u.needs_text {
            w.emit_fmt(format_args!("{}.dispose();", ref_text_name(&r.name)));
        }
    }
}

// ═══════════════════════════════════════════════════════════
// RefOpExpr formatting (Step 7)
// ═══════════════════════════════════════════════════════════

/// Flutter expression formatter — Dart-family syntax with the Flutter
/// `format_ref_op` override (GlobalKey + on-demand controller accessors).
///
/// Everything else delegates to the inner `DartFormatExpr` so Flutter output
/// stays byte-identical to the Dart family except for ref ops.
pub(crate) struct FlutterFormatExpr {
    pub(crate) inner: DartFormatExpr,
}

impl FlutterFormatExpr {
    pub(crate) fn new(event_param_name: Option<String>) -> Self {
        Self {
            inner: DartFormatExpr { event_param_name },
        }
    }
}

impl FormatExpr for FlutterFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        self.inner.json_config()
    }

    fn codegen_target(&self) -> Target {
        self.inner.codegen_target()
    }

    fn event_param_name(&self) -> Option<&str> {
        self.inner.event_param_name()
    }

    // Dart-family overrides delegate to the inner DartFormatExpr.
    fn format_variant_literal(
        &self,
        union_type: &str,
        variant: &str,
        fields: &[(String, ExprIR)],
    ) -> String {
        self.inner
            .format_variant_literal(union_type, variant, fields)
    }

    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        self.inner.format_match(matched, arms)
    }

    fn format_struct(&self, type_name: &str, fields: &str) -> String {
        self.inner.format_struct(type_name, fields)
    }

    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        self.inner.format_block_expr(statements, tail)
    }

    /// Semantic ref ops map to Flutter idioms on the ref's GlobalKey and
    /// on-demand controllers (Task 5.2).
    fn format_ref_op(&self, ref_name: &str, op: &RefOpIR) -> String {
        format_ref_op_flutter(ref_name, op, self)
    }
}

/// Format a `RefOpIR` for a ref name (Step 7).
///
/// Controllers are guaranteed to exist: `collect_ref_usage` declares them
/// exactly when the matching op appears anywhere in the component.
fn format_ref_op_flutter(ref_name: &str, op: &RefOpIR, fmt: &impl FormatExpr) -> String {
    let key = ref_key_name(ref_name);
    let focus = ref_focus_name(ref_name);
    let scroll = ref_scroll_name(ref_name);
    let text = ref_text_name(ref_name);
    match op {
        RefOpIR::Focus => format!("{}.requestFocus()", focus),
        RefOpIR::Blur => format!("{}.unfocus()", focus),
        RefOpIR::ScrollTo { x, y } => {
            // Vertical axis wins (DOM `scrollTo(y)` convention); a single
            // ScrollController covers the bound element's scroll axis.
            let target = match (y, x) {
                (Some(y), _) => fmt.format_expr(y),
                (None, Some(x)) => fmt.format_expr(x),
                (None, None) => return format!("{}.jumpTo({}.position.pixels)", scroll, scroll),
            };
            format!(
                "if ({}.hasClients) {{ {}.jumpTo({}); }}",
                scroll, scroll, target
            )
        }
        RefOpIR::ScrollBy { dx, dy } => {
            let delta = match (dy, dx) {
                (Some(dy), _) => fmt.format_expr(dy),
                (None, Some(dx)) => fmt.format_expr(dx),
                (None, None) => "0.0".to_string(),
            };
            // `animateTo` requires a duration/curve in Dart.
            format!(
                "if ({0}.hasClients) {{ {0}.animateTo({0}.position.pixels + {1}, duration: Duration(milliseconds: 200), curve: Curves.easeInOut); }}",
                scroll, delta
            )
        }
        RefOpIR::Measure { what } => {
            // RenderBox read at call time via the GlobalKey — safe before
            // layout via `?.` (degrades to null, matching React's `?.`).
            let box_expr = format!("({}.currentContext?.findRenderObject() as RenderBox?)", key);
            match what {
                MeasureWhat::Width => format!("{}?.size.width", box_expr),
                MeasureWhat::Height => format!("{}?.size.height", box_expr),
                MeasureWhat::Position => {
                    format!("{}?.localToGlobal(Offset.zero)", box_expr)
                }
            }
        }
        RefOpIR::Select => format!(
            "{0}.selection = TextSelection(baseOffset: 0, extentOffset: {0}.text.length)",
            text
        ),
        RefOpIR::SetSelection { start, end } => format!(
            "{0}.selection = TextSelection(baseOffset: {1}, extentOffset: {2})",
            text,
            fmt.format_expr(start),
            fmt.format_expr(end)
        ),
        RefOpIR::Value => format!("{}.text", text),
        // No trailing `;` — statement contexts append their own terminator
        // (an embedded `;` would double-terminate in ExprStmtIR position).
        // Pointer capture is unsupported on Flutter (design D3 / spec §10.3):
        // `throw` is a valid Dart expression everywhere a RefOpExpr lands, so
        // the failure is loud at runtime even if the compile diagnostic above
        // is bypassed. `throw` makes the containing statement unreachable —
        // legal Dart, no trailing expression required.
        RefOpIR::CapturePointer => {
            "throw UnsupportedError('capturePointer not supported on Flutter')".to_string()
        }
        RefOpIR::ReleasePointer => {
            "throw UnsupportedError('releasePointer not supported on Flutter')".to_string()
        }
        // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
        #[allow(unreachable_patterns)]
        _ => format!("/* RefOp::{op:?} on {ref_name} */"),
    }
}
