# OmniLang Grammar Documentation Design

## Scope

Two standalone grammar reference documents for AI agents and tool builders:

| File | Scope |
|------|-------|
| `docs/omniview/14-grammar-lang.md` | omni-lang core: lexer tokens, types, expressions, statements, top-level declarations, modules |
| `docs/omniview/15-grammar-view.md` | omni-view UI: components, stores, view templates, directives, styles, design tokens, animations |

## BNF Convention

Optimized for precise consumption by AI agents implementing parsers.

- `::=` production definition, `|` alternative
- Terminals: `'fn'` `'+'` `'{'` `'=>'`
- Lexical classes: `<ident>` `<int_literal>` `<string_literal>` `<float_literal>` `<template_string>`
- Grouping: `( )` group, `*` zero-or-more, `+` one-or-more, `?` optional
- Comments: `//` for disambiguation notes
- Precedence: separate table, NOT encoded in BNF
- Each production includes ≥1 code example
- Contextual keywords annotated (which tokens become identifiers in expression context)
- Disambiguation rules explicitly noted (map literal vs block, struct literal suffix, semicolon insertion)

## Source Files

All grammar derived from:

```
crates/omnilang-core/src/lexer/token.rs       — TokenKind enum (terminals)
crates/omnilang-core/src/parser/ast/mod.rs    — AST node types
crates/omnilang-core/src/parser/expr/mod.rs   — Pratt precedence table
crates/omnilang-core/src/parser/expr/prefix.rs — prefix parselets
crates/omnilang-core/src/parser/expr/infix.rs  — infix parselets
crates/omnilang-core/src/parser/expr/template.rs — template string
crates/omnilang-core/src/parser/stmt.rs       — statement parser
crates/omnilang-core/src/parser/top_level/*.rs — top-level declarations
crates/omnilang-core/src/parser/view/*.rs     — view element parser
crates/omnilang-core/src/parser/design.rs     — design system parser
```

## Key Sections (grammar-lang.md)

1. Lexical Structure — all TokenKind variants, comments, whitespace rules
2. Types — type annotation grammar (Named, Optional, Union, Fn, Array sugar)
3. Expressions — Pratt precedence table + full BNF per precedence level
4. Statements — all Stmt variants with BNF
5. Top-Level Declarations — function, struct, tagged union, type alias, const
6. Modules — import/export syntax
7. Appendix — full precedence table, keyword list, contextual keyword table

## Key Sections (grammar-view.md)

1. Component Definition — full body grammar (props, signals, computeds, events, refs, stores, lifecycles, view, style)
2. View Templates — element syntax (Omni components + HTML), props, children
3. Directives — @if/@else/@for/@slot/@empty/@native/@fallback/@error
4. Store System — store definition, action, getter
5. Style System — style blocks, CSS properties, bindings
6. Design System — design tokens, theme definitions
7. Animation — @motion, @timeline, trigger groups, keyframes
8. Appendix — @-directives table, lifecycle hooks, design token categories
