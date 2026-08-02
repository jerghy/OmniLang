//! Validate array language methods: rewrite legal `subject.filter(|x| ...)`
//! / `subject.map(|x| ...)` / `subject.count(|x| ...)` method calls to
//! `ExprIR::ArrayMethod` and reject malformed uses with diagnostics.
//!
//! Runs after component lowering (`ir/lower/mod.rs`), right after
//! `validate_ref_ops`. For each module, every expression in module-level
//! functions and in each component's view tree, effects, lifecycles,
//! computeds, and functions is scanned for `MethodCallExpr` whose method
//! name is one of the reserved array methods (`filter`, `map`, `count`).
//!
//! Rules (design doc 域 8, `array_method ::= expr '.' 'filter' '(' closure ')'`):
//!
//! - Exactly one argument → rewritten to `ExprIR::ArrayMethod { method,
//!   subject, predicate }`, so codegen backends can emit target-appropriate
//!   code (JS `filter`/`map`/`filter().length`; Dart `where(...).toList()` /
//!   `map(...).toList()` / `where(...).length`). The argument need not be a
//!   closure literal — a function reference is a valid predicate (matches
//!   the `array_filter`/`array_map` stdlib semantics).
//! - `count()` with no argument → error 「count 需要一个谓词闭包」 (design
//!   doc diagnostic table; Chinese message per the v2 policy recorded in the
//!   SDD ledger — design-doc diagnostics are emitted verbatim).
//! - A subject whose declared type is provably NOT an array (typed signal /
//!   param / local) → error. When the type cannot be proven (no annotation,
//!   struct types, computed expressions), the check is skipped — the type
//!   system today cannot prove or disprove array-ness there.
//! - A subject that names a declared element ref is left untouched — the
//!   ref-op validator owns `ref.*` dispatch and reports unknown ref
//!   operations (`ref.filter(...)` stays its error).
//! - `filter`/`map` with wrong arity (0 or >1 args) are left verbatim (no
//!   design-doc diagnostic; behavior unchanged from the unknown-method
//!   fallback).

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ArrayMethodIR, BlockIR, EventHandlerIR, ExprIR, ModuleIR, StatementIR, ViewTree};

/// The three reserved array language method names.
const ARRAY_METHODS: [&str; 3] = ["filter", "map", "count"];

/// True when `type_ann` provably names a non-array type (builtin scalar).
/// `Array<...>` / struct / unknown annotations return false (can't prove).
fn type_is_definitely_not_array(type_ann: &str) -> bool {
    let t = type_ann.trim();
    !t.starts_with("Array<")
        && !t.ends_with("[]")
        && matches!(
            t,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "int"
                | "Int"
                | "integer"
                | "isize"
                | "usize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "f32"
                | "f64"
                | "float"
                | "Float"
                | "num"
                | "number"
                | "String"
                | "str"
                | "string"
                | "bool"
                | "Bool"
                | "boolean"
                | "void"
                | "Void"
        )
}

/// Per-component context threaded through the walkers.
struct ArrayMethodCtx<'a> {
    /// Declared ref names — `ref.filter(...)` belongs to ref-op validation.
    ref_names: &'a [String],
    diagnostics: &'a mut Vec<Diagnostic>,
    file: Option<String>,
    line: u32,
    col: u32,
}

impl ArrayMethodCtx<'_> {
    /// Emit a compile error diagnostic at the component's source location.
    fn error(&mut self, message: String) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            kind: DiagnosticKind::EArrayMethod,
            message,
            file: self.file.clone(),
            line: self.line,
            col: self.col,
            end_line: None,
            end_col: None,
            suggestion: None,
        });
    }
}

/// Validate array methods for a single function body, rewriting legal
/// `subject.filter/map/count(predicate)` method calls to
/// `ExprIR::ArrayMethod` in place and pushing diagnostics for malformed
/// uses. `ref_names` are the declared refs of the enclosing component
/// (module-level functions have none) — ref subjects are left for the
/// ref-op validator.
fn validate_function_body(
    body: &mut BlockIR,
    ref_names: &[String],
    file: Option<String>,
    line: u32,
    col: u32,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut ctx = ArrayMethodCtx {
        ref_names,
        diagnostics,
        file,
        line,
        col,
    };
    walk_block_exprs_mut(body, &mut ctx);
}

/// Validate array methods for a module, rewriting legal
/// `subject.filter/map/count(predicate)` method calls to
/// `ExprIR::ArrayMethod` in place and pushing diagnostics for malformed
/// uses. Covers module-level functions AND all component expression
/// positions (view tree, effects, lifecycles, computeds, functions).
pub fn validate_array_methods(module: &mut ModuleIR, diagnostics: &mut Vec<Diagnostic>) {
    // Module-level functions (e.g. `pub fn run() { items.filter(...) }` on
    // script-style programs compiled to plain JS/Dart).
    for func in module.functions.values_mut() {
        let (line, col) = func.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
        validate_function_body(&mut func.body, &[], None, line, col, diagnostics);
    }

    // Store getters and actions (e.g. TodoStore.filteredItems getter with
    // `items.filter(|item| ...)` — the design doc §6.3 example).
    for store in module.stores.values_mut() {
        let (line, col) = store.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
        for getter in store.getters.values_mut() {
            let mut ctx = ArrayMethodCtx {
                ref_names: &[],
                diagnostics,
                file: store.source_file.clone(),
                line,
                col,
            };
            walk_expr_mut(getter, &mut ctx);
        }
        for action in &mut store.actions {
            validate_function_body(
                &mut action.body,
                &[],
                store.source_file.clone(),
                line,
                col,
                diagnostics,
            );
        }
    }

    // Component expressions — ref subjects keep their ref-op diagnostics.
    for component in module.components.values_mut() {
        let ref_names: Vec<String> = component.refs.iter().map(|r| r.name.clone()).collect();
        let (line, col) = component
            .source_span
            .map(|s| (s.line, s.col))
            .unwrap_or((0, 0));

        // Walk view tree expressions
        if let Some(ref mut view) = component.view {
            let mut ctx = ArrayMethodCtx {
                ref_names: &ref_names,
                diagnostics,
                file: component.source_file.clone(),
                line,
                col,
            };
            walk_view_tree_exprs_mut(view, &mut ctx);
        }

        // Walk expressions in effects
        for effect in &mut component.effects {
            validate_function_body(
                &mut effect.body,
                &ref_names,
                component.source_file.clone(),
                line,
                col,
                diagnostics,
            );
        }

        // Walk expressions in lifecycle hooks (mount/unmount bodies)
        for lifecycle in &mut component.lifecycles {
            validate_function_body(
                &mut lifecycle.body,
                &ref_names,
                component.source_file.clone(),
                line,
                col,
                diagnostics,
            );
        }

        // Walk expressions in computed properties
        for computed in &mut component.computeds {
            let mut ctx = ArrayMethodCtx {
                ref_names: &ref_names,
                diagnostics,
                file: component.source_file.clone(),
                line,
                col,
            };
            walk_expr_mut(&mut computed.body, &mut ctx);
        }

        // Walk expressions in component functions
        for func in &mut component.fns {
            validate_function_body(
                &mut func.body,
                &ref_names,
                component.source_file.clone(),
                line,
                col,
                diagnostics,
            );
        }
    }
}

/// Check a single expression for array-method calls, rewriting them to
/// `ExprIR::ArrayMethod` when well-formed.
fn check_expr(expr: &mut ExprIR, ctx: &mut ArrayMethodCtx) {
    let ExprIR::MethodCallExpr {
        object,
        method,
        args,
        is_optional,
    } = expr
    else {
        return;
    };
    if *is_optional || !ARRAY_METHODS.contains(&method.as_str()) {
        return;
    }
    // `ref.filter(...)` on a declared element ref belongs to ref-op
    // validation (unknown ref operation) — leave it untouched.
    let subject_is_ref = matches!(
        object.as_ref(),
        ExprIR::IdentifierExpr { name, .. } if ctx.ref_names.contains(name)
    );
    if subject_is_ref {
        return;
    }

    // Subject type check: a provably non-array subject is a type error.
    if let ExprIR::IdentifierExpr { name, type_ann, .. } = object.as_ref() {
        if type_ann
            .as_deref()
            .is_some_and(type_is_definitely_not_array)
        {
            ctx.error(format!(
                "{method} can only be called on arrays — '{name}' has type '{}'",
                type_ann.as_deref().expect("checked above")
            ));
            return;
        }
    }

    match args.len() {
        1 => {
            let predicate = args.pop().expect("one arg");
            // Move the subject out of its &mut borrow via a placeholder
            // replacement (the whole node is rewritten immediately after).
            let subject = std::mem::replace(
                object,
                Box::new(ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }),
            );
            *expr = ExprIR::ArrayMethod {
                method: match method.as_str() {
                    "map" => ArrayMethodIR::Map,
                    "count" => ArrayMethodIR::Count,
                    _ => ArrayMethodIR::Filter,
                },
                subject,
                predicate: Box::new(predicate),
            };
        }
        0 if method == "count" => {
            // Design doc diagnostic table: 「count 需要一个谓词闭包」
            // (v2 policy: design-doc diagnostics are emitted verbatim).
            ctx.error("count 需要一个谓词闭包".to_string());
        }
        _ => {
            // filter/map/count with wrong arity (or filter/map with no args)
            // stay verbatim — no design-doc diagnostic for these.
        }
    }
}

/// Recursively walk an expression tree, checking each node for array
/// methods. Called after `check_expr` so a rewritten `ArrayMethod` has its
/// subject/predicate expressions walked (nested array methods are legal).
fn walk_expr_mut(expr: &mut ExprIR, ctx: &mut ArrayMethodCtx) {
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
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            walk_expr_mut(subject, ctx);
            walk_expr_mut(predicate, ctx);
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
        ExprIR::ClosureExpr { params: _, body } => walk_expr_mut(body, ctx),
        ExprIR::AwaitExpr { operand } => walk_expr_mut(operand, ctx),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr_mut(field_expr, ctx);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr_mut(matched, ctx);
            for arm in arms {
                walk_block_exprs_mut(&mut arm.body, ctx);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (key, value) in entries {
                walk_expr_mut(key, ctx);
                walk_expr_mut(value, ctx);
            }
        }
        ExprIR::RefOpExpr { .. } => {}
    }
}

/// Walk every expression reachable from a statement.
fn walk_stmt_exprs_mut(stmt: &mut StatementIR, ctx: &mut ArrayMethodCtx) {
    match stmt {
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(value) = value {
                walk_expr_mut(value, ctx);
            }
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
            if let Some(intent) = intent {
                walk_mutation_exprs_mut(intent, ctx);
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                walk_expr_mut(arg, ctx);
            }
        }
        StatementIR::ExprStmtIR { expr } => walk_expr_mut(expr, ctx),
        StatementIR::BlockStmtIR { body } => walk_block_exprs_mut(body, ctx),
        StatementIR::WhileStmtIR { condition, body } => {
            walk_expr_mut(condition, ctx);
            walk_block_exprs_mut(body, ctx);
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            walk_expr_mut(iterable, ctx);
            walk_block_exprs_mut(body, ctx);
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        StatementIR::ThrowStmtIR { expr } => walk_expr_mut(expr, ctx),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            walk_block_exprs_mut(try_body, ctx);
            if let Some(catch_body) = catch_body {
                walk_block_exprs_mut(catch_body, ctx);
            }
            if let Some(finally_body) = finally_body {
                walk_block_exprs_mut(finally_body, ctx);
            }
        }
    }
}

/// Walk every expression reachable from a block.
fn walk_block_exprs_mut(block: &mut BlockIR, ctx: &mut ArrayMethodCtx) {
    for stmt in &mut block.statements {
        walk_stmt_exprs_mut(stmt, ctx);
    }
}

/// Walk the expression fields of a mutation intent.
fn walk_mutation_exprs_mut(intent: &mut crate::ir::MutationIntent, ctx: &mut ArrayMethodCtx) {
    match intent {
        crate::ir::MutationIntent::DirectAssign {
            value: Some(value), ..
        } => walk_expr_mut(value, ctx),
        crate::ir::MutationIntent::DirectAssign { value: None, .. } => {}
        crate::ir::MutationIntent::Increment {
            amount: Some(amount),
            ..
        }
        | crate::ir::MutationIntent::Decrement {
            amount: Some(amount),
            ..
        }
        | crate::ir::MutationIntent::Multiply {
            amount: Some(amount),
            ..
        }
        | crate::ir::MutationIntent::Divide {
            amount: Some(amount),
            ..
        }
        | crate::ir::MutationIntent::Modulo {
            amount: Some(amount),
            ..
        } => walk_expr_mut(amount, ctx),
        crate::ir::MutationIntent::Increment { .. }
        | crate::ir::MutationIntent::Decrement { .. }
        | crate::ir::MutationIntent::Multiply { .. }
        | crate::ir::MutationIntent::Divide { .. }
        | crate::ir::MutationIntent::Modulo { .. } => {}
        crate::ir::MutationIntent::Push {
            item: Some(item), ..
        } => walk_expr_mut(item, ctx),
        crate::ir::MutationIntent::Push { item: None, .. } => {}
        crate::ir::MutationIntent::Pop { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Recursively walk all ViewTree nodes and their expression fields.
fn walk_view_tree_exprs_mut(tree: &mut ViewTree, ctx: &mut ArrayMethodCtx) {
    match tree {
        ViewTree::Element(el) => {
            for expr in el.props.values_mut() {
                walk_expr_mut(expr, ctx);
            }
            for expr in el.class_map_bindings.values_mut() {
                walk_expr_mut(expr, ctx);
            }
            for handler in el.events.values_mut() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    walk_block_exprs_mut(body, ctx);
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
            walk_expr_mut(&mut list.iterable, ctx);
            if let Some(key_expr) = &mut list.key_expr {
                walk_expr_mut(key_expr, ctx);
            }
            if let Some(body) = &mut list.body {
                walk_view_tree_exprs_mut(body, ctx);
            }
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
    use crate::ir::{ComponentIR, FunctionIR, RefDeclIR, Visibility};
    use rustc_hash::FxHashMap;

    /// Helper: component with no declared refs.
    fn make_component() -> ComponentIR {
        ComponentIR {
            name: "Test".into(),
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
            source_span: None,
        });
    }

    fn method_call(object: &str, method: &str, args: Vec<ExprIR>) -> ExprIR {
        ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: object.into(),
                type_ann: Some("Array<Todo>".into()),
            }),
            method: method.to_string(),
            args,
            is_optional: false,
        }
    }

    fn closure(predicate: ExprIR) -> ExprIR {
        ExprIR::ClosureExpr {
            params: vec!["i".into()],
            body: Box::new(predicate),
        }
    }

    fn ident(name: &str) -> ExprIR {
        ExprIR::IdentifierExpr {
            name: name.into(),
            type_ann: None,
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
        let mut module = ModuleIR {
            name: "test".into(),
            components: FxHashMap::from_iter([("Test".to_string(), comp.clone())]),
            ..Default::default()
        };
        validate_array_methods(&mut module, &mut diagnostics);
        *comp = module.components.remove("Test").expect("component present");
        diagnostics
    }

    /// Run the pass over a module-level function (script-style programs).
    fn run_pass_module_fn(func: FunctionIR) -> (Vec<Diagnostic>, FunctionIR) {
        let mut diagnostics = Vec::new();
        let mut module = ModuleIR {
            name: "test".into(),
            functions: FxHashMap::from_iter([("run".to_string(), func)]),
            ..Default::default()
        };
        validate_array_methods(&mut module, &mut diagnostics);
        let func = module.functions.remove("run").expect("fn present");
        (diagnostics, func)
    }

    fn array_method_expr(method: ArrayMethodIR, subject: ExprIR, predicate: ExprIR) -> ExprIR {
        ExprIR::ArrayMethod {
            method,
            subject: Box::new(subject),
            predicate: Box::new(predicate),
        }
    }

    #[test]
    fn rewrites_filter_with_closure_to_array_method() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call("items", "filter", vec![closure(ident("i.done"))]),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::ArrayMethod {
                method,
                subject,
                predicate,
            } => {
                assert_eq!(*method, ArrayMethodIR::Filter);
                assert!(
                    matches!(subject.as_ref(), ExprIR::IdentifierExpr { name, .. } if name == "items")
                );
                assert!(matches!(predicate.as_ref(), ExprIR::ClosureExpr { .. }));
            }
            other => panic!("expected ArrayMethod, got: {:?}", other),
        }
    }

    #[test]
    fn rewrites_map_and_count_to_array_method() {
        for (name, expected) in [("map", ArrayMethodIR::Map), ("count", ArrayMethodIR::Count)] {
            let mut comp = make_component();
            push_expr_stmt(
                &mut comp,
                method_call("items", name, vec![closure(ident("i.done"))]),
            );
            let diags = run_pass(&mut comp);
            assert!(
                diags.is_empty(),
                "{name}: expected no diagnostics: {:?}",
                diags
            );
            match stmt_expr(&comp) {
                ExprIR::ArrayMethod { method, .. } => assert_eq!(*method, expected),
                other => panic!("{name}: expected ArrayMethod, got: {:?}", other),
            }
        }
    }

    #[test]
    fn function_reference_predicate_is_rewritten() {
        // items.filter(isDone) — a function reference is a valid predicate
        // (matches array_filter stdlib semantics), not just closures.
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call("items", "filter", vec![ident("isDone")]),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::ArrayMethod { predicate, .. } => {
                assert!(
                    matches!(predicate.as_ref(), ExprIR::IdentifierExpr { name, .. } if name == "isDone")
                );
            }
            other => panic!("expected ArrayMethod, got: {:?}", other),
        }
    }

    #[test]
    fn count_without_predicate_is_an_error() {
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("items", "count", vec![]));
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "count 需要一个谓词闭包");
        assert_eq!(diags[0].kind, DiagnosticKind::EArrayMethod);
        // The call stays a method call (nothing rewritten).
        assert!(matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }));
    }

    #[test]
    fn filter_without_predicate_stays_verbatim() {
        // No design-doc diagnostic for filter/map arity — unchanged behavior.
        let mut comp = make_component();
        push_expr_stmt(&mut comp, method_call("items", "filter", vec![]));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }));
    }

    #[test]
    fn non_array_subject_with_filter_is_an_error() {
        // signal count: i32 — .filter on a provably non-array subject.
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::MethodCallExpr {
                object: Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                    type_ann: Some("i32".into()),
                }),
                method: "filter".into(),
                args: vec![closure(ident("i.done"))],
                is_optional: false,
            },
        );
        let diags = run_pass(&mut comp);
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("can only be called on arrays"),
            "message: {}",
            diags[0].message
        );
        assert!(diags[0].message.contains("'count'"));
        // Not rewritten — stays a method call.
        assert!(matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }));
    }

    #[test]
    fn array_typed_subject_is_accepted() {
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call("items", "filter", vec![closure(ident("i.done"))]),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
    }

    #[test]
    fn untyped_subject_is_rewritten_without_type_check() {
        // No type annotation → the type system can't prove array-ness —
        // rewrite without a diagnostic (documented skip).
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::MethodCallExpr {
                object: Box::new(ExprIR::IdentifierExpr {
                    name: "xs".into(),
                    type_ann: None,
                }),
                method: "filter".into(),
                args: vec![closure(ident("x.done"))],
                is_optional: false,
            },
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(stmt_expr(&comp), ExprIR::ArrayMethod { .. }));
    }

    #[test]
    fn declared_ref_subject_is_left_for_ref_validation() {
        let mut comp = ComponentIR {
            name: "Test".into(),
            refs: vec![RefDeclIR {
                name: "box".into(),
                element_tag: None,
            }],
            ..ComponentIR::default()
        };
        push_expr_stmt(
            &mut comp,
            method_call("box", "filter", vec![closure(ident("i.done"))]),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(
            matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }),
            "ref subject must stay a method call for ref-op validation"
        );
    }

    #[test]
    fn optional_chain_filter_is_not_rewritten() {
        let mut comp = make_component();
        comp.fns.push(FunctionIR {
            name: "run".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: ExprIR::MethodCallExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "items".into(),
                            type_ann: Some("Array<Todo>".into()),
                        }),
                        method: "filter".into(),
                        args: vec![closure(ident("i.done"))],
                        is_optional: true,
                    },
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
    fn nested_array_methods_are_walked() {
        // items.filter(|i| others.map(|j| j.id).contains(i)) — inner method
        // calls inside the predicate body must be rewritten too.
        let inner = method_call("others", "map", vec![closure(ident("j.id"))]);
        let contains = ExprIR::MethodCallExpr {
            object: Box::new(inner),
            method: "contains".into(),
            args: vec![ident("i")],
            is_optional: false,
        };
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            method_call("items", "filter", vec![closure(contains)]),
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::ArrayMethod { predicate, .. } => match predicate.as_ref() {
                ExprIR::ClosureExpr { body, .. } => match body.as_ref() {
                    ExprIR::MethodCallExpr { object, .. } => {
                        assert!(
                            matches!(object.as_ref(), ExprIR::ArrayMethod { method, .. } if *method == ArrayMethodIR::Map),
                            "inner map must be rewritten, got: {:?}",
                            object
                        );
                    }
                    other => panic!("expected MethodCallExpr contains, got: {:?}", other),
                },
                other => panic!("expected ClosureExpr predicate, got: {:?}", other),
            },
            other => panic!("expected ArrayMethod, got: {:?}", other),
        }
    }

    #[test]
    fn rewrites_array_method_inside_computed_and_view() {
        // Computed bodies and view interpolations are both covered.
        let mut comp = make_component();
        comp.computeds.push(crate::ir::ComputedIntent {
            name: "evens".into(),
            r#type: crate::ir::TypeIR {
                kind: "Array".into(),
                type_args: vec![],
                is_nullable: false,
            },
            body: method_call("items", "filter", vec![closure(ident("i.done"))]),
            deps: vec![],
        });
        comp.view = Some(ViewTree::Interpolation(Box::new(method_call(
            "items",
            "count",
            vec![closure(ident("i.done"))],
        ))));
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        assert!(matches!(
            &comp.computeds[0].body,
            ExprIR::ArrayMethod {
                method: ArrayMethodIR::Filter,
                ..
            }
        ));
        assert!(matches!(
            comp.view.as_ref().unwrap(),
            ViewTree::Interpolation(expr)
                if matches!(expr.as_ref(), ExprIR::ArrayMethod { method: ArrayMethodIR::Count, .. })
        ));
    }

    #[test]
    fn non_array_method_calls_are_untouched() {
        // Regular methods (len, push, to_string, custom) stay method calls.
        for method in ["len", "push", "to_string", "customOp"] {
            let mut comp = make_component();
            push_expr_stmt(&mut comp, method_call("items", method, vec![]));
            let diags = run_pass(&mut comp);
            assert!(
                diags.is_empty(),
                "{method}: expected no diagnostics: {:?}",
                diags
            );
            assert!(
                matches!(stmt_expr(&comp), ExprIR::MethodCallExpr { .. }),
                "{method}: must stay a method call"
            );
        }
    }

    #[test]
    fn rewrites_array_methods_in_store_getters_and_actions() {
        // Design doc §6.3: `getter filteredItems = items.filter(...)` and
        // `items = items.map(...)` inside store actions.
        let store = crate::ir::StoreIR {
            name: "TodoStore".into(),
            source_file: Some("stores/TodoStore.omni".into()),
            getters: FxHashMap::from_iter([(
                "filteredItems".into(),
                method_call("items", "filter", vec![closure(ident("item.done"))]),
            )]),
            actions: vec![FunctionIR {
                name: "removeCompleted".into(),
                visibility: Visibility::Public,
                params: vec![],
                return_type: None,
                body: BlockIR {
                    statements: vec![StatementIR::LetStmtIR {
                        name: "items".into(),
                        value: Some(method_call("items", "map", vec![closure(ident("item.id"))])),
                        r#type: None,
                    }],
                },
                is_async: false,
                source_span: None,
            }],
            ..Default::default()
        };
        let mut diagnostics = Vec::new();
        let mut module = ModuleIR {
            name: "test".into(),
            stores: FxHashMap::from_iter([("TodoStore".to_string(), store.clone())]),
            ..Default::default()
        };
        validate_array_methods(&mut module, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics: {:?}",
            diagnostics
        );
        let store = module.stores.get("TodoStore").expect("store present");
        assert!(
            matches!(
                store.getters.get("filteredItems"),
                Some(ExprIR::ArrayMethod {
                    method: ArrayMethodIR::Filter,
                    ..
                })
            ),
            "store getter filter must be rewritten, got: {:?}",
            store.getters.get("filteredItems")
        );
        assert!(
            matches!(
                &store.actions[0].body.statements[0],
                StatementIR::LetStmtIR {
                    value: Some(ExprIR::ArrayMethod {
                        method: ArrayMethodIR::Map,
                        ..
                    }),
                    ..
                }
            ),
            "store action map must be rewritten, got: {:?}",
            store.actions[0].body.statements
        );
    }

    #[test]
    fn rewrites_array_methods_in_module_level_functions() {
        // Script-style programs: `pub fn run() { return items.filter(...); }`
        // at module level (no component) — the JavaScript runtime path.
        let func = FunctionIR {
            name: "run".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::ReturnStmtIR {
                    value: Some(method_call(
                        "items",
                        "count",
                        vec![closure(ident("i.done"))],
                    )),
                }],
            },
            is_async: false,
            source_span: None,
        };
        let (diags, out) = run_pass_module_fn(func);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match &out.body.statements[0] {
            StatementIR::ReturnStmtIR { value: Some(expr) } => {
                assert!(
                    matches!(
                        expr,
                        ExprIR::ArrayMethod {
                            method: ArrayMethodIR::Count,
                            ..
                        }
                    ),
                    "module fn count must be rewritten, got: {:?}",
                    expr
                );
            }
            other => panic!("expected ReturnStmtIR, got: {:?}", other),
        }
    }

    #[test]
    fn rewrites_array_method_expression_within_larger_expression() {
        // `items.filter(...).len()` — ArrayMethod nested as a MethodCallExpr
        // object; the outer call survives untouched.
        let mut comp = make_component();
        push_expr_stmt(
            &mut comp,
            ExprIR::MethodCallExpr {
                object: Box::new(method_call(
                    "items",
                    "filter",
                    vec![closure(ident("i.done"))],
                )),
                method: "len".into(),
                args: vec![],
                is_optional: false,
            },
        );
        let diags = run_pass(&mut comp);
        assert!(diags.is_empty(), "expected no diagnostics: {:?}", diags);
        match stmt_expr(&comp) {
            ExprIR::MethodCallExpr { object, method, .. } => {
                assert_eq!(method, "len");
                assert!(
                    matches!(
                        object.as_ref(),
                        ExprIR::ArrayMethod {
                            method: ArrayMethodIR::Filter,
                            ..
                        }
                    ),
                    "inner filter must be rewritten, got: {:?}",
                    object
                );
            }
            other => panic!("expected outer MethodCallExpr len, got: {:?}", other),
        }
    }

    #[test]
    fn array_method_ir_serde_roundtrip() {
        let expr = array_method_expr(
            ArrayMethodIR::Count,
            ident("items"),
            closure(ident("i.done")),
        );
        let json = serde_json::to_value(&expr).unwrap();
        let back: ExprIR = serde_json::from_value(json).unwrap();
        assert_eq!(
            format!("{back:?}"),
            format!("{expr:?}"),
            "serde roundtrip must preserve ArrayMethod"
        );
    }
}
