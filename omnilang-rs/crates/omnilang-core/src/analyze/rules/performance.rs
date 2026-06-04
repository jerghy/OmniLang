use crate::analyze::diagnostic::{Diagnostic, Severity};
use crate::analyze::registry::{AnalyzeContext, AnalyzeRule};

macro_rules! perf_rule {
    ($name:ident, $code:expr, $msg:expr) => {
        pub struct $name;
        impl AnalyzeRule for $name {
            fn code(&self) -> &'static str { $code }
            fn name(&self) -> &'static str { stringify!($name) }
            fn severity(&self) -> Severity { Severity::Perf }
            fn category(&self) -> &'static str { "performance" }
            fn message(&self) -> &'static str { $msg }
            fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
        }
    };
}

perf_rule!(F0101LargeCopy, "F0101", "Unnecessary large copy of container");
perf_rule!(F0102UnnecessaryAlloc, "F0102", "Unnecessary allocation detected");
perf_rule!(F0103BoxUnbox, "F0103", "Unnecessary boxing/unboxing (Rust target)");
perf_rule!(F0104LoopInvariant, "F0104", "Loop-invariant expression can be hoisted");
perf_rule!(F0105StringConcat, "F0105", "String concatenation in loop; consider using Array.join or StringBuilder");
perf_rule!(F0106RangeCheck, "F0106", "Redundant range check; index already bounds-checked");
perf_rule!(F0107UnnecessaryClone, "F0107", "Unnecessary clone; use reference instead");
