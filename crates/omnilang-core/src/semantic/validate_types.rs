//! Expression type inference and checking.
//!
//! Bottom-up type inference for expressions. Runs during the resolve pass
//! to detect type errors (binary ops, assignments, calls, returns) at
//! compile time. Uses [`SemanticType`] as the internal type representation.

use super::symbols::SymbolTable;
use super::types::SemanticType;
use crate::parser::ast::*;

/// Infer the type of an expression by bottom-up traversal.
/// Returns [`SemanticType::Any`] when inference fails (unknown variable, etc.);
/// callers should treat Any as a non-error silence.
pub fn infer_expr_type(expr: &Expr, symbols: &SymbolTable, scope: usize) -> SemanticType {
    match expr {
        Expr::Literal(lit, _) => match lit {
            Literal::Int(_) => SemanticType::Int,
            Literal::Float(_) => SemanticType::Float,
            Literal::String(_) => SemanticType::String,
            Literal::Bool(_) => SemanticType::Bool,
            Literal::Null => SemanticType::Null,
        },
        Expr::VarRef(name, _) => symbols
            .resolve(name)
            .and_then(|s| s.type_ann.as_ref())
            .map(|t| SemanticType::parse_type_string(t))
            .unwrap_or(SemanticType::Any),
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            let lhs = infer_expr_type(left, symbols, scope);
            let rhs = infer_expr_type(right, symbols, scope);
            binary_op_result_type(op, &lhs, &rhs)
        }
        Expr::UnaryOp { op, operand, .. } => match op {
            UnaryOpKind::Not => SemanticType::Bool,
            UnaryOpKind::Neg => {
                let inner = infer_expr_type(operand, symbols, scope);
                match inner {
                    SemanticType::Int | SemanticType::Float => inner,
                    _ => SemanticType::Any,
                }
            }
            UnaryOpKind::Await => infer_expr_type(operand, symbols, scope),
        },
        Expr::FuncCall { func, args: _, .. } => {
            if let Expr::VarRef(name, _) = func.as_ref() {
                if let Some(sym) = symbols.resolve(name) {
                    if let Some(ref ret_str) = sym.type_ann {
                        return SemanticType::parse_type_string(ret_str);
                    }
                }
            }
            SemanticType::Any
        }
        Expr::MethodCall { .. } => SemanticType::Any,
        Expr::IndexGet { .. } => SemanticType::Any,
        Expr::FieldAccess { .. } => SemanticType::Any,
        Expr::OptionalChain { .. } => SemanticType::Any,
        Expr::ArrayLiteral { elements, .. } => {
            if elements.is_empty() {
                SemanticType::Array(Box::new(SemanticType::Any))
            } else {
                let inner = infer_expr_type(&elements[0], symbols, scope);
                SemanticType::Array(Box::new(inner))
            }
        }
        Expr::MapLiteral { .. } => {
            SemanticType::Map(Box::new(SemanticType::String), Box::new(SemanticType::Any))
        }
        Expr::IfExpr {
            then_expr,
            else_expr,
            ..
        } => {
            let t = infer_expr_type(then_expr, symbols, scope);
            let e = infer_expr_type(else_expr, symbols, scope);
            if t == e {
                t
            } else {
                SemanticType::Any
            }
        }
        Expr::TernaryExpr {
            then_expr,
            else_expr,
            ..
        } => {
            let t = infer_expr_type(then_expr, symbols, scope);
            let e = infer_expr_type(else_expr, symbols, scope);
            if t == e {
                t
            } else {
                SemanticType::Any
            }
        }
        Expr::TemplateString { .. } => SemanticType::String,
        Expr::StructLiteral { .. } => SemanticType::Any,
        Expr::StoreFieldAccess { .. } => SemanticType::Any,
        Expr::Block { body, .. } => body
            .trailing_expr
            .as_ref()
            .map(|e| infer_expr_type(e, symbols, scope))
            .unwrap_or(SemanticType::Void),
        Expr::ClosureExpr { .. } => SemanticType::Any,
        Expr::AwaitExpr { operand, .. } => infer_expr_type(operand, symbols, scope),
        Expr::MatchExpr { arms, .. } => arms
            .first()
            .map(|arm| infer_block_type(&arm.body, symbols, scope))
            .unwrap_or(SemanticType::Void),
        Expr::SpreadExpr { operand, .. } => infer_expr_type(operand, symbols, scope),
        Expr::IsCheck { .. } => SemanticType::Bool,
        Expr::NotYetImplemented { .. } => SemanticType::Any,
        Expr::Error { .. } => SemanticType::Any,
    }
}

/// Infer the type of a block (its trailing expression, or Void if none).
fn infer_block_type(block: &Block, symbols: &SymbolTable, scope: usize) -> SemanticType {
    block
        .trailing_expr
        .as_ref()
        .map(|e| infer_expr_type(e, symbols, scope))
        .unwrap_or(SemanticType::Void)
}

/// Compute the result type of a binary operation.
/// Returns the result type on success, or an error message describing the mismatch.
pub fn binary_op_result_type(
    op: &BinaryOpKind,
    lhs: &SemanticType,
    rhs: &SemanticType,
) -> SemanticType {
    // Avoid cascading errors: Unknown type -> Any result
    if *lhs == SemanticType::Any || *rhs == SemanticType::Any {
        return SemanticType::Any;
    }
    match op {
        BinaryOpKind::Add => {
            if *lhs == SemanticType::String {
                return SemanticType::String;
            }
            numeric_op_result(lhs, rhs)
        }
        BinaryOpKind::Sub | BinaryOpKind::Mul | BinaryOpKind::Div | BinaryOpKind::Mod => {
            numeric_op_result(lhs, rhs)
        }
        BinaryOpKind::Eq | BinaryOpKind::Neq => SemanticType::Bool,
        BinaryOpKind::Lt | BinaryOpKind::Gt | BinaryOpKind::LtEq | BinaryOpKind::GtEq => {
            SemanticType::Bool
        }
        BinaryOpKind::And | BinaryOpKind::Or => SemanticType::Bool,
        BinaryOpKind::NullishCoalesce => match lhs {
            SemanticType::Optional(inner) => (**inner).clone(),
            _ => lhs.clone(),
        },
        BinaryOpKind::Assign
        | BinaryOpKind::AddAssign
        | BinaryOpKind::SubAssign
        | BinaryOpKind::MulAssign
        | BinaryOpKind::DivAssign
        | BinaryOpKind::ModAssign => rhs.clone(),
    }
}

fn numeric_op_result(lhs: &SemanticType, rhs: &SemanticType) -> SemanticType {
    match (lhs, rhs) {
        (SemanticType::Int, SemanticType::Int) => SemanticType::Int,
        (SemanticType::Int, SemanticType::Float) => SemanticType::Float,
        (SemanticType::Float, SemanticType::Int) => SemanticType::Float,
        (SemanticType::Float, SemanticType::Float) => SemanticType::Float,
        _ => SemanticType::Any,
    }
}

/// Check that a binary operation is type-safe.
/// Returns Ok(result_type) or Err(message).
pub fn check_binary_op(
    op: &BinaryOpKind,
    lhs: &SemanticType,
    rhs: &SemanticType,
) -> Result<SemanticType, String> {
    // Avoid cascading errors: Unknown type -> Any result
    if *lhs == SemanticType::Any || *rhs == SemanticType::Any {
        return Ok(SemanticType::Any);
    }
    match op {
        BinaryOpKind::Add => {
            if *lhs == SemanticType::String {
                return Ok(SemanticType::String);
            }
            if is_numeric(lhs) && is_numeric(rhs) {
                return Ok(numeric_op_result(lhs, rhs));
            }
            Err(format!(
                "Cannot add '{}' and '{}' — operands must be numeric or String",
                lhs, rhs
            ))
        }
        BinaryOpKind::Sub | BinaryOpKind::Mul | BinaryOpKind::Div | BinaryOpKind::Mod => {
            if is_numeric(lhs) && is_numeric(rhs) {
                Ok(numeric_op_result(lhs, rhs))
            } else {
                Err(format!(
                    "Arithmetic operator requires numeric operands, found '{}' and '{}'",
                    lhs, rhs
                ))
            }
        }
        BinaryOpKind::Eq | BinaryOpKind::Neq => Ok(SemanticType::Bool),
        BinaryOpKind::Lt | BinaryOpKind::Gt | BinaryOpKind::LtEq | BinaryOpKind::GtEq => {
            if is_numeric(lhs) && is_numeric(rhs) {
                Ok(SemanticType::Bool)
            } else {
                Err(format!(
                    "Comparison requires numeric operands, found '{}' and '{}'",
                    lhs, rhs
                ))
            }
        }
        BinaryOpKind::And | BinaryOpKind::Or => {
            if *lhs == SemanticType::Bool && *rhs == SemanticType::Bool {
                Ok(SemanticType::Bool)
            } else {
                Err(format!(
                    "Logical operator requires Bool operands, found '{}' and '{}'",
                    lhs, rhs
                ))
            }
        }
        BinaryOpKind::NullishCoalesce => match lhs {
            SemanticType::Optional(inner) => Ok((**inner).clone()),
            _ => Ok(lhs.clone()),
        },
        BinaryOpKind::Assign
        | BinaryOpKind::AddAssign
        | BinaryOpKind::SubAssign
        | BinaryOpKind::MulAssign
        | BinaryOpKind::DivAssign
        | BinaryOpKind::ModAssign => Ok(rhs.clone()),
    }
}

/// Check that a value can be assigned to a target of the given type.
pub fn check_assignment(
    target_type: &SemanticType,
    value_type: &SemanticType,
) -> Result<(), String> {
    if types_compatible(target_type, value_type) {
        Ok(())
    } else {
        Err(format!(
            "Type mismatch: cannot assign '{}' to '{}'",
            value_type, target_type
        ))
    }
}

/// Check that function call arguments match parameter types.
pub fn check_call(
    func_name: &str,
    param_types: &[SemanticType],
    arg_types: &[SemanticType],
    return_type: &SemanticType,
) -> Result<SemanticType, String> {
    if param_types.len() != arg_types.len() {
        return Err(format!(
            "Function '{}' expects {} arguments, but {} were provided",
            func_name,
            param_types.len(),
            arg_types.len()
        ));
    }
    for (i, (param, arg)) in param_types.iter().zip(arg_types.iter()).enumerate() {
        if !types_compatible(param, arg) {
            return Err(format!(
                "Argument {} to '{}': expected '{}', found '{}'",
                i + 1,
                func_name,
                param,
                arg
            ));
        }
    }
    Ok(return_type.clone())
}

/// Check that a return value matches the declared return type.
pub fn check_return(expected: &SemanticType, actual: &SemanticType) -> Result<(), String> {
    if *expected == SemanticType::Void && *actual != SemanticType::Void {
        return Err(format!(
            "Function returns void but a value of type '{}' is returned",
            actual
        ));
    }
    if *expected != SemanticType::Void && *actual == SemanticType::Void {
        return Err(format!(
            "Function expects return type '{}' but returns nothing",
            expected
        ));
    }
    if types_compatible(expected, actual) {
        Ok(())
    } else {
        Err(format!(
            "Return type mismatch: expected '{}', found '{}'",
            expected, actual
        ))
    }
}

/// Check if two types are compatible (assignment-compatible).
/// Rules:
///   - Same type => compatible
///   - Any is compatible with everything
///   - Int => Float (numeric widening)
///   - T => T? (optional wrapping, warning)
///   - T? accepts T (optional unwrapping)
///   - Null is compatible with T?
pub fn types_compatible(expected: &SemanticType, actual: &SemanticType) -> bool {
    if *expected == SemanticType::Any || *actual == SemanticType::Any {
        return true;
    }
    if expected == actual {
        return true;
    }
    // Numeric widening: Int can go where Float is expected
    if *expected == SemanticType::Float && *actual == SemanticType::Int {
        return true;
    }
    // Narrow integer types (i8, i16, u8, etc.) accept Int literals
    if *actual == SemanticType::Int {
        if let SemanticType::Named(name) = expected {
            if matches!(
                name.as_str(),
                "i8" | "i16" | "i32" | "i64" | "isize" | "usize" | "u8" | "u16" | "u32" | "u64"
            ) {
                return true;
            }
        }
    }
    // Array<T> compatibility: recursive element type check
    if let (SemanticType::Array(elem_e), SemanticType::Array(elem_a)) = (expected, actual) {
        return types_compatible(elem_e, elem_a);
    }
    // Map<K,V> compatibility: recursive key and value type check
    if let (SemanticType::Map(ek_e, ev_e), SemanticType::Map(ek_a, ev_a)) = (expected, actual) {
        return types_compatible(ek_e, ek_a) && types_compatible(ev_e, ev_a);
    }
    // Null is compatible with Optional<T> (check before unwrapping Optional)
    if *actual == SemanticType::Null {
        return matches!(expected, SemanticType::Optional(_));
    }
    // T? accepts T (Optional unwrapping)
    if let SemanticType::Optional(inner) = expected {
        return types_compatible(inner, actual);
    }
    // T accepts T? (Optional wrapping)
    if let SemanticType::Optional(inner) = actual {
        return types_compatible(expected, inner);
    }
    false
}

fn is_numeric(t: &SemanticType) -> bool {
    matches!(t, SemanticType::Int | SemanticType::Float)
}

// ═══════════════════════════════════════════════════════
// Stdlib method-name validation (Task 4.10, ai LOW)
// ═══════════════════════════════════════════════════════

/// Known stdlib method surface for String receivers. Source of truth:
/// `docs/omniview/12-standard-library.md` §内置方法 plus the codegen
/// method tables (React `translate_method_call`, Dart
/// `format_method_call`) — i.e. only names that translate to working code
/// on the P0 backends or are part of the documented language floor.
const STRING_METHODS: &[&str] = &[
    "len",
    "to_string",
    "to_uppercase",
    "to_lowercase",
    "get",
    "starts_with",
    "ends_with",
    "contains",
    "substring",
    "split",
    "replace",
    "trim",
    "to_float",
];

/// Numeric (Int/Float) receiver surface.
const NUMERIC_METHODS: &[&str] = &["to_string", "to_float", "abs"];

/// Bool receiver surface.
const BOOL_METHODS: &[&str] = &["to_string"];

/// Array receiver surface — includes the language methods rewritten by
/// `validate_array_methods` (filter/map/count), which are legal on arrays
/// even though they never reach codegen as plain method calls.
const ARRAY_METHODS: &[&str] = &[
    "len",
    "push",
    "pop",
    "get",
    "is_empty",
    "concat",
    "filter",
    "map",
    "count",
    "slice",
    "splice",
    "reverse",
    "sort",
    "contains",
    "join",
    "to_string",
];

/// Map receiver surface — maps compile to plain objects/Records, so only
/// the indexer-backed `get` and the dispatched `contains` translate on
/// every backend.
const MAP_METHODS: &[&str] = &["get", "contains"];

/// Lowercase and strip underscores/hyphens — `toUpperCase` and
/// `to_uppercase` normalize to the same key.
fn normalize_method(name: &str) -> String {
    name.to_lowercase().replace(['_', '-'], "")
}

/// Suggest the closest known stdlib method, in priority order:
/// 1. case/underscore-insensitive equality (`toUppercase` → `to_uppercase`)
/// 2. normalized prefix, ≥3 chars (`toUpper` → `to_uppercase`,
///    `length` → `len`)
/// 3. edit distance ≤ 3 (`lenght` → `len`)
fn suggest_method<'a>(attempt: &str, known: &'a [&'a str]) -> Option<&'a str> {
    let norm = normalize_method(attempt);
    if norm.len() < 2 {
        return None;
    }
    if let Some(m) = known.iter().find(|k| normalize_method(k) == norm) {
        return Some(m);
    }
    if norm.len() >= 3 {
        if let Some(m) = known
            .iter()
            .find(|k| normalize_method(k).starts_with(&norm))
        {
            return Some(m);
        }
    }
    crate::error::suggest_similar_name(attempt, known)
}

/// Validate a method call against the stdlib surface of its concrete
/// receiver type. Returns the warning message when the method is unknown;
/// `None` when it is known or the receiver type has no provable method
/// surface (Any / Named / Optional / Union / Fn / Void / Null — skipped so
/// an unknown receiver type never cascades into warnings).
pub fn check_method_name(receiver: &SemanticType, method: &str) -> Option<String> {
    let known: &[&str] = match receiver {
        SemanticType::String => STRING_METHODS,
        SemanticType::Int | SemanticType::Float => NUMERIC_METHODS,
        SemanticType::Bool => BOOL_METHODS,
        SemanticType::Array(_) => ARRAY_METHODS,
        SemanticType::Map(_, _) => MAP_METHODS,
        _ => return None,
    };
    if known.contains(&method) {
        return None;
    }
    Some(match suggest_method(method, known) {
        Some(s) => format!("未知方法 \"{method}\"（{receiver}）——最近合法建议 \"{s}\""),
        None => format!("未知方法 \"{method}\"（{receiver}）"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DiagnosticKind;
    use crate::error::Severity;
    use crate::lexer::tokenize;
    use crate::lexer::Span;
    use crate::parser::ast::{BinaryOpKind, Expr, Literal};
    use crate::parser::parse;
    use crate::semantic::analyze;
    use crate::semantic::symbols::SymbolTable;
    use crate::semantic::types::SemanticType;

    // ── Helpers ──────────────────────────────────────────

    /// Parse and semantically analyze source, return diagnostics.
    fn check_source(source: &str) -> Vec<crate::error::Diagnostic> {
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let result = analyze(&program);
        result.diagnostics
    }

    /// Assert that analyzing source produces at least one E0010 error.
    fn assert_type_error(source: &str) {
        let diags = check_source(source);
        let type_errors: Vec<_> = diags
            .iter()
            .filter(|d| matches!(d.kind, DiagnosticKind::E0010))
            .collect();
        assert!(
            !type_errors.is_empty(),
            "Expected E0010 type error, but none found.\nSource: {}\nDiagnostics: {:?}",
            source,
            diags
        );
    }

    /// Assert that analyzing source produces NO E0010 errors.
    fn assert_no_type_error(source: &str) {
        let diags = check_source(source);
        let type_errors: Vec<_> = diags
            .iter()
            .filter(|d| matches!(d.kind, DiagnosticKind::E0010))
            .collect();
        assert!(
            type_errors.is_empty(),
            "Expected no E0010 type error, but found: {:?}\nSource: {}",
            type_errors,
            source
        );
    }

    // ── Literal type inference ──────────────────────────

    #[test]
    fn literal_int_is_int() {
        assert_eq!(SemanticType::parse_type_string("i32"), SemanticType::Int);
        assert_eq!(
            infer_expr_type(
                &Expr::Literal(
                    Literal::Int(42),
                    Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0
                    }
                ),
                &SymbolTable::new(),
                0
            ),
            SemanticType::Int
        );
    }

    #[test]
    fn literal_string_is_string() {
        assert_eq!(
            infer_expr_type(
                &Expr::Literal(
                    Literal::String("hi".to_string()),
                    Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0
                    }
                ),
                &SymbolTable::new(),
                0
            ),
            SemanticType::String
        );
    }

    // ── Binary operator checking ────────────────────────

    #[test]
    fn add_int_int_ok() {
        let result = check_binary_op(&BinaryOpKind::Add, &SemanticType::Int, &SemanticType::Int);
        assert!(result.is_ok(), "Int + Int should be valid");
        assert_eq!(result.unwrap(), SemanticType::Int);
    }

    #[test]
    fn add_int_float_ok() {
        let result = check_binary_op(&BinaryOpKind::Add, &SemanticType::Int, &SemanticType::Float);
        assert!(result.is_ok(), "Int + Float should be valid");
        assert_eq!(result.unwrap(), SemanticType::Float);
    }

    #[test]
    fn add_int_string_error() {
        let result = check_binary_op(
            &BinaryOpKind::Add,
            &SemanticType::Int,
            &SemanticType::String,
        );
        assert!(result.is_err(), "Int + String should error");
    }

    #[test]
    fn add_string_any_ok() {
        let result = check_binary_op(
            &BinaryOpKind::Add,
            &SemanticType::String,
            &SemanticType::Int,
        );
        assert!(result.is_ok(), "String + Int should be valid (concat)");
        assert_eq!(result.unwrap(), SemanticType::String);
    }

    #[test]
    fn eq_int_int_returns_bool() {
        let result = check_binary_op(&BinaryOpKind::Eq, &SemanticType::Int, &SemanticType::Int);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SemanticType::Bool);
    }

    #[test]
    fn and_int_bool_error() {
        let result = check_binary_op(&BinaryOpKind::And, &SemanticType::Int, &SemanticType::Bool);
        assert!(result.is_err(), "Int && Bool should error");
    }

    #[test]
    fn compare_int_string_error() {
        let result = check_binary_op(&BinaryOpKind::Lt, &SemanticType::Int, &SemanticType::String);
        assert!(result.is_err(), "Int < String should error");
    }

    // ── Assignment checking ─────────────────────────────

    #[test]
    fn assignment_same_type_ok() {
        let result = check_assignment(&SemanticType::Int, &SemanticType::Int);
        assert!(result.is_ok());
    }

    #[test]
    fn assignment_int_to_string_error() {
        let result = check_assignment(&SemanticType::String, &SemanticType::Int);
        assert!(result.is_err());
    }

    #[test]
    fn assignment_int_to_any_ok() {
        let result = check_assignment(&SemanticType::Any, &SemanticType::Int);
        assert!(result.is_ok());
    }

    #[test]
    fn assignment_int_to_float_ok() {
        // Numeric widening
        let result = check_assignment(&SemanticType::Float, &SemanticType::Int);
        assert!(result.is_ok());
    }

    #[test]
    fn assignment_null_to_optional_ok() {
        let opt_string = SemanticType::Optional(Box::new(SemanticType::String));
        let result = check_assignment(&opt_string, &SemanticType::Null);
        assert!(result.is_ok());
    }

    #[test]
    fn assignment_null_to_string_error() {
        let result = check_assignment(&SemanticType::String, &SemanticType::Null);
        assert!(result.is_err());
    }

    #[test]
    fn assignment_value_to_optional_ok() {
        let opt_string = SemanticType::Optional(Box::new(SemanticType::String));
        let result = check_assignment(&opt_string, &SemanticType::String);
        assert!(result.is_ok());
    }

    // ── Return checking ─────────────────────────────────

    #[test]
    fn return_matching_type_ok() {
        let result = check_return(&SemanticType::Int, &SemanticType::Int);
        assert!(result.is_ok());
    }

    #[test]
    fn return_mismatch_error() {
        let result = check_return(&SemanticType::Int, &SemanticType::String);
        assert!(result.is_err());
    }

    #[test]
    fn return_value_in_void_fn_error() {
        let result = check_return(&SemanticType::Void, &SemanticType::Int);
        assert!(result.is_err());
    }

    #[test]
    fn return_void_in_non_void_fn_error() {
        let result = check_return(&SemanticType::Int, &SemanticType::Void);
        assert!(result.is_err());
    }

    // ── Integration: source-level type checking ─────────

    #[test]
    fn valid_source_no_type_errors() {
        assert_no_type_error("fn main() { let x: i32 = 42; let y = x + 1; }");
    }

    #[test]
    fn add_int_to_string_produces_type_error() {
        let source = "fn main() { let x: i32 = 1; let y = x + \"hello\"; }";
        assert_type_error(source);
    }

    #[test]
    fn return_type_mismatch_produces_error() {
        let source = "fn get_num() -> i32 { return \"oops\"; }";
        assert_type_error(source);
    }

    #[test]
    fn return_void_with_value_produces_error() {
        let source = "fn main() -> void { return 42; }";
        assert_type_error(source);
    }

    #[test]
    fn string_concat_is_valid() {
        assert_no_type_error("fn main() { let s: String = \"hello\"; let t = s + \" world\"; }");
    }

    #[test]
    fn bool_comparison_is_valid() {
        assert_no_type_error("fn main() { let a: bool = true; let b = a == false; }");
    }

    #[test]
    fn comparison_returns_bool() {
        assert_no_type_error("fn main() { let x: i32 = 1; let y: i32 = 2; let b: bool = x < y; }");
    }

    // ── Stdlib method-name validation (Task 4.10, ai LOW) ──

    /// Task 4.10 Step 1 — unknown method on a concrete receiver must warn
    /// AND name the method AND suggest the stdlib spelling. Regression:
    /// `s.toUpper()` (String receiver) used to compile clean ("No issues
    /// found") and only fail at runtime with a TypeError.
    ///
    /// NOTE: the plan's literal source `view V { ... }` does not parse
    /// (views only exist inside components) and `Target::All` does not
    /// exist — the semantic pass is target-independent, so the in-crate
    /// `check_source` path exercises the same warning through analyze().
    #[test]
    fn unknown_method_on_concrete_type_warns() {
        let diags = check_source("component V { fn f(s: String) { s.toUpper() } view { <div/> } }");
        let d = diags
            .iter()
            .find(|d| d.message.contains("toUpper"))
            .expect("must warn");
        assert_eq!(d.severity, Severity::Warning);
        assert!(
            d.message.contains("to_uppercase"),
            "missing stdlib suggestion: {}",
            d.message
        );
    }

    /// Known stdlib methods on every concrete receiver must stay silent
    /// (zero false positives).
    #[test]
    fn known_stdlib_methods_do_not_warn() {
        let src = r#"component V {
            fn f(s: String, n: i32, items: Array<i32>, cfg: Map<String, i32>) {
                s.to_uppercase(); s.len(); s.get(0); s.starts_with("a");
                n.to_string(); n.to_float();
                items.push(1); items.len(); items.get(0); items.filter(|x| x > 0);
                cfg.get("k"); cfg.contains("k");
            }
            view { <div/> }
        }"#;
        let diags = check_source(src);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert!(
            warnings.is_empty(),
            "known methods must not warn, got: {:?}",
            warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// An Any-typed receiver (no annotation) must be skipped — warning on
    /// it would cascade from a type the checker cannot see.
    #[test]
    fn unknown_method_on_any_receiver_is_skipped() {
        let diags = check_source("component V { fn f(x) { x.toUpper() } view { <div/> } }");
        assert!(
            diags.iter().all(|d| d.severity != Severity::Warning),
            "Any receiver must not warn, got: {:?}",
            diags
        );
    }

    /// Signals own a method surface of their own (set/inc/push/pop/...) —
    /// checking them against the stdlib table would be a false positive.
    #[test]
    fn signal_receiver_methods_do_not_warn() {
        let src = r#"component V {
            signal s: i32 = 0;
            signal list: Array<String> = [];
            fn f() { s.set(1); s.inc(); list.push("x"); list.pop(); }
            view { <div/> }
        }"#;
        let diags = check_source(src);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert!(
            warnings.is_empty(),
            "signal methods must not warn, got: {:?}",
            warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// Element refs dispatch to the ref-op API (focus/measure/scrollTo/...)
    /// — not the stdlib table.
    #[test]
    fn ref_receiver_methods_do_not_warn() {
        let src = r#"component V {
            ref box: Element;
            fn f() { box.focus(); box.measure(); }
            view { <div/> }
        }"#;
        let diags = check_source(src);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert!(
            warnings.is_empty(),
            "ref methods must not warn, got: {:?}",
            warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// Wrong-case / near-miss spellings must all suggest the stdlib form.
    #[test]
    fn case_variants_get_stdlib_suggestion() {
        for (attempt, expected) in [
            ("toUpper", "to_uppercase"),
            ("toUppercase", "to_uppercase"),
            ("toLower", "to_lowercase"),
            ("length", "len"),
            ("tostring", "to_string"),
        ] {
            let src =
                format!("component V {{ fn f(s: String) {{ s.{attempt}() }} view {{ <div/> }} }}");
            let diags = check_source(&src);
            let d = diags
                .iter()
                .find(|d| d.message.contains(attempt))
                .unwrap_or_else(|| panic!("{attempt}: must warn"));
            assert!(
                d.message.contains(expected),
                "{attempt}: missing suggestion '{expected}': {}",
                d.message
            );
        }
    }

    /// Unknown method on a typed array / map / numeric receiver warns too.
    #[test]
    fn unknown_method_warns_on_array_numeric_and_map() {
        let src = r#"component V {
            fn f(items: Array<i32>, n: i32, cfg: Map<String, i32>) {
                items.frobnicate(); n.frobnicate(); cfg.frobnicate();
            }
            view { <div/> }
        }"#;
        let diags = check_source(src);
        let method_warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning && d.message.contains("frobnicate"))
            .collect();
        assert_eq!(
            method_warnings.len(),
            3,
            "each concrete receiver must warn once, got: {:?}",
            diags
        );
    }

    // ── check_method_name unit tests ─────────────────────

    #[test]
    fn check_method_name_string_unknown_with_suggestion() {
        let msg = check_method_name(&SemanticType::String, "toUpper").expect("must warn");
        assert!(msg.contains("toUpper"), "must name the method: {}", msg);
        assert!(
            msg.contains("to_uppercase"),
            "missing stdlib suggestion: {}",
            msg
        );
    }

    #[test]
    fn check_method_name_known_returns_none() {
        assert!(check_method_name(&SemanticType::String, "len").is_none());
        assert!(check_method_name(&SemanticType::String, "to_uppercase").is_none());
        assert!(check_method_name(&SemanticType::Int, "to_string").is_none());
        assert!(check_method_name(&SemanticType::Float, "abs").is_none());
        assert!(check_method_name(&SemanticType::Bool, "to_string").is_none());
        assert!(
            check_method_name(&SemanticType::Array(Box::new(SemanticType::Int)), "filter")
                .is_none()
        );
        assert!(
            check_method_name(&SemanticType::Array(Box::new(SemanticType::Int)), "slice").is_none()
        );
        assert!(check_method_name(
            &SemanticType::Map(Box::new(SemanticType::String), Box::new(SemanticType::Int)),
            "get"
        )
        .is_none());
    }

    #[test]
    fn check_method_name_unknown_receiver_types_are_skipped() {
        // Any / Named (user structs) / Optional — no provable method
        // surface; warning on them would cascade.
        assert!(check_method_name(&SemanticType::Any, "toUpper").is_none());
        assert!(check_method_name(&SemanticType::Named("User".into()), "anything").is_none());
        assert!(check_method_name(
            &SemanticType::Optional(Box::new(SemanticType::String)),
            "toUpper"
        )
        .is_none());
    }
}
