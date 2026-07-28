# Semantic Type Checker Design (P4-2)

## Overview

Add bottom-up expression type inference to the semantic analysis resolve pass.
Catch type errors at compile time before they propagate to 9 codegen backends.

## Architecture

### New File: `semantic/types.rs`

Core type representation — a new `SemanticType` enum, independent from both AST `TypeAnnotation` (parser) and IR `TypeIR` (codegen):

```rust
pub enum SemanticType {
    Int,                                           // i32, i64, int
    Float,                                         // f64, float
    String,                                        // String, str
    Bool,                                          // bool
    Void,                                          // void, ()
    Null,                                          // null
    Named(String),                                 // User, Point (custom structs)
    Array(Box<SemanticType>),                      // Array<T>
    Map(Box<SemanticType>, Box<SemanticType>),     // Map<K, V>
    Optional(Box<SemanticType>),                   // T?
    Union(Vec<SemanticType>),                       // T | U
    Fn(Vec<SemanticType>, Box<SemanticType>),      // fn(params) -> T
    Any,                                           // unknown / inference failure
}
```

Conversion: `TypeAnnotation → SemanticType` via `fn from_type_annotation(ta: &TypeAnnotation) -> SemanticType`.

### Modified File: `semantic/validate_types.rs` (rewrite)

Move existing bind/store/prop validations to a new `validate_component.rs`.
Rewrite `validate_types.rs` as the type-checking entry point:

```
pub fn infer_expr_type(expr: &Expr, symbols: &SymbolTable, current_scope: usize) -> SemanticType
pub fn check_binary_op(op: &BinaryOpKind, lhs: &SemanticType, rhs: &SemanticType) -> Result<SemanticType, String>
pub fn check_assignment(target_type: &SemanticType, value_type: &SemanticType) -> Result<(), String>
pub fn check_call(param_types: &[SemanticType], arg_types: &[SemanticType], return_type: &SemanticType) -> Result<SemanticType, String>
pub fn check_return(expected: &SemanticType, actual: &SemanticType) -> Result<(), String>
```

### Modified File: `semantic/resolve.rs`

Insert type-checking calls into `resolve_expr_refs()`, `resolve_block_refs()`, `resolve_stmt_refs()`.
Checks run AFTER VarRef resolution (so we know types from symbol table) but BEFORE the next expression.

### Pipeline Position

```
tokenize → parse → [collect → resolve+type-check → validate_component → validate_*] → lower → codegen
                                ↑ type checks here            ↑ existing moved here
```

## Type Check Rules

### Binary operators

| Operator | LHS | RHS | Result |
|----------|-----|-----|--------|
| `+ - * / %` | Int, Float | Int, Float | wider of LHS/RHS |
| `+` | String | any | String (concat, auto-stringify RHS) |
| `== !=` | any | any | Bool |
| `< > <= >=` | Int, Float | Int, Float | Bool |
| `&& \|\|` | Bool | Bool | Bool |
| `??` | T? | T | T |
| `= += -= etc` | T | T | T |

### Assignments

- `let x: T = expr` → expr type must be compatible with T
- `let x = expr` → infer T from expr
- `x = expr` → expr type must be compatible with x's declared type
- `let mut x: T = initial` → `x = new_value` must match T on subsequent assignments

### Function calls

- Named function: look up params from SymbolTable, check each arg
- Method call: not checked (target type unknown without full inference)
- Store action: not checked (action params not in current scope)

### Return statements

- `fn foo() -> T { return expr; }` → expr type must be compatible with T
- `return;` in non-void → error
- `return expr;` in void → error

### Type compatibility rules

```
T ≅ T             same type
Int ≅ Float        numeric widening (Float is wider)
Float ≅ Int        numeric narrowing (warning, not error)
T? ≅ T             unwrapping Optional
T ≅ T?             wrapping in Optional (warning)
Null ≅ T?          null is compatible with Optional<T>
Any ≅ T            Any accepts anything (for gradual typing / unknown)
```

## Diagnostics

New diagnostic code: **E0010** (type-mismatch) — replaces the current IR-level lint that only does string matching.

Format: `"Type mismatch: expected '{expected}', found '{actual}'"`

## Tests (20+ cases)

```
// Type matching (should pass)
✓ literal_int_has_type_int
✓ literal_string_has_type_string
✓ add_int_int_returns_int
✓ add_int_float_returns_float
✓ eq_int_int_returns_bool
✓ var_decl_infers_type_from_init
✓ var_decl_with_annotation_matching_init
✓ function_call_matching_params
✓ return_matches_declared_type
✓ optional_unwrap_in_assignment

// Type mismatches (should error)
✗ add_int_string → error
✗ and_int_bool → error
✗ assignment_type_mismatch → error
✗ function_call_arg_count_mismatch → error
✗ function_call_arg_type_mismatch → error
✗ return_type_mismatch → error
✗ return_value_in_void_fn → error
✗ void_return_in_non_void_fn → error
✗ compare_int_string → error
✗ null_assigned_to_non_optional → error
```

## Files Changed

| File | Change |
|------|--------|
| `semantic/types.rs` | **NEW** — SemanticType enum, conversion, helpers |
| `semantic/validate_types.rs` | **REWRITE** — type checking logic |
| `semantic/validate_component.rs` | **NEW** — existing bind/store/prop checks moved here |
| `semantic/resolve.rs` | **MODIFY** — insert type-check calls into resolve walk |
| `semantic/mod.rs` | **MODIFY** — register new modules |
| `semantic/lints/type_mismatch.rs` | **MODIFY** — trivially, update to recognize E0010 |
| `error.rs` | **MODIFY** — add E0010 to DiagnosticKind |

## What this does NOT do (out of scope)

- No type variable / unification / generics
- No flow-sensitive narrowing (if x is T { ... })
- No struct field type checking
- No cross-function / cross-module inference
- No method call type checking
- No closure parameter inference
- Does not modify TypeIR serialization format
