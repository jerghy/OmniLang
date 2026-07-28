# Task 1 Report: DCE CallExpr function name tracking

**Status**: DONE

## Change Summary

**File**: `crates/omnilang-core/src/ir/optimize.rs`

**Problem**: The `walk_expr_for_idents` function destructured `ExprIR::CallExpr` with `{ args, .. }`, discarding the `name` field. This meant function call expressions like `fullName()` never registered `fullName` as a used identifier, causing the dead computed-property eliminator to remove computeds that were only referenced through function calls. At runtime this produced `fullName is not defined` errors.

**Fix**: Changed the pattern to `{ name, args }` and added `used.insert(name.clone())` before iterating over args. This ensures the function name is tracked as a used identifier, preventing its elimination.

```rust
// Before:
ExprIR::CallExpr { args, .. } => {
    for arg in args {
        walk_expr_for_idents(arg, used);
    }
}

// After:
ExprIR::CallExpr { name, args } => {
    used.insert(name.clone());
    for arg in args {
        walk_expr_for_idents(arg, used);
    }
}
```

## Test Results

- 29 tests run, all passed (ir::optimize test module)
- No regressions in existing test coverage

## Concerns

None. The fix is minimal, correct, and safely scoped.
