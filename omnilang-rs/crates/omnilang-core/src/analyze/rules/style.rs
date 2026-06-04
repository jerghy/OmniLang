use crate::analyze::diagnostic::{Diagnostic, Severity, SourceLocation};
use crate::analyze::registry::{AnalyzeContext, AnalyzeRule};
use crate::ast::*;
use std::collections::HashSet;

macro_rules! style_rule {
    ($name:ident, $code:expr, $msg:expr) => {
        pub struct $name;
        impl AnalyzeRule for $name {
            fn code(&self) -> &'static str { $code }
            fn name(&self) -> &'static str { stringify!($name) }
            fn severity(&self) -> Severity { Severity::Style }
            fn category(&self) -> &'static str { "style" }
            fn message(&self) -> &'static str { $msg }
            fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
        }
    };
}

style_rule!(S0103DeadCode, "S0103", "Dead code detected");
style_rule!(S0104ComplexExpression, "S0104", "Expression can be simplified");
style_rule!(S0105NonIdiomaticName, "S0105", "Name does not follow OmniLang conventions");
style_rule!(S0106MutableNotMutated, "S0106", "Variable declared 'mut' but never mutated");
style_rule!(S0109RedundantAnnotation, "S0109", "Redundant type annotation");
style_rule!(S0110TooManyParams, "S0110", "Function has too many parameters");
style_rule!(S0111TooManyLocals, "S0111", "Function has too many local variables");

// S0101: Unused variable
pub struct S0101UnusedVariable;
impl AnalyzeRule for S0101UnusedVariable {
    fn code(&self) -> &'static str { "S0101" }
    fn name(&self) -> &'static str { "unused_variable" }
    fn severity(&self) -> Severity { Severity::Style }
    fn category(&self) -> &'static str { "style" }
    fn message(&self) -> &'static str { "Unused variable" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        for func in &ctx.program.functions {
            if let Some(ref body) = func.body {
                let mut declared = vec![];
                let mut used = HashSet::new();
                for stmt in &body.stmts {
                    if let Stmt::VarDecl(vd) = stmt {
                        declared.push((vd.name.clone(), vd.span.clone()));
                    }
                    collect_var_refs_stmt(stmt, &mut used);
                }
                for (name, span) in &declared {
                    if !used.contains(name) && name != "_" {
                        diags.push(Diagnostic::new("S0101", Severity::Style, "style",
                            &format!("Unused variable '{}'", name))
                            .with_location(SourceLocation::from(span)));
                    }
                }
            }
        }
        diags
    }
}

fn collect_var_refs_stmt(stmt: &Stmt, used: &mut HashSet<String>) {
    match stmt {
        Stmt::VarDecl(vd) => collect_var_refs_expr(&vd.init, used),
        Stmt::Assign(a) => { collect_var_refs_expr(&a.value, used); if let Some(ref idx) = a.index { collect_var_refs_expr(idx, used); } }
        Stmt::IfStmt(s) => {
            collect_var_refs_expr(&s.condition, used);
            for inner in &s.then_body.stmts { collect_var_refs_stmt(inner, used); }
            for (c, b) in &s.else_ifs { collect_var_refs_expr(c, used); for inner in &b.stmts { collect_var_refs_stmt(inner, used); } }
            if let Some(ref b) = s.else_body { for inner in &b.stmts { collect_var_refs_stmt(inner, used); } }
        }
        Stmt::WhileStmt(s) => { collect_var_refs_expr(&s.condition, used); for inner in &s.body.stmts { collect_var_refs_stmt(inner, used); } }
        Stmt::ForInStmt(s) => { collect_var_refs_expr(&s.iterable, used); for inner in &s.body.stmts { collect_var_refs_stmt(inner, used); } }
        Stmt::ReturnStmt(s) => { if let Some(ref v) = s.value { collect_var_refs_expr(v, used); } }
        Stmt::PrintStmt(s) => collect_var_refs_expr(&s.arg, used),
        Stmt::ExprStmt(s) => collect_var_refs_expr(&s.expr, used),
        _ => {}
    }
}

fn collect_var_refs_expr(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::VarRef(vr) => { used.insert(vr.name.clone()); }
        Expr::BinaryOp(b) => { collect_var_refs_expr(&b.left, used); collect_var_refs_expr(&b.right, used); }
        Expr::UnaryOp(u) => collect_var_refs_expr(&u.operand, used),
        Expr::FuncCall(fc) => { for a in &fc.args { collect_var_refs_expr(a, used); } }
        Expr::MethodCall(mc) => { collect_var_refs_expr(&mc.object, used); for a in &mc.args { collect_var_refs_expr(a, used); } }
        Expr::IndexGet(ig) => { collect_var_refs_expr(&ig.object, used); collect_var_refs_expr(&ig.index, used); }
        _ => {}
    }
}

// S0102: Unused import
pub struct S0102UnusedImport;
impl AnalyzeRule for S0102UnusedImport {
    fn code(&self) -> &'static str { "S0102" }
    fn name(&self) -> &'static str { "unused_import" }
    fn severity(&self) -> Severity { Severity::Style }
    fn category(&self) -> &'static str { "style" }
    fn message(&self) -> &'static str { "Unused import" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        // Collect all used function/type names
        let mut used_names = HashSet::new();
        for func in &ctx.program.functions {
            if let Some(ref body) = func.body {
                for stmt in &body.stmts { collect_names_in_stmt(stmt, &mut used_names); }
            }
        }
        let mut diags = vec![];
        for imp in &ctx.program.imports {
            for (name, _) in &imp.names {
                if !used_names.contains(name) {
                    diags.push(Diagnostic::new("S0102", Severity::Style, "style",
                        &format!("Unused import '{}'", name)));
                }
            }
        }
        diags
    }
}

fn collect_names_in_stmt(stmt: &Stmt, used: &mut HashSet<String>) {
    match stmt {
        Stmt::VarDecl(vd) => collect_names_in_expr(&vd.init, used),
        Stmt::Assign(a) => { collect_names_in_expr(&a.value, used); if let Some(ref idx) = a.index { collect_names_in_expr(idx, used); } }
        Stmt::PrintStmt(s) => collect_names_in_expr(&s.arg, used),
        Stmt::ExprStmt(s) => collect_names_in_expr(&s.expr, used),
        Stmt::ReturnStmt(s) => { if let Some(ref v) = s.value { collect_names_in_expr(v, used); } }
        _ => {}
    }
}

fn collect_names_in_expr(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::VarRef(vr) => { used.insert(vr.name.clone()); }
        Expr::FuncCall(fc) => { if let Expr::VarRef(vr) = &*fc.func { used.insert(vr.name.clone()); } }
        Expr::StructLiteral(sl) => { used.insert(sl.type_name.clone()); }
        _ => {}
    }
}

// S0107: Structural type coincidence (nominal types accidentally compatible)
pub struct S0107Structural;
impl AnalyzeRule for S0107Structural {
    fn code(&self) -> &'static str { "S0107" }
    fn name(&self) -> &'static str { "structural_coincidence" }
    fn severity(&self) -> Severity { Severity::Style }
    fn category(&self) -> &'static str { "style" }
    fn message(&self) -> &'static str { "Structurally compatible types may be accidentally interchangeable" }
    fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
}

// S0108: Backslash in import path
pub struct S0108BackslashPath;
impl AnalyzeRule for S0108BackslashPath {
    fn code(&self) -> &'static str { "S0108" }
    fn name(&self) -> &'static str { "backslash_path" }
    fn severity(&self) -> Severity { Severity::Style }
    fn category(&self) -> &'static str { "style" }
    fn message(&self) -> &'static str { "Use forward slash in import paths" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        ctx.program.imports.iter().filter(|imp| imp.path.contains('\\')).map(|imp| {
            Diagnostic::new("S0108", Severity::Style, "style", "Import path uses backslash; use forward slash for cross-platform compatibility")
                .with_location(SourceLocation::from(&imp.span))
        }).collect()
    }
}
