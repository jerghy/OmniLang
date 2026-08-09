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
//! | pointer capture | (none — hard error → `@drag`) | `ref.capturePointer()`/`ref.releasePointer()` — unsupported on Flutter; compile error + generated `throw UnsupportedError`, hint points at `@drag` (ADR-073 decision 3: capture is a compiler-internal detail — Flutter's @drag emits a GestureDetector onPan wiring) |
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

/// Compile-error hint for pointer capture on Flutter — the hard error is
/// RETAINED per ADR-073 decision 3 (explicit capturePointer/releasePointer
/// stays a compile error, guarding against mechanism-leak regression), but
/// the hint now points at the semantic replacement: `@drag` compiles to a
/// GestureDetector pan wiring (ADR-073 decision 4 — D3 evolved from hard
/// error to target-difference note; the generated code still throws
/// `UnsupportedError` so failure is loud even if the diagnostic is
/// bypassed).
const FLUTTER_POINTER_CAPTURE_ERROR: &str =
    "Flutter 不支持指针捕获，请用 @drag 指令代替（编译为 GestureDetector onPan 接线）";

/// v4 compiler HIGH-2: dual-axis `scrollTo(x, y)` is silently narrowed to
/// the y axis on Flutter — `format_ref_op_flutter` scrolls a single axis
/// (vertical wins, x is dropped). The design contract promises a dual-axis
/// difference warning (设计文档 域 1 映射表) — this message surfaces on the
/// codegen warnings channel. No value evaluation is needed: x is ALWAYS
/// ignored on Flutter, whatever the expression.
const DUAL_AXIS_SCROLL_TO_WARNING: &str =
    "[flutter] scrollTo(x, y)：Flutter 单轴滚动——x 被忽略（垂直优先，设计契约双轴差异警告）";

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
    /// `ref.scrollTo(x, y)` with BOTH axes given — Flutter scrolls a single
    /// axis (vertical wins) and silently drops x; drives the v4 HIGH-2
    /// design-contract warning (one per component, see
    /// `collect_ref_usage`).
    pub dual_axis_scroll_to: bool,
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
///
/// The third return value carries v4 HIGH-2 warnings for dual-axis
/// `scrollTo(x, y)` calls — one `(message, span)` per component (span = the
/// component's source span; `RefOpIR`/`ExprIR` carry no finer span).
/// `collect_ref_usage` result: (per-ref usage map, hard compile errors,
/// dual-axis `scrollTo(x, y)` warnings as (message, span)).
pub(crate) type RefUsageSummary = (
    FxHashMap<String, RefUsage>,
    Vec<Diagnostic>,
    Vec<(String, Option<crate::lexer::Span>)>,
);

pub(crate) fn collect_ref_usage(comp: &ComponentIR) -> RefUsageSummary {
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
    // v4 compiler HIGH-2: dual-axis scrollTo(x, y) — one [flutter] warning
    // per component. `RefOpIR` carries no source span (ExprIR spans exist
    // only on view elements, and the call is usually in a fn body), so the
    // anchor is the component's span — the same anchor the pointer-capture
    // errors above use.
    let dual_axis_warnings: Vec<(String, Option<crate::lexer::Span>)> =
        if usage.values().any(|u| u.dual_axis_scroll_to) {
            vec![(DUAL_AXIS_SCROLL_TO_WARNING.to_string(), comp.source_span)]
        } else {
            Vec::new()
        };
    (usage, errors, dual_axis_warnings)
}

/// Record a `RefOpExpr`'s capability flags.
fn record_ref_op(ref_name: &str, op: &RefOpIR, usage: &mut FxHashMap<String, RefUsage>) {
    let u = usage.entry(ref_name.to_string()).or_default();
    match op {
        RefOpIR::Focus | RefOpIR::Blur => u.needs_focus = true,
        RefOpIR::ScrollTo { x, y } => {
            u.needs_scroll = true;
            // v4 compiler HIGH-2: dual-axis scrollTo(x, y) — Flutter scrolls
            // a single axis (vertical wins, `format_ref_op_flutter` drops x),
            // so the x argument is silently discarded. Record the flag here
            // (record_ref_op runs once per RefOpExpr node inside the existing
            // usage traversal) — collect_ref_usage surfaces one warning per
            // component at the end.
            if x.is_some() && y.is_some() {
                u.dual_axis_scroll_to = true;
            }
        }
        RefOpIR::ScrollBy { .. } => u.needs_scroll = true,
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

/// Whether any element in the view carries a `@drag` binding (ADR-073) —
/// this drives the single `Offset? _dragStart;` State field the
/// GestureDetector pan callbacks share (the drag-start snapshot lives in
/// the State's class-instance space, not on the element).
pub(crate) fn has_drag_bindings(comp: &ComponentIR) -> bool {
    let Some(view) = &comp.view else {
        return false;
    };
    let mut found = false;
    scan_drag_bindings(view, &mut found);
    found
}

fn scan_drag_bindings(node: &ViewTree, found: &mut bool) {
    if *found {
        return;
    }
    match node {
        ViewTree::Element(el) => {
            if el.drag_binding.is_some() {
                *found = true;
                return;
            }
            for child in &el.children {
                scan_drag_bindings(child, found);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    scan_drag_bindings(body, found);
                }
            }
            if let Some(ref fb) = cond.fallback {
                scan_drag_bindings(fb, found);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                scan_drag_bindings(body, found);
            }
            if let Some(ref empty) = list.empty_body {
                scan_drag_bindings(empty, found);
            }
        }
        ViewTree::Fragment(children) => {
            for c in children {
                scan_drag_bindings(c, found);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                scan_drag_bindings(c, found);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Which `form.*` builtin signals the view actually reads (`form.valid`,
/// `form.errors`, `form.submitting`). The Flutter backend degrades these to
/// constants — validation errors surface through the native `errorText`
/// pipeline instead — so constants are only emitted when referenced, else
/// `dart analyze` flags them unused.
///
/// The scan covers exactly the expression contexts that receive the Flutter
/// `form` mapping (`FlutterFormatExpr::new_with_context(.., form_present)`):
/// element prop values, semantic bindings, inline handler bodies (all
/// nesting depths — `format_stmt` recurses), interpolations — plus component
/// fn bodies / computeds / effects / lifecycles (Task 8b: `form_present` now
/// threads into those contexts too, so their `form.*` reads must reach the
/// same reachability-gated constants).
pub(crate) struct FormBuiltinUsage {
    pub valid: bool,
    pub errors: bool,
    pub submitting: bool,
}

pub(crate) fn form_builtin_usage(comp: &ComponentIR) -> FormBuiltinUsage {
    let mut usage = FormBuiltinUsage {
        valid: false,
        errors: false,
        submitting: false,
    };
    if let Some(view) = &comp.view {
        scan_view_for_form_builtin(view, &mut usage);
    }
    for f in &comp.fns {
        scan_block_for_form_builtin(&f.body.statements, &mut usage);
    }
    for e in &comp.effects {
        scan_block_for_form_builtin(&e.body.statements, &mut usage);
    }
    for c in &comp.computeds {
        scan_expr_for_form_builtin(&c.body, &mut usage);
    }
    for lc in &comp.lifecycles {
        scan_block_for_form_builtin(&lc.body.statements, &mut usage);
    }
    usage
}

fn scan_view_for_form_builtin(node: &ViewTree, usage: &mut FormBuiltinUsage) {
    if usage.valid && usage.errors && usage.submitting {
        return;
    }
    match node {
        ViewTree::Element(el) => {
            for v in el.props.values() {
                scan_expr_for_form_builtin(v, usage);
            }
            for v in el.semantic_bindings.values() {
                scan_expr_for_form_builtin(v, usage);
            }
            for h in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = h {
                    scan_block_for_form_builtin(&body.statements, usage);
                }
            }
            for child in &el.children {
                scan_view_for_form_builtin(child, usage);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                scan_expr_for_form_builtin(&branch.condition, usage);
                if let Some(ref body) = branch.body {
                    scan_view_for_form_builtin(body, usage);
                }
            }
            if let Some(ref fb) = cond.fallback {
                scan_view_for_form_builtin(fb, usage);
            }
        }
        ViewTree::ListRendering(list) => {
            scan_expr_for_form_builtin(&list.iterable, usage);
            if let Some(ref body) = list.body {
                scan_view_for_form_builtin(body, usage);
            }
            if let Some(ref body) = list.empty_body {
                scan_view_for_form_builtin(body, usage);
            }
        }
        ViewTree::Fragment(children) => {
            for c in children {
                scan_view_for_form_builtin(c, usage);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                scan_view_for_form_builtin(c, usage);
            }
        }
        ViewTree::Interpolation(expr) => scan_expr_for_form_builtin(expr, usage),
        ViewTree::Text(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

fn scan_block_for_form_builtin(statements: &[StatementIR], usage: &mut FormBuiltinUsage) {
    for stmt in statements {
        if usage.valid && usage.errors && usage.submitting {
            return;
        }
        match stmt {
            StatementIR::LetStmtIR { value: Some(v), .. } => {
                scan_expr_for_form_builtin(v, usage);
            }
            StatementIR::ExprStmtIR { expr } => scan_expr_for_form_builtin(expr, usage),
            StatementIR::ReturnStmtIR { value: Some(v), .. } => {
                scan_expr_for_form_builtin(v, usage);
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                scan_expr_for_form_builtin(condition, usage);
                scan_block_for_form_builtin(&then_body.statements, usage);
                for (c, b) in else_ifs {
                    scan_expr_for_form_builtin(c, usage);
                    scan_block_for_form_builtin(&b.statements, usage);
                }
                if let Some(b) = else_body {
                    scan_block_for_form_builtin(&b.statements, usage);
                }
            }
            StatementIR::WhileStmtIR { condition, body } => {
                scan_expr_for_form_builtin(condition, usage);
                scan_block_for_form_builtin(&body.statements, usage);
            }
            StatementIR::ForInStmtIR { iterable, body, .. } => {
                scan_expr_for_form_builtin(iterable, usage);
                scan_block_for_form_builtin(&body.statements, usage);
            }
            StatementIR::MutationStmt { intent: Some(i) } => {
                scan_mutation_for_form_builtin(i, usage);
            }
            StatementIR::EventEmitStmtIR { args, .. } => {
                for a in args {
                    scan_expr_for_form_builtin(a, usage);
                }
            }
            StatementIR::BlockStmtIR { body } => {
                scan_block_for_form_builtin(&body.statements, usage);
            }
            StatementIR::ThrowStmtIR { expr } => scan_expr_for_form_builtin(expr, usage),
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                scan_block_for_form_builtin(&try_body.statements, usage);
                if let Some(b) = catch_body {
                    scan_block_for_form_builtin(&b.statements, usage);
                }
                if let Some(b) = finally_body {
                    scan_block_for_form_builtin(&b.statements, usage);
                }
            }
            StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
}

fn scan_mutation_for_form_builtin(intent: &MutationIntent, usage: &mut FormBuiltinUsage) {
    match intent {
        MutationIntent::DirectAssign { value: Some(v), .. } => {
            scan_expr_for_form_builtin(v, usage);
        }
        MutationIntent::Increment {
            amount: Some(v), ..
        }
        | MutationIntent::Decrement {
            amount: Some(v), ..
        }
        | MutationIntent::Multiply {
            amount: Some(v), ..
        }
        | MutationIntent::Divide {
            amount: Some(v), ..
        }
        | MutationIntent::Modulo {
            amount: Some(v), ..
        } => scan_expr_for_form_builtin(v, usage),
        MutationIntent::Push { item: Some(v), .. } => scan_expr_for_form_builtin(v, usage),
        MutationIntent::Pop { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

fn scan_expr_for_form_builtin(expr: &ExprIR, usage: &mut FormBuiltinUsage) {
    if usage.valid && usage.errors && usage.submitting {
        return;
    }
    match expr {
        ExprIR::FieldAccessExpr { object, field } | ExprIR::OptionalChainExpr { object, field } => {
            if is_form_identifier(object) {
                record_form_builtin(field, usage);
            } else {
                scan_expr_for_form_builtin(object, usage);
            }
        }
        ExprIR::CallExpr { name: _, args } => {
            for a in args {
                scan_expr_for_form_builtin(a, usage);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            scan_expr_for_form_builtin(left, usage);
            scan_expr_for_form_builtin(right, usage);
        }
        ExprIR::UnaryOpExpr { operand, .. } => scan_expr_for_form_builtin(operand, usage),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            scan_expr_for_form_builtin(cond, usage);
            scan_expr_for_form_builtin(then_expr, usage);
            scan_expr_for_form_builtin(else_expr, usage);
        }
        ExprIR::ArrayExpr { elements } => {
            for e in elements {
                scan_expr_for_form_builtin(e, usage);
            }
        }
        ExprIR::SpreadExpr { operand } => scan_expr_for_form_builtin(operand, usage),
        ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                scan_expr_for_form_builtin(v, usage);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for a in call_args {
                scan_expr_for_form_builtin(a, usage);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            scan_expr_for_form_builtin(object, usage);
            for a in args {
                scan_expr_for_form_builtin(a, usage);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            scan_block_for_form_builtin(statements, usage);
            scan_expr_for_form_builtin(tail, usage);
        }
        ExprIR::IndexExpr { object, index } => {
            scan_expr_for_form_builtin(object, usage);
            scan_expr_for_form_builtin(index, usage);
        }
        ExprIR::AwaitExpr { operand } => scan_expr_for_form_builtin(operand, usage),
        ExprIR::ClosureExpr { body, .. } => scan_expr_for_form_builtin(body, usage),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, v) in fields {
                scan_expr_for_form_builtin(v, usage);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            scan_expr_for_form_builtin(matched, usage);
            for arm in arms {
                scan_block_for_form_builtin(&arm.body.statements, usage);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                scan_expr_for_form_builtin(k, usage);
                scan_expr_for_form_builtin(v, usage);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            scan_expr_for_form_builtin(subject, usage);
            scan_expr_for_form_builtin(predicate, usage);
        }
        ExprIR::RefOpExpr { op, .. } => {
            scan_ref_op_for_form_builtin(op, usage);
        }
        ExprIR::LiteralExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Scan `RefOpIR` child expressions (args like x/y/dx/dy/start/end/value)
/// for form builtin identifiers.
fn scan_ref_op_for_form_builtin(op: &RefOpIR, usage: &mut FormBuiltinUsage) {
    match op {
        RefOpIR::ScrollTo { x, y } => {
            scan_opt_expr_for_form_builtin(x.as_deref(), usage);
            scan_opt_expr_for_form_builtin(y.as_deref(), usage);
        }
        RefOpIR::ScrollBy { dx, dy } => {
            scan_opt_expr_for_form_builtin(dx.as_deref(), usage);
            scan_opt_expr_for_form_builtin(dy.as_deref(), usage);
        }
        RefOpIR::SetSelection { start, end } => {
            scan_expr_for_form_builtin(start, usage);
            scan_expr_for_form_builtin(end, usage);
        }
        // No-arg ops (Focus/Blur/Measure/Select/Value/CapturePointer/
        // ReleasePointer) carry no child expressions.
        _ => {}
    }
}

fn scan_opt_expr_for_form_builtin(expr: Option<&ExprIR>, usage: &mut FormBuiltinUsage) {
    if let Some(e) = expr {
        scan_expr_for_form_builtin(e, usage);
    }
}

fn is_form_identifier(expr: &ExprIR) -> bool {
    matches!(
        expr,
        ExprIR::IdentifierExpr { name, .. } if name == "form"
    )
}

fn record_form_builtin(field: &str, usage: &mut FormBuiltinUsage) {
    match field {
        "valid" => usage.valid = true,
        "submitting" => usage.submitting = true,
        // Any other field (incl. `errors` itself) resolves through
        // `formErrors` in the Flutter formatter (ref.rs format_special_field_access).
        _ => usage.errors = true,
    }
}

/// The `_DragEvent` carrier class spec for a component (ADR-073, Fix round
/// 2): which DragIntent fields the component's drag handlers actually read
/// (union across every `@drag` / `@dragEnd` binding), and whether the class
/// is referenced at all.
pub(crate) struct DragEventClassSpec {
    /// Fields the carrier class must declare, in schema order (x, y, dx,
    /// dy, pointerId, startX, startY). Empty when no handler reads any
    /// DragIntent field — the class then carries a bare `const _DragEvent();`
    /// (the call site emits `_DragEvent()` for arity-1 handlers that ignore
    /// the event object).
    pub fields: Vec<String>,
    /// True when any drag handler is called WITH a `_DragEvent(...)` carrier
    /// — i.e. any handler that is not zero-param (zero-param handlers are
    /// called bare, mirroring `drag_handler_call`'s ref_is_zero_param_fn
    /// predicate). False ⇒ the emitted code contains no `_DragEvent`
    /// reference ⇒ the class must not be emitted at all (an unreferenced
    /// private class would itself be a dart `unused_element` warning).
    pub needed: bool,
}

/// The DragIntent schema order (ADR-073) — shared by the class declaration
/// here and the extractor list in `drag_event_object_flutter` (view.rs).
const DRAG_SCHEMA_ORDER: [&str; 7] = ["x", "y", "dx", "dy", "pointerId", "startX", "startY"];

/// Mirror of `ViewContext::ref_is_zero_param_fn` (view.rs): a zero-param
/// drag handler is called BARE by `drag_handler_call`, so it never
/// references the carrier class. Unknown fns are treated as param-bearing
/// (the same conservative fallback as the call site).
fn drag_handler_is_zero_param(
    handler: &EventHandlerIR,
    comp: &ComponentIR,
    ir: &IRProgram,
) -> bool {
    let EventHandlerIR::FnRef { r#ref, .. } = handler else {
        return false; // non-FnRef drag handlers are unreachable (validate_drag)
    };
    comp.fns
        .iter()
        .any(|f| f.name == *r#ref && f.params.is_empty())
        || ir
            .all_functions()
            .iter()
            .any(|f| f.name == *r#ref && f.params.is_empty())
}

/// Merge one handler's used_fields into the running union. None = full
/// DragIntent schema (conservative; unreachable for drag — lower backfills
/// Some for every FnRef, but a None must not drop fields).
fn drag_merge_used_fields(
    union: &mut Option<FxHashSet<String>>,
    full_schema: &mut bool,
    handler: &EventHandlerIR,
) {
    match handler.used_fields() {
        None => *full_schema = true,
        Some(fs) => {
            let set = union.get_or_insert_with(FxHashSet::default);
            for f in fs {
                set.insert(f.clone());
            }
        }
    }
}

fn scan_drag_class_spec(
    node: &ViewTree,
    comp: &ComponentIR,
    ir: &IRProgram,
    union: &mut Option<FxHashSet<String>>,
    full_schema: &mut bool,
    needed: &mut bool,
) {
    match node {
        ViewTree::Element(el) => {
            if let Some(drag) = &el.drag_binding {
                // carrier reference: any non-zero-param drag handler is
                // called with a _DragEvent(...) argument (drag_handler_call)
                *needed |= !drag_handler_is_zero_param(&drag.on_drag, comp, ir);
                drag_merge_used_fields(union, full_schema, &drag.on_drag);
                if let Some(end) = &drag.on_drag_end {
                    *needed |= !drag_handler_is_zero_param(end, comp, ir);
                    drag_merge_used_fields(union, full_schema, end);
                }
            }
            for child in &el.children {
                scan_drag_class_spec(child, comp, ir, union, full_schema, needed);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    scan_drag_class_spec(body, comp, ir, union, full_schema, needed);
                }
            }
            if let Some(ref fb) = cond.fallback {
                scan_drag_class_spec(fb, comp, ir, union, full_schema, needed);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                scan_drag_class_spec(body, comp, ir, union, full_schema, needed);
            }
            if let Some(ref empty) = list.empty_body {
                scan_drag_class_spec(empty, comp, ir, union, full_schema, needed);
            }
        }
        ViewTree::Fragment(children) => {
            for c in children {
                scan_drag_class_spec(c, comp, ir, union, full_schema, needed);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for c in fallback {
                scan_drag_class_spec(c, comp, ir, union, full_schema, needed);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// The union of DragIntent fields read by every `@drag` / `@dragEnd`
/// handler in the component, in schema order (ADR-073), plus whether the
/// `_DragEvent` carrier class is referenced at all. Fix round 2 of the
/// ADR-073 review: the class used to hardcode all 7 fields, which produced
/// 5 `unused_element_parameter` warnings in dart analyze the first time a
/// real demo (@drag on ElementRefDemo) read only dx/dy.
pub(crate) fn drag_event_class_spec(comp: &ComponentIR, ir: &IRProgram) -> DragEventClassSpec {
    let mut spec = DragEventClassSpec {
        fields: Vec::new(),
        needed: false,
    };
    let Some(view) = &comp.view else {
        return spec;
    };
    let mut union: Option<FxHashSet<String>> = None;
    let mut full_schema = false;
    scan_drag_class_spec(
        view,
        comp,
        ir,
        &mut union,
        &mut full_schema,
        &mut spec.needed,
    );
    if full_schema {
        spec.fields = DRAG_SCHEMA_ORDER.iter().map(|s| s.to_string()).collect();
    } else if let Some(set) = union {
        // schema order — declaration order is stable regardless of which
        // handler read which field first
        let mut fields: Vec<String> = DRAG_SCHEMA_ORDER.iter().map(|s| s.to_string()).collect();
        fields.retain(|f| set.contains(f));
        spec.fields = fields;
    }
    spec
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
    // Task 3.7: shared with the effect/handler read conversion — the
    // callback-legal Riverpod 3.0 form.
    let callback_legal = super::expr::watch_to_read(&rewritten);
    format_expr_ir_flutter(&callback_legal)
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
    /// Phase 3 (ADR-071 decision 2): D1/D2 signal names — identifiers map to
    /// the ValueNotifier form `_name.value` (the State field is renamed
    /// `_name`). Empty for paths without component context (lib fns, signal
    /// initial values in non-component files, view-tree expressions).
    pub(crate) direct_write_signals: FxHashSet<String>,
    /// Task 5 (@load): when a `FutureProvider` name is set, the builtin
    /// `loading` / `error` / `data` identifiers map to the AsyncValue
    /// accessors (`todosAsync.isLoading`, ...). `None` outside component
    /// view/event contexts.
    pub(crate) load_async_name: Option<String>,
    /// Task 5 (@form): when the component declares a `<form @form>`, the
    /// builtin `form.valid` / `form.errors` / `form.submitting` field reads
    /// map to the reachability-gated locals. `false` outside view contexts.
    pub(crate) form_present: bool,
    /// Task 5 (@load): the reserved `retry` identifier maps to the retry
    /// action expression (`ref.invalidate(fetchTodosProvider)`) — as a VALUE
    /// (`() => ...` closure for callbacks) or as a CALL (the raw invalidate
    /// expr when invoked `retry()`). `None` outside @load component contexts.
    pub(crate) retry_expr: Option<String>,
}

impl FlutterFormatExpr {
    pub(crate) fn new(event_param_name: Option<String>) -> Self {
        Self {
            inner: DartFormatExpr { event_param_name },
            direct_write_signals: FxHashSet::default(),
            load_async_name: None,
            form_present: false,
            retry_expr: None,
        }
    }

    /// Phase 3: constructor carrying the component's D1/D2 set — every
    /// identifier resolving to a direct-write signal emits `_name.value`.
    pub(crate) fn new_with_direct_write(
        event_param_name: Option<String>,
        direct_write_signals: &FxHashSet<String>,
    ) -> Self {
        Self {
            inner: DartFormatExpr { event_param_name },
            direct_write_signals: direct_write_signals.clone(),
            load_async_name: None,
            form_present: false,
            retry_expr: None,
        }
    }

    /// Task 5: context-carrying constructor for component view/event
    /// expression formatting — carries the @load async local name, the
    /// @form presence, and the retry action so builtins (loading/error/data,
    /// form.*, retry) map to their Flutter runtime shapes.
    pub(crate) fn new_with_context(
        event_param_name: Option<String>,
        direct_write_signals: &FxHashSet<String>,
        load_async_name: Option<String>,
        form_present: bool,
        retry_expr: Option<String>,
    ) -> Self {
        Self {
            inner: DartFormatExpr { event_param_name },
            direct_write_signals: direct_write_signals.clone(),
            load_async_name,
            form_present,
            retry_expr,
        }
    }
}

impl FormatExpr for FlutterFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        self.inner.json_config()
    }

    fn codegen_target(&self) -> Target {
        // Flutter, not Dart: stdlib modules that are Flutter-only (storage —
        // SharedPreferences) must translate here. The inner DartFormatExpr
        // reports Dart, so delegating would silently fall back to the plain
        // call form (`storage_get(...)` → undefined method).
        Target::Flutter
    }

    fn event_param_name(&self) -> Option<&str> {
        self.inner.event_param_name()
    }

    // Phase 3 (ADR-071 decision 2): a D1/D2 signal identifier resolves to
    // the ValueNotifier cell — `_name.value` (the State field was renamed
    // `_name`; bare `name` would be an undeclared identifier in Dart).
    // Task 5 (@load): the builtin `loading` / `error` / `data` identifiers
    // resolve to AsyncValue accessors on the hoisted watch local.
    // Everything else stays the Dart-family default (bare name).
    fn format_identifier(&self, name: &str) -> String {
        // Task 5 (@load): `retry` as a VALUE (e.g. `onPressed={retry}`) must
        // be a callable — the `() => ref.invalidate(...)` closure. As a CALL
        // (`retry()` in an expression) format_call below emits the action
        // directly, so this closure shape is only the value form.
        if name == "retry" {
            if let Some(retry) = &self.retry_expr {
                return format!("() => {retry}");
            }
        }
        if let Some(async_name) = &self.load_async_name {
            match name {
                // `@if loading` — loading is a Bool signal; `AsyncValue.isLoading`.
                "loading" => return format!("{async_name}.isLoading"),
                // `@if error != ""` — error is a String signal; AsyncValue
                // surfaces `error` (Object?) — toString + null-coalesce. The
                // parens are MANDATORY: Dart's `??` binds looser than `!=`.
                "error" => return format!("({async_name}.error?.toString() ?? \"\")"),
                // `@for item in data` — data is an Array signal; AsyncValue
                // 3.x removed `valueOrNull` — `asData?.value` is List<T>?,
                // null-coalesce to an empty list.
                "data" => return format!("{async_name}.asData?.value ?? const []"),
                _ => {}
            }
        }
        if self.direct_write_signals.contains(name) {
            format!("_{}.value", name)
        } else {
            name.to_string()
        }
    }

    // Task 5 (@form): the builtin `form.X` field reads map to the
    // reachability-gated locals (formErrors / formValid / formSubmitting).
    // The `errors` map is keyed by the `bind:value` field name — `name`
    // (the LAST path segment of the bound signal, per FormFieldIR.name).
    fn format_special_field_access(&self, object: &ExprIR, field: &str) -> Option<String> {
        if !self.form_present {
            return None;
        }
        if let ExprIR::IdentifierExpr { name, .. } = object {
            if name == "form" {
                return match field {
                    "valid" => Some("formValid".to_string()),
                    "submitting" => Some("formSubmitting".to_string()),
                    "errors" => Some("formErrors".to_string()),
                    other => Some(format!("formErrors[\"{other}\"]")),
                };
            }
        }
        // Nested chain `form.errors.<field>` (and `form?.errors.<field>` /
        // `form.errors?.<field>`): a Dart Map has no `.field` member, so the
        // index access `formErrors["<field>"]` is required. The inner object
        // must be rooted at `form` — matching any `x?.errors` by field name
        // would misfire on unrelated maps (same shape as react/expr.rs
        // form_errors_index_access).
        let (inner_obj, inner_field) = match object {
            ExprIR::FieldAccessExpr { object, field }
            | ExprIR::OptionalChainExpr { object, field } => (object.as_ref(), field.as_str()),
            _ => return None,
        };
        if inner_field == "errors"
            && matches!(inner_obj, ExprIR::IdentifierExpr { name, .. } if name == "form")
        {
            Some(format!("formErrors[\"{field}\"]"))
        } else {
            None
        }
    }

    // Delegate to the Dart-family method table (`.len()` → `.length`,
    // `.to_string()` → `.toString()`, ...) instead of the JS-default
    // verbatim form.
    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        self.inner.format_method_call(obj, method, args)
    }

    fn format_optional_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        self.inner.format_optional_method_call(obj, method, args)
    }

    // Task 5.8a: the IR-aware method-table arms (get/contains/concat shape
    // decisions) and the `dynamic` closure-param annotation live on the
    // inner DartFormatExpr — Flutter output must use them too.
    fn format_method_call_with_ir(
        &self,
        object: &ExprIR,
        method: &str,
        args: &[ExprIR],
        is_optional: bool,
        obj_str: &str,
        arg_strs: &[String],
    ) -> String {
        self.inner
            .format_method_call_with_ir(object, method, args, is_optional, obj_str, arg_strs)
    }

    fn format_closure(&self, params: &str, body: &str) -> String {
        self.inner.format_closure(params, body)
    }

    fn format_closure_body(&self, body: &ExprIR, body_str: &str) -> String {
        self.inner.format_closure_body(body, body_str)
    }

    // Dart `+` on Strings requires both operands to be Strings — delegate
    // the coercion rule to the Dart family.
    fn format_binary_op(
        &self,
        op: &str,
        left: &ExprIR,
        right: &ExprIR,
        left_str: &str,
        right_str: &str,
    ) -> String {
        self.inner
            .format_binary_op(op, left, right, left_str, right_str)
    }

    // The language's `println` / `print` statements map to Dart's `print`
    // (the shared default would emit a bare call — undefined method).
    fn format_call(&self, name: &str, args: &[String]) -> String {
        // Task 5 (@load): `retry()` — the shared CallExpr arm passes the RAW
        // callee name here (not via format_identifier), so a call must emit
        // the retry action itself, not the value-form closure.
        if name == "retry" {
            if let Some(retry) = &self.retry_expr {
                return retry.clone();
            }
        }
        match name {
            "println" | "print" => format!("print({})", args.join(", ")),
            _ => {
                if let Some(native) =
                    crate::codegen::try_format_stdlib_call(name, args, Target::Flutter)
                {
                    return native;
                }
                format!("{}({})", name, args.join(", "))
            }
        }
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

    fn format_struct_field(&self, key: &str, value: &str) -> String {
        self.inner.format_struct_field(key, value)
    }

    fn format_struct_field_named(&self, key: &str, value: &str) -> String {
        self.inner.format_struct_field_named(key, value)
    }

    fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
        self.inner.format_block_expr(statements, tail)
    }

    /// Semantic ref ops map to Flutter idioms on the ref's GlobalKey and
    /// on-demand controllers (Task 5.2).
    fn format_ref_op(&self, ref_name: &str, op: &RefOpIR) -> String {
        format_ref_op_flutter(ref_name, op, self)
    }

    /// Measure-position parity (Task 5.8): `ref.measure()` emits the
    /// DOMRect-parity wrapper — the language surface is
    /// `x`/`y`/`width`/`height` over the Offset/Size pair, so field
    /// accesses read the wrapper's getters directly.
    fn format_ref_op_field_access(&self, object: &ExprIR, field: &str) -> Option<String> {
        let op = match object {
            ExprIR::RefOpExpr { op, .. } => op,
            _ => return None,
        };
        match op {
            RefOpIR::Measure {
                what: MeasureWhat::Position,
            } => {
                // The wrapper exposes x/y/width/height as nullable getters —
                // field accesses read them directly.
                let measure_expr = self.format_expr(object);
                let mapped = match field {
                    "x" | "y" | "width" | "height" => format!("{measure_expr}.{field}"),
                    f => format!("{measure_expr}.{f}"),
                };
                Some(mapped)
            }
            _ => None,
        }
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
            // T5.7: the old `as RenderBox?` cast THREW a TypeError when the
            // render object is not a RenderBox (RenderViewport, ...) — the
            // `is RenderBox ? box : null` pattern degrades to null instead.
            // Task 5.8: the TRUE branch needs an explicit `as RenderBox`
            // cast — the un-cast branch's static type was `RenderObject?`,
            // so `localToGlobal` resolved on RenderObject (which has no such
            // method).
            let render = format!("{}.currentContext?.findRenderObject()", key);
            let box_expr = format!("({render} is RenderBox ? {render} as RenderBox : null)");
            match what {
                // `?? 0` — React parity (react/expr.rs MeasureWhat): the
                // nullable chain degrades to null before layout; the
                // non-nullable `double` fields of the drag fixture would
                // otherwise be an invalid_assignment (double? → double).
                MeasureWhat::Width => format!("{}?.size.width ?? 0", box_expr),
                MeasureWhat::Height => format!("{}?.size.height ?? 0", box_expr),
                MeasureWhat::Position => {
                    // DOMRect-parity wrapper (Task 5.8): the language's
                    // measure() surface is x/y/width/height — a raw
                    // Offset.localToGlobal has dx/dy and no size, and a
                    // local variable holding the result (`var rect =
                    // box.measure()`) made `rect.x` an undefined
                    // getter.
                    format!("_omni_measure({box_expr})")
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
