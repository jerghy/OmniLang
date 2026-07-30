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
                "i8" | "i16" | "i32" | "i64" | "isize" | "usize"
                    | "u8" | "u16" | "u32" | "u64"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DiagnosticKind;
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
        let program = parse(tokens, source).expect("parse should succeed");
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
}
