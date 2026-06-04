use crate::analyze::diagnostic::{Diagnostic, Severity, SourceLocation};
use crate::analyze::registry::{AnalyzeContext, AnalyzeRule};
use crate::analyze::walker;
use crate::ast::*;

macro_rules! async_rule {
    ($name:ident, $code:expr, $sev:expr, $msg:expr) => {
        pub struct $name;
        impl AnalyzeRule for $name {
            fn code(&self) -> &'static str { $code }
            fn name(&self) -> &'static str { stringify!($name) }
            fn severity(&self) -> Severity { $sev }
            fn category(&self) -> &'static str { "async" }
            fn message(&self) -> &'static str { $msg }
            fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
        }
    };
}

async_rule!(E0203UnhandledAwait, "E0203", Severity::Warning, "Unhandled await result");
async_rule!(E0204AwaitInLoop, "E0204", Severity::Warning, "Await in loop may cause sequential execution");
async_rule!(E0205BlockingInAsync, "E0205", Severity::Warning, "Blocking call in async function");
async_rule!(E0206MissingErrorHandling, "E0206", Severity::Warning, "Missing error handling for async operation");
async_rule!(E0207MissingMainWrapper, "E0207", Severity::Error, "Async main missing runtime wrapper");

// E0201: await in non-async function
pub struct E0201AwaitNonAsync;
impl AnalyzeRule for E0201AwaitNonAsync {
    fn code(&self) -> &'static str { "E0201" }
    fn name(&self) -> &'static str { "await_non_async" }
    fn severity(&self) -> Severity { Severity::Error }
    fn category(&self) -> &'static str { "async" }
    fn message(&self) -> &'static str { "await used outside async function" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        for func in &ctx.program.functions {
            if !func.is_async {
                if let Some(ref body) = func.body {
                    walker::walk_block(body, &mut |expr| {
                        if let Expr::AwaitExpr(ae) = expr {
                            diags.push(Diagnostic::new("E0201", Severity::Error, "async", "await used in non-async function")
                                .with_location(SourceLocation::from(&ae.span))
                                .with_help("Mark the function as 'async fn' or remove the await"));
                        }
                    });
                }
            }
        }
        diags
    }
}

// E0202: async function without await
pub struct E0202AsyncWithoutAwait;
impl AnalyzeRule for E0202AsyncWithoutAwait {
    fn code(&self) -> &'static str { "E0202" }
    fn name(&self) -> &'static str { "async_without_await" }
    fn severity(&self) -> Severity { Severity::Warning }
    fn category(&self) -> &'static str { "async" }
    fn message(&self) -> &'static str { "Async function has no await expression" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        for func in &ctx.program.functions {
            if func.is_async {
                let mut has_await = false;
                if let Some(ref body) = func.body {
                    walker::walk_block(body, &mut |expr| {
                        if matches!(expr, Expr::AwaitExpr(_)) { has_await = true; }
                    });
                }
                if !has_await {
                    diags.push(Diagnostic::new("E0202", Severity::Warning, "async", "Async function has no await")
                        .with_location(SourceLocation::from(&func.span))
                        .with_help("Remove 'async' or add an await expression"));
                }
            }
        }
        diags
    }
}
