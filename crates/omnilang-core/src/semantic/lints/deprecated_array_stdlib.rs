//! Warns on calls to the deprecated `array_filter` / `array_map` stdlib
//! functions — the language methods `subject.filter(...)` /
//! `subject.map(...)` (design doc 域 8, rewritten to `ExprIR::ArrayMethod`
//! by the array-method validator) are the one-name-per-intent replacement
//! (task 4.6 D-E prove verdict: corpus migrated, gates green, stdlib forms
//! kept only as a compat back-door).
//!
//! Category: style
//! Severity: warning

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::lexer::Span;
use crate::parser::ast::*;
use rustc_hash::FxHashMap;

/// Deprecated stdlib array functions and their language-method form.
/// `count` has no stdlib counterpart (the language method is the only name).
const DEPRECATED_FNS: [(&str, &str); 2] = [
    ("array_filter", "subject.filter(|x| ...)"),
    ("array_map", "subject.map(|x| ...)"),
];

pub struct DeprecatedArrayStdlib;

impl LintRule for DeprecatedArrayStdlib {
    fn name(&self) -> &str {
        "deprecated-array-stdlib"
    }
    fn category(&self) -> &str {
        "style"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check_ast(&self, program: &Program, _file_path: &str) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for module in program.modules.values() {
            for func in &module.functions {
                walk_block(&func.body, &mut diags);
            }
            for comp in &module.components {
                for sig in &comp.signals {
                    walk_expr(&sig.init, &mut diags);
                }
                for c in &comp.computeds {
                    walk_expr(&c.body, &mut diags);
                }
                for lc in &comp.lifecycles {
                    walk_block(&lc.body, &mut diags);
                }
                for func in &comp.fns {
                    walk_block(&func.body, &mut diags);
                }
                for action in &comp.actions {
                    walk_block(&action.body, &mut diags);
                }
                for getter in &comp.getters {
                    walk_expr(&getter.body, &mut diags);
                }
                if let Some(ref view) = comp.view {
                    for node in &view.children {
                        walk_view_node(node, &mut diags);
                    }
                }
            }
            for store in &module.stores {
                for sig in &store.signals {
                    walk_expr(&sig.init, &mut diags);
                }
                for getter in &store.getters {
                    walk_expr(&getter.body, &mut diags);
                }
                for action in &store.actions {
                    walk_block(&action.body, &mut diags);
                }
            }
        }
        diags
    }
}

/// Emit a deprecation diagnostic for an `array_filter`/`array_map` call.
fn check_call(func: &Expr, args: &[Expr], span: &Span, diags: &mut Vec<LintDiagnostic>) {
    let Expr::VarRef(name, _) = func else {
        return;
    };
    let Some((_, replacement)) = DEPRECATED_FNS.iter().find(|(n, _)| *n == name) else {
        return;
    };
    diags.push(LintDiagnostic {
        rule: "deprecated-array-stdlib".into(),
        severity: LintSeverity::Warning,
        message: format!(
            "`{name}` is deprecated — use the language method `{replacement}` instead (one name per intent)"
        ),
        file: String::new(),
        line: span.line,
        col: span.col,
    });
    // Keep walking the arguments — a nested call (e.g. inside the
    // predicate closure body) may hold another deprecated call.
    for arg in args {
        walk_expr(arg, diags);
    }
}

fn walk_block(block: &Block, diags: &mut Vec<LintDiagnostic>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::VarDecl { init: Some(e), .. } => walk_expr(e, diags),
            Stmt::VarDecl { init: None, .. } => {}
            Stmt::ExprStmt { expr: e, .. } => walk_expr(e, diags),
            Stmt::Assign { value, .. } => walk_expr(value, diags),
            Stmt::Return { value: Some(e), .. } => walk_expr(e, diags),
            Stmt::Return { .. } => {}
            Stmt::If {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                walk_expr(condition, diags);
                walk_block(then_body, diags);
                for (cond, body) in else_ifs {
                    walk_expr(cond, diags);
                    walk_block(body, diags);
                }
                if let Some(body) = else_body {
                    walk_block(body, diags);
                }
            }
            Stmt::While { condition, body, .. } => {
                walk_expr(condition, diags);
                walk_block(body, diags);
            }
            Stmt::ForIn { iterable, body, .. } => {
                walk_expr(iterable, diags);
                walk_block(body, diags);
            }
            Stmt::Print { arg, .. } => walk_expr(arg, diags),
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::EventEmit { args, .. } => {
                for arg in args {
                    walk_expr(arg, diags);
                }
            }
            Stmt::Throw { expr, .. } => walk_expr(expr, diags),
            Stmt::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                walk_block(try_body, diags);
                if let Some(b) = catch_body {
                    walk_block(b, diags);
                }
                if let Some(b) = finally_body {
                    walk_block(b, diags);
                }
            }
            Stmt::Error { .. } => {}
        }
    }
    if let Some(expr) = &block.trailing_expr {
        walk_expr(expr, diags);
    }
}

fn walk_view_node(node: &ViewNode, diags: &mut Vec<LintDiagnostic>) {
    match node {
        ViewNode::Element(el) => {
            for prop in &el.props {
                match prop {
                    ElementProp::Static { value, .. } => walk_expr(value, diags),
                    ElementProp::Style { entries } => {
                        for (_, value) in entries {
                            walk_expr(value, diags);
                        }
                    }
                    ElementProp::Class {
                        value: ClassValue::ClassMap(entries),
                    } => {
                        for (_, value) in entries {
                            walk_expr(value, diags);
                        }
                    }
                    ElementProp::Class { .. } => {}
                    ElementProp::Bind { .. } => {}
                    ElementProp::RefBinding { signal, .. } => walk_expr(signal, diags),
                    ElementProp::Event { handler, .. } => match handler {
                        EventHandler::InlineFn { body, .. } => walk_block(body, diags),
                        EventHandler::FnRef(_) | EventHandler::StoreAction { .. } => {}
                    },
                    ElementProp::SlotContent { content, .. } => {
                        for n in content {
                            walk_view_node(n, diags);
                        }
                    }
                    ElementProp::A11y(spec) => {
                        if let Some(hidden) = &spec.hidden {
                            walk_expr(hidden, diags);
                        }
                        if let Some(alt) = &spec.alt {
                            walk_expr(alt, diags);
                        }
                    }
                    ElementProp::MotionTrigger(trigger) => walk_expr(&trigger.motion_expr, diags),
                    ElementProp::Variant { .. } | ElementProp::Ref(_) => {}
                }
            }
            for child in &el.children {
                walk_view_node(child, diags);
            }
        }
        ViewNode::IfDirective(directive) => {
            walk_expr(&directive.condition, diags);
            for node in &directive.body {
                walk_view_node(node, diags);
            }
            for (cond, body) in &directive.else_ifs {
                walk_expr(cond, diags);
                for node in body {
                    walk_view_node(node, diags);
                }
            }
            if let Some(else_body) = &directive.else_body {
                for node in else_body {
                    walk_view_node(node, diags);
                }
            }
        }
        ViewNode::ForDirective(directive) => {
            walk_expr(&directive.iterable, diags);
            if let Some(key) = &directive.key_expr {
                walk_expr(key, diags);
            }
            for node in &directive.body {
                walk_view_node(node, diags);
            }
            if let Some(empty) = &directive.empty_body {
                for node in empty {
                    walk_view_node(node, diags);
                }
            }
        }
        ViewNode::SlotDirective { default_content, .. } => {
            if let Some(content) = default_content {
                for node in content {
                    walk_view_node(node, diags);
                }
            }
        }
        ViewNode::Text(_) => {}
        ViewNode::Interpolation(expr) => walk_expr(expr, diags),
    }
}

fn walk_expr(expr: &Expr, diags: &mut Vec<LintDiagnostic>) {
    match expr {
        Expr::FuncCall { func, args, span } => check_call(func, args, span, diags),
        Expr::Literal(..) | Expr::VarRef(..) => {}
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, diags);
            walk_expr(right, diags);
        }
        Expr::UnaryOp { operand, .. } => walk_expr(operand, diags),
        Expr::MethodCall { object, args, .. } => {
            walk_expr(object, diags);
            for a in args {
                walk_expr(a, diags);
            }
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            walk_expr(object, diags);
        }
        Expr::IndexGet { object, index, .. } => {
            walk_expr(object, diags);
            walk_expr(index, diags);
        }
        Expr::ArrayLiteral { elements, .. } => {
            for e in elements {
                walk_expr(e, diags);
            }
        }
        Expr::MapLiteral { entries, .. } => {
            for (k, v) in entries {
                walk_expr(k, diags);
                walk_expr(v, diags);
            }
        }
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        }
        | Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            walk_expr(condition, diags);
            walk_expr(then_expr, diags);
            walk_expr(else_expr, diags);
        }
        Expr::TemplateString { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    walk_expr(e, diags);
                }
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, val) in fields {
                walk_expr(val, diags);
            }
        }
        Expr::StoreFieldAccess { .. } => {}
        Expr::Block { body, .. } => walk_block(body, diags),
        Expr::ClosureExpr { body, .. } => walk_expr(body, diags),
        Expr::AwaitExpr { operand, .. } => walk_expr(operand, diags),
        Expr::MatchExpr { matched, arms, .. } => {
            walk_expr(matched, diags);
            for arm in arms {
                walk_block(&arm.body, diags);
            }
        }
        Expr::IsCheck { .. } => {}
        Expr::SpreadExpr { operand, .. } => walk_expr(operand, diags),
        Expr::NotYetImplemented { .. } | Expr::Error { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Span;
    use rustc_hash::FxHashMap;

    fn span(line: u32, col: u32) -> Span {
        Span {
            start: 0,
            end: 1,
            line,
            col,
        }
    }

    fn call_expr(name: &str, args: Vec<Expr>) -> Expr {
        call_expr_at(name, args, 1, 1)
    }

    fn call_expr_at(name: &str, args: Vec<Expr>, line: u32, col: u32) -> Expr {
        Expr::FuncCall {
            func: Box::new(Expr::VarRef(name.into(), span(line, col))),
            args,
            span: span(line, col),
        }
    }

    fn ident(name: &str) -> Expr {
        Expr::VarRef(name.into(), span(1, 1))
    }

    fn closure_expr(predicate: Expr) -> Expr {
        Expr::ClosureExpr {
            params: vec!["x".into()],
            body: Box::new(predicate),
            span: span(2, 20),
        }
    }

    fn program_with_component_fn(name: &str, body: Vec<Stmt>) -> Program {
        let mut modules = FxHashMap::default();
        modules.insert(
            "test.omni".to_string(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                components: vec![ComponentDef {
                    name: "Test".into(),
                    props: vec![],
                    signals: vec![],
                    computeds: vec![],
                    events: vec![],
                    refs: vec![],
                    store_refs: vec![],
                    inline_stores: vec![],
                    lifecycles: vec![],
                    fns: vec![FunctionDef {
                        visibility: Visibility::Private,
                        name: name.into(),
                        params: vec![],
                        return_type: None,
                        is_async: false,
                        body: Block {
                            stmts: body,
                            trailing_expr: None,
                            span: span(1, 1),
                        },
                        span: span(1, 1),
                    }],
                    actions: vec![],
                    getters: vec![],
                    motions: vec![],
                    slots: vec![],
                    view: None,
                    styles: vec![],
                    span: span(1, 1),
                }],
                stores: vec![],
                designs: vec![],
                imports: vec![],
                exports: vec![],
                motions: vec![],
                consts: vec![],
            },
        );
        Program { modules }
    }

    #[test]
    fn warns_on_array_filter_call() {
        let program = program_with_component_fn(
            "run",
            vec![Stmt::ExprStmt {
                expr: call_expr_at(
                    "array_filter",
                    vec![ident("items"), closure_expr(ident("x.done"))],
                    2,
                    10,
                ),
                span: span(2, 10),
            }],
        );
        let diags = DeprecatedArrayStdlib.check_ast(&program, "test.omni");
        assert_eq!(diags.len(), 1, "expected 1 warning: {:?}", diags);
        assert!(diags[0].message.contains("array_filter"));
        assert!(diags[0].message.contains("filter(|x| ...)"));
        assert_eq!(diags[0].severity, LintSeverity::Warning);
        assert_eq!(diags[0].line, 2);
    }

    #[test]
    fn warns_on_array_map_call() {
        let program = program_with_component_fn(
            "run",
            vec![Stmt::ExprStmt {
                expr: call_expr_at("array_map", vec![ident("items"), ident("f")], 3, 5),
                span: span(3, 5),
            }],
        );
        let diags = DeprecatedArrayStdlib.check_ast(&program, "test.omni");
        assert_eq!(diags.len(), 1, "expected 1 warning: {:?}", diags);
        assert!(diags[0].message.contains("array_map"));
        assert!(diags[0].message.contains("map(|x| ...)"));
        assert_eq!(diags[0].line, 3);
    }

    #[test]
    fn does_not_warn_on_other_stdlib_or_language_methods() {
        // array_length (no language counterpart), array_sort, array_find
        // (no dual name), and the language method form itself must stay
        // silent.
        let program = program_with_component_fn(
            "run",
            vec![
                Stmt::ExprStmt {
                    expr: call_expr("array_length", vec![ident("items")]),
                    span: span(1, 1),
                },
                Stmt::ExprStmt {
                    expr: call_expr("array_sort", vec![ident("items"), ident("cmp")]),
                    span: span(1, 1),
                },
                Stmt::ExprStmt {
                    expr: Expr::MethodCall {
                        object: Box::new(ident("items")),
                        method: "filter".into(),
                        args: vec![closure_expr(ident("x.done"))],
                        span: span(1, 1),
                    },
                    span: span(1, 1),
                },
                Stmt::ExprStmt {
                    expr: call_expr("array_find", vec![ident("items"), ident("f")]),
                    span: span(1, 1),
                },
            ],
        );
        let diags = DeprecatedArrayStdlib.check_ast(&program, "test.omni");
        assert!(diags.is_empty(), "expected no warnings: {:?}", diags);
    }

    #[test]
    fn warns_on_nested_call_in_predicate() {
        // array_filter(items, |x| array_map(xs, |y| y)) — the inner call in
        // the closure body is found too.
        let program = program_with_component_fn(
            "run",
            vec![Stmt::ExprStmt {
                expr: call_expr(
                    "array_filter",
                    vec![ident("items"), closure_expr(call_expr("array_map", vec![ident("xs"), ident("f")]))],
                ),
                span: span(1, 1),
            }],
        );
        let diags = DeprecatedArrayStdlib.check_ast(&program, "test.omni");
        assert_eq!(diags.len(), 2, "expected 2 warnings: {:?}", diags);
    }
}
