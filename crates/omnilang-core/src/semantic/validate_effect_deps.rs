//! Validate `@effect(deps)` declarations: the declared deps must cover every
//! store-field read in the effect body, else the effect silently never
//! re-fires when that field changes (the NotificationsPanel class of silent
//! bug — 13-expert review perf HIGH-3 sibling).
//!
//! Contract:
//! - Every `Store.field` read in the body (including reads inside closures,
//!   blocks, match arms, and nested statements) requires a declared dep of
//!   `Store.field` (qualified) or `field` (bare — the codegen resolves bare
//!   store-signal deps through its store signal map, so `@effect(items)`
//!   reading `TodoStore.items` is legal).
//! - Body reads of non-store identifiers (signals, props, computeds, locals)
//!   are NOT required in deps — computeds re-derive via their own memo.
//! - `Store.action(...)` calls are not reactive reads and never require deps.
//! - A no-arg `@effect { ... }` (empty deps) re-runs on every update and is
//!   not validated (nothing can go stale).
//! - `@unmount` cleanup bodies are walked through the same collector (task
//!   2.6): their store reads enter the analysis. Cleanups declare no deps —
//!   the React codegen DERIVES the unmount deps from the body reads
//!   (`useEffect(() => () => {...}, [count])`, react/component/effect.rs), so
//!   a cleanup read can never go stale and never raises E_EFFECT_DEPS_MISSING
//!   (auto-covered, mirroring the no-arg-effect exemption).
//!
//! Runs after component lowering (`ir/lower/mod.rs`), like
//! `validate_ref_ops` / `validate_semantic_bindings`.

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::walk_stmt;
use crate::ir::{ComponentIR, ExprIR, IRProgram, LifecycleKind, StatementIR};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Diagnostic message prefix for a missing effect dep — verbatim from the
/// semantic DSL v2 plan (域 5):「effect 依赖漏掉：Store.field」.
const MISSING_DEP_MSG_PREFIX: &str = "effect 依赖漏掉：";

/// Validate every `@effect(deps)` of a component against the store-field
/// reads in its body. Emits one `E_EFFECT_DEPS_MISSING` error per missing
/// dep (deterministic order — reads sorted by qualified name).
pub fn validate_effect_deps(
    component: &ComponentIR,
    program: &IRProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if component.effects.is_empty() && component.lifecycles.is_empty() {
        return;
    }

    // store_alias → field names (signals + getters). Action names are NOT
    // fields: `Store.action(...)` calls in an effect body are not reactive
    // reads and must not require deps.
    let store_fields: FxHashMap<&str, FxHashSet<&str>> = program
        .all_stores()
        .into_iter()
        .map(|(name, store)| {
            let mut fields = FxHashSet::default();
            fields.extend(store.signals.keys().map(String::as_str));
            fields.extend(store.getters.keys().map(String::as_str));
            (name, fields)
        })
        .collect();

    let (line, col) = component
        .source_span
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));

    for effect in &component.effects {
        // No-arg @effect (empty deps) re-runs on every update — nothing can
        // go stale, nothing to cover.
        if effect.deps.is_empty() {
            continue;
        }
        let reads = collect_store_field_reads(&effect.body.statements, &store_fields);

        for read in reads {
            // Matching rule (field-exact preferred): a declared dep covers the
            // read as `Store.field` (qualified) or as the bare field name
            // (`field` — resolves through the codegen store signal map).
            let field = read.rsplit_once('.').map(|(_, f)| f).unwrap_or(&read);
            let covered = effect.deps.iter().any(|d| d == &read || d == field);
            if !covered {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    kind: DiagnosticKind::EEffectDepsMissing,
                    message: format!("{}{}", MISSING_DEP_MSG_PREFIX, read),
                    file: component.source_file.clone(),
                    line,
                    col,
                    end_line: None,
                    end_col: None,
                    suggestion: Some(format!("在 @effect 依赖中加入：{}", read)),
                });
            }
        }
    }

    // `@unmount` cleanup bodies (task 2.6): walked through the shared
    // collector so their store reads enter the analysis. Cleanups declare no
    // deps — the React codegen derives them from the body reads, so a
    // cleanup read can never go stale and must NOT raise E_EFFECT_DEPS_MISSING
    // (auto-covered). The walk keeps the collection honest — a cleanup read
    // is indistinguishable from an effect-body read to the codegen, so any
    // future codegen regression (derived deps dropped) is caught by the
    // react-side tests, and the contract is pinned here.
    for lc in &component.lifecycles {
        if lc.kind == LifecycleKind::Unmount {
            let _ = collect_store_field_reads(&lc.body.statements, &store_fields);
        }
    }
}

/// Collect the qualified store-field reads `Store.field` from a statement
/// list, deduped and sorted (deterministic order).
///
/// Four expression shapes can carry a store-field read (mirrors the codegen
/// collectors in `lints/helpers.rs` / react store_slices.rs):
///   - `Store.field` → FieldAccessExpr { Ident(Store), field }
///   - `Store?.field` → OptionalChainExpr { Ident(Store), field }
///   - `Store.field(...)` method call → StoreFieldAccessExpr (actions are
///     filtered out below by the store-fields check)
///   - `Store.method(...)` → MethodCallExpr { Ident(Store), method }
///
/// Each candidate is verified against the store's fields (signals + getters)
/// — action names, locals, props, and computeds fail the check and never
/// count as reads. Reads inside closures, blocks, match arms, and nested
/// statements are all included (full statement walker).
fn collect_store_field_reads(
    stmts: &[StatementIR],
    store_fields: &FxHashMap<&str, FxHashSet<&str>>,
) -> Vec<String> {
    let mut reads: Vec<String> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for stmt in stmts {
        walk_stmt(stmt, &mut |expr| {
            let candidate: Option<(&str, &str)> = match expr {
                ExprIR::FieldAccessExpr { object, field } => match object.as_ref() {
                    ExprIR::IdentifierExpr { name, .. } => Some((name, field)),
                    _ => None,
                },
                ExprIR::OptionalChainExpr { object, field } => match object.as_ref() {
                    ExprIR::IdentifierExpr { name, .. } => Some((name, field)),
                    _ => None,
                },
                ExprIR::StoreFieldAccessExpr {
                    store_alias,
                    field_name,
                    ..
                } => Some((store_alias, field_name)),
                ExprIR::MethodCallExpr { object, method, .. } => match object.as_ref() {
                    ExprIR::IdentifierExpr { name, .. } => Some((name, method)),
                    _ => None,
                },
                _ => None,
            };
            if let Some((store, field)) = candidate {
                let is_field = store_fields
                    .get(store)
                    .is_some_and(|fields| fields.contains(field));
                if is_field {
                    let qualified = format!("{}.{}", store, field);
                    if seen.insert(qualified.clone()) {
                        reads.push(qualified);
                    }
                }
            }
        });
    }
    reads.sort();
    reads
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        BlockIR, EffectIntent, FunctionIR, LifecycleIntent, LifecycleKind, ModuleIR, StatementIR,
        StoreIR, TypeIR, TypeKind, Visibility,
    };
    use rustc_hash::FxHashMap;

    /// Minimal program with a single store: signals {notifications},
    /// getter {unreadCount}, action {clear}.
    fn store_program() -> IRProgram {
        let store = StoreIR {
            name: "NotificationStore".into(),
            signals: FxHashMap::from_iter([
                (
                    "notifications".into(),
                    TypeIR {
                        kind: TypeKind::Named {
                            name: "Array".into(),
                        },
                        type_args: vec![],
                        is_nullable: false,
                    },
                ),
                (
                    "maxCount".into(),
                    TypeIR {
                        kind: TypeKind::Int,
                        type_args: vec![],
                        is_nullable: false,
                    },
                ),
            ]),
            getters: FxHashMap::from_iter([(
                "unreadCount".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!(0),
                },
            )]),
            actions: vec![FunctionIR {
                name: "clear".into(),
                visibility: Visibility::Private,
                params: vec![],
                return_type: None,
                body: BlockIR { statements: vec![] },
                is_async: false,
                source_span: None,
            }],
            ..Default::default()
        };
        let module = ModuleIR {
            name: "main".into(),
            stores: FxHashMap::from_iter([("NotificationStore".into(), store)]),
            ..Default::default()
        };
        IRProgram {
            modules: FxHashMap::from_iter([("main".into(), module)]),
            ..Default::default()
        }
    }

    fn store_field_read(store: &str, field: &str) -> ExprIR {
        ExprIR::StoreFieldAccessExpr {
            store_alias: store.into(),
            field_name: field.into(),
            call_args: vec![],
        }
    }

    /// The real parser shape for `Store.field` reads: the parser lowers
    /// `NotificationStore.unreadCount` to `FieldAccessExpr { Ident, field }`.
    fn field_access_read(store: &str, field: &str) -> ExprIR {
        ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::ident(store)),
            field: field.into(),
        }
    }

    /// The parser shape for `Store.method(args)` calls.
    fn method_call(store: &str, method: &str) -> ExprIR {
        ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::ident(store)),
            method: method.into(),
            args: vec![],
            is_optional: false,
        }
    }

    fn make_component(effects: Vec<EffectIntent>) -> ComponentIR {
        ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "Test".into(),
            effects,
            ..ComponentIR::default()
        }
    }

    fn effect(deps: Vec<&str>, statements: Vec<StatementIR>) -> EffectIntent {
        EffectIntent {
            deps: deps.into_iter().map(String::from).collect(),
            body: BlockIR { statements },
        }
    }

    fn expr_stmt(expr: ExprIR) -> StatementIR {
        StatementIR::ExprStmtIR { expr }
    }

    fn run(component: &ComponentIR, program: &IRProgram) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        validate_effect_deps(component, program, &mut diags);
        diags
    }

    #[test]
    fn missing_dep_errors_with_verbatim_message() {
        // `@effect(notifications)` with a body reading
        // NotificationStore.unreadCount — the dep never covers the read.
        // Uses the real parser shape (FieldAccessExpr).
        let comp = make_component(vec![effect(
            vec!["notifications"],
            vec![expr_stmt(field_access_read(
                "NotificationStore",
                "unreadCount",
            ))],
        )]);
        let diags = run(&comp, &store_program());
        assert_eq!(
            diags.len(),
            1,
            "expected exactly one diagnostic: {:?}",
            diags
        );
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].kind, DiagnosticKind::EEffectDepsMissing);
        assert_eq!(
            diags[0].message, "effect 依赖漏掉：NotificationStore.unreadCount",
            "verbatim message required, got: {}",
            diags[0].message
        );
        assert!(
            diags[0]
                .suggestion
                .as_deref()
                .is_some_and(|s| s.contains("NotificationStore.unreadCount")),
            "suggestion must name the missing dep, got: {:?}",
            diags[0].suggestion
        );
    }

    #[test]
    fn qualified_dep_covers_read() {
        let comp = make_component(vec![effect(
            vec!["NotificationStore.unreadCount"],
            vec![expr_stmt(store_field_read(
                "NotificationStore",
                "unreadCount",
            ))],
        )]);
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
    }

    #[test]
    fn bare_field_dep_covers_read() {
        // `@effect(unreadCount)` — the codegen resolves the bare store-signal
        // dep through its store signal map (T3.3 slice contract), so the bare
        // form must count as coverage.
        let comp = make_component(vec![effect(
            vec!["unreadCount"],
            vec![expr_stmt(store_field_read(
                "NotificationStore",
                "unreadCount",
            ))],
        )]);
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
    }

    #[test]
    fn closure_internal_reads_are_counted() {
        // perf HIGH-3 blind spot: a store read that only happens inside a
        // closure body must still be counted as a body read.
        let closure = ExprIR::ClosureExpr {
            params: vec!["n".into()],
            body: Box::new(store_field_read("NotificationStore", "unreadCount")),
        };
        let comp = make_component(vec![effect(
            vec!["notifications"],
            vec![expr_stmt(closure)],
        )]);
        let diags = run(&comp, &store_program());
        assert_eq!(
            diags.len(),
            1,
            "expected exactly one diagnostic: {:?}",
            diags
        );
        assert_eq!(
            diags[0].message,
            "effect 依赖漏掉：NotificationStore.unreadCount"
        );
    }

    #[test]
    fn closure_internal_read_covered_by_qualified_dep_passes() {
        let closure = ExprIR::ClosureExpr {
            params: vec!["n".into()],
            body: Box::new(store_field_read("NotificationStore", "unreadCount")),
        };
        let comp = make_component(vec![effect(
            vec!["NotificationStore.unreadCount"],
            vec![expr_stmt(closure)],
        )]);
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
    }

    #[test]
    fn store_action_calls_are_not_reads() {
        // `NotificationStore.clear()` in a body is an action call, not a
        // reactive field read — it must not require a dep. Both lowering
        // shapes are covered: StoreFieldAccessExpr and MethodCallExpr.
        let comp = make_component(vec![effect(
            vec!["notifications"],
            vec![
                expr_stmt(store_field_read("NotificationStore", "clear")),
                expr_stmt(method_call("NotificationStore", "clear")),
            ],
        )]);
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
    }

    #[test]
    fn method_call_on_store_signal_requires_dep() {
        // `NotificationStore.notifications.len()` — the inner FieldAccessExpr
        // read is counted (the MethodCallExpr itself is not a field read).
        let read = ExprIR::MethodCallExpr {
            object: Box::new(field_access_read("NotificationStore", "notifications")),
            method: "len".into(),
            args: vec![],
            is_optional: false,
        };
        let comp = make_component(vec![effect(vec!["unreadCount"], vec![expr_stmt(read)])]);
        let diags = run(&comp, &store_program());
        let messages: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(
            messages,
            vec!["effect 依赖漏掉：NotificationStore.notifications"],
            "the field read nested under the method call must be counted, got: {:?}",
            messages
        );
    }

    #[test]
    fn optional_chain_read_is_counted() {
        let read = ExprIR::OptionalChainExpr {
            object: Box::new(ExprIR::ident("NotificationStore")),
            field: "unreadCount".into(),
        };
        let comp = make_component(vec![effect(vec!["notifications"], vec![expr_stmt(read)])]);
        let diags = run(&comp, &store_program());
        assert_eq!(
            diags.len(),
            1,
            "expected exactly one diagnostic: {:?}",
            diags
        );
        assert_eq!(
            diags[0].message,
            "effect 依赖漏掉：NotificationStore.unreadCount"
        );
    }

    #[test]
    fn non_store_object_fields_are_not_reads() {
        // `user.name` / `item.value` where the object is a local (not a
        // store) — never requires a dep.
        let comp = make_component(vec![effect(
            vec!["notifications"],
            vec![
                expr_stmt(ExprIR::FieldAccessExpr {
                    object: Box::new(ExprIR::ident("user")),
                    field: "name".into(),
                }),
                expr_stmt(ExprIR::MethodCallExpr {
                    object: Box::new(ExprIR::ident("list")),
                    method: "push".into(),
                    args: vec![],
                    is_optional: false,
                }),
            ],
        )]);
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
    }

    #[test]
    fn no_arg_effect_without_deps_is_skipped() {
        // `@effect { ... }` runs on every update — no deps to cover.
        let comp = make_component(vec![effect(
            vec![],
            vec![expr_stmt(store_field_read(
                "NotificationStore",
                "unreadCount",
            ))],
        )]);
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
    }

    #[test]
    fn component_without_effects_is_a_noop() {
        let comp = make_component(vec![]);
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty());
    }

    #[test]
    fn multiple_missing_deps_each_error_in_sorted_order() {
        let comp = make_component(vec![effect(
            vec!["notifications"],
            vec![
                expr_stmt(store_field_read("NotificationStore", "unreadCount")),
                expr_stmt(store_field_read("NotificationStore", "maxCount")),
                // duplicate read — must dedupe
                expr_stmt(store_field_read("NotificationStore", "maxCount")),
            ],
        )]);
        let diags = run(&comp, &store_program());
        let messages: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(
            messages,
            vec![
                "effect 依赖漏掉：NotificationStore.maxCount",
                "effect 依赖漏掉：NotificationStore.unreadCount",
            ],
            "one diagnostic per missing dep, deduped, sorted; got: {:?}",
            messages
        );
        assert!(
            diags
                .iter()
                .all(|d| d.severity == Severity::Error
                    && d.kind == DiagnosticKind::EEffectDepsMissing),
            "all missing-dep diagnostics must be E_EFFECT_DEPS_MISSING errors"
        );
    }

    #[test]
    fn read_inside_nested_statement_is_counted() {
        // A read inside an if condition/branch, let initializer, and mutation
        // value must all be counted (full statement walker).
        let read = store_field_read("NotificationStore", "unreadCount");
        let if_stmt = StatementIR::IfStmtIR {
            condition: read.clone(),
            then_body: BlockIR {
                statements: vec![expr_stmt(read.clone())],
            },
            else_ifs: vec![],
            else_body: None,
        };
        let let_stmt = StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(read.clone()),
            r#type: None,
            is_mut: false,
        };
        let comp = make_component(vec![effect(vec!["notifications"], vec![if_stmt, let_stmt])]);
        let diags = run(&comp, &store_program());
        assert_eq!(
            diags.len(),
            1,
            "expected exactly one diagnostic: {:?}",
            diags
        );
        assert_eq!(
            diags[0].message,
            "effect 依赖漏掉：NotificationStore.unreadCount"
        );
    }

    // ── Task 2.6: @unmount cleanup reads ────────────────────
    //
    // The `@unmount` cleanup closure has NO declared deps — the React codegen
    // derives them from the body reads (`}, [count]);`), so a cleanup store
    // read can never go stale and must NOT raise E_EFFECT_DEPS_MISSING. The
    // pass still descends into the cleanup body through the shared collector
    // (cleanup reads are part of the analysis; the codegen's derivation is
    // the enforcement point).

    fn store_fields_map(program: &IRProgram) -> FxHashMap<&str, FxHashSet<&str>> {
        program
            .all_stores()
            .into_iter()
            .map(|(name, store)| {
                let mut fields = FxHashSet::default();
                fields.extend(store.signals.keys().map(String::as_str));
                fields.extend(store.getters.keys().map(String::as_str));
                (name, fields)
            })
            .collect()
    }

    #[test]
    fn unmount_cleanup_reads_are_collected_and_auto_covered() {
        // The cleanup body reads NotificationStore.unreadCount with NO
        // declared deps anywhere — auto-covered by the codegen derivation,
        // so the pass must stay silent.
        let unmount = LifecycleIntent {
            kind: LifecycleKind::Unmount,
            body: BlockIR {
                statements: vec![expr_stmt(store_field_read(
                    "NotificationStore",
                    "unreadCount",
                ))],
            },
        };
        let comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "Test".into(),
            effects: vec![effect(vec!["notifications"], vec![])],
            lifecycles: vec![unmount.clone()],
            ..ComponentIR::default()
        };
        let diags = run(&comp, &store_program());
        assert!(
            diags.is_empty(),
            "cleanup reads are auto-covered by codegen-derived deps — got: {:?}",
            diags
        );
        // And the collector descends into the cleanup body — the read is
        // part of the analysis.
        let reads = collect_store_field_reads(
            &unmount.body.statements,
            &store_fields_map(&store_program()),
        );
        assert_eq!(
            reads,
            vec!["NotificationStore.unreadCount"],
            "cleanup body reads must be collected, got: {:?}",
            reads
        );
    }

    #[test]
    fn unmount_cleanup_closure_internal_reads_are_collected() {
        // Mirrors the @effect closure descent (perf HIGH-3 blind spot): a
        // store read that only happens inside a closure nested in the
        // cleanup body must still be collected.
        let closure = ExprIR::ClosureExpr {
            params: vec!["n".into()],
            body: Box::new(store_field_read("NotificationStore", "unreadCount")),
        };
        let unmount = LifecycleIntent {
            kind: LifecycleKind::Unmount,
            body: BlockIR {
                statements: vec![expr_stmt(closure)],
            },
        };
        let comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "Test".into(),
            lifecycles: vec![unmount.clone()],
            ..ComponentIR::default()
        };
        let diags = run(&comp, &store_program());
        assert!(
            diags.is_empty(),
            "cleanup reads are auto-covered — got: {:?}",
            diags
        );
        let reads = collect_store_field_reads(
            &unmount.body.statements,
            &store_fields_map(&store_program()),
        );
        assert_eq!(
            reads,
            vec!["NotificationStore.unreadCount"],
            "closure-internal cleanup reads must be collected, got: {:?}",
            reads
        );
    }

    #[test]
    fn unmount_cleanup_action_calls_are_not_reads() {
        // `NotificationStore.clear()` in a cleanup is an action call, not a
        // reactive field read — never a dep.
        let unmount = LifecycleIntent {
            kind: LifecycleKind::Unmount,
            body: BlockIR {
                statements: vec![
                    expr_stmt(store_field_read("NotificationStore", "clear")),
                    expr_stmt(method_call("NotificationStore", "clear")),
                ],
            },
        };
        let comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "Test".into(),
            lifecycles: vec![unmount.clone()],
            ..ComponentIR::default()
        };
        let diags = run(&comp, &store_program());
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        let reads = collect_store_field_reads(
            &unmount.body.statements,
            &store_fields_map(&store_program()),
        );
        assert!(
            reads.is_empty(),
            "action calls must not be collected as reads, got: {:?}",
            reads
        );
    }
}
