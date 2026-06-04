use crate::analyze::diagnostic::{Diagnostic, Severity, SourceLocation};
use crate::analyze::registry::{AnalyzeContext, AnalyzeRule};
use crate::analyze::walker;
use crate::ast::*;

macro_rules! port_rule {
    ($name:ident, $code:expr, $msg:expr) => {
        pub struct $name;
        impl AnalyzeRule for $name {
            fn code(&self) -> &'static str { $code }
            fn name(&self) -> &'static str { stringify!($name) }
            fn severity(&self) -> Severity { Severity::Port }
            fn category(&self) -> &'static str { "portability" }
            fn message(&self) -> &'static str { $msg }
            fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
        }
    };
}

port_rule!(P0103Nan, "P0103", "NaN handling differs across targets");
port_rule!(P0104StringEnc, "P0104", "String encoding differs across targets (UTF-16 vs UTF-8)");
port_rule!(P0105FloatPrecision, "P0105", "Floating-point precision may differ across targets");
port_rule!(P0107AsyncScheduling, "P0107", "Async scheduling order differs across targets");
port_rule!(P0108PlatformApi, "P0108", "Platform API not available on all targets");
port_rule!(P0112FloatToIntLoss, "P0112", "Float-to-int conversion may lose precision");
port_rule!(P0113RegexFlavor, "P0113", "Regex flavor differs across targets");
port_rule!(P0114NumberLiteral, "P0114", "Number literal may overflow in some targets");

// P0101: Integer division behavior differs across targets
pub struct P0101Idiv;
impl AnalyzeRule for P0101Idiv {
    fn code(&self) -> &'static str { "P0101" }
    fn name(&self) -> &'static str { "cross_lang_div_behavior" }
    fn severity(&self) -> Severity { Severity::Port }
    fn category(&self) -> &'static str { "portability" }
    fn message(&self) -> &'static str { "Integer division behavior differs across targets" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        walker::walk_program(ctx.program, &mut |expr| {
            if let Expr::BinaryOp(b) = expr {
                if matches!(b.op, BinaryOpKind::Div | BinaryOpKind::Mod) {
                    if let Some(ty) = ctx.type_map.get(&b.left.node_id()) {
                        if ty.base_name() == "i32" {
                            diags.push(Diagnostic::new("P0101", Severity::Port, "portability",
                                "Integer division (/ or %) may behave differently: JS returns float, Python floors, Dart/Rust truncate")
                                .with_location(SourceLocation::from(&b.span))
                                .with_help("Use _idiv()/_irem() from omni/math for consistent truncating behavior"));
                        }
                    }
                }
            }
        });
        diags
    }
}

// P0102: Float comparison differs across targets
pub struct P0102FloatCmp;
impl AnalyzeRule for P0102FloatCmp {
    fn code(&self) -> &'static str { "P0102" }
    fn name(&self) -> &'static str { "float_cmp" }
    fn severity(&self) -> Severity { Severity::Port }
    fn category(&self) -> &'static str { "portability" }
    fn message(&self) -> &'static str { "Float comparison may differ across targets" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        walker::walk_program(ctx.program, &mut |expr| {
            if let Expr::BinaryOp(b) = expr {
                if matches!(b.op, BinaryOpKind::Eq | BinaryOpKind::Ne | BinaryOpKind::Lt | BinaryOpKind::Gt | BinaryOpKind::Le | BinaryOpKind::Ge) {
                    if let Some(ty) = ctx.type_map.get(&b.left.node_id()) {
                        if ty.base_name() == "f64" {
                            diags.push(Diagnostic::new("P0102", Severity::Port, "portability",
                                "Float comparison may differ due to IEEE754 implementation variance")
                                .with_location(SourceLocation::from(&b.span)));
                        }
                    }
                }
            }
        });
        diags
    }
}

// P0106: Closure capture semantics differ
pub struct P0106ClosureCapture;
impl AnalyzeRule for P0106ClosureCapture {
    fn code(&self) -> &'static str { "P0106" }
    fn name(&self) -> &'static str { "closure_capture" }
    fn severity(&self) -> Severity { Severity::Port }
    fn category(&self) -> &'static str { "portability" }
    fn message(&self) -> &'static str { "Closure capture semantics differ across targets" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        walker::walk_program(ctx.program, &mut |expr| {
            if let Expr::ClosureExpr(ce) = expr {
                // Check if any captured variables are mutated
                if !ce.params.is_empty() {
                    diags.push(Diagnostic::new("P0106", Severity::Port, "portability",
                        "Closure capture: JS/Dart/Python use reference capture; Rust uses move+Cell/RefCell")
                        .with_location(SourceLocation::from(&ce.span)));
                }
            }
        });
        diags
    }
}

// P0109: Integer overflow differs
pub struct P0109Overflow;
impl AnalyzeRule for P0109Overflow {
    fn code(&self) -> &'static str { "P0109" }
    fn name(&self) -> &'static str { "overflow" }
    fn severity(&self) -> Severity { Severity::Port }
    fn category(&self) -> &'static str { "portability" }
    fn message(&self) -> &'static str { "Integer overflow behavior differs across targets" }
    fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
}

// P0110: Division by zero behavior differs
pub struct P0110Divzero;
impl AnalyzeRule for P0110Divzero {
    fn code(&self) -> &'static str { "P0110" }
    fn name(&self) -> &'static str { "divzero" }
    fn severity(&self) -> Severity { Severity::Port }
    fn category(&self) -> &'static str { "portability" }
    fn message(&self) -> &'static str { "Division by zero: JS returns Infinity, others throw" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic> {
        let mut diags = vec![];
        walker::walk_program(ctx.program, &mut |expr| {
            if let Expr::BinaryOp(b) = expr {
                if matches!(b.op, BinaryOpKind::Div | BinaryOpKind::Mod) {
                    if let Expr::Literal(lit) = &*b.right {
                        if matches!(lit.value, LiteralValue::Int(0)) {
                            diags.push(Diagnostic::new("P0110", Severity::Port, "portability",
                                "Division by literal zero: JS returns Infinity, others throw exception")
                                .with_location(SourceLocation::from(&b.span)));
                        }
                    }
                }
            }
        });
        diags
    }
}

// P0111: JS generics erased
pub struct P0111GenericsJs;
impl AnalyzeRule for P0111GenericsJs {
    fn code(&self) -> &'static str { "P0111" }
    fn name(&self) -> &'static str { "generics_js" }
    fn severity(&self) -> Severity { Severity::Port }
    fn category(&self) -> &'static str { "portability" }
    fn message(&self) -> &'static str { "Generics are erased in JS plain mode" }
    fn check(&self, _ctx: &AnalyzeContext) -> Vec<Diagnostic> { vec![] }
}
