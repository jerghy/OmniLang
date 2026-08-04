// ═══════════════════════════════════════════════════════════
// Effect codegen: @effect/@mount/@unmount → useEffect
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::codegen::react::ReactCodegen;
use crate::codegen::CodeWriter;
use crate::ir::walk::walk_stmt;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

impl ReactCodegen {
    /// Generate useEffect calls for @effect declarations and lifecycle hooks
    /// (@mount, @unmount, @effect).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gen_effects(
        &self,
        w: &mut CodeWriter,
        comp: &ComponentIR,
        store_signal_map: &FxHashMap<String, String>,
        store_signal_owners: &FxHashMap<String, String>,
        signal_names: &FxHashSet<String>,
        ref_names: &FxHashSet<String>,
        store_field_renames: &FxHashMap<String, String>,
        store_fields: &FxHashMap<String, FxHashSet<String>>,
        hoisted_signals: &FxHashSet<String>,
        pruned_signal_names: &FxHashSet<String>,
        possibly_undefined: &FxHashSet<String>,
    ) {
        // Effects (useEffect) — wrap in async IIFE if body contains await
        for effect in &comp.effects {
            // Resolve store signal deps to their emitted variable names:
            // a slice-subscribed store field deps on the slice local
            // (`[notes]`, T3.3); otherwise the qualified name
            // (`NotificationStore.notifications`) or the bare name.
            // Hoisted signals (perf M3) are module-scope consts — never
            // reactive — and must not enter the dep array (eslint
            // react-hooks flags outer-scope values as "unnecessary
            // dependency").
            let deps: Vec<String> = effect
                .deps
                .iter()
                .filter(|d| !hoisted_signals.contains(*d))
                .map(|d| {
                    let qualified = store_signal_map
                        .get(d)
                        .cloned()
                        .unwrap_or_else(|| d.clone());
                    store_field_renames
                        .get(&qualified)
                        .cloned()
                        .unwrap_or(qualified)
                })
                .collect();
            let deps_str = deps.join(", ");
            let body_has_await = effect.body.statements.iter().any(stmt_contains_await);
            if body_has_await {
                w.emit("useEffect(() => {");
                w.indent();
                w.emit("(async () => {");
                w.indent();
                for stmt in &effect.body.statements {
                    expr::generate_statement_tsx(
                        &mut *w,
                        stmt,
                        signal_names,
                        store_signal_owners,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                    );
                }
                w.dedent();
                w.emit("})();");
                w.dedent();
                w.emit_fmt(format_args!("}}, [{}]);", deps_str));
            } else {
                w.emit("useEffect(() => {");
                w.indent();
                for stmt in &effect.body.statements {
                    expr::generate_statement_tsx(
                        &mut *w,
                        stmt,
                        signal_names,
                        store_signal_owners,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                    );
                }
                w.dedent();
                w.emit_fmt(format_args!("}}, [{}]);", deps_str));
            }
        }

        // Lifecycles (mount/unmount)
        for lc in &comp.lifecycles {
            match lc.kind {
                LifecycleKind::Mount => {
                    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
                    if body_has_await {
                        w.emit("useEffect(() => {");
                        w.indent();
                        w.emit("(async () => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                                pruned_signal_names,
                                possibly_undefined,
                            );
                        }
                        w.dedent();
                        w.emit("})();");
                        w.dedent();
                        w.emit("}, []);");
                    } else {
                        w.emit("useEffect(() => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                                pruned_signal_names,
                                possibly_undefined,
                            );
                        }
                        w.dedent();
                        w.emit("}, []);");
                    }
                }
                LifecycleKind::Unmount => {
                    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
                    w.emit("useEffect(() => {");
                    w.indent();
                    w.emit("return () => {");
                    w.indent();
                    if body_has_await {
                        w.emit("(async () => {");
                        w.indent();
                    }
                    for stmt in &lc.body.statements {
                        expr::generate_statement_tsx(
                            &mut *w,
                            stmt,
                            signal_names,
                            store_signal_owners,
                            ref_names,
                            store_field_renames,
                            pruned_signal_names,
                            possibly_undefined,
                        );
                    }
                    if body_has_await {
                        w.dedent();
                        w.emit("})();");
                    }
                    w.dedent();
                    w.emit("};");
                    w.dedent();
                    // Task 2.6 (ts/react M4): the cleanup closure reads must
                    // enter the deps — with `[]` the cleanup runs at unmount
                    // with the FIRST-render values (Timer's `final count`
                    // was always 0). Derived from the body reads: with
                    // `[count]` React re-registers the cleanup on every
                    // count change, so the unmount cleanup closes over the
                    // LATEST value.
                    //
                    // CONTRACT CONFLICT (mid-term item, review Important-1):
                    // derived deps re-run the WHOLE cleanup on every dep
                    // change — a state WRITE inside the cleanup executes
                    // early (Timer's `running = false` fired on every tick
                    // and killed the `if running` gate; the demo write was
                    // removed). Cleanups should only READ — a write + read
                    // shape needs a ref channel (latest-value ref, deps `[]`)
                    // or an explicit once-only contract. Tracked for the
                    // orchestrator (task 2.6 report concerns).
                    let unmount_deps = unmount_deps(
                        &lc.body,
                        comp,
                        store_signal_map,
                        store_field_renames,
                        store_fields,
                        hoisted_signals,
                    );
                    let deps_str = unmount_deps.join(", ");
                    w.emit_fmt(format_args!("}}, [{}]);", deps_str));
                }
                LifecycleKind::Effect => {
                    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
                    if body_has_await {
                        w.emit("useEffect(() => {");
                        w.indent();
                        w.emit("(async () => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                                pruned_signal_names,
                                possibly_undefined,
                            );
                        }
                        w.dedent();
                        w.emit("})();");
                        w.dedent();
                        w.emit("});");
                    } else {
                        w.emit("useEffect(() => {");
                        w.indent();
                        for stmt in &lc.body.statements {
                            expr::generate_statement_tsx(
                                &mut *w,
                                stmt,
                                signal_names,
                                store_signal_owners,
                                ref_names,
                                store_field_renames,
                                pruned_signal_names,
                                possibly_undefined,
                            );
                        }
                        w.dedent();
                        w.emit("});");
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Unmount cleanup deps derivation (task 2.6, ts/react M4)
// ═══════════════════════════════════════════════════════════

/// Derive the useEffect deps for an `@unmount` cleanup from its body reads.
///
/// The cleanup closure runs at unmount with the values captured when the
/// effect was last registered — a read not in the deps closes over the
/// FIRST-render value (stale cleanup, Timer final-count bug). Every reactive
/// read enters the deps:
///   - component signals / computeds / props (bare names)
///   - store fields `Store.field` (resolved through the slice renames like
///     `@effect` deps — slice local, or the qualified whole-store read stays)
///
/// setState setters, store actions, function calls, and body locals never
/// enter the deps (stable or non-reactive — including them would re-register
/// the cleanup on every render and change the one-shot unmount semantics).
/// Hoisted signals (perf M3) are module-scope consts — never reactive — and
/// are excluded like the other stable names.
fn unmount_deps(
    body: &BlockIR,
    comp: &ComponentIR,
    store_signal_map: &FxHashMap<String, String>,
    store_field_renames: &FxHashMap<String, String>,
    store_fields: &FxHashMap<String, FxHashSet<String>>,
    hoisted_signals: &FxHashSet<String>,
) -> Vec<String> {
    // Reactive bare names: component signals/computeds/props + store signal
    // names. Store ALIASES are excluded — a `Store.x` read resolves to the
    // qualified field (or slice local), never the whole-store alias.
    let mut reactive: FxHashSet<&str> = FxHashSet::default();
    reactive.extend(
        comp.signals
            .iter()
            .map(|s| s.name.as_str())
            .filter(|n| !hoisted_signals.contains(*n)),
    );
    reactive.extend(comp.computeds.iter().map(|c| c.name.as_str()));
    reactive.extend(comp.props.keys().map(String::as_str));
    reactive.extend(store_signal_map.keys().map(String::as_str));

    // Body locals (let-bound / for-in vars / catch vars) shadow reactive
    // names and never enter the deps.
    let mut locals: FxHashSet<&str> = FxHashSet::default();
    collect_body_locals(&body.statements, &mut locals);

    let reads = collect_reactive_reads(&body.statements, &reactive, &locals, store_fields);
    resolve_reads(reads, store_signal_map, store_field_renames)
}

/// Collect reactive reads from a statement list — the walk shared by
/// `unmount_deps` (task 2.6) and `fn_read_deps` (task 2.8):
///   - qualified store-field reads (`Store.field`, `Store?.field`,
///     store-field methods) — resolved through the slice renames later;
///   - bare identifier reads in the `reactive` set, excluding
///     `locals`-shadowed names (let-bound / for-in / catch vars, and — for
///     fn bodies — the fn's own params).
fn collect_reactive_reads(
    stmts: &[StatementIR],
    reactive: &FxHashSet<&str>,
    locals: &FxHashSet<&str>,
    store_fields: &FxHashMap<String, FxHashSet<String>>,
) -> Vec<String> {
    let mut reads: Vec<String> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for stmt in stmts {
        walk_stmt(stmt, &mut |expr| match expr {
            // `Store.field` / `Store?.field` — qualified read (the store
            // alias itself is never a bare dep).
            ExprIR::FieldAccessExpr { object, field }
            | ExprIR::OptionalChainExpr { object, field } => {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if store_fields
                        .get(name.as_str())
                        .is_some_and(|f| f.contains(field.as_str()))
                    {
                        push_read(format!("{}.{}", name, field), &mut seen, &mut reads);
                    }
                }
            }
            ExprIR::StoreFieldAccessExpr {
                store_alias,
                field_name,
                ..
            } => {
                if store_fields
                    .get(store_alias.as_str())
                    .is_some_and(|f| f.contains(field_name.as_str()))
                {
                    push_read(
                        format!("{}.{}", store_alias, field_name),
                        &mut seen,
                        &mut reads,
                    );
                }
            }
            // `Store.method(...)` — a store-field method call is a read
            // (e.g. `Store.items.len()` via its inner FieldAccessExpr arm
            // above); a store ACTION call (`Store.clear()`) never is.
            ExprIR::MethodCallExpr { object, method, .. } => {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if store_fields
                        .get(name.as_str())
                        .is_some_and(|f| f.contains(method.as_str()))
                    {
                        push_read(format!("{}.{}", name, method), &mut seen, &mut reads);
                    }
                }
            }
            // Bare identifier reads (signals/computeds/props/store signals),
            // excluding body locals.
            ExprIR::IdentifierExpr { name, .. }
                if reactive.contains(name.as_str()) && !locals.contains(name.as_str()) =>
            {
                push_read(name.clone(), &mut seen, &mut reads);
            }
            _ => {}
        });
    }
    reads
}

/// Resolve reads like `@effect` deps: bare store-signal names →
/// `Store.field`, then the slice renames → slice local (the qualified
/// whole-store read stays when no rename exists).
fn resolve_reads(
    reads: Vec<String>,
    store_signal_map: &FxHashMap<String, String>,
    store_field_renames: &FxHashMap<String, String>,
) -> Vec<String> {
    reads
        .into_iter()
        .map(|read| {
            let qualified = store_signal_map.get(&read).cloned().unwrap_or(read);
            store_field_renames
                .get(&qualified)
                .cloned()
                .unwrap_or(qualified)
        })
        .collect()
}

/// The useCallback dep array for a component-local fn passed BY VALUE to a
/// child component (task 2.8, perf M4).
///
/// The React gate (verify-react.sh, eslint-plugin-react-hooks v7) RESOLVES
/// the callback identifier and flags every dependency divergence against
/// the EMITTED body — so this collector mirrors the emitted body's
/// references, not the raw IR reads:
///   - component signals / computeds / props / event props / store signal
///     bare names → bare deps (resolved through the slice renames);
///   - qualified store refs (`TodoStore.items`, `TodoStore.add(x)`) → the
///     slice local when subscribed per field, else the qualified name
///     (whole-store alias);
///   - component-local fn calls → the fn name (the const is recreated every
///     render);
///   - whole-store alias referenced as a value → the alias name (only when
///     no slice renames exist for the alias — a sliced alias is never
///     emitted as a const, so the dep would be TS2304).
///
/// Excluded — stable per the rule: useState setters, useRef objects,
/// module-scope lib fns/imports, fn params, body locals, and closure
/// params. Also excluded: mutation TARGET references — `count = count + 1`
/// emits `setCount((s) => s + 1)` (no `count` reference) and
/// `TodoStore.items.push(x)` emits `useTodoStore.setState(...)` (no bare
/// `TodoStore`/`items` reference) — including them would be an
/// "unnecessary dependency" error.
///
/// The fn's own params shadow reactive names — a param in the dep array
/// would be TS2304 at the wrapper site (it is not in scope there).
#[allow(clippy::too_many_arguments)]
pub(super) fn fn_read_deps(
    fun: &FunctionIR,
    comp: &ComponentIR,
    local_fn_names: &FxHashSet<String>,
    store_signal_map: &FxHashMap<String, String>,
    store_field_renames: &FxHashMap<String, String>,
    store_fields: &FxHashMap<String, FxHashSet<String>>,
    store_actions: &FxHashMap<String, FxHashSet<String>>,
    used_store_refs: &FxHashSet<String>,
    hoisted_signals: &FxHashSet<String>,
) -> Vec<String> {
    // Bare-name universe the rule treats as reactive. Hoisted signals
    // (perf M3) are module-scope consts — excluded like the other stable
    // names (a hoisted const in the dep array is an eslint
    // "unnecessary dependency" error).
    let mut reactive: FxHashSet<String> = FxHashSet::default();
    reactive.extend(
        comp.signals
            .iter()
            .map(|s| s.name.clone())
            .filter(|n| !hoisted_signals.contains(n)),
    );
    reactive.extend(comp.computeds.iter().map(|c| c.name.clone()));
    reactive.extend(comp.props.keys().cloned());
    reactive.extend(comp.events.iter().map(|e| e.name.clone()));
    reactive.extend(store_signal_map.keys().cloned());

    let mut locals: FxHashSet<&str> = FxHashSet::default();
    locals.extend(fun.params.iter().map(|p| p.name.as_str()));
    collect_body_locals(&fun.body.statements, &mut locals);

    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut reads: Vec<String> = Vec::new();
    let collector = EmittedRefCollector {
        reactive: &reactive,
        store_fields,
        store_actions,
        store_signal_map,
        renames: store_field_renames,
        local_fns: local_fn_names,
        used_store_refs,
    };
    for stmt in &fun.body.statements {
        collector.walk_stmt(stmt, &locals, &mut reads, &mut seen);
    }

    // Resolve like `@effect` deps (bare store-signal names → `Store.field`,
    // then the slice renames → slice local), deduping post-resolution — a
    // read referenced both bare (`items`) and qualified (`TodoStore.items`)
    // converges on the same slice local.
    let mut resolved: Vec<String> = Vec::new();
    let mut seen_resolved: FxHashSet<String> = FxHashSet::default();
    for read in reads {
        let qualified = store_signal_map.get(&read).cloned().unwrap_or(read);
        let final_name = store_field_renames
            .get(&qualified)
            .cloned()
            .unwrap_or(qualified);
        if seen_resolved.insert(final_name.clone()) {
            resolved.push(final_name);
        }
    }
    resolved
}

/// Context for [`EmittedRefCollector`] — the emitted-body reference mirror
/// described on [`fn_read_deps`].
struct EmittedRefCollector<'a> {
    /// Bare reactive names: signals ∪ computeds ∪ props ∪ event props ∪
    /// store signal names.
    reactive: &'a FxHashSet<String>,
    /// Store alias → fields+getters (qualified-read detection).
    store_fields: &'a FxHashMap<String, FxHashSet<String>>,
    /// Store alias → actions (qualified-action-call detection).
    store_actions: &'a FxHashMap<String, FxHashSet<String>>,
    /// Bare store-signal name → `Store.field` (resolution step 1).
    store_signal_map: &'a FxHashMap<String, String>,
    /// Qualified `Store.field` → slice local (resolution step 2).
    renames: &'a FxHashMap<String, String>,
    /// Component-local fn names (call sites emit the fn name).
    local_fns: &'a FxHashSet<String>,
    /// Used store aliases (a bare alias ref can only be whole-store).
    used_store_refs: &'a FxHashSet<String>,
}

impl<'a> EmittedRefCollector<'a> {
    /// Whether the alias is whole-store: no slice rename exists for it, so
    /// `const TodoStore = useTodoStore();` IS emitted and bare alias refs
    /// are valid deps.
    fn alias_is_whole_store(&self, alias: &str) -> bool {
        let prefix = format!("{}.", alias);
        !self.renames.keys().any(|k| k.starts_with(&prefix))
    }

    fn push(&self, name: &str, reads: &mut Vec<String>, seen: &mut FxHashSet<String>) {
        if seen.insert(name.to_string()) {
            reads.push(name.to_string());
        }
    }

    /// A bare identifier reference in the emitted body.
    fn push_ident(
        &self,
        name: &str,
        locals: &FxHashSet<&str>,
        reads: &mut Vec<String>,
        seen: &mut FxHashSet<String>,
    ) {
        if locals.contains(name) {
            return;
        }
        if self.reactive.contains(name) {
            self.push(name, reads, seen);
            return;
        }
        // Whole-store alias referenced as a value (`return TodoStore;`).
        if self.used_store_refs.contains(name) && self.alias_is_whole_store(name) {
            self.push(name, reads, seen);
        }
    }

    /// A qualified store reference (`TodoStore.items` / `TodoStore.add(x)`)
    /// — the emitted form is the slice local, or the qualified member
    /// expression for whole-store aliases.
    fn push_qualified(
        &self,
        alias: &str,
        member: &str,
        reads: &mut Vec<String>,
        seen: &mut FxHashSet<String>,
    ) {
        let qualified = format!("{}.{}", alias, member);
        if let Some(local) = self.renames.get(&qualified) {
            self.push(local, reads, seen);
        } else {
            self.push(&qualified, reads, seen);
        }
    }

    fn walk_block(
        &self,
        body: &BlockIR,
        locals: &FxHashSet<&str>,
        reads: &mut Vec<String>,
        seen: &mut FxHashSet<String>,
    ) {
        for stmt in &body.statements {
            self.walk_stmt(stmt, locals, reads, seen);
        }
    }

    fn walk_stmt(
        &self,
        stmt: &StatementIR,
        locals: &FxHashSet<&str>,
        reads: &mut Vec<String>,
        seen: &mut FxHashSet<String>,
    ) {
        match stmt {
            StatementIR::ExprStmtIR { expr } => self.walk_expr(expr, locals, reads, seen),
            StatementIR::LetStmtIR { value, .. } => {
                if let Some(ref v) = value {
                    self.walk_expr(v, locals, reads, seen);
                }
            }
            StatementIR::ReturnStmtIR { value } => {
                if let Some(ref v) = value {
                    self.walk_expr(v, locals, reads, seen);
                }
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                self.walk_expr(condition, locals, reads, seen);
                self.walk_block(then_body, locals, reads, seen);
                for (cond, body) in else_ifs {
                    self.walk_expr(cond, locals, reads, seen);
                    self.walk_block(body, locals, reads, seen);
                }
                if let Some(ref eb) = else_body {
                    self.walk_block(eb, locals, reads, seen);
                }
            }
            StatementIR::WhileStmtIR { condition, body } => {
                self.walk_expr(condition, locals, reads, seen);
                self.walk_block(body, locals, reads, seen);
            }
            StatementIR::ForInStmtIR { iterable, body, .. } => {
                self.walk_expr(iterable, locals, reads, seen);
                self.walk_block(body, locals, reads, seen);
            }
            StatementIR::BlockStmtIR { body } => self.walk_block(body, locals, reads, seen),
            StatementIR::MutationStmt { intent } => {
                // Mutation targets never appear in the emitted code:
                // component signals emit setter calls (`setX(...)`), store
                // signals emit `useStore.setState((s) => ...)`. Walk the
                // value/amount/item with the target excluded.
                let (target, exprs): (Option<&SignalRefIR>, Vec<&ExprIR>) = match intent {
                    None => (None, Vec::new()),
                    Some(MutationIntent::DirectAssign { target, value, .. }) => {
                        (Some(target), value.as_deref().into_iter().collect())
                    }
                    Some(MutationIntent::Increment { target, amount })
                    | Some(MutationIntent::Decrement { target, amount })
                    | Some(MutationIntent::Multiply { target, amount })
                    | Some(MutationIntent::Divide { target, amount })
                    | Some(MutationIntent::Modulo { target, amount }) => {
                        (Some(target), amount.as_deref().into_iter().collect())
                    }
                    Some(MutationIntent::Push { target, item }) => {
                        (Some(target), item.as_deref().into_iter().collect())
                    }
                    Some(MutationIntent::Pop { target }) => (Some(target), Vec::new()),
                    // MutationIntent is #[non_exhaustive] — cross-crate
                    // matches need a wildcard.
                    #[allow(unreachable_patterns)]
                    _ => (None, Vec::new()),
                };
                let mut local_reads: Vec<String> = Vec::new();
                let mut local_seen: FxHashSet<String> = FxHashSet::default();
                for expr in exprs {
                    self.walk_expr(expr, locals, &mut local_reads, &mut local_seen);
                }
                if let Some(target) = target {
                    local_reads.retain(|r| r != &target.name);
                    if let Some(owner) = self.store_signal_map.get(&target.name) {
                        let qualified = format!("{}.{}", owner, target.name);
                        local_reads.retain(|r| r != &qualified);
                    }
                }
                for read in local_reads {
                    self.push(&read, reads, seen);
                }
            }
            StatementIR::ThrowStmtIR { expr } => self.walk_expr(expr, locals, reads, seen),
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.walk_block(try_body, locals, reads, seen);
                if let Some(ref cb) = catch_body {
                    self.walk_block(cb, locals, reads, seen);
                }
                if let Some(ref fb) = finally_body {
                    self.walk_block(fb, locals, reads, seen);
                }
            }
            StatementIR::EventEmitStmtIR { event_name, args } => {
                // `emit onX(v)` emits `onX(v);` — the event prop is a
                // reference the rule flags.
                self.push_ident(event_name, locals, reads, seen);
                for arg in args {
                    self.walk_expr(arg, locals, reads, seen);
                }
            }
            StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
            #[allow(unreachable_patterns)]
            _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
        }
    }

    fn walk_expr(
        &self,
        expr: &ExprIR,
        locals: &FxHashSet<&str>,
        reads: &mut Vec<String>,
        seen: &mut FxHashSet<String>,
    ) {
        match expr {
            ExprIR::IdentifierExpr { name, .. } => self.push_ident(name, locals, reads, seen),
            ExprIR::FieldAccessExpr { object, field }
            | ExprIR::OptionalChainExpr { object, field } => {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if self
                        .store_fields
                        .get(name.as_str())
                        .is_some_and(|f| f.contains(field.as_str()))
                    {
                        self.push_qualified(name, field, reads, seen);
                        return;
                    }
                }
                self.walk_expr(object, locals, reads, seen);
            }
            ExprIR::MethodCallExpr {
                object,
                method,
                args,
                ..
            } => {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if self
                        .store_fields
                        .get(name.as_str())
                        .is_some_and(|f| f.contains(method.as_str()))
                        || self
                            .store_actions
                            .get(name.as_str())
                            .is_some_and(|a| a.contains(method.as_str()))
                    {
                        // `TodoStore.items.length` / `TodoStore.add(x)` —
                        // the whole call resolves to the slice local (or
                        // stays qualified for whole-store aliases); the
                        // object's bare alias is never emitted separately.
                        self.push_qualified(name, method, reads, seen);
                        for arg in args {
                            self.walk_expr(arg, locals, reads, seen);
                        }
                        return;
                    }
                }
                self.walk_expr(object, locals, reads, seen);
                for arg in args {
                    self.walk_expr(arg, locals, reads, seen);
                }
            }
            ExprIR::StoreFieldAccessExpr {
                store_alias,
                field_name,
                call_args,
            } => {
                if self
                    .store_fields
                    .get(store_alias.as_str())
                    .is_some_and(|f| f.contains(field_name.as_str()))
                    || self
                        .store_actions
                        .get(store_alias.as_str())
                        .is_some_and(|a| a.contains(field_name.as_str()))
                {
                    self.push_qualified(store_alias, field_name, reads, seen);
                }
                for arg in call_args {
                    self.walk_expr(arg, locals, reads, seen);
                }
            }
            ExprIR::CallExpr { name, args } => {
                // Component-local fn call — the fn const is recreated every
                // render, so the name is a dependency. Lib fns (module
                // scope) and imports never are.
                if self.local_fns.contains(name) {
                    self.push(name, reads, seen);
                }
                for arg in args {
                    self.walk_expr(arg, locals, reads, seen);
                }
            }
            ExprIR::ClosureExpr { params, body } => {
                // Closure params shadow reactive names — they are local to
                // the closure and never flagged by the rule.
                let mut shadowed = locals.clone();
                shadowed.extend(params.iter().map(String::as_str));
                self.walk_expr(body, &shadowed, reads, seen);
            }
            ExprIR::BinaryOpExpr { left, right, .. } => {
                self.walk_expr(left, locals, reads, seen);
                self.walk_expr(right, locals, reads, seen);
            }
            ExprIR::UnaryOpExpr { operand, .. } => self.walk_expr(operand, locals, reads, seen),
            ExprIR::TernaryIR {
                cond,
                then_expr,
                else_expr,
            } => {
                self.walk_expr(cond, locals, reads, seen);
                self.walk_expr(then_expr, locals, reads, seen);
                self.walk_expr(else_expr, locals, reads, seen);
            }
            ExprIR::ArrayExpr { elements } => {
                for elem in elements {
                    self.walk_expr(elem, locals, reads, seen);
                }
            }
            ExprIR::MapExpr { entries } => {
                for (k, v) in entries {
                    self.walk_expr(k, locals, reads, seen);
                    self.walk_expr(v, locals, reads, seen);
                }
            }
            ExprIR::BlockExpr { statements, tail } => {
                for stmt in statements {
                    self.walk_stmt(stmt, locals, reads, seen);
                }
                self.walk_expr(tail, locals, reads, seen);
            }
            ExprIR::MatchExpr { matched, arms } => {
                self.walk_expr(matched, locals, reads, seen);
                for arm in arms {
                    self.walk_block(&arm.body, locals, reads, seen);
                }
            }
            ExprIR::VariantLiteral { fields, .. } | ExprIR::StructExpr { fields, .. } => {
                for (_, v) in fields {
                    self.walk_expr(v, locals, reads, seen);
                }
            }
            ExprIR::SpreadExpr { operand } => self.walk_expr(operand, locals, reads, seen),
            ExprIR::IndexExpr { object, index } => {
                self.walk_expr(object, locals, reads, seen);
                self.walk_expr(index, locals, reads, seen);
            }
            ExprIR::AwaitExpr { operand } => self.walk_expr(operand, locals, reads, seen),
            ExprIR::ArrayMethod {
                subject, predicate, ..
            } => {
                self.walk_expr(subject, locals, reads, seen);
                self.walk_expr(predicate, locals, reads, seen);
            }
            ExprIR::RefOpExpr { op, .. } => {
                // Ref ops carry only value expressions (scrollTo(x, y)…);
                // ref names themselves are stable (useRef) and never deps.
                collect_ref_op_value_exprs(op, locals, reads, seen, self);
            }
            ExprIR::LiteralExpr { .. } => {}
            #[allow(unreachable_patterns)]
            _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
        }
    }
}

/// Walk the value expressions carried by a `RefOpIR` (the ref object itself
/// is stable — useRef — and never a dependency).
fn collect_ref_op_value_exprs<'a>(
    op: &RefOpIR,
    locals: &FxHashSet<&str>,
    reads: &mut Vec<String>,
    seen: &mut FxHashSet<String>,
    collector: &EmittedRefCollector<'a>,
) {
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
                collector.walk_expr(x, locals, reads, seen);
            }
            if let Some(y) = y {
                collector.walk_expr(y, locals, reads, seen);
            }
        }
        RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                collector.walk_expr(dx, locals, reads, seen);
            }
            if let Some(dy) = dy {
                collector.walk_expr(dy, locals, reads, seen);
            }
        }
        RefOpIR::SetSelection { start, end } => {
            collector.walk_expr(start, locals, reads, seen);
            collector.walk_expr(end, locals, reads, seen);
        }
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Push a read unless already seen (dedupe — the walker visits nested
/// expressions like `count.to_string()`'s object twice).
fn push_read(read: String, seen: &mut FxHashSet<String>, reads: &mut Vec<String>) {
    if seen.insert(read.clone()) {
        reads.push(read);
    }
}

/// Collect locally-bound names in a statement list — they shadow reactive
/// names and never belong in the deps.
fn collect_body_locals<'a>(stmts: &'a [StatementIR], acc: &mut FxHashSet<&'a str>) {
    for stmt in stmts {
        match stmt {
            StatementIR::LetStmtIR { name, .. } => {
                acc.insert(name.as_str());
            }
            StatementIR::ForInStmtIR { var_name, body, .. } => {
                acc.insert(var_name.as_str());
                collect_body_locals(&body.statements, acc);
            }
            StatementIR::IfStmtIR {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_body_locals(&then_body.statements, acc);
                for (_, b) in else_ifs {
                    collect_body_locals(&b.statements, acc);
                }
                if let Some(b) = else_body {
                    collect_body_locals(&b.statements, acc);
                }
            }
            StatementIR::WhileStmtIR { body, .. } => collect_body_locals(&body.statements, acc),
            StatementIR::BlockStmtIR { body } => collect_body_locals(&body.statements, acc),
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_var,
                catch_body,
                finally_body,
                ..
            } => {
                collect_body_locals(&try_body.statements, acc);
                if let Some(cv) = catch_var {
                    acc.insert(cv);
                }
                if let Some(b) = catch_body {
                    collect_body_locals(&b.statements, acc);
                }
                if let Some(b) = finally_body {
                    collect_body_locals(&b.statements, acc);
                }
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Await detection helpers
// ═══════════════════════════════════════════════════════════

/// Check if a StatementIR or any nested statement contains an `await` expression.
pub(super) fn stmt_contains_await(stmt: &StatementIR) -> bool {
    match stmt {
        StatementIR::ExprStmtIR { expr } => expr_contains_await(expr),
        StatementIR::LetStmtIR { value, .. } => value.as_ref().is_some_and(expr_contains_await),
        StatementIR::ReturnStmtIR { value } => value.as_ref().is_some_and(expr_contains_await),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            if expr_contains_await(condition) {
                return true;
            }
            if then_body.statements.iter().any(stmt_contains_await) {
                return true;
            }
            for (_, elif_body) in else_ifs {
                if elif_body.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            if let Some(ref eb) = else_body {
                if eb.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            false
        }
        StatementIR::WhileStmtIR { body, .. } => body.statements.iter().any(stmt_contains_await),
        StatementIR::ForInStmtIR {
            var_name: _,
            iterable,
            body,
        } => expr_contains_await(iterable) || body.statements.iter().any(stmt_contains_await),
        StatementIR::BlockStmtIR { body } => body.statements.iter().any(stmt_contains_await),
        StatementIR::ThrowStmtIR { expr } => expr_contains_await(expr),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var: _,
            catch_body,
            finally_body,
        } => {
            if try_body.statements.iter().any(stmt_contains_await) {
                return true;
            }
            if let Some(ref cb) = catch_body {
                if cb.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            if let Some(ref fb) = finally_body {
                if fb.statements.iter().any(stmt_contains_await) {
                    return true;
                }
            }
            false
        }
        StatementIR::MutationStmt { .. }
        | StatementIR::BreakStmtIR
        | StatementIR::ContinueStmtIR
        | StatementIR::EventEmitStmtIR { .. } => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Check if an ExprIR contains an `await` expression anywhere in its tree.
pub(super) fn expr_contains_await(expr: &ExprIR) -> bool {
    match expr {
        ExprIR::AwaitExpr { .. } => true,
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_contains_await(left) || expr_contains_await(right)
        }
        ExprIR::UnaryOpExpr { operand, .. } => expr_contains_await(operand),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_contains_await(cond)
                || expr_contains_await(then_expr)
                || expr_contains_await(else_expr)
        }
        ExprIR::CallExpr { args, .. } => args.iter().any(expr_contains_await),
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_contains_await(object) || args.iter().any(expr_contains_await)
        }
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            expr_contains_await(object)
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => call_args.iter().any(expr_contains_await),
        ExprIR::ArrayExpr { elements } => elements.iter().any(expr_contains_await),
        ExprIR::SpreadExpr { operand } => expr_contains_await(operand),
        ExprIR::BlockExpr { statements, tail } => {
            statements.iter().any(stmt_contains_await) || expr_contains_await(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_contains_await(object) || expr_contains_await(index)
        }
        ExprIR::ClosureExpr { body, .. } => expr_contains_await(body),
        ExprIR::MatchExpr { matched, arms } => {
            if expr_contains_await(matched) {
                return true;
            }
            arms.iter()
                .any(|arm| arm.body.statements.iter().any(stmt_contains_await))
        }
        ExprIR::VariantLiteral { fields, .. } => fields.iter().any(|(_, v)| expr_contains_await(v)),
        ExprIR::StructExpr { fields, .. } => fields.iter().any(|(_, v)| expr_contains_await(v)),
        ExprIR::LiteralExpr { .. } | ExprIR::IdentifierExpr { .. } => false,
        ExprIR::MapExpr { entries } => entries
            .iter()
            .any(|(k, v)| expr_contains_await(k) || expr_contains_await(v)),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => expr_contains_await(subject) || expr_contains_await(predicate),
        // Ref ops are synchronous DOM operations — no await inside.
        ExprIR::RefOpExpr { .. } => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// Motion/effect detection in view tree
// ═══════════════════════════════════════════════════════════

/// Check if a view tree has motion triggers or effects that need useEffect.
/// All trigger types (Enter, Hover, Press, InView, Scroll, DragHorizontal,
/// DragVertical) generate useEffect hooks. Also checks in_view_once/in_view_repeat,
/// scroll, and drag fields.
pub(super) fn view_has_motion_or_effects(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            // Check for motion behaviors that generate useEffect
            if let Some(ref motion) = el.motion {
                // All TriggerKind variants generate useEffect hooks
                if motion.triggers.iter().any(|_| true) {
                    return true;
                }
                // in_view_once / in_view_repeat generates useEffect
                if motion.in_view_once.is_some() || motion.in_view_repeat.is_some() {
                    return true;
                }
            }
            // Note: @scroll / @drag-* control dimensions now live on the
            // trigger kinds (Task 5.1), so the trigger check above already
            // covers them — no separate scroll/drag field scan needed.
            // Check children
            el.children.iter().any(view_has_motion_or_effects)
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_has_motion_or_effects))
                || cond
                    .fallback
                    .as_ref()
                    .is_some_and(view_has_motion_or_effects)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_has_motion_or_effects)
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(view_has_motion_or_effects)
        }
        ViewTree::Fragment(children) => children.iter().any(view_has_motion_or_effects),
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_motion_or_effects),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}
