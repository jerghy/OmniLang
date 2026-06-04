use crate::analyze::diagnostic::{Diagnostic, Severity};
use crate::analyze::registry::{AnalyzeContext, AnalyzeRule};

macro_rules! ownership_rule {
    ($name:ident, $code:expr, $sev:expr, $msg:expr) => {
        pub struct $name;
        impl AnalyzeRule for $name {
            fn code(&self) -> &'static str { $code }
            fn name(&self) -> &'static str { stringify!($name) }
            fn severity(&self) -> Severity { $sev }
            fn category(&self) -> &'static str { "ownership" }
            fn message(&self) -> &'static str { $msg }
            fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
        }
    };
}

ownership_rule!(E0306ClosureEscape, "E0306", Severity::Warning, "Closure may escape its defining scope (Rust: needs Cell/RefCell)");
ownership_rule!(E0307BorrowConflict, "E0307", Severity::Error, "Borrow conflict detected (Rust target)");
ownership_rule!(E0308CrossLangOwnership, "E0308", Severity::Warning, "Cross-language ownership semantics differ");
ownership_rule!(E0309UseAfterMove, "E0309", Severity::Error, "Use after move (Rust target)");
