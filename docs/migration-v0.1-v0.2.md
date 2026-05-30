# OmniLang v0.1 → v0.2 Migration Guide

## Overview
v0.2 is a fundamental version leap: from "Rust unified semantics" to
"idiomatic transpilation + static analysis".

## Breaking Changes

### Type Annotations Now Optional
v0.1 required all variable declarations to have type annotations.
v0.2 supports type inference: `let x = 42;` is valid.

### Structural Typing Default
Types are structurally compatible by default. Use `nominal struct`
for nominal type boundaries.

### Union Types
New `T | U` syntax and `T?` sugar for optional types.

### Match Expressions
New `match x { pattern => body }` syntax replaces if/else chains.

### Async/Await
New `async fn` and `await` keywords for asynchronous programming.

## New Features
- Template strings: `` `Hello, ${name}!` ``
- Optional chaining: `obj?.field`
- Null coalescing: `a ?? b`
- Destructuring: `let [a, b] = arr;`
- Closures: `|a, b| a + b` and `(a, b) => a + b`
- Generics: `fn identity<T>(val: T) -> T`
- Structs: `struct Point { x: f64, y: f64 }`
- Tagged unions: `type Option<T> = Some { value: T } | None`
- Classes: `class Counter { ... }`

## Static Analysis
Run `omnic check` to find portability issues, type errors, and style warnings.
Use `omnic fix` to auto-fix common issues.
Use `omnic check --explain CODE` for detailed rule explanations.

## CLI Changes
- New: `omnic check`, `omnic fix`, `omnic init`
- Planned: `omnic run`, `omnic test`, `omnic fmt`, `omnic lsp`
