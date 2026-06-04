mod type_rules;
mod async_rules;
mod ownership;
mod portability;
mod performance;
mod style;
mod security;

pub use type_rules::*;
pub use async_rules::*;
pub use ownership::*;
pub use portability::*;
pub use performance::*;
pub use style::*;
pub use security::*;

use crate::analyze::registry::RuleRegistry;

/// Build the default registry with all 60 rules.
pub fn build_default_registry() -> RuleRegistry {
    let mut r = RuleRegistry::new();

    // Type errors (E01xx) - 16 rules
    r.register(type_rules::E0101TypeMismatch);
    r.register(type_rules::E0102MissingField);
    r.register(type_rules::E0103ExtraField);
    r.register(type_rules::E0104AnnotationNeeded);
    r.register(type_rules::E0105MatchExhaust);
    r.register(type_rules::E0106DeadMatchArm);
    r.register(type_rules::E0107RecursiveType);
    r.register(type_rules::E0108MethodNotFound);
    r.register(type_rules::E0109GenericConstraint);
    r.register(type_rules::E0110UnboundedGeneric);
    r.register(type_rules::E0111VariantCount);
    r.register(type_rules::E0310BorrowLifetime);
    r.register(type_rules::E0401MultipleInheritance);
    r.register(type_rules::E0402DynTraitOverhead);
    r.register(type_rules::E0403InterfaceImplCheck);
    r.register(type_rules::E0501ModuleCycle);

    // Async (E02xx) - 7 rules
    r.register(async_rules::E0201AwaitNonAsync);
    r.register(async_rules::E0202AsyncWithoutAwait);
    r.register(async_rules::E0203UnhandledAwait);
    r.register(async_rules::E0204AwaitInLoop);
    r.register(async_rules::E0205BlockingInAsync);
    r.register(async_rules::E0206MissingErrorHandling);
    r.register(async_rules::E0207MissingMainWrapper);

    // Ownership (E03xx) - 4 rules
    r.register(ownership::E0306ClosureEscape);
    r.register(ownership::E0307BorrowConflict);
    r.register(ownership::E0308CrossLangOwnership);
    r.register(ownership::E0309UseAfterMove);

    // Portability (P01xx) - 14 rules
    r.register(portability::P0101Idiv);
    r.register(portability::P0102FloatCmp);
    r.register(portability::P0103Nan);
    r.register(portability::P0104StringEnc);
    r.register(portability::P0105FloatPrecision);
    r.register(portability::P0106ClosureCapture);
    r.register(portability::P0107AsyncScheduling);
    r.register(portability::P0108PlatformApi);
    r.register(portability::P0109Overflow);
    r.register(portability::P0110Divzero);
    r.register(portability::P0111GenericsJs);
    r.register(portability::P0112FloatToIntLoss);
    r.register(portability::P0113RegexFlavor);
    r.register(portability::P0114NumberLiteral);

    // Performance (F01xx) - 7 rules
    r.register(performance::F0101LargeCopy);
    r.register(performance::F0102UnnecessaryAlloc);
    r.register(performance::F0103BoxUnbox);
    r.register(performance::F0104LoopInvariant);
    r.register(performance::F0105StringConcat);
    r.register(performance::F0106RangeCheck);
    r.register(performance::F0107UnnecessaryClone);

    // Style (S01xx) - 11 rules
    r.register(style::S0101UnusedVariable);
    r.register(style::S0102UnusedImport);
    r.register(style::S0103DeadCode);
    r.register(style::S0104ComplexExpression);
    r.register(style::S0105NonIdiomaticName);
    r.register(style::S0106MutableNotMutated);
    r.register(style::S0107Structural);
    r.register(style::S0108BackslashPath);
    r.register(style::S0109RedundantAnnotation);
    r.register(style::S0110TooManyParams);
    r.register(style::S0111TooManyLocals);

    // Security (SEC01xx) - 3 rules
    r.register(security::SEC0101SqlInject);
    r.register(security::SEC0102HardcodedCredentials);
    r.register(security::SEC0103UnsafeEval);

    r
}
