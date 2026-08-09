//! Validate ref operations: rewrite legal ref method calls / field reads to
//! `ExprIR::RefOpExpr` and reject DOM-specific APIs with migration hints.
//!
//! Runs after component lowering (`ir/lower/mod.rs`). For each component with
//! declared refs (`ref name: Element` bound via `@ref={name}`), every expression
//! in the view tree, effects, lifecycles, computeds, and functions is scanned
//! for `MethodCallExpr` / `FieldAccessExpr` whose object is a declared ref name.
//!
//! Legal method calls (focus, blur, scrollTo, scrollBy, select, setSelection,
//! measure, capturePointer, releasePointer) are rewritten to
//! `ExprIR::RefOpExpr { ref_name, op }` so codegen backends know the intent and
//! can emit target-appropriate code (React refs, OmniView `__omni_el` wrappers,
//! Flutter GlobalKeys). Legal field reads (`ref.width`/`ref.height`/`ref.position`
//! → `RefOpIR::Measure { what }`, `ref.value` → `RefOpIR::Value`) are rewritten
//! the same way. Imperative writes (`ref.value = x` / `ref.setValue(x)`) are
//! hard-diagnosed: the `RefOpIR::SetValue` op was removed (11 → 10 ops) — the
//! two-way `bind:value` binding covers the write intent.
//!
//! Identifiers bound as parameters, locals, closure params, loop variables, or
//! match-pattern bindings shadow a same-named ref and are never rewritten.
//!
//! DOM-specific APIs that break cross-target compilation (`style`, `innerHTML`,
//! `querySelector`, `getBoundingClientRect`, `offsetWidth`, ...) produce compile
//! errors with migration hints instead of silently generating browser-only JS.

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{
    BlockIR, ComponentIR, EventHandlerIR, ExprIR, MeasureWhat, MutationIntent, RefOpIR,
    StatementIR, ViewTree,
};
use crate::lexer::Span;
use rustc_hash::FxHashSet;

/// Migration hint for the removed imperative ref write (`ref.value = x` /
/// `ref.setValue(x)`) — verbatim from the v2 design doc 域 1 诊断表. The
/// two-way `bind:value` binding is the supported write path.
const SETVALUE_REMOVED_HINT: &str = "命令式写已删除，用 bind:value 双向绑定";

/// Diagnostic for the removed `capturePointer(id)` / `releasePointer(id)`
/// argument (design D3) — verbatim from the v2 design doc 域 1 诊断表. The
/// id was a DOM pointerId platform-shape leak; both ops are parameterless
/// and multi-pointer locking is unsupported.
const POINTER_CAPTURE_NOARG_HINT: &str = "无参数；多指针锁定不支持";

/// Constructor for a legal ref operation from the source call's argument list.
/// Returns `None` when required arguments are missing (e.g. `setSelection()`
/// without start/end) — the caller emits an arity diagnostic.
type RefOpConstructor = fn(Vec<ExprIR>) -> Option<RefOpIR>;

/// Legal ref method dispatch table: (method name, source signature, constructor).
///
/// A plain Vec + linear scan is used instead of phf (no extra dependency); the
/// table is tiny and the scan runs once per ref method call in a component.
fn get_legal_ref_ops() -> Vec<(&'static str, &'static str, RefOpConstructor)> {
    vec![
        ("focus", "focus()", |_| Some(RefOpIR::Focus)),
        ("blur", "blur()", |_| Some(RefOpIR::Blur)),
        ("scrollTo", "scrollTo(x, y)", |a| {
            Some(RefOpIR::ScrollTo {
                x: a.first().cloned().map(Box::new),
                y: a.get(1).cloned().map(Box::new),
            })
        }),
        ("scrollBy", "scrollBy(dx, dy)", |a| {
            Some(RefOpIR::ScrollBy {
                dx: a.first().cloned().map(Box::new),
                dy: a.get(1).cloned().map(Box::new),
            })
        }),
        ("select", "select()", |_| Some(RefOpIR::Select)),
        ("setSelection", "setSelection(start, end)", |a| {
            if a.len() < 2 {
                None
            } else {
                Some(RefOpIR::SetSelection {
                    start: Box::new(a[0].clone()),
                    end: Box::new(a[1].clone()),
                })
            }
        }),
        ("measure", "measure()", |_| {
            Some(RefOpIR::Measure {
                what: MeasureWhat::Position,
            })
        }),
        // capturePointer / releasePointer are NOT in the dispatch table: both
        // are parameterless (design D3) and reject any argument with a
        // dedicated diagnostic — see check_expr.
    ]
}

/// DOM methods that must not be called on refs, with migration hints.
fn get_illegal_ref_ops() -> Vec<(&'static str, &'static str)> {
    vec![
        ("style", "use style:{...} binding with signals instead"),
        ("innerHTML", "use text interpolation {value} instead"),
        ("addEventListener", "use @on* event binding syntax instead"),
        ("querySelector", "violates component encapsulation"),
        ("classList", "use class: binding syntax instead"),
        ("parentElement", "violates component encapsulation"),
        ("textContent", "use text interpolation {value} instead"),
        ("getBoundingClientRect", "use ref.measure() instead"),
    ]
}

/// DOM fields that must not be read on refs, with migration hints.
fn get_illegal_ref_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("offsetWidth", "use ref.measure().width instead"),
        ("offsetHeight", "use ref.measure().height instead"),
        ("style", "use style:{...} binding with signals instead"),
    ]
}

/// Per-component context threaded through the walkers: the declared ref names,
/// the diagnostics sink, the current source location for error reporting, and
/// a scope stack of identifiers bound in the current function/block (params,
/// locals, closure params, loop vars, pattern bindings). An identifier bound
/// in any enclosing scope shadows a same-named ref.
///
/// Position threading (Task 4.11): `span` is the most precise location known
/// for the expression being walked — the enclosing view element's
/// `source_span` inside a view, the function's `source_span` inside `fn`
/// bodies, else the component's `source_span` (`base_span`). Diagnostics
/// anchor there instead of the component start line.
struct RefOpCtx<'a> {
    ref_names: &'a [String],
    diagnostics: &'a mut Vec<Diagnostic>,
    file: Option<String>,
    span: Option<Span>,
    base_span: Option<Span>,
    scopes: Vec<FxHashSet<String>>,
}

impl RefOpCtx<'_> {
    /// Emit a compile error diagnostic at the current source location.
    /// `suggestion` carries the verbatim migration hint (if any) — the
    /// message stays human-readable for docs-gate prefix matching.
    fn error(&mut self, message: String, suggestion: Option<String>) {
        let (line, col) = self.span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            kind: DiagnosticKind::E0006,
            message,
            file: self.file.clone(),
            line,
            col,
            end_line: None,
            end_col: None,
            suggestion,
        });
    }

    /// Enter a new lexical scope (block, function body, closure, pattern arm).
    fn push_scope(&mut self) {
        self.scopes.push(FxHashSet::default());
    }

    /// Leave the innermost lexical scope.
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Bind an identifier in the innermost scope (shadows a same-named ref).
    fn bind(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    /// True when `name` is bound in any enclosing scope (i.e. shadows a ref).
    fn is_shadowed(&self, name: &str) -> bool {
        self.scopes.iter().any(|scope| scope.contains(name))
    }
}

/// Validate ref operations for a single component, rewriting legal ref
/// method calls / field reads in place and pushing diagnostics for illegal
/// or unknown operations. No-op for components without declared refs.
pub fn validate_ref_ops(component: &mut ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let ref_names: Vec<String> = component.refs.iter().map(|r| r.name.clone()).collect();
    if ref_names.is_empty() {
        return;
    }

    let mut ctx = RefOpCtx {
        ref_names: &ref_names,
        diagnostics,
        file: component.source_file.clone(),
        span: component.source_span,
        base_span: component.source_span,
        scopes: Vec::new(),
    };

    // Walk view tree expressions — view elements narrow the position to
    // their own span (see walk_view_tree_exprs_mut).
    if let Some(ref mut view) = component.view {
        walk_view_tree_exprs_mut(view, &mut ctx);
    }

    // Walk expressions in effects. Effects carry no span — reset to the
    // component span (the view walk may have left an element span behind).
    for effect in &mut component.effects {
        ctx.span = ctx.base_span;
        walk_block_exprs_mut(&mut effect.body, &mut ctx);
    }

    // Walk expressions in lifecycle hooks (mount/unmount bodies)
    for lifecycle in &mut component.lifecycles {
        ctx.span = ctx.base_span;
        walk_block_exprs_mut(&mut lifecycle.body, &mut ctx);
    }

    // Walk expressions in computed properties
    for computed in &mut component.computeds {
        ctx.span = ctx.base_span;
        walk_expr_mut(&mut computed.body, &mut ctx);
    }

    // Walk expressions in functions — params shadow same-named refs; the
    // function's own span anchors its diagnostics.
    for func in &mut component.fns {
        ctx.span = func.source_span.or(ctx.base_span);
        ctx.push_scope();
        for param in &func.params {
            ctx.bind(&param.name);
        }
        walk_block_exprs_mut(&mut func.body, &mut ctx);
        ctx.pop_scope();
    }
}

/// Check a single expression for ref operations, rewriting it to
/// `ExprIR::RefOpExpr` when the object is a declared ref. Non-ref objects
/// (locals, params, store fields) are left untouched.
fn check_expr(expr: &mut ExprIR, ctx: &mut RefOpCtx) {
    match expr {
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            ..
        } => {
            if let ExprIR::IdentifierExpr { name: ref_name, .. } = object.as_ref() {
                if ctx.ref_names.contains(ref_name) && !ctx.is_shadowed(ref_name) {
                    // capturePointer / releasePointer are parameterless (D3):
                    // any argument is a hard error — the id was a DOM
                    // pointerId platform-shape leak and multi-pointer locking
                    // is unsupported. The zero-arg form is legal and rewritten.
                    if method == "capturePointer" || method == "releasePointer" {
                        if !args.is_empty() {
                            ctx.error(
                                format!("{} {}", method, POINTER_CAPTURE_NOARG_HINT),
                                Some(POINTER_CAPTURE_NOARG_HINT.to_string()),
                            );
                        } else {
                            *expr = ExprIR::RefOpExpr {
                                ref_name: ref_name.clone(),
                                op: if method == "capturePointer" {
                                    RefOpIR::CapturePointer
                                } else {
                                    RefOpIR::ReleasePointer
                                },
                            };
                        }
                        return;
                    }
                    // Legal operations are rewritten so codegen knows the intent.
                    let legal_ops = get_legal_ref_ops();
                    if let Some((_, signature, constructor)) =
                        legal_ops.iter().find(|(m, _, _)| *m == method.as_str())
                    {
                        match constructor(args.clone()) {
                            Some(op) => {
                                *expr = ExprIR::RefOpExpr {
                                    ref_name: ref_name.clone(),
                                    op,
                                };
                            }
                            None => {
                                ctx.error(
                                    format!(
                                        "ref.{}() requires more arguments — expected: ref.{}",
                                        method, signature
                                    ),
                                    None,
                                );
                            }
                        }
                        return;
                    }
                    // Known DOM APIs get a migration hint; anything else is unknown.
                    let illegal_ops = get_illegal_ref_ops();
                    if let Some((_, hint)) = illegal_ops.iter().find(|(m, _)| *m == method.as_str())
                    {
                        ctx.error(
                            format!("ref.{}() is not allowed — {}", method, hint),
                            Some(hint.to_string()),
                        );
                    } else {
                        ctx.error(format!("unknown ref operation: ref.{}()", method), None);
                    }
                }
            }
        }
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name: ref_name, .. } = object.as_ref() {
                if ctx.ref_names.contains(ref_name) && !ctx.is_shadowed(ref_name) {
                    match field.as_str() {
                        // Codegen emits offsetWidth / offsetHeight / the rect
                        // ({x, y}) for the requested dimension.
                        "width" => {
                            *expr = ExprIR::RefOpExpr {
                                ref_name: ref_name.clone(),
                                op: RefOpIR::Measure {
                                    what: MeasureWhat::Width,
                                },
                            };
                        }
                        "height" => {
                            *expr = ExprIR::RefOpExpr {
                                ref_name: ref_name.clone(),
                                op: RefOpIR::Measure {
                                    what: MeasureWhat::Height,
                                },
                            };
                        }
                        "position" => {
                            *expr = ExprIR::RefOpExpr {
                                ref_name: ref_name.clone(),
                                op: RefOpIR::Measure {
                                    what: MeasureWhat::Position,
                                },
                            };
                        }
                        "value" => {
                            *expr = ExprIR::RefOpExpr {
                                ref_name: ref_name.clone(),
                                op: RefOpIR::Value,
                            };
                        }
                        _ => {
                            let illegal_fields = get_illegal_ref_fields();
                            if let Some((_, hint)) =
                                illegal_fields.iter().find(|(f, _)| *f == field.as_str())
                            {
                                ctx.error(
                                    format!("ref.{} is not allowed — {}", field, hint),
                                    Some(hint.to_string()),
                                );
                            } else {
                                ctx.error(format!("unknown ref field: ref.{}", field), None);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Recursively walk an expression tree, checking each node for ref operations.
/// Called after `check_expr` so a rewritten `RefOpExpr` has its argument
/// expressions walked (nested ref ops in arguments are legal).
fn walk_expr_mut(expr: &mut ExprIR, ctx: &mut RefOpCtx) {
    check_expr(expr, ctx);
    match expr {
        ExprIR::LiteralExpr { .. } | ExprIR::IdentifierExpr { .. } => {}
        ExprIR::FieldAccessExpr { object, .. } => walk_expr_mut(object, ctx),
        ExprIR::OptionalChainExpr { object, .. } => walk_expr_mut(object, ctx),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_mut(left, ctx);
            walk_expr_mut(right, ctx);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_mut(operand, ctx),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_mut(cond, ctx);
            walk_expr_mut(then_expr, ctx);
            walk_expr_mut(else_expr, ctx);
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr_mut(arg, ctx);
            }
        }
        ExprIR::ArrayExpr { elements } => {
            for elem in elements {
                walk_expr_mut(elem, ctx);
            }
        }
        ExprIR::SpreadExpr { operand } => walk_expr_mut(operand, ctx),
        ExprIR::StructExpr { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr_mut(field_expr, ctx);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                walk_expr_mut(arg, ctx);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr_mut(object, ctx);
            for arg in args {
                walk_expr_mut(arg, ctx);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                walk_stmt_exprs_mut(stmt, ctx);
            }
            walk_expr_mut(tail, ctx);
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr_mut(object, ctx);
            walk_expr_mut(index, ctx);
        }
        ExprIR::ClosureExpr { params, body } => {
            // Closure params shadow same-named refs inside the closure body.
            ctx.push_scope();
            for param in params {
                ctx.bind(param);
            }
            walk_expr_mut(body, ctx);
            ctx.pop_scope();
        }
        ExprIR::AwaitExpr { operand } => walk_expr_mut(operand, ctx),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr_mut(field_expr, ctx);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr_mut(matched, ctx);
            for arm in arms {
                // Pattern bindings shadow same-named refs inside the arm body.
                ctx.push_scope();
                if let crate::ir::MatchPatternKindIR::Variant { bindings, .. } = &arm.pattern.kind {
                    for binding in bindings {
                        ctx.bind(binding);
                    }
                }
                walk_block_exprs_mut(&mut arm.body, ctx);
                ctx.pop_scope();
            }
        }
        ExprIR::MapExpr { entries } => {
            for (key, value) in entries {
                walk_expr_mut(key, ctx);
                walk_expr_mut(value, ctx);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            walk_expr_mut(subject, ctx);
            walk_expr_mut(predicate, ctx);
        }
        ExprIR::RefOpExpr { op, .. } => walk_ref_op_children_mut(op, ctx),
        ExprIR::BuiltinToastCall { msg, .. } => walk_expr_mut(msg, ctx),
    }
}

/// Walk the `ExprIR` children carried by a `RefOpIR` operation so nested ref
/// operations in arguments (e.g. `ref.scrollTo(0, otherRef.measure().y)`) are
/// rewritten too.
fn walk_ref_op_children_mut(op: &mut RefOpIR, ctx: &mut RefOpCtx) {
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
                walk_expr_mut(x, ctx);
            }
            if let Some(y) = y {
                walk_expr_mut(y, ctx);
            }
        }
        RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                walk_expr_mut(dx, ctx);
            }
            if let Some(dy) = dy {
                walk_expr_mut(dy, ctx);
            }
        }
        RefOpIR::SetSelection { start, end } => {
            walk_expr_mut(start, ctx);
            walk_expr_mut(end, ctx);
        }
    }
}

/// Walk every expression reachable from a statement.
fn walk_stmt_exprs_mut(stmt: &mut StatementIR, ctx: &mut RefOpCtx) {
    match stmt {
        StatementIR::LetStmtIR { name, value, .. } => {
            // The initializer is evaluated before the binding exists, so walk
            // the value first, then bind the name (shadows a same-named ref
            // for the rest of the scope).
            if let Some(value) = value {
                walk_expr_mut(value, ctx);
            }
            ctx.bind(name);
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            walk_expr_mut(condition, ctx);
            walk_block_exprs_mut(then_body, ctx);
            for (cond, body) in else_ifs {
                walk_expr_mut(cond, ctx);
                walk_block_exprs_mut(body, ctx);
            }
            if let Some(else_body) = else_body {
                walk_block_exprs_mut(else_body, ctx);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(value) = value {
                walk_expr_mut(value, ctx);
            }
        }
        StatementIR::MutationStmt { intent } => {
            // Ref field writes and direct ref writes are hard-diagnosed here:
            // `myRef.value = x` / `myRef.setValue(x)` / `myRef = x` once
            // lowered to `RefOpIR::SetValue`, but the imperative write op was
            // removed (11 → 10 ops) — the two-way `bind:value` binding covers
            // the write intent, so each write form emits the migration hint.
            // Non-ref targets (signals, store fields, locals) are left
            // untouched — the React codegen consults `field_object` only as
            // a mutation discriminator (write-only DCE guard, review I1/N1:
            // owner-field writes are never pruned), so semantic behavior is
            // unchanged for them.
            if let Some(intent) = intent {
                match intent {
                    MutationIntent::DirectAssign {
                        target,
                        value,
                        field_object,
                    } => {
                        // Walk the assigned value first — it may contain
                        // nested ref reads/ops that must still be rewritten
                        // or diagnosed.
                        if let Some(value) = value.as_deref_mut() {
                            walk_expr_mut(value, ctx);
                        }
                        let mut diagnosed = false;
                        // `owner.field = x` writes with a declared ref owner.
                        if let Some(obj) = field_object.as_ref() {
                            if ctx.ref_names.contains(obj) && !ctx.is_shadowed(obj) {
                                match target.name.as_str() {
                                    "value" => {
                                        ctx.error(
                                            SETVALUE_REMOVED_HINT.to_string(),
                                            Some(SETVALUE_REMOVED_HINT.to_string()),
                                        );
                                        diagnosed = true;
                                    }
                                    field_name => {
                                        let illegal_fields = get_illegal_ref_fields();
                                        if let Some((_, hint)) =
                                            illegal_fields.iter().find(|(f, _)| *f == field_name)
                                        {
                                            ctx.error(
                                                format!(
                                                    "ref.{} is not allowed — {}",
                                                    field_name, hint
                                                ),
                                                Some(hint.to_string()),
                                            );
                                        } else {
                                            ctx.error(
                                                format!("unknown ref field: ref.{}", field_name),
                                                None,
                                            );
                                        }
                                        diagnosed = true;
                                    }
                                }
                            }
                        }
                        // Direct writes targeting the ref itself: `myRef = x`
                        // (lowered from ref.setValue(x) / ref.set(x) method
                        // calls and bare assigns).
                        if !diagnosed
                            && ctx.ref_names.contains(&target.name)
                            && !ctx.is_shadowed(&target.name)
                        {
                            ctx.error(
                                SETVALUE_REMOVED_HINT.to_string(),
                                Some(SETVALUE_REMOVED_HINT.to_string()),
                            );
                        }
                    }
                    MutationIntent::Increment { amount, .. }
                    | MutationIntent::Decrement { amount, .. }
                    | MutationIntent::Multiply { amount, .. }
                    | MutationIntent::Divide { amount, .. }
                    | MutationIntent::Modulo { amount, .. } => {
                        if let Some(amount) = amount {
                            walk_expr_mut(amount, ctx);
                        }
                    }
                    MutationIntent::Push { item, .. } => {
                        if let Some(item) = item {
                            walk_expr_mut(item, ctx);
                        }
                    }
                    MutationIntent::Pop { .. } => {}
                }
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                walk_expr_mut(arg, ctx);
            }
        }
        StatementIR::ExprStmtIR { expr } => walk_expr_mut(expr, ctx),
        StatementIR::BlockStmtIR { body } => {
            ctx.push_scope();
            walk_block_exprs_mut(body, ctx);
            ctx.pop_scope();
        }
        StatementIR::WhileStmtIR { condition, body } => {
            walk_expr_mut(condition, ctx);
            ctx.push_scope();
            walk_block_exprs_mut(body, ctx);
            ctx.pop_scope();
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            walk_expr_mut(iterable, ctx);
            ctx.push_scope();
            ctx.bind(var_name);
            walk_block_exprs_mut(body, ctx);
            ctx.pop_scope();
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        StatementIR::ThrowStmtIR { expr } => walk_expr_mut(expr, ctx),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            catch_var,
            finally_body,
            ..
        } => {
            ctx.push_scope();
            walk_block_exprs_mut(try_body, ctx);
            ctx.pop_scope();
            if let Some(catch_body) = catch_body {
                ctx.push_scope();
                if let Some(catch_var) = catch_var {
                    ctx.bind(catch_var);
                }
                walk_block_exprs_mut(catch_body, ctx);
                ctx.pop_scope();
            }
            if let Some(finally_body) = finally_body {
                ctx.push_scope();
                walk_block_exprs_mut(finally_body, ctx);
                ctx.pop_scope();
            }
        }
    }
}

/// Walk every expression reachable from a block.
fn walk_block_exprs_mut(block: &mut BlockIR, ctx: &mut RefOpCtx) {
    for stmt in &mut block.statements {
        walk_stmt_exprs_mut(stmt, ctx);
    }
}

/// Recursively walk all ViewTree nodes and their expression fields: Element
/// props, class map bindings, inline event handler bodies, Conditional
/// conditions/branches, ListRendering iterables/key_exprs/bodies,
/// Interpolations, Slot fallbacks, and Fragment children.
fn walk_view_tree_exprs_mut(tree: &mut ViewTree, ctx: &mut RefOpCtx) {
    match tree {
        ViewTree::Element(el) => {
            // Task 4.11: diagnostics found inside this element's props /
            // handlers / children anchor at the element's own span, not the
            // component start line.
            ctx.span = el.source_span.or(ctx.base_span);
            for expr in el.props.values_mut() {
                walk_expr_mut(expr, ctx);
            }
            for expr in el.class_map_bindings.values_mut() {
                walk_expr_mut(expr, ctx);
            }
            for handler in el.events.values_mut() {
                if let EventHandlerIR::InlineFn { body, params, .. } = handler {
                    // Handler params shadow same-named refs in the body.
                    ctx.push_scope();
                    for param in params {
                        ctx.bind(param);
                    }
                    walk_block_exprs_mut(body, ctx);
                    ctx.pop_scope();
                }
            }
            for child in &mut el.children {
                walk_view_tree_exprs_mut(child, ctx);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &mut cond.branches {
                walk_expr_mut(&mut branch.condition, ctx);
                if let Some(body) = &mut branch.body {
                    walk_view_tree_exprs_mut(body, ctx);
                }
            }
            if let Some(fallback) = &mut cond.fallback {
                walk_view_tree_exprs_mut(fallback, ctx);
            }
        }
        ViewTree::ListRendering(list) => {
            // The iterable is evaluated outside the item binding; the key expr
            // and body see the item binding (which shadows a same-named ref).
            walk_expr_mut(&mut list.iterable, ctx);
            ctx.push_scope();
            ctx.bind(&list.item_binding.name);
            if let Some(key_expr) = &mut list.key_expr {
                walk_expr_mut(key_expr, ctx);
            }
            if let Some(body) = &mut list.body {
                walk_view_tree_exprs_mut(body, ctx);
            }
            ctx.pop_scope();
            if let Some(empty_body) = &mut list.empty_body {
                walk_view_tree_exprs_mut(empty_body, ctx);
            }
        }
        ViewTree::Interpolation(expr) => walk_expr_mut(expr, ctx),
        ViewTree::Text(_) => {}
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_view_tree_exprs_mut(child, ctx);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_tree_exprs_mut(child, ctx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{FunctionIR, RefDeclIR, Visibility};
    use crate::lexer::Span;

    /// Dummy span shared by the fn-level test helpers: every diagnostic
    /// emitted from a helper-built function must anchor at this position
    /// (Task 4.11 — fn context uses FunctionIR.source_span).
    const FN_SPAN: Span = Span {
        start: 0,
        end: 4,
        line: 7,
        col: 3,
    };

    /// Helper: component with a single declared ref named "box".
    fn make_component() -> ComponentIR {
        ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "Test".into(),
            refs: vec![RefDeclIR {
                name: "box".into(),
                element_tag: None,
            }],
            ..ComponentIR::default()
        }
    }

    /// Helper: push a function with a single expression statement to a component.
    fn push_expr_stmt(comp: &mut ComponentIR, expr: ExprIR) {
        comp.fns.push(FunctionIR {
            name: "run".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR { expr }],
            },
            is_async: false,
            source_span: Some(FN_SPAN),
        });
    }

    fn method_call(object: &str, method: &str, args: Vec<ExprIR>) -> ExprIR {
        ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::ident(object)),
            method: method.to_string(),
            args,
            is_optional: false,
        }
    }

    /// Extract the single statement expression from the first function.
    fn stmt_expr(comp: &ComponentIR) -> &ExprIR {
        match &comp.fns[0].body.statements[0] {
            StatementIR::ExprStmtIR { expr } => expr,
            other => panic!("expected ExprStmtIR, got: {:?}", other),
        }
    }

    fn run_pass(comp: &mut ComponentIR) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        validate_ref_ops(comp, &mut diagnostics);
        diagnostics
    }

    #[test]
    fn rewrites_focus_to_ref_op_expr() {
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("box", "focus", vec![]));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { ref_name, op } => {
                assert_eq!(ref_name, "box");
                assert!(matches!(op, RefOpIR::Focus));
            }
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    #[test]
    fn rewrites_scroll_to_with_args() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call(
                "box",
                "scrollTo",
                vec![
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(0),
                    },
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(100),
                    },
                ],
            ),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { op, .. } => match op {
                RefOpIR::ScrollTo { x, y } => {
                    assert!(matches!(x.as_deref(), Some(ExprIR::LiteralExpr { .. })));
                    assert!(matches!(y.as_deref(), Some(ExprIR::LiteralExpr { .. })));
                }
                other => panic!("expected ScrollTo, got: {:?}", other),
            },
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    #[test]
    fn rewrites_set_selection_with_two_args() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call(
                "box",
                "setSelection",
                vec![
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
                    },
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(2),
                    },
                ],
            ),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { op, .. } => {
                assert!(matches!(op, RefOpIR::SetSelection { .. }));
            }
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    #[test]
    fn missing_required_args_is_an_error() {
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("box", "setSelection", vec![]));
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("setSelection"));
        assert!(diags[0].message.contains("requires more arguments"));
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
    }

    #[test]
    fn rewrites_parameterless_capture_pointer() {
        // Design D3: capturePointer() is parameterless — zero args stay legal
        // and rewrite to the unit RefOpIR.
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("box", "capturePointer", vec![]));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { ref_name, op } => {
                assert_eq!(ref_name, "box");
                assert!(matches!(op, RefOpIR::CapturePointer));
            }
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    #[test]
    fn rewrites_parameterless_release_pointer() {
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("box", "releasePointer", vec![]));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { op, .. } => assert!(matches!(op, RefOpIR::ReleasePointer)),
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    #[test]
    fn capture_pointer_with_args_is_error() {
        // Design D3: `capturePointer(1)` — any argument is a hard error with
        // the verbatim hint from the 域 1 诊断表 (the id was a DOM pointerId
        // platform-shape leak; multi-pointer locking is unsupported).
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call(
                "box",
                "capturePointer",
                vec![ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }],
            ),
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1, "expected exactly one diagnostic");
        assert!(
            diags[0].message.contains("capturePointer 无参数"),
            "expected 'capturePointer 无参数', got: {:?}",
            diags[0].message
        );
        assert!(
            diags[0].message.contains("多指针锁定不支持"),
            "expected '多指针锁定不支持', got: {:?}",
            diags[0].message
        );
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some(POINTER_CAPTURE_NOARG_HINT),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
        // The call is not rewritten — compile fails on the error.
        assert!(matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }));
    }

    #[test]
    fn release_pointer_with_args_is_error() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call(
                "box",
                "releasePointer",
                vec![ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }],
            ),
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1, "expected exactly one diagnostic");
        assert!(
            diags[0].message.contains("releasePointer 无参数")
                && diags[0].message.contains("多指针锁定不支持"),
            "expected the parameterless hint, got: {:?}",
            diags[0].message
        );
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some(POINTER_CAPTURE_NOARG_HINT),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
        assert!(matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }));
    }

    #[test]
    fn rewrites_width_field_to_measure() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::FieldAccessExpr {
                object: Box::new(ExprIR::ident("box")),
                field: "width".into(),
            },
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { ref_name, op } => {
                assert_eq!(ref_name, "box");
                assert!(matches!(
                    op,
                    RefOpIR::Measure {
                        what: MeasureWhat::Width
                    }
                ));
            }
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    #[test]
    fn rewrites_height_and_position_fields_to_measure() {
        for (field, expected) in [
            ("height", MeasureWhat::Height),
            ("position", MeasureWhat::Position),
        ] {
            let mut comp = make_component();
            push_expr_stmt(
                &mut comp,
                ExprIR::FieldAccessExpr {
                    object: Box::new(ExprIR::ident("box")),
                    field: field.into(),
                },
            );
            let diags = run_pass(&mut comp);
            assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
            match stmt_expr(&comp) {
                ExprIR::RefOpExpr { op, .. } => {
                    assert!(
                        matches!(op, RefOpIR::Measure { what } if *what == expected),
                        "field {}: expected {:?}, got {:?}",
                        field,
                        expected,
                        op
                    );
                }
                other => panic!("expected RefOpExpr, got: {:?}", other),
            }
        }
    }

    #[test]
    fn rewrites_value_field_to_value_op() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::FieldAccessExpr {
                object: Box::new(ExprIR::ident("box")),
                field: "value".into(),
            },
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { op, .. } => assert!(matches!(op, RefOpIR::Value)),
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    #[test]
    fn illegal_field_offset_width_errors_with_hint() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::FieldAccessExpr {
                object: Box::new(ExprIR::ident("box")),
                field: "offsetWidth".into(),
            },
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("offsetWidth"));
        assert!(diags[0].message.contains("ref.measure().width"));
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some("use ref.measure().width instead"),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
    }

    #[test]
    fn illegal_field_style_errors_with_hint() {
        // `box.style` — the outermost object of `box.style.color` is a
        // FieldAccessExpr, so only the inner `box.style` is flagged.
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::FieldAccessExpr {
                object: Box::new(ExprIR::FieldAccessExpr {
                    object: Box::new(ExprIR::ident("box")),
                    field: "style".into(),
                }),
                field: "color".into(),
            },
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("style"));
        assert!(diags[0].message.contains("style:{...}"));
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some("use style:{...} binding with signals instead"),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
    }

    #[test]
    fn illegal_method_get_bounding_client_rect_errors_with_hint() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call("box", "getBoundingClientRect", vec![]),
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("getBoundingClientRect"));
        assert!(diags[0].message.contains("ref.measure()"));
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some("use ref.measure() instead"),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
    }

    #[test]
    fn unknown_ref_method_errors() {
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("box", "bogus", vec![]));
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unknown ref operation"));
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert!(
            diags[0].suggestion.is_none(),
            "no migration hint for unknown ops: {:?}",
            diags[0]
        );
    }

    #[test]
    fn unknown_ref_field_errors() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::FieldAccessExpr {
                object: Box::new(ExprIR::ident("box")),
                field: "bogus".into(),
            },
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unknown ref field"));
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
    }

    #[test]
    fn non_ref_object_is_left_untouched() {
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("obj", "focus", vec![]));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(
            matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }),
            "non-ref method call must not be rewritten"
        );
    }

    #[test]
    fn component_without_refs_is_a_noop() {
        let mut comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "NoRefs".into(),
            ..ComponentIR::default()
        };
        push_expr_stmt(&mut comp, method_call("box", "focus", vec![]));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty());
        assert!(matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }));
    }

    #[test]
    fn rewrites_nested_ref_op_inside_call_args() {
        let mut comp = make_component();
        // run(box.focus()) — the ref op is nested inside a CallExpr arg.
        push_expr_stmt(
            &mut comp,
            ExprIR::CallExpr {
                name: "run".into(),
                args: vec![method_call("box", "focus", vec![])],
            },
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::CallExpr { args, .. } => {
                assert!(matches!(&args[0], ExprIR::RefOpExpr { .. }));
            }
            other => panic!("expected CallExpr, got: {:?}", other),
        }
    }

    #[test]
    fn walks_inline_event_handler_bodies() {
        let mut comp = make_component();
        comp.view = Some(ViewTree::Element(Box::new(crate::ir::ElementIR {
            tag: "button".into(),
            events: {
                let mut events = rustc_hash::FxHashMap::default();
                events.insert(
                    "onPress".into(),
                    EventHandlerIR::InlineFn {
                        body: BlockIR {
                            statements: vec![StatementIR::ExprStmtIR {
                                expr: method_call("box", "focus", vec![]),
                            }],
                        },
                        params: vec![],
                        event_data: None,
                        used_fields: None,
                        needs_prevent_default: false,
                        needs_stop_propagation: false,
                    },
                );
                events
            },
            props: rustc_hash::FxHashMap::default(),
            bindings: rustc_hash::FxHashMap::default(),
            children: vec![],
            style_classes: None,
            inline_style: None,
            construction_kind: crate::ir::ConstructionKind::Normal,
            class_map_bindings: rustc_hash::FxHashMap::default(),
            source_span: None,
            motion: None,
            ref_binding: None,
            semantic_bindings: rustc_hash::FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
        })));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        let ViewTree::Element(el) = comp.view.as_ref().unwrap() else {
            panic!("expected element view");
        };
        let EventHandlerIR::InlineFn { body, .. } = &el.events["onPress"] else {
            panic!("expected inline fn handler");
        };
        assert!(matches!(
            &body.statements[0],
            StatementIR::ExprStmtIR {
                expr: ExprIR::RefOpExpr { .. }
            }
        ));
    }

    #[test]
    fn illegal_ref_op_in_view_element_anchors_element_line() {
        // Task 4.11: a ref-op error found inside a view element's inline
        // handler must point at the ELEMENT line (ElementIR.source_span),
        // not the component start line.
        let mut comp = make_component();
        comp.view = Some(ViewTree::Element(Box::new(crate::ir::ElementIR {
            tag: "button".into(),
            events: {
                let mut events = rustc_hash::FxHashMap::default();
                events.insert(
                    "onPress".into(),
                    EventHandlerIR::InlineFn {
                        body: BlockIR {
                            statements: vec![StatementIR::ExprStmtIR {
                                expr: method_call("box", "getBoundingClientRect", vec![]),
                            }],
                        },
                        params: vec![],
                        event_data: None,
                        used_fields: None,
                        needs_prevent_default: false,
                        needs_stop_propagation: false,
                    },
                );
                events
            },
            props: rustc_hash::FxHashMap::default(),
            bindings: rustc_hash::FxHashMap::default(),
            children: vec![],
            style_classes: None,
            inline_style: None,
            construction_kind: crate::ir::ConstructionKind::Normal,
            class_map_bindings: rustc_hash::FxHashMap::default(),
            source_span: Some(Span {
                start: 10,
                end: 30,
                line: 5,
                col: 1,
            }),
            motion: None,
            ref_binding: None,
            semantic_bindings: rustc_hash::FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
        })));
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("getBoundingClientRect"));
        assert_eq!(
            diags[0].line, 5,
            "must anchor at the element line, got: {:?}",
            diags[0]
        );
        assert_eq!(diags[0].col, 1);
    }

    #[test]
    fn walks_effect_bodies() {
        let mut comp = make_component();
        comp.effects.push(crate::ir::EffectIntent {
            deps: vec![],
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: method_call("box", "blur", vec![]),
                }],
            },
        });
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(
            &comp.effects[0].body.statements[0],
            StatementIR::ExprStmtIR {
                expr: ExprIR::RefOpExpr { .. }
            }
        ));
    }

    #[test]
    fn walks_view_interpolations() {
        let mut comp = make_component();
        comp.view = Some(ViewTree::Interpolation(Box::new(method_call(
            "box",
            "measure",
            vec![],
        ))));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(
            comp.view.as_ref().unwrap(),
            ViewTree::Interpolation(expr) if matches!(expr.as_ref(), ExprIR::RefOpExpr { .. })
        ));
    }

    #[test]
    fn measure_method_maps_to_position_measure() {
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("box", "measure", vec![]));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::RefOpExpr { op, .. } => assert!(matches!(
                op,
                RefOpIR::Measure {
                    what: MeasureWhat::Position
                }
            )),
            other => panic!("expected RefOpExpr, got: {:?}", other),
        }
    }

    /// Helper: push a function with a single mutation statement to a component.
    fn push_mutation_stmt(comp: &mut ComponentIR, intent: MutationIntent) {
        comp.fns.push(FunctionIR {
            name: "run".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(intent),
                }],
            },
            is_async: false,
            source_span: Some(FN_SPAN),
        });
    }

    fn direct_assign(target: &str, value: ExprIR, field_object: Option<&str>) -> MutationIntent {
        MutationIntent::DirectAssign {
            target: crate::ir::SignalRefIR {
                name: target.into(),
            },
            value: Some(Box::new(value)),
            field_object: field_object.map(String::from),
        }
    }

    #[test]
    fn ref_value_field_write_is_diagnosed() {
        // myRef.value = x — lowered with the owner preserved in field_object.
        // The imperative write op (RefOpIR::SetValue) was removed (11 → 10
        // ops); two-way bind:value covers the write intent, so this must be
        // a hard error with the migration hint.
        let mut comp = make_component();
        push_mutation_stmt(
            &mut comp,
            direct_assign(
                "value",
                ExprIR::LiteralExpr {
                    value: serde_json::json!("hi"),
                },
                Some("box"),
            ),
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("命令式写已删除"),
            "expected migration hint, got: {:?}",
            diags[0].message
        );
        assert!(
            diags[0].message.contains("bind:value"),
            "expected bind:value hint, got: {:?}",
            diags[0].message
        );
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some(SETVALUE_REMOVED_HINT),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
        // The mutation is not rewritten — it stays a mutation (the compile
        // fails on the error, so no codegen output is produced).
        assert!(matches!(
            &comp.fns[0].body.statements[0],
            StatementIR::MutationStmt { .. }
        ));
    }

    #[test]
    fn ref_set_value_method_call_is_diagnosed() {
        // myRef.setValue(x) / myRef.set(x) — lowered to a DirectAssign whose
        // target name is the ref itself.
        let mut comp = make_component();
        push_mutation_stmt(
            &mut comp,
            direct_assign(
                "box",
                ExprIR::LiteralExpr {
                    value: serde_json::json!(42),
                },
                None,
            ),
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("命令式写已删除"),
            "expected migration hint, got: {:?}",
            diags[0].message
        );
        assert!(
            diags[0].message.contains("bind:value"),
            "expected bind:value hint, got: {:?}",
            diags[0].message
        );
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some(SETVALUE_REMOVED_HINT),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
        assert!(matches!(
            &comp.fns[0].body.statements[0],
            StatementIR::MutationStmt { .. }
        ));
    }

    #[test]
    fn signal_assign_is_left_untouched() {
        // count.set(5) on a signal — target is not a ref, must stay a mutation.
        let mut comp = make_component();
        push_mutation_stmt(
            &mut comp,
            direct_assign(
                "count",
                ExprIR::LiteralExpr {
                    value: serde_json::json!(5),
                },
                None,
            ),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(
            &comp.fns[0].body.statements[0],
            StatementIR::MutationStmt { .. }
        ));
    }

    #[test]
    fn store_field_assign_is_left_untouched() {
        // MyStore.count = x — the owner is a store, not a ref.
        let mut comp = make_component();
        push_mutation_stmt(
            &mut comp,
            direct_assign(
                "count",
                ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                },
                Some("MyStore"),
            ),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(
            &comp.fns[0].body.statements[0],
            StatementIR::MutationStmt { .. }
        ));
    }

    #[test]
    fn illegal_ref_field_write_errors() {
        // myRef.offsetWidth = x — illegal ref field write.
        let mut comp = make_component();
        push_mutation_stmt(
            &mut comp,
            direct_assign(
                "offsetWidth",
                ExprIR::LiteralExpr {
                    value: serde_json::json!(10),
                },
                Some("box"),
            ),
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("offsetWidth"));
        assert!(diags[0].message.contains("ref.measure().width"));
        assert_eq!(diags[0].line, FN_SPAN.line, "must anchor at the fn span");
        assert_eq!(
            diags[0].suggestion.as_deref(),
            Some("use ref.measure().width instead"),
            "verbatim hint must land in suggestion: {:?}",
            diags[0]
        );
        // The mutation stays a mutation (nothing rewritten).
        assert!(matches!(
            &comp.fns[0].body.statements[0],
            StatementIR::MutationStmt { .. }
        ));
    }

    #[test]
    fn local_shadowing_skips_rewrite() {
        // let box = 5; box.focus() — the local shadows the ref.
        let mut comp = make_component();
        comp.fns.push(FunctionIR {
            name: "run".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![
                    StatementIR::LetStmtIR {
                        name: "box".into(),
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!(5),
                        }),
                        r#type: None,
                        is_mut: false,
                    },
                    StatementIR::ExprStmtIR {
                        expr: method_call("box", "focus", vec![]),
                    },
                ],
            },
            is_async: false,
            source_span: None,
        });
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(
            &comp.fns[0].body.statements[1],
            StatementIR::ExprStmtIR {
                expr: ExprIR::MethodCallExpr { .. }
            }
        ));
    }

    #[test]
    fn param_shadowing_skips_rewrite() {
        // fn run(box: i32) { box.focus() } — the param shadows the ref.
        let mut comp = make_component();
        comp.fns.push(FunctionIR {
            name: "run".into(),
            visibility: Visibility::Private,
            params: vec![crate::ir::ParamIR {
                name: "box".into(),
                r#type: None,
            }],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: method_call("box", "focus", vec![]),
                }],
            },
            is_async: false,
            source_span: None,
        });
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }));
    }

    #[test]
    fn ref_use_before_local_binding_still_rewrites() {
        // box.focus(); let box = 5; — the first call happens before the
        // binding exists, so it refers to the ref.
        let mut comp = make_component();
        comp.fns.push(FunctionIR {
            name: "run".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![
                    StatementIR::ExprStmtIR {
                        expr: method_call("box", "focus", vec![]),
                    },
                    StatementIR::LetStmtIR {
                        name: "box".into(),
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!(5),
                        }),
                        r#type: None,
                        is_mut: false,
                    },
                ],
            },
            is_async: false,
            source_span: None,
        });
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(
            &comp.fns[0].body.statements[0],
            StatementIR::ExprStmtIR {
                expr: ExprIR::RefOpExpr { .. }
            }
        ));
    }
}
