use crate::analyze::diagnostic::{Diagnostic, Severity};
use crate::analyze::registry::{AnalyzeContext, AnalyzeRule};
use crate::ast::*;

macro_rules! type_rule {
    ($name:ident, $code:expr, $sev:expr, $msg:expr) => {
        pub struct $name;
        impl AnalyzeRule for $name {
            fn code(&self) -> &'static str { $code }
            fn name(&self) -> &'static str { stringify!($name) }
            fn severity(&self) -> Severity { $sev }
            fn category(&self) -> &'static str { "type" }
            fn message(&self) -> &'static str { $msg }
            fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
        }
    };
}

type_rule!(E0101TypeMismatch, "E0101", Severity::Error, "Type mismatch");
type_rule!(E0102MissingField, "E0102", Severity::Error, "Missing required field");
type_rule!(E0103ExtraField, "E0103", Severity::Error, "Extra field in struct literal");
type_rule!(E0104AnnotationNeeded, "E0104", Severity::Error, "Type annotation needed");
type_rule!(E0106DeadMatchArm, "E0106", Severity::Warning, "Dead match arm");
type_rule!(E0107RecursiveType, "E0107", Severity::Error, "Recursive type without indirection");
type_rule!(E0108MethodNotFound, "E0108", Severity::Error, "Method not found on type");
type_rule!(E0109GenericConstraint, "E0109", Severity::Error, "Generic constraint not satisfied");
type_rule!(E0110UnboundedGeneric, "E0110", Severity::Warning, "Unbounded generic parameter");
type_rule!(E0111VariantCount, "E0111", Severity::Warning, "Too few variants in tagged union");
type_rule!(E0310BorrowLifetime, "E0310", Severity::Warning, "Borrow lifetime issue (Rust target)");
type_rule!(E0401MultipleInheritance, "E0401", Severity::Error, "Multiple inheritance not supported");
type_rule!(E0402DynTraitOverhead, "E0402", Severity::Warning, "dyn Trait has runtime overhead (Rust target)");
type_rule!(E0403InterfaceImplCheck, "E0403", Severity::Error, "Interface implementation incomplete");
type_rule!(E0501ModuleCycle, "E0501", Severity::Error, "Circular module dependency");

// E0105: Match exhaustiveness check
pub struct E0105MatchExhaust;
impl AnalyzeRule for E0105MatchExhaust {
    fn code(&self) -> &'static str { "E0105" }
    fn name(&self) -> &'static str { "match_exhaust" }
    fn severity(&self) -> Severity { Severity::Error }
    fn category(&self) -> &'static str { "type" }
    fn message(&self) -> &'static str { "Match may not be exhaustive" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        for func in &ctx.program.functions {
            if let Some(ref body) = func.body {
                for stmt in &body.stmts {
                    check_stmt_simple(stmt, &mut diags);
                }
                if let Some(ref trailing) = body.trailing_expr {
                    check_expr_simple(trailing, &mut diags);
                }
            }
        }
        diags
    }
}

fn check_stmt_simple(stmt: &Stmt, diags: &mut Vec<Diagnostic>) {
    match stmt {
        Stmt::VarDecl(vd) => check_expr_simple(&vd.init, diags),
        Stmt::Assign(a) => {
            check_expr_simple(&a.value, diags);
            if let Some(ref idx) = a.index { check_expr_simple(idx, diags); }
        }
        Stmt::IfStmt(s) => {
            check_expr_simple(&s.condition, diags);
            for inner in &s.then_body.stmts { check_stmt_simple(inner, diags); }
        }
        Stmt::PrintStmt(ps) => check_expr_simple(&ps.arg, diags),
        Stmt::ReturnStmt(rs) => { if let Some(ref v) = rs.value { check_expr_simple(v, diags); } }
        Stmt::ExprStmt(es) => check_expr_simple(&es.expr, diags),
        _ => {}
    }
}

fn check_expr_simple(expr: &Expr, diags: &mut Vec<Diagnostic>) {
    if let Expr::MatchExpr(me) = expr {
        let has_wildcard = me.arms.iter().any(|a| matches!(a.pattern, MatchPattern::Wildcard));
        if !has_wildcard && !me.arms.is_empty() {
            diags.push(Diagnostic::new("E0105", Severity::Error, "type", "Match may not be exhaustive")
                .with_help("Add a wildcard arm '_' or cover all variants"));
        }
    }
}
