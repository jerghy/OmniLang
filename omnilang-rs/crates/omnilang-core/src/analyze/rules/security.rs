use crate::analyze::diagnostic::{Diagnostic, Severity};
use crate::analyze::registry::{AnalyzeContext, AnalyzeRule};

macro_rules! sec_rule {
    ($name:ident, $code:expr, $msg:expr) => {
        pub struct $name;
        impl AnalyzeRule for $name {
            fn code(&self) -> &'static str { $code }
            fn name(&self) -> &'static str { stringify!($name) }
            fn severity(&self) -> Severity { Severity::Sec }
            fn category(&self) -> &'static str { "security" }
            fn message(&self) -> &'static str { $msg }
            fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
        }
    };
}

sec_rule!(SEC0101SqlInject, "SEC0101", "Potential SQL injection vulnerability");
sec_rule!(SEC0102HardcodedCredentials, "SEC0102", "Hardcoded credentials detected");
sec_rule!(SEC0103UnsafeEval, "SEC0103", "Unsafe dynamic evaluation detected");
