# Semantic Type Checker (P4-2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bottom-up expression type inference to the semantic analysis pass, catching type errors (binary ops, assignments, function calls, returns) at compile time before they reach 9 codegen backends.

**Architecture:** New `SemanticType` enum (`semantic/types.rs`) serves as internal inference IR, independent from parser `TypeAnnotation` and codegen `TypeIR`. Core inference function `infer_expr_type()` runs during resolve pass, with dedicated check functions for binary ops, assignments, calls, and returns. All diagnostics use error code E0010.

**Tech Stack:** Rust, existing `omnilang-core` crate, `rustc-hash` for FxHashMap.

## Global Constraints

- 481 existing tests must continue passing
- No modification to `TypeAnnotation` (parser), `TypeIR` (codegen), or any codegen backend
- All new tests in `#[cfg(test)] mod tests` within `validate_types.rs`
- Follow existing diagnostic pattern: `Diagnostic::error(DiagnosticKind::E0010, ...)`
- `cargo clippy` must produce zero warnings on new code

---

## File Structure

| File | Responsibility |
|------|---------------|
| `semantic/types.rs` ← **NEW** | `SemanticType` enum, `Display`, `from_type_annotation()`, `parse_type_string()` |
| `semantic/validate_component.rs` ← **NEW** | Existing `validate_bind_targets`, `validate_store_action_refs`, `validate_prop_assignment_in_component` moved from `validate_types.rs` |
| `semantic/validate_types.rs` ← **REWRITE** | `infer_expr_type()`, `check_binary_op()`, `check_assignment()`, `check_call()`, `check_return()`, `types_compatible()` |
| `semantic/resolve.rs` ← **MODIFY** | Insert type-check calls into `resolve_stmt_refs()` for VarDecl, Return, ExprStmt with assignment |
| `semantic/mod.rs` ← **MODIFY** | Register `types` and `validate_component` modules; re-export `SemanticType` |
| `error.rs` ← **MODIFY** | Add `E0010` variant to `DiagnosticKind` |

---

### Task 1: Create `semantic/types.rs` — SemanticType enum

**Files:**
- Create: `crates/omnilang-core/src/semantic/types.rs`
- Modify: `crates/omnilang-core/src/error.rs` (add E0010)

**Interfaces:**
- Consumes: `crate::parser::ast::TypeAnnotation`
- Produces: `SemanticType` enum, `SemanticType::from_type_annotation()`, `SemanticType::parse_type_string()`, `Display for SemanticType`, `DiagnosticKind::E0010`

- [ ] **Step 1: Write the file**

```rust
//! Semantic type representation for type inference.
//! Independent from parser TypeAnnotation (AST) and codegen TypeIR.
//! Used internally during semantic analysis resolve pass.

use crate::parser::ast::TypeAnnotation;
use std::fmt;

/// Internal type representation for semantic type checking.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticType {
    Int,
    Float,
    String,
    Bool,
    Void,
    Null,
    /// User-defined named type (struct, type alias)
    Named(String),
    Array(Box<SemanticType>),
    Map(Box<SemanticType>, Box<SemanticType>),
    Optional(Box<SemanticType>),
    Union(Vec<SemanticType>),
    Fn(Vec<SemanticType>, Box<SemanticType>),
    /// Fallback — inference failed or unknown type
    Any,
}

impl SemanticType {
    /// Convert a parser TypeAnnotation into a SemanticType.
    /// Maps built-in type names (i32, String, bool, etc.) to their SemanticType variants.
    /// Composite types (Array<T>, Map<K,V>, Optional<T>, Union, Fn) are recursively converted.
    pub fn from_type_annotation(ta: &TypeAnnotation) -> Self {
        match ta {
            TypeAnnotation::Named { name, type_args, .. } => match name.as_str() {
                "i32" | "i64" | "int" | "Int" | "integer" => SemanticType::Int,
                "f64" | "f32" | "float" | "Float" | "num" | "number" => SemanticType::Float,
                "String" | "str" => SemanticType::String,
                "bool" | "Bool" | "boolean" => SemanticType::Bool,
                "void" | "Void" | "()" | "unit" => SemanticType::Void,
                "null" | "Null" | "nil" | "None" => SemanticType::Null,
                "Array" if type_args.len() == 1 => {
                    SemanticType::Array(Box::new(Self::from_type_annotation(&type_args[0])))
                }
                "Map" if type_args.len() == 2 => SemanticType::Map(
                    Box::new(Self::from_type_annotation(&type_args[0])),
                    Box::new(Self::from_type_annotation(&type_args[1])),
                ),
                _ => SemanticType::Named(name.clone()),
            },
            TypeAnnotation::Optional { base, .. } => {
                SemanticType::Optional(Box::new(Self::from_type_annotation(base)))
            }
            TypeAnnotation::Union { types, .. } => {
                SemanticType::Union(types.iter().map(Self::from_type_annotation).collect())
            }
            TypeAnnotation::Fn { params, return_type, .. } => SemanticType::Fn(
                params.iter().map(Self::from_type_annotation).collect(),
                Box::new(Self::from_type_annotation(return_type)),
            ),
        }
    }

    /// Parse a type from a string representation (for Symbol type_ann lookups).
    /// Handles simple built-in names; complex types fall back to Any.
    pub fn parse_type_string(s: &str) -> Self {
        match s {
            "i32" | "i64" | "int" | "Int" | "integer" => SemanticType::Int,
            "f64" | "f32" | "float" | "Float" | "num" | "number" => SemanticType::Float,
            "String" | "str" => SemanticType::String,
            "bool" | "Bool" | "boolean" => SemanticType::Bool,
            "void" | "Void" | "()" | "unit" => SemanticType::Void,
            "null" | "Null" | "nil" | "None" => SemanticType::Null,
            _ => SemanticType::Named(s.to_string()),
        }
    }
}

impl fmt::Display for SemanticType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticType::Int => write!(f, "Int"),
            SemanticType::Float => write!(f, "Float"),
            SemanticType::String => write!(f, "String"),
            SemanticType::Bool => write!(f, "Bool"),
            SemanticType::Void => write!(f, "Void"),
            SemanticType::Null => write!(f, "Null"),
            SemanticType::Named(name) => write!(f, "{}", name),
            SemanticType::Array(inner) => write!(f, "Array<{}>", inner),
            SemanticType::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            SemanticType::Optional(inner) => write!(f, "{}?", inner),
            SemanticType::Union(types) => {
                let names: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", names.join(" | "))
            }
            SemanticType::Fn(params, ret) => {
                let p: Vec<String> = params.iter().map(|t| t.to_string()).collect();
                write!(f, "fn({}) -> {}", p.join(", "), ret)
            }
            SemanticType::Any => write!(f, "<unknown>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_int() {
        assert_eq!(SemanticType::Int.to_string(), "Int");
    }

    #[test]
    fn display_optional_string() {
        let t = SemanticType::Optional(Box::new(SemanticType::String));
        assert_eq!(t.to_string(), "String?");
    }

    #[test]
    fn display_array_int() {
        let t = SemanticType::Array(Box::new(SemanticType::Int));
        assert_eq!(t.to_string(), "Array<Int>");
    }

    #[test]
    fn display_union() {
        let t = SemanticType::Union(vec![SemanticType::Int, SemanticType::String]);
        assert_eq!(t.to_string(), "Int | String");
    }

    #[test]
    fn parse_type_string_i32() {
        assert_eq!(SemanticType::parse_type_string("i32"), SemanticType::Int);
    }

    #[test]
    fn parse_type_string_unknown() {
        assert_eq!(
            SemanticType::parse_type_string("User"),
            SemanticType::Named("User".to_string())
        );
    }
}
```

- [ ] **Step 2: Add E0010 to error.rs**

In `crates/omnilang-core/src/error.rs`, add `E0010` to the `DiagnosticKind` enum, after `E0009`:

```rust
// Type mismatch — type checking error
E0010,
```

And update the `ts_rs` export bindings in the same file if needed (search for the `#[derive]` listing of variants and add E0010 there too).

- [ ] **Step 3: Build check**

Run: `cargo check -p omnilang-core`
Expected: compiles cleanly

- [ ] **Step 4: Run tests**

Run: `cargo test -p omnilang-core --lib -- semantic::types`
Expected: 6 new tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/types.rs
git commit -m "feat(semantic): add SemanticType enum for type inference (P4-2)

Add internal type representation independent from parser TypeAnnotation
and codegen TypeIR. Supports built-in primitives, Array, Map, Optional,
Union, Fn types, and Any fallback. Includes Display and parsing helpers.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Extract existing validations to `validate_component.rs`

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_component.rs`
- Modify: `crates/omnilang-core/src/semantic/validate_types.rs` (remove moved code)
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:101-104` (update import path)

**Interfaces:**
- Consumes: `SemanticAnalyzer`, `ComponentDef`, `Module`, `walk_view_nodes`
- Produces: `SemanticAnalyzer::validate_bind_targets()`, `validate_store_action_refs()`, `validate_prop_assignment_in_component()` — same signatures, moved location

- [ ] **Step 1: Create `validate_component.rs` with moved code**

Copy the three `impl SemanticAnalyzer` blocks from `validate_types.rs` into `validate_component.rs` with the module declaration:

```rust
//! Component-level validations: bind targets, store action refs, prop assignment.
//!
//! Extracted from `validate_types.rs` to separate type inference (new)
//! from structural component validation.

use super::walk_view::walk_view_nodes;
use super::SemanticAnalyzer;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

// ── Bind target validation ──────────────────────────

impl SemanticAnalyzer {
    /// bind: must target a signal, not a store action or fn
    pub(super) fn validate_bind_targets(&mut self, comp: &ComponentDef) {
        // (COPY existing implementation verbatim from validate_types.rs)
        let signal_names: FxHashSet<&str> = comp.signals.iter().map(|s| s.name.as_str()).collect();
        // ... rest of existing implementation ...
    }
}

// ── Store action references ─────────────────────────

impl SemanticAnalyzer {
    /// Validate that EventHandler::StoreAction references point to real stores and actions.
    pub(super) fn validate_store_action_refs(&mut self, comp: &ComponentDef, module: &Module) {
        // (COPY existing implementation verbatim from validate_types.rs)
        // ... rest of existing implementation ...
    }
}

// ── Prop assignment ─────────────────────────────────

impl SemanticAnalyzer {
    /// Validate that component props are not assigned to within the component.
    pub(super) fn validate_prop_assignment_in_component(&mut self, comp: &ComponentDef) {
        // (COPY existing implementation verbatim from validate_types.rs)
        // ... rest of existing implementation ...
    }
}
```

The actual code to copy is the ENTIRE content of `validate_types.rs` as-is into `validate_component.rs` (changing only the module doc comment).

- [ ] **Step 2: Clear `validate_types.rs`**

Replace `validate_types.rs` with a minimal placeholder:

```rust
//! Expression type inference and checking.
//! Populated in Task 3.

// (placeholder — type checking logic added in Task 3)
```

- [ ] **Step 3: Update `resolve.rs` import**

In `resolve.rs`, the calls at lines 101-104 are:
```rust
self.validate_bind_targets(comp);
self.validate_store_action_refs(comp, module);
self.validate_store_field_access(comp);
self.validate_prop_assignment_in_component(comp);
```

These calls remain unchanged — they're method calls on `self`, and the methods now live in `validate_component.rs`. No import change needed (the module is registered in `mod.rs` in Task 5).

- [ ] **Step 4: Build check**

Run: `cargo check -p omnilang-core`
Expected: compiles cleanly (all moved code is identical)

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p omnilang-core --lib`
Expected: 481 passed, 0 failed (no behavioral change — pure code move)

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_component.rs crates/omnilang-core/src/semantic/validate_types.rs
git commit -m "refactor(semantic): extract component validations to validate_component.rs

Move bind target, store action ref, and prop assignment validations
out of validate_types.rs. The file will be repurposed for expression
type inference in the next task.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Implement type inference core in `validate_types.rs`

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_types.rs` (full rewrite)

**Interfaces:**
- Consumes: `SemanticType` (from Task 1), `Expr`, `Stmt`, `BinaryOpKind`, `UnaryOpKind`, `Literal`, `SymbolTable`
- Produces: `infer_expr_type()`, `check_binary_op()`, `check_assignment()`, `check_call()`, `check_return()`, `types_compatible()`

- [ ] **Step 1: Write infer_expr_type()**

```rust
//! Expression type inference and checking.
//!
//! Bottom-up type inference for expressions. Runs during the resolve pass
//! to detect type errors (binary ops, assignments, calls, returns) at
//! compile time. Uses [`SemanticType`] as the internal type representation.

use super::symbols::{Symbol, SymbolKind, SymbolTable};
use super::types::SemanticType;
use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::parser::ast::*;
use crate::lexer::Span;

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
        Expr::VarRef(name, _) => {
            symbols.lookup(name, scope)
                .and_then(|s| s.type_ann.as_ref())
                .map(|t| SemanticType::parse_type_string(t))
                .unwrap_or(SemanticType::Any)
        }
        Expr::BinaryOp { left, op, right, .. } => {
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
        Expr::FuncCall { func, args, .. } => {
            if let Expr::VarRef(name, _) = func.as_ref() {
                if let Some(sym) = symbols.lookup(name, scope) {
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
        Expr::IfExpr { then_expr, else_expr, .. } => {
            let t = infer_expr_type(then_expr, symbols, scope);
            let e = infer_expr_type(else_expr, symbols, scope);
            if t == e { t } else { SemanticType::Any }
        }
        Expr::TernaryExpr { then_expr, else_expr, .. } => {
            let t = infer_expr_type(then_expr, symbols, scope);
            let e = infer_expr_type(else_expr, symbols, scope);
            if t == e { t } else { SemanticType::Any }
        }
        Expr::TemplateString { .. } => SemanticType::String,
        Expr::StructLiteral { .. } => SemanticType::Any,
        Expr::StoreFieldAccess { .. } => SemanticType::Any,
        Expr::Block { body, .. } => {
            body.trailing_expr.as_ref()
                .map(|e| infer_expr_type(e, symbols, scope))
                .unwrap_or(SemanticType::Void)
        }
        Expr::ClosureExpr { .. } => SemanticType::Any,
        Expr::AwaitExpr { operand, .. } => infer_expr_type(operand, symbols, scope),
        Expr::MatchExpr { arms, .. } => {
            arms.first()
                .map(|arm| infer_block_type(&arm.body, symbols, scope))
                .unwrap_or(SemanticType::Void)
        }
        Expr::SpreadExpr { operand, .. } => infer_expr_type(operand, symbols, scope),
        Expr::IsCheck { .. } => SemanticType::Bool,
        Expr::NotYetImplemented { .. } => SemanticType::Any,
        Expr::Error { .. } => SemanticType::Any,
    }
}

/// Infer the type of a block (its trailing expression, or Void if none).
fn infer_block_type(block: &Block, symbols: &SymbolTable, scope: usize) -> SemanticType {
    block.trailing_expr.as_ref()
        .map(|e| infer_expr_type(e, symbols, scope))
        .unwrap_or(SemanticType::Void)
}

/// Compute the result type of a binary operation.
/// Returns the result type on success, or an error message describing the mismatch.
pub fn binary_op_result_type(op: &BinaryOpKind, lhs: &SemanticType, rhs: &SemanticType) -> SemanticType {
    match op {
        BinaryOpKind::Add => {
            if *lhs == SemanticType::String || *rhs == SemanticType::String {
                return SemanticType::String;
            }
            numeric_op_result(lhs, rhs)
        }
        | BinaryOpKind::Mul | BinaryOpKind::Div | BinaryOpKind::Mod => {
            numeric_op_result(lhs, rhs)
        }
        BinaryOpKind::Eq | BinaryOpKind::Neq => SemanticType::Bool,
        BinaryOpKind::Lt | BinaryOpKind::Gt | BinaryOpKind::LtEq | BinaryOpKind::GtEq => SemanticType::Bool,
        BinaryOpKind::And | BinaryOpKind::Or => SemanticType::Bool,
        BinaryOpKind::NullishCoalesce => {
            match lhs {
                SemanticType::Optional(inner) => (**inner).clone(),
                _ => lhs.clone(),
            }
        }
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
```

- [ ] **Step 2: Write type check functions**

Add these after the inference functions:

```rust
/// Check that a binary operation is type-safe.
/// Returns Ok(result_type) or Err(message).
pub fn check_binary_op(
    op: &BinaryOpKind,
    lhs: &SemanticType,
    rhs: &SemanticType,
) -> Result<SemanticType, String> {
    match op {
        BinaryOpKind::Add => {
            if *lhs == SemanticType::String || *rhs == SemanticType::String {
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
        BinaryOpKind::NullishCoalesce => {
            match lhs {
                SemanticType::Optional(inner) => Ok((**inner).clone()),
                _ => Ok(lhs.clone()),
            }
        }
        BinaryOpKind::Assign
        | BinaryOpKind::AddAssign
        | BinaryOpKind::SubAssign
        | BinaryOpKind::MulAssign
        | BinaryOpKind::DivAssign
        | BinaryOpKind::ModAssign => Ok(rhs.clone()),
    }
}

/// Check that a value can be assigned to a target of the given type.
pub fn check_assignment(target_type: &SemanticType, value_type: &SemanticType) -> Result<(), String> {
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
///   - Same type → compatible
///   - Any is compatible with everything
///   - Int → Float (numeric widening)
///   - T → T? (optional wrapping, warning)
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
    // T? accepts T
    if let SemanticType::Optional(inner) = expected {
        return types_compatible(inner, actual);
    }
    // T accepts T?
    if let SemanticType::Optional(inner) = actual {
        return types_compatible(expected, inner);
    }
    // Null is compatible with Optional<T>
    if *actual == SemanticType::Null {
        return matches!(expected, SemanticType::Optional(_));
    }
    false
}

fn is_numeric(t: &SemanticType) -> bool {
    matches!(t, SemanticType::Int | SemanticType::Float)
}
```

- [ ] **Step 3: Write unit tests in the same file**

Append `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::{analyze, SemanticAnalyzer};
    use rustc_hash::FxHashMap;

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
        let type_errors: Vec<_> = diags.iter()
            .filter(|d| matches!(d.kind, DiagnosticKind::E0010))
            .collect();
        assert!(!type_errors.is_empty(),
            "Expected E0010 type error, but none found.\nSource: {}\nDiagnostics: {:?}",
            source, diags);
    }

    /// Assert that analyzing source produces NO E0010 errors.
    fn assert_no_type_error(source: &str) {
        let diags = check_source(source);
        let type_errors: Vec<_> = diags.iter()
            .filter(|d| matches!(d.kind, DiagnosticKind::E0010))
            .collect();
        assert!(type_errors.is_empty(),
            "Expected no E0010 type error, but found: {:?}\nSource: {}",
            type_errors, source);
    }

    // ── Literal type inference ──────────────────────────

    #[test]
    fn literal_int_is_int() {
        assert_eq!(SemanticType::parse_type_string("i32"), SemanticType::Int);
        assert_eq!(
            infer_expr_type(
                &Expr::Literal(Literal::Int(42),
                    Span { start: 0, end: 0, line: 0, col: 0 }),
                &SymbolTable::new(), 0
            ),
            SemanticType::Int
        );
    }

    #[test]
    fn literal_string_is_string() {
        assert_eq!(
            infer_expr_type(
                &Expr::Literal(Literal::String("hi".to_string()),
                    Span { start: 0, end: 0, line: 0, col: 0 }),
                &SymbolTable::new(), 0
            ),
            SemanticType::String
        );
    }

    // ── Binary operator checking ────────────────────────

    #[test]
    fn add_int_int_ok() {
        let result = check_binary_op(
            &BinaryOpKind::Add, &SemanticType::Int, &SemanticType::Int
        );
        assert!(result.is_ok(), "Int + Int should be valid");
        assert_eq!(result.unwrap(), SemanticType::Int);
    }

    #[test]
    fn add_int_float_ok() {
        let result = check_binary_op(
            &BinaryOpKind::Add, &SemanticType::Int, &SemanticType::Float
        );
        assert!(result.is_ok(), "Int + Float should be valid");
        assert_eq!(result.unwrap(), SemanticType::Float);
    }

    #[test]
    fn add_int_string_error() {
        let result = check_binary_op(
            &BinaryOpKind::Add, &SemanticType::Int, &SemanticType::String
        );
        assert!(result.is_err(), "Int + String should error");
    }

    #[test]
    fn add_string_any_ok() {
        let result = check_binary_op(
            &BinaryOpKind::Add, &SemanticType::String, &SemanticType::Int
        );
        assert!(result.is_ok(), "String + Int should be valid (concat)");
        assert_eq!(result.unwrap(), SemanticType::String);
    }

    #[test]
    fn eq_int_int_returns_bool() {
        let result = check_binary_op(
            &BinaryOpKind::Eq, &SemanticType::Int, &SemanticType::Int
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SemanticType::Bool);
    }

    #[test]
    fn and_int_bool_error() {
        let result = check_binary_op(
            &BinaryOpKind::And, &SemanticType::Int, &SemanticType::Bool
        );
        assert!(result.is_err(), "Int && Bool should error");
    }

    #[test]
    fn compare_int_string_error() {
        let result = check_binary_op(
            &BinaryOpKind::Lt, &SemanticType::Int, &SemanticType::String
        );
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
```

- [ ] **Step 4: Build check**

Run: `cargo check -p omnilang-core`
Expected: compiles cleanly (tests may have unused imports — that's fine, they're `#[cfg(test)]`)

- [ ] **Step 5: Run type checker unit tests**

Run: `cargo test -p omnilang-core --lib -- semantic::validate_types`
Expected: 25 new tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_types.rs
git commit -m "feat(semantic): implement bottom-up expression type inference (P4-2)

Add infer_expr_type() with check functions for binary ops, assignments,
function calls, and return statements. Uses new SemanticType enum.
25 unit tests covering type matching, mismatches, numeric widening,
optional/null compatibility.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Integrate type checking into `resolve.rs`

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs`

**Interfaces:**
- Consumes: `infer_expr_type`, `check_binary_op`, `check_assignment`, `check_return` (from Task 3)
- Produces: Type diagnostics emitted during resolve pass

- [ ] **Step 1: Add type checking to VarDecl in resolve_stmt_refs**

In `resolve_stmt_refs()`, modify the `Stmt::VarDecl` arm. After the existing declaration logic, add type checking for the initializer:

```rust
use super::validate_types::{check_assignment, check_binary_op, check_return, infer_expr_type};
use super::types::SemanticType;

// Inside resolve_stmt_refs, in the VarDecl arm, AFTER existing init resolution:
Stmt::VarDecl {
    name,
    init,
    type_ann,
    ..
} => {
    // ... existing declaration logic (lines 186-199) ...

    if let Some(e) = init {
        self.resolve_expr_refs(e, vctx);
        // NEW: type-check initializer against annotation
        if let Some(ref ta) = type_ann {
            let expected = SemanticType::from_type_annotation(ta);
            let actual = infer_expr_type(e, &self.symbols, self.symbols.current_scope());
            if actual != SemanticType::Any {
                if let Err(msg) = check_assignment(&expected, &actual) {
                    self.diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        kind: DiagnosticKind::E0010,
                        message: msg,
                        file: None,
                        line: 0,
                        col: 0,
                        end_line: None,
                        end_col: None,
                        suggestion: None,
                    });
                }
            }
        }
    }
}
```

- [ ] **Step 2: Add type checking to Return statement**

In `resolve_stmt_refs`, modify the `Stmt::Return` arm. After doing VarRef resolution on the value, also check return type against the function signature:

```rust
Stmt::Return { value, .. } => {
    if let Some(e) = value {
        self.resolve_expr_refs(e, vctx);
    }
    // NEW: check return type against current function's declared return type
    if let Some(ref func_type_str) = vctx.current_function_return_type {
        let expected = SemanticType::parse_type_string(func_type_str);
        let actual = match value {
            Some(ref e) => infer_expr_type(e, &self.symbols, self.symbols.current_scope()),
            None => SemanticType::Void,
        };
        if actual != SemanticType::Any {
            if let Err(msg) = check_return(&expected, &actual) {
                self.diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    kind: DiagnosticKind::E0010,
                    message: msg,
                    file: None,
                    line: 0,
                    col: 0,
                    end_line: None,
                    end_col: None,
                    suggestion: None,
                });
            }
        }
    }
}
```

- [ ] **Step 3: Add type checking to ExprStmt with binary assignment ops**

In `resolve_stmt_refs`, modify the `Stmt::ExprStmt` arm. After resolving refs, check binary ops with assignment:

```rust
Stmt::ExprStmt { expr, .. } => {
    self.resolve_expr_refs(expr, vctx);
    // NEW: type-check binary ops (arithmetic type mismatches)
    if let Expr::BinaryOp { left, op, right, .. } = expr {
        let lhs = infer_expr_type(left, &self.symbols, self.symbols.current_scope());
        let rhs = infer_expr_type(right, &self.symbols, self.symbols.current_scope());
        if lhs != SemanticType::Any && rhs != SemanticType::Any {
            if let Err(msg) = check_binary_op(op, &lhs, &rhs) {
                self.diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    kind: DiagnosticKind::E0010,
                    message: msg,
                    file: None,
                    line: 0,
                    col: 0,
                    end_line: None,
                    end_col: None,
                    suggestion: None,
                });
            }
        }
    }
}
```

- [ ] **Step 4: Add `current_function_return_type` to ValidationContext**

In `semantic/validate_inline.rs`, add the field to ValidationContext:

```rust
pub struct ValidationContext {
    pub is_async: bool,
    pub loop_depth: u32,
    /// The return type of the enclosing function, if any.
    /// Set when entering a function scope; used for return type checking.
    pub current_function_return_type: Option<String>,
}
```

In `resolve.rs`, when entering function scopes, set this field. In `resolve_and_validate_module`, modify the function traversal:

```rust
// For component functions:
for f in &comp.fns {
    // ... existing scope setup ...
    let mut fn_vctx = ValidationContext {
        is_async: f.is_async,
        loop_depth: 0,
        current_function_return_type: f.return_type.as_ref().map(|t| super::helpers::type_to_string(t)),
    };
    self.resolve_block_refs(&f.body, &mut fn_vctx);
    // ... existing scope teardown ...
}
```

- [ ] **Step 5: Build check**

Run: `cargo check -p omnilang-core`
Expected: compiles cleanly

- [ ] **Step 6: Run all tests to verify no regressions**

Run: `cargo test -p omnilang-core --lib`
Expected: 481 + 25 = ~506 passed, 0 failed

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs crates/omnilang-core/src/semantic/validate_inline.rs
git commit -m "feat(semantic): integrate type checking into resolve pass (P4-2)

Add type checks for variable declarations (init vs annotation), return
statements (value vs declared return type), and binary operations.
Add current_function_return_type to ValidationContext.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Register modules in `mod.rs`

**Files:**
- Modify: `crates/omnilang-core/src/semantic/mod.rs`

**Interfaces:**
- Consumes: `types` module, `validate_component` module (from Tasks 1, 2)
- Produces: Modules registered and resolvable by other semantic code

- [ ] **Step 1: Update semantic/mod.rs**

Add the two new modules to the `mod` declarations:

```rust
mod types;                   // NEW
mod validate_component;      // NEW
```

And update the existing `mod` declarations to include `validate_component` (currently `validate_types` methods are directly used):

The final `mod` block should be:
```rust
mod collect;
mod helpers;
mod motion;
mod resolve;
mod types;                   // NEW
mod validate_component;      // NEW
mod validate_inline;
mod validate_motion;
mod validate_motion_triggers;
mod validate_store_access;
mod validate_types;
```

- [ ] **Step 2: Update error.rs — add E0010**

Add `E0010` to DiagnosticKind:

```rust
E0010,  // Type mismatch — type checking error
```

Place it after E0009 (prop assignment) — the next available error code:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticKind {
    E0001, // Syntax error
    E0002, // Duplicate symbol
    E0003, // Unresolved reference
    E0004, // Reserved
    E0005, // Bind target
    E0006, // Store field access
    E0007, // Store action ref
    E0008, // Motion validation
    E0009, // Prop assignment
    E0010, // Type mismatch — NEW
    // ... lint codes continue ...
    UndefinedRef,
    TypeMismatch,
    // ...
}
```

- [ ] **Step 3: Build check**

Run: `cargo check -p omnilang-core`
Expected: compiles cleanly

- [ ] **Step 4: Run all tests**

Run: `cargo test -p omnilang-core --lib`
Expected: ~506 passed, 0 failed

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/mod.rs
git commit -m "chore(semantic): register types and validate_component modules (P4-2)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Clippy check and final verification

**Files:**
- No new files — verify all previous changes

- [ ] **Step 1: Run clippy**

```bash
cargo clippy -p omnilang-core -- -D warnings
```

Expected: zero warnings.

If warnings appear, fix them in-place (typically unused imports in `#[cfg(test)]` blocks need `#[allow(unused_imports)]` or restructuring).

- [ ] **Step 2: Run full test suite one final time**

```bash
cargo test -p omnilang-core --lib
```

Expected: ~506 passed, 0 failed.

- [ ] **Step 3: Commit any clippy fixes**

```bash
git add -A
git commit -m "chore(semantic): fix clippy warnings in type checker (P4-2)

Co-Authored-By: Claude <noreply@anthropic.com>"
```
