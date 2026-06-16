# OmniLang Import Module System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the full import/export module system for OmniLang across Python framework, Rust compiler DLL, and all 4 logic + 4 UI codegen targets.

**Architecture:** The work spans 6 phases matching the spec: (1) syntax/parsing in both Python grammars and the Rust compiler, (2) Python ModuleLoader V2 with path map building, (3) Rust multi-file parse + cross-file validation + cycle detection, (4) Python codegen for all 8 targets, (5) CLI and config unification, (6) editor integration. The Rust DLL stays filesystem-free — Python does all I/O + codegen, Rust receives `{source_map, path_mapping}` and returns `{success, errors}` (validation only). Codegen remains in Python where all 8 target generators already live.

**Tech Stack:** Python 3.10+ (Lark parser, dataclasses), Rust 2021 edition (hand-written recursive-descent parser, serde, thiserror), TOML for config.

---

## File Map

```
omnilang/grammar/omnilang.lark          MODIFY — add pub, export from, import as, import type rules
omniview/grammar.lark                   MODIFY — same grammar additions
omnilang/grammar/_core.lark             MODIFY — add Pub keyword token if needed
omnilang/ast_nodes.py                   MODIFY — add PubModifier, ExportFromStmt, ImportNamespaceStmt
omniview/ast_nodes.py                   MODIFY — add PubModifier, ExportFromStmt, ImportNamespaceStmt
omnilang/parser.py                      MODIFY — handle new grammar transformer rules
omniview/parser.py                      MODIFY — handle new grammar transformer rules
omnilang/modules/resolver.py            MODIFY — enforce .omni extension, alias from config
omnilang/modules/loader.py              MODIFY — complete _extract_imports
omniview/modules/loader.py              MODIFY — build {path: source} map, enforce logic↛view boundary
omnilang/config.py                      MODIFY — add ui_target, aliases, source/output dirs
omnilang/codegen/javascript.py          MODIFY — update import/export codegen
omnilang/codegen/rust.py                MODIFY — update import/export codegen
omnilang/codegen/python.py              MODIFY — update import/export codegen
omnilang/codegen/dart.py                MODIFY — update import/export codegen
omniview/codegen/react.py               MODIFY — update import/export codegen
omniview/codegen/vue.py                 MODIFY — update import/export codegen
omniview/codegen/dioxus.py              MODIFY — update import/export codegen
omniview/codegen/flutter.py             MODIFY — update import/export codegen
omniview/codegen/base.py                MODIFY — update base generator if needed
omniview/codegen/rust_bridge.py         CREATE — Python ctypes bridge to Rust DLL
omniview/cli.py                         MODIFY — update build/init commands
omnilang/cli.py                         MODIFY — update build/init commands
omni-compiler/src/lexer/token.rs        MODIFY — add Pub token
omni-compiler/src/lexer/mod.rs          MODIFY — add "pub" keyword mapping, verify From/As
omni-compiler/src/parser/ast.rs         MODIFY — new AST nodes (ImportStmt enum, Visibility, TypeAlias), updated Program
omni-compiler/src/parser/mod.rs         MODIFY — new parse dispatch
omni-compiler/src/parser/top_level.rs   MODIFY — rewrite parse_import, add parse_export, add parse_type_alias
omni-compiler/src/semantic/mod.rs       MODIFY — handle new import variants in all 5 passes
omni-compiler/src/semantic/cycles.rs    CREATE — Tarjan SCC cycle detection with path_mapping
omni-compiler/src/ir/mod.rs             MODIFY — skip import statements in IR lowering
omni-compiler/src/lib.rs                MODIFY — multi-file validate_multi API, omni_validate_multi C FFI
omni-compiler/tests/lexer_tests.rs      MODIFY — add pub token test
omni-compiler/tests/parser_tests.rs     MODIFY — add import/export/pub tests
```

---

## Phase 1: Syntax & Parsing

### Task 1.1: Add `pub` token to Rust lexer

**Files:**
- Modify: `omni-compiler/src/lexer/token.rs:7-154`
- Modify: `omni-compiler/src/lexer/mod.rs`

- [ ] **Step 1: Add `Pub` variant to TokenKind enum**

In `omni-compiler/src/lexer/token.rs`, add `Pub` after existing keyword variants (near `Fn`, `Struct`, etc.):

```rust
// In TokenKind enum, add after existing keywords:
Pub,
```

- [ ] **Step 2: Add "pub" and verify "from"/"as" keyword mappings in lexer**

In `omni-compiler/src/lexer/mod.rs`, find the `read_ident_or_keyword()` method and add the `"pub"` mapping alongside other keywords:

```rust
"pub" => TokenKind::Pub,
```

Also verify that `"from"` and `"as"` are already mapped:
```rust
"from" => TokenKind::From,   // should already exist
"as" => TokenKind::As,       // should already exist
```

If `From` or `As` variants are missing from `TokenKind` in `token.rs`, add them:
```rust
From,
As,
```

Insert in alphabetical order among the existing keyword match arms.

- [ ] **Step 3: Add lexer test for `pub` keyword**

In `omni-compiler/tests/lexer_tests.rs`, add:

```rust
#[test]
fn test_pub_keyword() {
    let tokens = tokenize("pub fn add(a: i32, b: i32) -> i32 { a + b }");
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    // Find Pub token
    assert!(kinds.iter().any(|k| matches!(k, TokenKind::Pub)));
}
```

- [ ] **Step 4: Run lexer tests**

```bash
cd omni-compiler && cargo test test_pub_keyword
```

Expected: test compiles and passes (Pub token recognized).

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/lexer/token.rs omni-compiler/src/lexer/mod.rs omni-compiler/tests/lexer_tests.rs
git commit -m "feat(rust-lexer): add Pub token and keyword mapping"
```

---

### Task 1.2: Add new AST nodes to Rust compiler

> **Naming note:** Rust uses `ExportFrom` for re-exports while Python uses `ExportFromStmt`.
> This is intentional — Rust convention drops the `Stmt` suffix since the type system
> already communicates the node kind, while Python AST nodes use `Stmt` suffixes
> for consistency with the existing codebase pattern. Same concept, idiomatic name per language.

**Files:**
- Modify: `omni-compiler/src/parser/ast.rs`

- [ ] **Step 1: Add `PubModifier` enum**

Add before the existing `ImportStmt` struct (around line 477):

```rust
/// Visibility modifier for declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}
```

- [ ] **Step 2: Update `ImportStmt` struct**

Replace the current `ImportStmt` (lines 480-483) with expanded variants:

```rust
/// Named import: `import { A, B } from "./path.omni";`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedImport {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// Namespace import: `import "./path.omni" as name;`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamespaceImport {
    pub path: String,
    pub alias: String,
    pub span: Span,
}

/// Type-only import: `import type { A, B } from "./path.omni";`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeImport {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// Re-export: `export { A, B } from "./path.omni";`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportFrom {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// All import variants in one enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportStmt {
    Named(NamedImport),
    Namespace(NamespaceImport),
    TypeOnly(TypeImport),
    Lib(String), // "omniview" — bare import marker
}
```

- [ ] **Step 3: Add `Visibility` to `FunctionDef`, `StructDef`, and `TypeAlias`**

Add `pub visibility: Visibility` field to `FunctionDef`:

```rust
pub struct FunctionDef {
    pub visibility: Visibility,  // new field
    pub name: String,
    // ... existing fields
}
```

Add `pub visibility: Visibility` field to `StructDef`:

```rust
pub struct StructDef {
    pub visibility: Visibility,  // new field
    pub name: String,
    // ... existing fields
}
```

Add `pub visibility: Visibility` field to `TypeAlias` (create if it doesn't exist; otherwise add the field):

```rust
pub struct TypeAlias {
    pub visibility: Visibility,  // new field
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}
```

- [ ] **Step 4: Update `Program` struct**

Replace the `imports: Vec<ImportStmt>` field and add `exports`:

```rust
pub struct Program {
    pub components: Vec<ComponentDef>,
    pub stores: Vec<StoreDef>,
    pub themes: Vec<ThemeDef>,
    pub functions: Vec<FunctionDef>,
    pub structs: Vec<StructDef>,
    pub type_aliases: Vec<TypeAlias>,
    pub tagged_unions: Vec<TaggedUnionDef>,
    pub imports: Vec<ImportStmt>,
    pub exports: Vec<ExportFrom>,
}
```

- [ ] **Step 5: Update all existing code that constructs these structs**

Search for `FunctionDef {` and `StructDef {` constructors across the parser and add `visibility: Visibility::Private` as default. Search for `Program {` and add `exports: vec![]`.

- [ ] **Step 6: Verify compilation**

```bash
cd omni-compiler && cargo check
```

Expected: compiles with warnings only (new variants not yet handled in match statements).

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/src/parser/ast.rs omni-compiler/src/parser/*.rs omni-compiler/src/semantic/*.rs omni-compiler/src/ir/*.rs
git commit -m "feat(rust-ast): add Visibility, ImportStmt enum variants, ExportFrom, update Program"
```

---

### Task 1.3: Rewrite Rust parser for new import syntax

**Files:**
- Modify: `omni-compiler/src/parser/top_level.rs` (parse_import function around line 560)
- Modify: `omni-compiler/src/parser/mod.rs` (parse_program dispatch)

- [ ] **Step 1: Add import dispatch helper in `top_level.rs`**

Add a function that dispatches to the right parser based on what follows `import`:

```rust
/// Parse any import variant after consuming the `import` token.
pub fn parse_import(&mut self) -> Result<ImportStmt, ParseError> {
    self.advance(); // consume 'import'

    // Check for: import omniview;  (bare library import)
    if self.peek_kind() == TokenKind::Ident
        && !self.peek_is_brace()
    {
        let name = self.expect_ident("expected library name")?;
        // Only 'omniview' is a valid bare import
        if name != "omniview" {
            return Err(self.error(format!(
                "unknown library import '{}'. Only 'import omniview;' is valid",
                name
            )));
        }
        self.expect_token(TokenKind::Semicolon, "expected ';'")?;
        return Ok(ImportStmt::Lib(name));
    }

    // Check for: import type { ... } from "..."
    if self.peek_kind() == TokenKind::Type {
        self.advance(); // consume 'type'
        let names = self.parse_import_name_list()?;
        self.expect_token(TokenKind::From, "expected 'from'")?;
        let path = self.expect_string("expected import path string")?;
        self.consume_semicolon();
        return Ok(ImportStmt::TypeOnly(TypeImport {
            names,
            path,
            span: self.span(),
        }));
    }

    // Check for: import { A, B } from "./path.omni";
    if self.peek_kind() == TokenKind::LBrace {
        let names = self.parse_import_name_list()?;
        self.expect_token(TokenKind::From, "expected 'from'")?;
        let path = self.expect_string("expected import path string")?;
        self.consume_semicolon();
        return Ok(ImportStmt::Named(NamedImport {
            names,
            path,
            span: self.span(),
        }));
    }

    // Check for: import "./path.omni" as name;
    if self.peek_kind() == TokenKind::StringLiteral(_) {
        let path = self.expect_string("expected import path string")?;
        self.expect_token(TokenKind::As, "expected 'as' after path in namespace import")?;
        let alias = self.expect_ident("expected namespace alias")?;
        self.consume_semicolon();
        return Ok(ImportStmt::Namespace(NamespaceImport {
            path,
            alias,
            span: self.span(),
        }));
    }

    Err(self.error("expected '{{', 'type', string literal, or library name after 'import'"))
}

/// Parse { Name1, Name2 } or Name1, Name2
fn parse_import_name_list(&mut self) -> Result<Vec<String>, ParseError> {
    let mut names = Vec::new();
    let has_braces = self.peek_kind() == TokenKind::LBrace;
    if has_braces {
        self.advance(); // consume '{'
    }
    names.push(self.expect_ident("expected import name")?);
    while self.peek_kind() == TokenKind::Comma {
        self.advance(); // consume ','
        names.push(self.expect_ident("expected import name")?);
    }
    if has_braces {
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
    }
    Ok(names)
}
```

- [ ] **Step 2: Add `parse_export` function**

```rust
/// Parse: export { A, B } from "./path.omni";
pub fn parse_export(&mut self) -> Result<ExportFrom, ParseError> {
    self.advance(); // consume 'export'
    let names = self.parse_import_name_list()?;
    self.expect_token(TokenKind::From, "expected 'from'")?;
    let path = self.expect_string("expected export path string")?;
    self.consume_semicolon();
    Ok(ExportFrom {
        names,
        path,
        span: self.span(),
    })
}
```

- [ ] **Step 3: Add helper methods to Parser**

In `omni-compiler/src/parser/mod.rs`, add:

```rust
fn peek_is_brace(&self) -> bool {
    matches!(self.peek_kind(), TokenKind::LBrace)
}

fn consume_semicolon(&mut self) {
    if self.peek_kind() == TokenKind::Semicolon {
        self.advance();
    }
}

fn expect_string(&mut self, msg: &str) -> Result<String, ParseError> {
    let t = self.advance();
    match &t.kind {
        TokenKind::StringLiteral(s) => Ok(s.clone()),
        _ => Err(self.error(msg)),
    }
}

pub fn error(&self, msg: impl Into<String>) -> ParseError {
    ParseError {
        message: msg.into(),
        pos: self.pos,
    }
}
```

- [ ] **Step 4: Update `parse_program` in `mod.rs`**

Add `Export` token dispatch in the top-level match:

```rust
TokenKind::Export => {
    let exp = self.parse_export()?;
    program.exports.push(exp);
}
```

- [ ] **Step 5: Update `parse_function_def` to handle `pub`**

In `parse_function_def` (in `top_level.rs`), check for `Pub` token before `Fn`:

```rust
pub fn parse_function_def(&mut self) -> Result<FunctionDef, ParseError> {
    let visibility = if self.peek_kind() == TokenKind::Pub {
        self.advance();
        Visibility::Public
    } else {
        Visibility::Private
    };
    self.expect_token(TokenKind::Fn, "expected 'fn'")?;
    // ... rest of existing function parsing
    // Add `visibility` to the FunctionDef constructor
}
```

- [ ] **Step 6: Update `parse_struct_def` to handle `pub`**

Same pattern — check for `Pub` before `Struct`:

```rust
pub fn parse_struct_def(&mut self) -> Result<StructDef, ParseError> {
    let visibility = if self.peek_kind() == TokenKind::Pub {
        self.advance();
        Visibility::Public
    } else {
        Visibility::Private
    };
    // ... rest
}
```

- [ ] **Step 6a: Add `parse_type_alias` with `pub` support**

```rust
/// Parse: [pub] type Name = TypeAnnotation;
pub fn parse_type_alias(&mut self) -> Result<TypeAlias, ParseError> {
    let visibility = if self.peek_kind() == TokenKind::Pub {
        self.advance();
        Visibility::Public
    } else {
        Visibility::Private
    };
    self.expect_token(TokenKind::Type, "expected 'type'")?;
    let name = self.expect_ident("expected type alias name")?;
    self.expect_token(TokenKind::Eq, "expected '=' in type alias")?;
    let type_ann = self.parse_type_annotation()?;
    self.consume_semicolon();
    Ok(TypeAlias {
        visibility,
        name,
        type_ann,
        span: self.span(),
    })
}
```

- [ ] **Step 7: Verify `From` and `As` tokens** (also covered in Task 1.1 Step 2)

These are prerequisites for the parser. If not already present in `TokenKind`, they were added in Task 1.1.

- [ ] **Step 8: Verify compilation and fix match exhaustiveness**

```bash
cd omni-compiler && cargo check 2>&1
```

Fix any non-exhaustive match errors in semantic analysis and IR modules by adding catch-all arms. These will be replaced with proper handling in Task 3.2.

- [ ] **Step 9: Run existing parser tests (expect failures on import tests)**

```bash
cd omni-compiler && cargo test --test parser_tests
```

The existing `test_parse_import` test uses the old `import React from "react"` syntax — it will fail. This is expected.

- [ ] **Step 10: Commit**

```bash
git add omni-compiler/src/parser/top_level.rs omni-compiler/src/parser/mod.rs omni-compiler/src/lexer/token.rs omni-compiler/src/lexer/mod.rs
git commit -m "feat(rust-parser): rewrite parse_import with all variants, add parse_export, pub modifier"
```

---

### Task 1.4: Update Python grammars for new syntax

**Files:**
- Modify: `omnilang/grammar/omnilang.lark`
- Modify: `omniview/grammar.lark`

- [ ] **Step 1: Update `omnilang/grammar/omnilang.lark`**

Replace the existing import/export rules (lines 10-15) with:

```lark
program: (function_def | struct_def | tagged_union_def | class_def | interface_def | import_stmt | import_lib | import_type_stmt | import_namespace_stmt | export_stmt | export_from_stmt | impl_block)*

// ── Imports ──────────────────────────────────

// import omniview;  — library import (mode marker)
import_lib: "import" CNAME ";"

// import { A, B } from "./path.omni";
import_stmt: "import" "{" CNAME ("," CNAME)* "}" "from" STRING_LIT ";"

// import type { A, B } from "./path.omni";
import_type_stmt: "import" "type" "{" CNAME ("," CNAME)* "}" "from" STRING_LIT ";"

// import "./path.omni" as name;
import_namespace_stmt: "import" STRING_LIT "as" CNAME ";"

// ── Exports ──────────────────────────────────

// export { A, B } from "./path.omni";
export_from_stmt: "export" "{" CNAME ("," CNAME)* "}" "from" STRING_LIT ";"

// pub fn / pub struct / pub type — visibility modifier on declarations
// (pub is handled inside function_def/struct_def/type_alias rules)

export_stmt: "export" (function_def | struct_def | class_def | tagged_union_def | interface_def)

// ── Visibility ───────────────────────────────

// Add pub modifier to declarations that support it
// function_def gains optional "pub" prefix
function_def: ["pub"] "fn" CNAME generic_params? "(" [params] ")" ["->" type_ann] block
// struct_def gains optional "pub" prefix
struct_def: ["pub"] "struct" CNAME generic_params? "{" [struct_field (";" struct_field)* ";"] "}"
// type_alias: add "pub" prefix support
type_alias: ["pub"] "type" CNAME generic_params? "=" type_ann ";"
```

- [ ] **Step 2: Update `omniview/grammar.lark`**

Replace the existing program/import rules (lines 19-22) with:

```lark
program: (import_lib | import_stmt | import_type_stmt | import_namespace_stmt | export_from_stmt)* (component_def | store_def | theme_def)*

// ── Imports ──────────────────────────────────

import_lib: "import" CNAME ";"
import_stmt: "import" "{" CNAME ("," CNAME)* "}" "from" STRING_LIT ";"
import_type_stmt: "import" "type" "{" CNAME ("," CNAME)* "}" "from" STRING_LIT ";"
import_namespace_stmt: "import" STRING_LIT "as" CNAME ";"

// ── Exports ──────────────────────────────────

export_from_stmt: "export" "{" CNAME ("," CNAME)* "}" "from" STRING_LIT ";"
```

- [ ] **Step 3: Verify grammar parses correctly**

```bash
cd D:/space/OmniLang && python -c "
import lark
with open('omnilang/grammar/omnilang.lark') as f:
    g = lark.Lark(f.read(), start='program', parser='lalr')
print('omnilang grammar OK')

with open('omniview/grammar.lark') as f:
    g = lark.Lark(f.read(), start='program', parser='lalr')
print('omniview grammar OK')
"
```

Expected: both print "OK" without errors.

- [ ] **Step 4: Commit**

```bash
git add omnilang/grammar/omnilang.lark omniview/grammar.lark
git commit -m "feat(grammar): add namespace import, export from, import type, pub modifier rules"
```

---

### Task 1.5: Update Python AST nodes

**Files:**
- Modify: `omnilang/ast_nodes.py`
- Modify: `omniview/ast_nodes.py`

- [ ] **Step 1: Add new AST node dataclasses to `omnilang/ast_nodes.py`**

Add after the existing `ImportStmt` (around line 507):

```python
@dataclass(frozen=True)
class ImportNamespaceStmt:
    """import "./path.omni" as name;"""
    path: str
    alias: str
    span: Span | None = None

@dataclass(frozen=True)
class ExportFromStmt:
    """export { A, B } from "./path.omni";"""
    names: list[str]
    path: str
    span: Span | None = None

@dataclass(frozen=True)
class PubModifier:
    """pub visibility marker — not a standalone node, used as field"""
    pass
```

- [ ] **Step 2: Add `visibility` field to `FunctionDef`**

Add `is_pub: bool = False` parameter to `FunctionDef`:

```python
@dataclass(frozen=True)
class FunctionDef:
    name: str
    params: list[Param]
    return_type: TypeAnnotation | None
    body: Block
    is_async: bool = False
    is_pub: bool = False          # NEW
    is_exported: bool = False
    # ... rest
```

- [ ] **Step 3: Add `is_pub` field to `StructDef`**

```python
@dataclass(frozen=True)
class StructDef:
    name: str
    fields: list[StructField]
    is_pub: bool = False          # NEW
    # ... rest
```

- [ ] **Step 4: Add `is_pub` to `TypeAlias` if it exists, or create it**

Search for type alias node. If `TypeAlias` exists, add `is_pub: bool = False`. If not, check how `type ID = int;` is represented currently.

- [ ] **Step 5: Update `Program` to include new import types**

```python
@dataclass(frozen=True)
class Program:
    functions: list[FunctionDef] = field(default_factory=list)
    # ... existing
    imports: list[ImportStmt] = field(default_factory=list)
    type_imports: list[ImportTypeStmt] = field(default_factory=list)      # NEW
    namespace_imports: list[ImportNamespaceStmt] = field(default_factory=list)  # NEW
    exports: list[ExportStmt] = field(default_factory=list)
    export_froms: list[ExportFromStmt] = field(default_factory=list)      # NEW
    lib_imports: list[str] = field(default_factory=list)
```

- [ ] **Step 6: Mirror changes in `omniview/ast_nodes.py`**

Apply the same new AST nodes to the OmniView AST. The OmniView `Program` should also gain the new import fields.

- [ ] **Step 7: Commit**

```bash
git add omnilang/ast_nodes.py omniview/ast_nodes.py
git commit -m "feat(python-ast): add ImportNamespaceStmt, ExportFromStmt, is_pub fields, update Program"
```

---

### Task 1.6: Update Python Lark transformers

**Files:**
- Modify: `omnilang/parser.py`
- Modify: `omniview/parser.py`

- [ ] **Step 1: Update `omnilang/parser.py` program handler**

In the `program` method, add handling for new statement types in the `for item in items` loop (around line 91):

```python
elif isinstance(item, ImportNamespaceStmt):
    namespace_imports.append(item)
elif isinstance(item, ExportFromStmt):
    export_froms.append(item)
elif isinstance(item, ImportTypeStmt):
    type_imports.append(item)
```

Update the `Program(...)` constructor call to include the new fields:

```python
return Program(
    functions=funcs,
    structs=structs,
    tagged_unions=tagged_unions,
    classes=classes,
    imports=imports,
    type_imports=type_imports,           # NEW
    namespace_imports=namespace_imports,  # NEW
    exports=exports,
    export_froms=export_froms,           # NEW
    lib_imports=lib_imports,
    span=self._span(*items),
)
```

- [ ] **Step 2: Add new transformer methods to `omnilang/parser.py`**

```python
def import_namespace_stmt(self, items):
    """Handle import_namespace_stmt: 'import' STRING_LIT 'as' CNAME ';'"""
    path = ""
    alias = ""
    for item in items:
        if isinstance(item, Token) and item.type == "STRING_LIT":
            path = str(item)[1:-1]  # strip quotes
        elif isinstance(item, Token) and item.type == "CNAME":
            alias = str(item)
    return ImportNamespaceStmt(path=path, alias=alias, span=self._span(*items))

def import_type_stmt(self, items):
    """Handle import_type_stmt: 'import' 'type' '{' CNAME (',' CNAME)* '}' 'from' STRING_LIT ';'"""
    names = []
    path = ""
    for item in items:
        if isinstance(item, Token) and item.type == "CNAME":
            names.append(str(item))
        elif isinstance(item, Token) and item.type == "STRING_LIT":
            path = str(item)[1:-1]
    return ImportTypeStmt(names=names, path=path, span=self._span(*items))

def export_from_stmt(self, items):
    """Handle export_from_stmt: 'export' '{' CNAME (',' CNAME)* '}' 'from' STRING_LIT ';'"""
    names = []
    path = ""
    for item in items:
        if isinstance(item, Token) and item.type == "CNAME":
            names.append(str(item))
        elif isinstance(item, Token) and item.type == "STRING_LIT":
            path = str(item)[1:-1]
    return ExportFromStmt(names=names, path=path, span=self._span(*items))
```

- [ ] **Step 3: Handle `pub` modifier in function_def and struct_def**

In `function_def`, check for the `"pub"` keyword at the start of items. Add `is_pub=True` if present:

```python
def function_def(self, items):
    # items may start with Token('pub') if declared pub
    is_pub = False
    if items and isinstance(items[0], Token) and str(items[0]) == "pub":
        is_pub = True
        items = items[1:]  # skip pub token
    # ... rest of existing parsing, pass is_pub=is_pub to FunctionDef(...)
```

Same pattern for `struct_def`.

- [ ] **Step 4: Apply same changes to `omniview/parser.py`**

Mirror the transformer method additions for `import_namespace_stmt`, `import_type_stmt`, and `export_from_stmt`. Update the `program` method to handle the new statement types. Note: OmniView's `program` rule already separates imports from components — ensure new import variants go into the import list, not the component list.

- [ ] **Step 5: Test parsing with a quick smoke test**

```bash
cd D:/space/OmniLang && python -c "
from omnilang.parser import parse
src = '''
import omniview;
import { add, User } from \"./lib.omni\";
import \"./math.omni\" as math;
import type { Order } from \"./types.omni\";
export { Button } from \"./ui.omni\";
pub fn main() { return 1; }
pub struct Point { x: i32; y: i32; }
'''
prog = parse(src)
print('imports:', len(prog.imports))
print('lib_imports:', prog.lib_imports)
print('type_imports:', len(prog.type_imports))
print('namespace_imports:', len(prog.namespace_imports))
print('export_froms:', len(prog.export_froms))
for f in prog.functions:
    print(f'fn {f.name} is_pub={f.is_pub}')
for s in prog.structs:
    print(f'struct {s.name} is_pub={s.is_pub}')
"
```

Expected: all counts and flags correct.

- [ ] **Step 6: Commit**

```bash
git add omnilang/parser.py omniview/parser.py
git commit -m "feat(python-parser): handle namespace import, type import, export from, pub modifier in transformers"
```

---

## Phase 2: ModuleLoader V2 (Python)

### Task 2.1: Complete `omnilang/modules/loader.py` import extraction

**Files:**
- Modify: `omnilang/modules/loader.py`

- [ ] **Step 1: Implement `_extract_imports` method**

Replace the placeholder `_extract_imports` that currently returns `[]` with real extraction logic:

```python
def _extract_imports(self, program) -> list[str]:
    """Extract all file paths referenced in import statements.
    
    Returns list of paths as written in source (before resolution).
    """
    paths: list[str] = []
    
    # Named imports: import { A } from "./path.omni"
    for imp in getattr(program, 'imports', []):
        paths.append(imp.path)
    
    # Type-only imports: import type { A } from "./path.omni"
    for imp in getattr(program, 'type_imports', []):
        paths.append(imp.path)
    
    # Namespace imports: import "./path.omni" as name
    for imp in getattr(program, 'namespace_imports', []):
        paths.append(imp.path)
    
    # Export-from: export { A } from "./path.omni" — also a dependency
    for exp in getattr(program, 'export_froms', []):
        paths.append(exp.path)
    
    return paths
```

- [ ] **Step 2: Also implement `_extract_exports` properly**

```python
def _extract_exports(self, program) -> dict[str, str]:
    """Build map of exported name -> source file path.
    
    For omnilang (pure logic): exports are explicitly declared via `export { X } from`.
    Functions/structs with `is_pub=True` (but no `export`) are visible for import
    from THIS file but not re-exported.
    """
    exports: dict[str, str] = {}
    
    # Functions with `is_pub=True` are exportable from this file
    for func in getattr(program, 'functions', []):
        if getattr(func, 'is_pub', False) or getattr(func, 'is_exported', False):
            exports[func.name] = "(this file)"
    
    # Structs with `is_pub=True`
    for struct in getattr(program, 'structs', []):
        if getattr(struct, 'is_pub', False):
            exports[struct.name] = "(this file)"
    
    # Re-exports: export { Name } from "./other.omni"
    for exp_from in getattr(program, 'export_froms', []):
        for name in exp_from.names:
            exports[name] = exp_from.path
    
    return exports
```

- [ ] **Step 3: Verify with a test case**

```bash
cd D:/space/OmniLang && python -c "
from omnilang.modules.loader import ModuleLoader
loader = ModuleLoader(Path('.'))
# Just verify the methods don't crash on a parsed program
from omnilang.parser import parse
src = 'pub fn add(a: i32, b: i32) -> i32 { a + b }'
prog = parse(src)
print('imports:', loader._extract_imports(prog))
print('exports:', loader._extract_exports(prog))
"
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/modules/loader.py
git commit -m "feat(loader): implement _extract_imports and _extract_exports for omnilang"
```

---

### Task 2.2: Build `{path: source}` map in OmniView ModuleLoader

**Files:**
- Modify: `omniview/modules/loader.py`

- [ ] **Step 1: Add `build_source_map` method with path mapping**

Add to the `ModuleLoader` class:

```python
def build_source_map(self, entry_file: str | Path) -> tuple[dict[str, str], dict[str, str]]:
    """Load all transitive dependencies and return (source_map, path_mapping).
    
    source_map: {resolved_absolute_path: source_text}
    path_mapping: {original_import_path: resolved_absolute_path}
    
    The path_mapping is critical for the Rust compiler: import statements in
    source code contain unresolved paths like "./lib.omni" or "@components/button.omni",
    but source_map is keyed by resolved absolute paths. The Rust compiler uses
    path_mapping to bridge the two during cross-file validation.
    
    Args:
        entry_file: Path to entry .omni file.
    
    Returns:
        Tuple of (source_map, path_mapping).
    """
    self.load(entry_file)  # populate self._loaded
    
    source_map: dict[str, str] = {}
    path_mapping: dict[str, str] = {}
    
    for file_path, module in self._loaded.items():
        abs_path = str(file_path.resolve())
        source_map[abs_path] = module.source
        
        # Map each original import path to its resolved absolute path
        for imp in module.imports:
            # imp is the original path as written in source, e.g. "./lib.omni"
            resolved = self.resolver.resolve(imp, file_path)
            if resolved:
                path_mapping[imp] = str(resolved.path.resolve())
    
    return source_map, path_mapping
```

- [ ] **Step 2: Update `load` method return type**

Add source text preservation to `LoadedModule` if not already present (check line 18):

```python
@dataclass
class LoadedModule:
    path: Path
    source: str            # ensure this field exists
    program: Any           # AST
    ir_map: dict | None = None
    imports: list[str] = field(default_factory=list)
    exported: list[str] = field(default_factory=list)
    import_lines: dict[str, int] = field(default_factory=dict)
```

- [ ] **Step 3: Verify source map and path mapping**

```bash
cd D:/space/OmniLang && python -c "
from pathlib import Path
from omniview.modules.loader import ModuleLoader
import tempfile, os

# Create temp test files
d = Path(tempfile.mkdtemp())
f1 = d / 'main.omni'
f1.write_text('import omniview;\nimport { Button } from \"./button.omni\";\ncomponent App { view { <Text value=\"hi\"/> } }')
f2 = d / 'button.omni'
f2.write_text('import omniview;\ncomponent Button { prop label: string; view { <button>{label}</button> } }')

loader = ModuleLoader(str(d))
source_map, path_mapping = loader.build_source_map(str(f1))
print('Files loaded:', len(source_map))
for path in source_map:
    print(f'  source: {path}')
print('Path mappings:')
for orig, resolved in path_mapping.items():
    print(f'  {orig} → {resolved}')
"
```

- [ ] **Step 4: Commit**

```bash
git add omniview/modules/loader.py
git commit -m "feat(loader): add build_source_map for Rust DLL input"
```

---

### Task 2.3: Enforce logic↛view boundary in ModuleLoader

**Files:**
- Modify: `omniview/modules/loader.py`

- [ ] **Step 1: Add boundary check helper**

```python
def _is_view_file(self, source: str) -> bool:
    """Check if a .omni source file is a view (UI) file.
    
    A file is a view file if it contains `import omniview;`.
    """
    return 'import omniview' in source

def _check_boundary(self, from_file: Path, to_file: Path, to_source: str):
    """Enforce: logic files cannot import view files.
    
    Raises ValueError if a logic file tries to import a view file.
    """
    from_source = self._loaded.get(from_file)
    if from_source is None:
        return
    
    from_is_view = self._is_view_file(from_source.source)
    to_is_view = self._is_view_file(to_source)
    
    if not from_is_view and to_is_view:
        raise ValueError(
            f"logic file '{from_file.name}' cannot import view file '{to_file.name}'. "
            f"View files can import logic files, but not the reverse."
        )
```

- [ ] **Step 2: Call boundary check during `_load_recursive`**

Find where imports are resolved and loaded recursively (around line 140 in `_load_recursive`). After reading the dependency source, call:

```python
# After reading dep_source, before parsing
self._check_boundary(file_path, resolved_path, dep_source)
```

- [ ] **Step 3: Verify boundary enforcement**

```bash
cd D:/space/OmniLang && python -c "
from pathlib import Path
from omniview.modules.loader import ModuleLoader
import tempfile

d = Path(tempfile.mkdtemp())
# logic file importing view file — should fail
f1 = d / 'main.omni'
f1.write_text('import { Button } from \"./button.omni\";\nfn main() { return 1; }')
f2 = d / 'button.omni'
f2.write_text('import omniview;\ncomponent Button { prop label: string; view { <button>{label}</button> } }')

try:
    loader = ModuleLoader(str(d))
    loader.load(str(f1))
    print('ERROR: should have raised')
except ValueError as e:
    print(f'OK: {e}')
"
```

Expected: prints "OK: logic file ... cannot import view file ..."

- [ ] **Step 4: Commit**

```bash
git add omniview/modules/loader.py
git commit -m "feat(loader): enforce logic→view import boundary"
```

---

### Task 2.4: Enforce `.omni` extension and update resolver

**Files:**
- Modify: `omnilang/modules/resolver.py`

- [ ] **Step 1: Add mandatory `.omni` extension check**

Replace the `_ensure_omni` method with a validation approach:

```python
def resolve(self, import_path: str, from_file: Path | None = None) -> ModulePath | None:
    """Resolve an import path to a ModulePath.
    
    The import path MUST have a .omni extension. This is enforced here.
    Returns None if the file cannot be found.
    """
    # Validate: .omni extension is mandatory
    if not import_path.endswith('.omni'):
        # Allow bare 'omniview' as a library marker (not a file import)
        if import_path == 'omniview':
            return None  # handled by caller
        return None  # invalid — missing .omni extension
    
    # Path aliases: @alias/pkg.omni
    if import_path.startswith('@'):
        parts = import_path.split('/', 1)
        alias_key = parts[0]
        if len(parts) == 1:
            return None
        subpath = parts[1]
        
        if alias_key not in self.aliases:
            return None  # alias not found
        
        base = self.aliases[alias_key]
        resolved = Path(base) / subpath
        return ModulePath(
            path=resolved.resolve(),
            display=import_path,
            is_std=(alias_key == '@std'),
        )
    
    # Relative paths: ./foo.omni or ../foo.omni
    if import_path.startswith('./') or import_path.startswith('../'):
        base = from_file.parent if from_file else self.project_root
        resolved = (base / import_path).resolve()
        return ModulePath(
            path=resolved,
            display=import_path,
            is_std=False,
        )
    
    # Standard library: omit @std prefix if bare std/
    # (all std imports should use @std/ prefix now)
    
    # Project-relative (fallback)
    resolved = (self.project_root / import_path).resolve()
    return ModulePath(
        path=resolved,
        display=import_path,
        is_std=False,
    )

def add_alias(self, name: str, target: str):
    """Register a path alias (from .omnilang.toml [aliases] section)."""
    self.aliases[name] = target

def add_aliases(self, aliases: dict[str, str]):
    """Register multiple path aliases at once."""
    self.aliases.update(aliases)
```

- [ ] **Step 2: Verify resolver changes**

```bash
cd D:/space/OmniLang && python -c "
from pathlib import Path
from omnilang.modules.resolver import ModuleResolver

r = ModuleResolver(Path('.'), aliases={'@lib': './src/lib'}, std_path=Path('stdlib'))
# Test .omni enforcement
result = r.resolve('noext', Path('test.omni'))
print('No extension:', result)  # should be None

result = r.resolve('./lib.omni', Path('test.omni'))
print('Valid relative:', result)

# Test alias
result = r.resolve('@lib/utils.omni', Path('test.omni'))
print('Alias:', result)
"
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/modules/resolver.py
git commit -m "feat(resolver): enforce .omni extension, support config-based aliases"
```

---

## Phase 3: Cross-File Semantic Analysis (Rust)

### Task 3.1: Update Rust compiler API for multi-file validation

**Files:**
- Modify: `omni-compiler/src/lib.rs`

**Design note:** The Rust DLL does **parsing + cross-file validation + cycle detection only**. It does NOT do codegen. Codegen for all 8 targets is implemented in Python (Phase 4). This is because all existing codegen infrastructure already lives in Python (`omnilang/codegen/*.py`, `omniview/codegen/*.py`), and codegen is mostly string manipulation that's easier to maintain in Python. The Rust DLL's role is the computationally intensive work: parsing all files, building symbol tables, and validating cross-file references.

- [ ] **Step 1: Add multi-file validation function**

Add a new public function that accepts multiple sources with resolved paths:

```rust
use std::collections::HashMap;

/// Result of multi-file validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct MultiFileResult {
    pub success: bool,
    /// Parsed programs keyed by resolved absolute path (returned even on validation
    /// errors so Python can use them for codegen if only warnings exist).
    pub programs: HashMap<String, Program>,
    pub errors: Vec<String>,
}

/// Validate multiple .omni source files with cross-file checks.
///
/// This is the primary API for the Python framework. Python does all file I/O
/// and path resolution, builds:
///   - `sources`: HashMap<ResolvedAbsolutePath, SourceText>
///   - `path_mapping`: HashMap<OriginalImportPath, ResolvedAbsolutePath>
///
/// The `path_mapping` is critical: import statements in source code contain
/// unresolved paths like "./lib.omni" or "@components/button.omni", but the
/// `sources` map is keyed by resolved absolute paths. The mapping bridges
/// the two so cross-file validation can find target files.
///
/// The compiler parses all files, resolves cross-file imports using the mapping,
/// validates visibility and cycles. Codegen is done separately in Python.
pub fn validate_multi(
    sources: HashMap<String, String>,
    path_mapping: HashMap<String, String>,
) -> MultiFileResult {
    let mut errors: Vec<String> = Vec::new();
    let mut programs: HashMap<String, Program> = HashMap::new();

    // Phase 1: Parse all sources
    for (path, source) in &sources {
        match compile_single_parse(source) {
            Ok(program) => {
                programs.insert(path.clone(), program);
            }
            Err(e) => {
                errors.push(format!("{}: parse error: {}", path, e));
            }
        }
    }

    if !errors.is_empty() {
        return MultiFileResult {
            success: false,
            programs,
            errors,
        };
    }

    // Phase 2: Per-file consistency checks (omniview double-check)
    for (path, program) in &programs {
        if let Err(file_errors) = validate_file_consistency(path, program, &sources) {
            errors.extend(file_errors);
        }
    }

    // Phase 3: Cross-file import validation
    if let Err(more_errors) = validate_cross_file(&programs, &path_mapping) {
        errors.extend(more_errors);
    }

    // Phase 4: Cycle detection
    let cycles = detect_cycles(&programs, &path_mapping);
    for cycle in &cycles {
        errors.push(format!(
            "circular import detected: {}",
            cycle.join(" → ")
        ));
    }

    MultiFileResult {
        success: errors.is_empty(),
        programs,
        errors,
    }
}

fn compile_single_parse(source: &str) -> Result<Program, String> {
    let tokens = tokenize(source);
    parse(&tokens).map_err(|e| e.message)
}
```

- [ ] **Step 2: Add `validate_file_consistency` (omniview double-check)**

Per spec §1.1: if a file has `import omniview;` but no UI declarations, or has UI declarations but no `import omniview;`, it's an error.

```rust
/// Check per-file consistency: omniview marker must match file contents.
fn validate_file_consistency(
    file_path: &str,
    program: &Program,
    sources: &HashMap<String, String>,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    
    let has_omniview_marker = program.imports.iter().any(|imp| {
        matches!(imp, ImportStmt::Lib(name) if name == "omniview")
    });
    
    let has_ui_declarations = !program.components.is_empty()
        || !program.stores.is_empty()
        || !program.themes.is_empty();
    
    if has_omniview_marker && !has_ui_declarations {
        errors.push(format!(
            "{}: has 'import omniview;' but no component/store/theme declarations. \
             Either add UI declarations or remove the omniview import.",
            file_path
        ));
    }
    
    if !has_omniview_marker && has_ui_declarations {
        errors.push(format!(
            "{}: has UI declarations (component/store/theme) but missing 'import omniview;'. \
             Add 'import omniview;' to mark this as a view file.",
            file_path
        ));
    }
    
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

- [ ] **Step 3: Add `validate_cross_file` using path_mapping**

```rust
fn validate_cross_file(
    programs: &HashMap<String, Program>,
    path_mapping: &HashMap<String, String>,  // original_import_path → resolved_absolute_path
) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();

    // Build pub-visible symbol table for each file
    let mut pub_symbols: HashMap<&str, HashMap<String, SymbolKind>> = HashMap::new();
    
    for (path, program) in programs {
        let mut symbols = HashMap::new();
        for func in &program.functions {
            if func.visibility == Visibility::Public {
                symbols.insert(func.name.clone(), SymbolKind::Function);
            }
        }
        for strukt in &program.structs {
            if strukt.visibility == Visibility::Public {
                symbols.insert(strukt.name.clone(), SymbolKind::Struct);
            }
        }
        for ta in &program.type_aliases {
            if ta.visibility == Visibility::Public {
                symbols.insert(ta.name.clone(), SymbolKind::TypeAlias);
            }
        }
        // components, stores, themes are always public
        for comp in &program.components {
            symbols.insert(comp.name.clone(), SymbolKind::Component);
        }
        for store in &program.stores {
            symbols.insert(store.name.clone(), SymbolKind::Store);
        }
        // Re-exports
        for exp in &program.exports {
            for name in &exp.names {
                symbols.insert(name.clone(), SymbolKind::ReExport(exp.path.clone()));
            }
        }
        pub_symbols.insert(path.as_str(), symbols);
    }

    // Validate each import in each file
    for (file_path, program) in programs {
        for import in &program.imports {
            match import {
                ImportStmt::Named(named) => {
                    // Resolve the import path using the mapping from Python
                    let resolved = path_mapping.get(&named.path);
                    match resolved {
                        Some(target) => {
                            errors.extend(validate_named_import(
                                file_path, &named.names, target,
                                programs, &pub_symbols,
                            ));
                        }
                        None => {
                            errors.push(format!(
                                "{}: could not resolve import path '{}'",
                                file_path, named.path
                            ));
                        }
                    }
                }
                ImportStmt::Namespace(ns) => {
                    let resolved = path_mapping.get(&ns.path);
                    match resolved {
                        Some(target) => {
                            if !programs.contains_key(target.as_str()) {
                                errors.push(format!(
                                    "{}: imported file '{}' (resolved to '{}') not found in compilation",
                                    file_path, ns.path, target
                                ));
                            }
                        }
                        None => {
                            errors.push(format!(
                                "{}: could not resolve import path '{}'",
                                file_path, ns.path
                            ));
                        }
                    }
                }
                ImportStmt::TypeOnly(ti) => {
                    let resolved = path_mapping.get(&ti.path);
                    match resolved {
                        Some(target) => {
                            errors.extend(validate_named_import(
                                file_path, &ti.names, target,
                                programs, &pub_symbols,
                            ));
                        }
                        None => {
                            errors.push(format!(
                                "{}: could not resolve import path '{}'",
                                file_path, ti.path
                            ));
                        }
                    }
                }
                ImportStmt::Lib(_) => {
                    // 'import omniview;' — validated in validate_file_consistency
                }
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

- [ ] **Step 4: Add helper types and validation function**

```rust
#[derive(Debug, Clone, PartialEq)]
enum SymbolKind {
    Function,
    Struct,
    TypeAlias,
    Component,
    Store,
    Theme,
    ReExport(String), // re-exports point to another file
}

fn validate_named_import(
    importing_file: &str,
    names: &[String],
    target_path: &str,  // now a resolved absolute path (from path_mapping)
    programs: &HashMap<String, Program>,
    pub_symbols: &HashMap<&str, HashMap<String, SymbolKind>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    
    let target_symbols = match pub_symbols.get(target_path) {
        Some(s) => s,
        None => {
            errors.push(format!(
                "{}: imported file '{}' not found in compilation",
                importing_file, target_path
            ));
            return errors;
        }
    };
    
    for name in names {
        if !target_symbols.contains_key(name) {
            errors.push(format!(
                "{}: '{}' is not pub-visible in '{}'",
                importing_file, name, target_path
            ));
        }
    }
    
    errors
}
```

- [ ] **Step 5: Add Tarjan SCC cycle detection**

Create `omni-compiler/src/semantic/cycles.rs`:

```rust
use std::collections::{HashMap, HashSet};
use crate::parser::ast::{Program, ImportStmt};

/// Detect circular imports using Tarjan's strongly connected components algorithm.
/// `path_mapping` resolves each import's path to the canonical file key used in `programs`.
/// Returns list of cycles found (each cycle is a list of file paths).
pub fn detect_cycles(
    programs: &HashMap<String, Program>,
    path_mapping: &HashMap<String, String>,
) -> Vec<Vec<String>> {
    let file_paths: Vec<&String> = programs.keys().collect();
    let path_to_idx: HashMap<&String, usize> = file_paths
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, i))
        .collect();
    
    let n = file_paths.len();
    let mut graph: Vec<Vec<usize>> = vec![vec![]; n];
    
    // Build adjacency list from imports, resolving through path_mapping
    for (path, program) in programs {
        let from = path_to_idx[path];
        for import in &program.imports {
            let import_path = match import {
                ImportStmt::Named(n) => &n.path,
                ImportStmt::Namespace(ns) => &ns.path,
                ImportStmt::TypeOnly(t) => &t.path,
                ImportStmt::Lib(_) => continue,
            };
            // Resolve import path to canonical key
            if let Some(resolved) = path_mapping.get(import_path) {
                if let Some(&to) = path_to_idx.get(resolved) {
                    graph[from].push(to);
                }
            }
        }
        // Also add edges for re-exports
        for exp in &program.exports {
            if let Some(resolved) = path_mapping.get(&exp.path) {
                if let Some(&to) = path_to_idx.get(resolved) {
                    graph[from].push(to);
                }
            }
        }
    }
    
    // Tarjan's SCC
    struct State {
        index: usize,
        stack: Vec<usize>,
        on_stack: HashSet<usize>,
        indices: Vec<Option<usize>>,
        lowlinks: Vec<usize>,
        cycles: Vec<Vec<String>>,
    }
    
    let mut state = State {
        index: 0,
        stack: Vec::new(),
        on_stack: HashSet::new(),
        indices: vec![None; n],
        lowlinks: vec![0; n],
        cycles: Vec::new(),
    };
    
    fn strongconnect(
        v: usize,
        graph: &[Vec<usize>],
        file_paths: &[&String],
        state: &mut State,
    ) {
        state.indices[v] = Some(state.index);
        state.lowlinks[v] = state.index;
        state.index += 1;
        state.stack.push(v);
        state.on_stack.insert(v);
        
        for &w in &graph[v] {
            if state.indices[w].is_none() {
                strongconnect(w, graph, file_paths, state);
                state.lowlinks[v] = state.lowlinks[v].min(state.lowlinks[w]);
            } else if state.on_stack.contains(&w) {
                state.lowlinks[v] = state.lowlinks[v].min(state.indices[w].unwrap());
            }
        }
        
        if state.lowlinks[v] == state.indices[v].unwrap() {
            let mut scc = Vec::new();
            loop {
                let w = state.stack.pop().unwrap();
                state.on_stack.remove(&w);
                scc.push(file_paths[w].to_string());
                if w == v {
                    break;
                }
            }
            if scc.len() > 1 {
                state.cycles.push(scc);
            }
        }
    }
    
    for v in 0..n {
        if state.indices[v].is_none() {
            strongconnect(v, &graph, &file_paths, &mut state);
        }
    }
    
    state.cycles
}
```

- [ ] **Step 6: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Fix any compilation errors.

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/src/lib.rs omni-compiler/src/semantic/cycles.rs omni-compiler/src/semantic/mod.rs
git commit -m "feat(rust-semantic): multi-file validation API, omniview double-check, cross-file validation with path_mapping, Tarjan cycle detection"
```

---

### Task 3.2: Update semantic analysis and IR lowering for new AST variants

**Files:**
- Modify: `omni-compiler/src/semantic/mod.rs`
- Modify: `omni-compiler/src/ir/mod.rs`

**Context:** Task 1.3 added `todo!()` catch-all arms in semantic analysis and IR lowering for the new `ImportStmt` enum variants and `ExportFrom`. This task replaces those with proper handling so the compiler doesn't panic at runtime.

- [ ] **Step 1: Handle imports in semantic analysis passes**

In `omni-compiler/src/semantic/mod.rs`, update each analysis pass to process the new import variants:

```rust
// In the symbol collection pass (Pass 1), process all import variants:
for import in &program.imports {
    match import {
        ImportStmt::Named(named) => {
            // Register imported names in the symbol table so they can be
            // resolved when used in expressions. The names are validated
            // by validate_cross_file before semantic analysis runs.
            for name in &named.names {
                symbols.declare(name, SymbolInfo {
                    kind: SymbolKind::Import,
                    source_file: named.path.clone(),
                    is_pub: false, // imports are file-local unless re-exported
                });
            }
        }
        ImportStmt::Namespace(ns) => {
            // Register the namespace alias as a module reference
            symbols.declare(&ns.alias, SymbolInfo {
                kind: SymbolKind::Namespace,
                source_file: ns.path.clone(),
                is_pub: false,
            });
        }
        ImportStmt::TypeOnly(ti) => {
            // Type-only imports: register in type scope only
            for name in &ti.names {
                symbols.declare_type(name, SymbolInfo {
                    kind: SymbolKind::TypeImport,
                    source_file: ti.path.clone(),
                    is_pub: false,
                });
            }
        }
        ImportStmt::Lib(name) => {
            // 'import omniview;' — no symbol registration needed
            // (validated by validate_file_consistency)
        }
    }
}

// Process re-exports from this file:
for exp in &program.exports {
    for name in &exp.names {
        symbols.declare(name, SymbolInfo {
            kind: SymbolKind::ReExport,
            source_file: exp.path.clone(),
            is_pub: true,
        });
    }
}
```

- [ ] **Step 2: Handle `pub` visibility in semantic analysis**

Update function/struct/type_alias registration to check `visibility`:

```rust
for func in &program.functions {
    symbols.declare(&func.name, SymbolInfo {
        kind: SymbolKind::Function,
        is_pub: func.visibility == Visibility::Public,
        // ...
    });
}
for strukt in &program.structs {
    symbols.declare(&strukt.name, SymbolInfo {
        kind: SymbolKind::Struct,
        is_pub: strukt.visibility == Visibility::Public,
        // ...
    });
}
for ta in &program.type_aliases {
    symbols.declare(&ta.name, SymbolInfo {
        kind: SymbolKind::TypeAlias,
        is_pub: ta.visibility == Visibility::Public,
        // ...
    });
}
```

- [ ] **Step 3: Update IR lowering to skip imports**

In `omni-compiler/src/ir/mod.rs`, the IR lowering pass should skip import statements (they have no runtime representation — they're resolved at compile time):

```rust
// In the AST→IR lowering loop, skip all import variants:
for item in &program.imports {
    // Imports are fully resolved during semantic analysis.
    // No IR instructions are generated for import statements.
    match item {
        ImportStmt::Named(_) | ImportStmt::Namespace(_)
        | ImportStmt::TypeOnly(_) | ImportStmt::Lib(_) => {}
    }
}
// Re-exports also generate no IR:
for _exp in &program.exports {
    // Re-exports are a compile-time construct only.
}
```

- [ ] **Step 4: Update single-file FFI functions**

The existing `omni_compile_to_ir` and `omni_compile_to_omniview` C FFI functions in `lib.rs` use the single-file `compile()` pipeline. Ensure they handle the new AST variants gracefully (skip imports, respect visibility):

```rust
// In the single-file compile() function, after parsing:
// Skip imports (they can't be validated in single-file mode, so emit a warning)
for import in &program.imports {
    match import {
        ImportStmt::Named(n) | ImportStmt::TypeOnly(n) => {
            // Single-file mode: imports can't be validated, emit warning
            // but don't fail — the multi-file API should be used instead
        }
        ImportStmt::Namespace(ns) => { /* same */ }
        ImportStmt::Lib(_) => { /* mode marker, already handled */ }
    }
}
```

- [ ] **Step 5: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: no `todo!()` panics remain, all match arms covered.

- [ ] **Step 6: Run full test suite**

```bash
cd omni-compiler && cargo test
```

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/src/semantic/mod.rs omni-compiler/src/ir/mod.rs omni-compiler/src/lib.rs
git commit -m "feat(rust-semantic): handle new import variants in semantic analysis and IR, update single-file FFI"
```

---

### Task 3.3: Add Rust parser tests for new syntax

**Files:**
- Modify: `omni-compiler/tests/parser_tests.rs`

- [ ] **Step 1: Replace old import test and add new tests**

Replace the old `test_parse_import` test (which uses outdated `import React from "react"` syntax) and add comprehensive tests:

```rust
use omni_compiler::lexer::tokenize;
use omni_compiler::parser::parse;
use omni_compiler::parser::ast::{ImportStmt, Visibility};

#[test]
fn test_parse_named_import() {
    let tokens = tokenize(r#"import { add, User } from "./lib.omni";"#);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.imports.len(), 1);
    match &program.imports[0] {
        ImportStmt::Named(n) => {
            assert_eq!(n.names, vec!["add", "User"]);
            assert_eq!(n.path, "./lib.omni");
        }
        _ => panic!("expected Named import"),
    }
}

#[test]
fn test_parse_namespace_import() {
    let tokens = tokenize(r#"import "./lib/math.omni" as math;"#);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    match &program.imports[0] {
        ImportStmt::Namespace(ns) => {
            assert_eq!(ns.path, "./lib/math.omni");
            assert_eq!(ns.alias, "math");
        }
        _ => panic!("expected Namespace import"),
    }
}

#[test]
fn test_parse_type_import() {
    let tokens = tokenize(r#"import type { User, Order } from "./types.omni";"#);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    match &program.imports[0] {
        ImportStmt::TypeOnly(t) => {
            assert_eq!(t.names, vec!["User", "Order"]);
            assert_eq!(t.path, "./types.omni");
        }
        _ => panic!("expected TypeOnly import"),
    }
}

#[test]
fn test_parse_lib_import() {
    let tokens = tokenize("import omniview;");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    match &program.imports[0] {
        ImportStmt::Lib(name) => assert_eq!(name, "omniview"),
        _ => panic!("expected Lib import"),
    }
}

#[test]
fn test_parse_export_from() {
    let tokens = tokenize(r#"export { Button, Card } from "./ui.omni";"#);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.exports.len(), 1);
    assert_eq!(program.exports[0].names, vec!["Button", "Card"]);
    assert_eq!(program.exports[0].path, "./ui.omni");
}

#[test]
fn test_parse_pub_function() {
    let tokens = tokenize("pub fn add(a: i32, b: i32) -> i32 { a + b }");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].visibility, Visibility::Public);
    assert_eq!(program.functions[0].name, "add");
}

#[test]
fn test_parse_private_function() {
    let tokens = tokenize("fn helper(x: i32) -> i32 { x * 2 }");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.functions[0].visibility, Visibility::Private);
}

#[test]
fn test_parse_pub_struct() {
    let tokens = tokenize("pub struct Point { x: i32; y: i32; }");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.structs.len(), 1);
    assert_eq!(program.structs[0].visibility, Visibility::Public);
}
```

- [ ] **Step 2: Run all parser tests**

```bash
cd omni-compiler && cargo test --test parser_tests
```

Expected: all tests pass (fix any issues).

- [ ] **Step 3: Run full Rust test suite**

```bash
cd omni-compiler && cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/tests/parser_tests.rs
git commit -m "test(rust-parser): comprehensive import/export/pub tests (8 new tests)"
```

---

## Phase 4: Codegen

### Task 4.1: Update JavaScript/TypeScript codegen

**Files:**
- Modify: `omnilang/codegen/javascript.py`

- [ ] **Step 1: Update `_gen_imports_exports` method**

Replace the existing method (around line 637) with full import/export handling:

```python
def _gen_imports_exports(self, prog: Program):
    """Generate ES module import/export statements for all import variants."""
    
    # Named imports: import { A, B } from "./path.omni"
    for imp in getattr(prog, 'imports', []):
        names = ", ".join(imp.names)
        # Strip .omni extension and replace with target extension
        js_path = imp.path.replace('.omni', '.js' if not self._ts_mode else '.ts')
        self._emit(f'import {{ {names} }} from "{js_path}";')
    
    # Namespace imports: import "./path.omni" as name
    for imp in getattr(prog, 'namespace_imports', []):
        js_path = imp.path.replace('.omni', '.js' if not self._ts_mode else '.ts')
        self._emit(f'import * as {imp.alias} from "{js_path}";')
    
    # Type-only imports: import type { A, B } from "./types.omni"
    for imp in getattr(prog, 'type_imports', []):
        if self._ts_mode:
            names = ", ".join(imp.names)
            ts_path = imp.path.replace('.omni', '.ts')
            self._emit(f'import type {{ {names} }} from "{ts_path}";')
        # In plain JS mode, type imports are omitted entirely
    
    # Re-exports: export { A, B } from "./ui.omni"
    for exp in getattr(prog, 'export_froms', []):
        names = ", ".join(exp.names)
        js_path = exp.path.replace('.omni', '.js' if not self._ts_mode else '.ts')
        self._emit(f'export {{ {names} }} from "{js_path}";')
    
    # Exported declarations (old-style export fn) — handled inline in function generators
    if getattr(prog, 'imports', []) or getattr(prog, 'namespace_imports', []):
        self._emit()  # blank line after imports
```

- [ ] **Step 2: Update function codegen for `pub` → `export`**

In the function generation, add `export` keyword for `pub` functions:

```python
def _gen_function(self, func: FunctionDef):
    # ... existing signature building ...
    export_kw = "export " if getattr(func, 'is_pub', False) else ""
    # ... prefix with export_kw ...
```

- [ ] **Step 3: Update struct codegen for `pub`**

Same pattern in struct generation:

```python
def _gen_struct_def(self, struct: StructDef):
    export_kw = "export " if getattr(struct, 'is_pub', False) else ""
    # ...
```

- [ ] **Step 4: Test with a sample program**

```bash
cd D:/space/OmniLang && python -c "
from omnilang.parser import parse
from omnilang.codegen.javascript import JavaScriptGenerator

src = '''
import { add } from \"./lib.omni\";
import \"./math.omni\" as math;
import type { User } from \"./types.omni\";
export { Button } from \"./ui.omni\";
pub fn main() { return add(1, 2); }
fn helper(x: i32) -> i32 { x * 2 }
pub struct Point { x: i32; y: i32; }
'''
prog = parse(src)
gen = JavaScriptGenerator()
output = gen.generate(prog, rename_map={}, type_map={})
print(output)
"
```

Expected: output contains `import { add } from "./lib.js";`, `import * as math from "./math.js";`, no `import type` in plain JS, `export { Button } from "./ui.js";`, `export function main()`, `function helper`, `export class Point`.

- [ ] **Step 5: Commit**

```bash
git add omnilang/codegen/javascript.py
git commit -m "feat(codegen-js): update import/export codegen for all new syntax variants"
```

---

### Task 4.2: Update Rust codegen

**Files:**
- Modify: `omnilang/codegen/rust.py`

- [ ] **Step 1: Add import/export generation to `_gen_program`**

Add before the function/struct generation loop:

```python
def _gen_imports_exports(self, prog: Program):
    """Generate Rust use statements and re-exports."""
    
    # Named imports: use crate::module::{Name1, Name2};
    for imp in getattr(prog, 'imports', []):
        module_path = imp.path.replace('.omni', '').replace('/', '::').lstrip('.')
        names = ", ".join(imp.names)
        self._emit(f'use crate::{module_path}::{{{names}}};')
    
    # Namespace imports: use crate::module as alias;
    for imp in getattr(prog, 'namespace_imports', []):
        module_path = imp.path.replace('.omni', '').replace('/', '::').lstrip('.')
        self._emit(f'use crate::{module_path} as {imp.alias};')
    
    # Type-only imports: same as named in Rust (no runtime effect)
    for imp in getattr(prog, 'type_imports', []):
        module_path = imp.path.replace('.omni', '').replace('/', '::').lstrip('.')
        names = ", ".join(imp.names)
        self._emit(f'use crate::{module_path}::{{{names}}};')
    
    # Re-exports: pub use crate::module::{Names};
    for exp in getattr(prog, 'export_froms', []):
        module_path = exp.path.replace('.omni', '').replace('/', '::').lstrip('.')
        names = ", ".join(exp.names)
        self._emit(f'pub use crate::{module_path}::{{{names}}};')
    
    if prog.imports or getattr(prog, 'namespace_imports', []):
        self._emit()
```

- [ ] **Step 2: Handle `pub` visibility on functions**

In `_gen_function`, prefix `pub` for public functions:

```python
visibility = "pub " if getattr(func, 'is_pub', False) else ""
# ... prepend visibility to fn declaration
```

- [ ] **Step 3: Handle `pub` visibility on structs**

Same for struct generation:

```python
visibility = "pub " if getattr(struct, 'is_pub', False) else ""
```

- [ ] **Step 4: Test**

```bash
cd D:/space/OmniLang && python -c "
from omnilang.parser import parse
from omnilang.codegen.rust import RustGenerator

src = 'import { add } from \"./lib.omni\"; pub fn main() { 1 } pub struct Point { x: i32; y: i32; }'
prog = parse(src)
gen = RustGenerator()
output = gen.generate(prog, rename_map={}, type_map={})
print(output)
"
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/codegen/rust.py
git commit -m "feat(codegen-rust): update import/export codegen with use statements and pub visibility"
```

---

### Task 4.3: Update Python codegen

**Files:**
- Modify: `omnilang/codegen/python.py`

- [ ] **Step 1: Add import generation to `_gen_program`**

In `_gen_program`, before generating functions:

```python
def _gen_imports(self, prog: Program):
    """Generate Python import statements."""
    
    # Named imports: from module import name1, name2
    for imp in getattr(prog, 'imports', []):
        module_path = imp.path.replace('.omni', '').replace('/', '.').lstrip('.')
        names = ", ".join(imp.names)
        self._emit(f'from {module_path} import {names}')
    
    # Namespace imports: import module as alias
    for imp in getattr(prog, 'namespace_imports', []):
        module_path = imp.path.replace('.omni', '').replace('/', '.').lstrip('.')
        self._emit(f'import {module_path} as {imp.alias}')
    
    # Type-only imports: omitted in Python (types are structural)
    # Re-exports: from module import names (Python has no true re-export)
    for exp in getattr(prog, 'export_froms', []):
        module_path = exp.path.replace('.omni', '').replace('/', '.').lstrip('.')
        names = ", ".join(exp.names)
        self._emit(f'from {module_path} import {names}')
```

- [ ] **Step 2: Python ignores `pub`** — no changes needed for visibility since Python has no native visibility control.

- [ ] **Step 3: Commit**

```bash
git add omnilang/codegen/python.py
git commit -m "feat(codegen-python): add import/export generation for all variants"
```

---

### Task 4.4: Update Dart codegen

**Files:**
- Modify: `omnilang/codegen/dart.py`

- [ ] **Step 1: Add import generation to `_gen_program`**

Before the function generation loop in `_gen_program`:

```python
# Generate imports
for imp in getattr(prog, 'imports', []):
    dart_path = imp.path.replace('.omni', '.dart')
    names = ", ".join(imp.names)
    self._emit(f"import '{dart_path}' show {names};")

for imp in getattr(prog, 'namespace_imports', []):
    dart_path = imp.path.replace('.omni', '.dart')
    self._emit(f"import '{dart_path}' as {imp.alias};")

# Type-only imports: omitted in Dart
# Re-exports:
for exp in getattr(prog, 'export_froms', []):
    dart_path = exp.path.replace('.omni', '.dart')
    names = ", ".join(exp.names)
    self._emit(f"export '{dart_path}' show {names};")
```

- [ ] **Step 2: Dart ignores `pub`** — all top-level declarations are public by default.

- [ ] **Step 3: Commit**

```bash
git add omnilang/codegen/dart.py
git commit -m "feat(codegen-dart): add import/export generation for all variants"
```

---

### Task 4.5: Update UI codegen (React, Vue, Dioxus, Flutter)

**Files:**
- Modify: `omniview/codegen/react.py`
- Modify: `omniview/codegen/vue.py`
- Modify: `omniview/codegen/dioxus.py`
- Modify: `omniview/codegen/flutter.py`

- [ ] **Step 1: Add import generation helper to `omniview/codegen/base.py`**

```python
def _gen_imports(self, prog) -> list[str]:
    """Generate import statements for UI targets. Returns list of output lines."""
    lines = []
    
    # Named imports
    for imp in getattr(prog, 'imports', []):
        names = ", ".join(imp.names)
        ext = self._target_extension()
        target_path = imp.path.replace('.omni', ext)
        lines.append(f'import {{ {names} }} from "{target_path}";')
    
    # Namespace imports
    for imp in getattr(prog, 'namespace_imports', []):
        ext = self._target_extension()
        target_path = imp.path.replace('.omni', ext)
        lines.append(f'import * as {imp.alias} from "{target_path}";')
    
    # Type-only imports: omitted in UI codegen
    # Re-exports
    for exp in getattr(prog, 'export_froms', []):
        names = ", ".join(exp.names)
        ext = self._target_extension()
        target_path = exp.path.replace('.omni', ext)
        lines.append(f'export {{ {names} }} from "{target_path}";')
    
    return lines

def _target_extension(self) -> str:
    """Override in subclasses to return target file extension."""
    return '.js'
```

- [ ] **Step 2: Update React codegen**

In `react.py`, call `_gen_imports` before component generation:

```python
def _gen_program(self, prog):
    for line in self._gen_imports(prog):
        self._emit(line)
    if prog.imports:
        self._emit()
    # ... existing component generation ...
```

- [ ] **Step 3: Update Vue codegen**

Same pattern — call `_gen_imports`, override `_target_extension` to return `.js`:

```python
def _target_extension(self) -> str:
    return '.js'
```

- [ ] **Step 4: Update Dioxus codegen**

Override `_target_extension` to return `.rs`. Adapt import format to Rust `use` syntax:

```python
def _gen_imports(self, prog) -> list[str]:
    lines = []
    for imp in getattr(prog, 'imports', []):
        module_path = imp.path.replace('.omni', '').replace('/', '::').lstrip('.')
        names = ", ".join(imp.names)
        lines.append(f'use crate::{module_path}::{{{names}}};')
    # ... etc
    return lines
```

- [ ] **Step 5: Update Flutter codegen**

Override `_target_extension` to return `.dart`. Adapt import format to Dart `import` syntax.

- [ ] **Step 6: Commit**

```bash
git add omniview/codegen/base.py omniview/codegen/react.py omniview/codegen/vue.py omniview/codegen/dioxus.py omniview/codegen/flutter.py
git commit -m "feat(codegen-ui): add import/export generation for React, Vue, Dioxus, Flutter"
```

---

## Phase 5: CLI & Config

### Task 5.1: Extend `.omnilang.toml` schema

**Files:**
- Modify: `omnilang/config.py`

- [ ] **Step 1: Add `ui_target` and `aliases` to `OmniConfig`**

```python
@dataclass
class SourceConfig:
    """[source] section in .omnilang.toml"""
    dir: str = "src"

@dataclass
class OutputConfig:
    """[output] section in .omnilang.toml"""
    dir: str = "build"

@dataclass
class OmniConfig:
    """Root configuration for an OmniLang project."""
    project_name: str = ""
    project_version: str = "0.1.0"
    
    # Target selections
    targets: list[str] = field(default_factory=lambda: ["rust", "dart", "js", "python"])
    ui_target: str = ""  # NEW: "react" | "vue" | "dioxus" | "flutter" | "omniview"
    
    # Path aliases
    aliases: dict[str, str] = field(default_factory=dict)  # NEW: @alias -> path mapping
    
    # Target-specific configs
    js: TargetJsConfig = field(default_factory=TargetJsConfig)
    rust: TargetRustConfig = field(default_factory=TargetRustConfig)
    python: TargetPythonConfig = field(default_factory=TargetPythonConfig)
    dart: TargetDartConfig = field(default_factory=TargetDartConfig)
    
    # Analyze
    analyze: AnalyzeConfig = field(default_factory=AnalyzeConfig)
    
    # Source/output (matches spec: [source] and [output] TOML sections)
    source: SourceConfig = field(default_factory=SourceConfig)
    output: OutputConfig = field(default_factory=OutputConfig)
```

- [ ] **Step 2: Update `load_config` to parse new fields**

In `load_config()`, add parsing for `ui_target` and `aliases`:

```python
raw_source = raw.get("source", {})
raw_output = raw.get("output", {})

config = OmniConfig(
    project_name=project.get("name", ""),
    project_version=project.get("version", "0.1.0"),
    targets=project.get("targets", ["rust", "dart", "js", "python"]),
    ui_target=project.get("ui_target", ""),      # NEW
    aliases=raw.get("aliases", {}),               # NEW
    source=SourceConfig(dir=raw_source.get("dir", "src")),    # NEW nested
    output=OutputConfig(dir=raw_output.get("dir", "build")),  # NEW nested
    # ... rest unchanged
)
```

- [ ] **Step 3: Update `DEFAULT_CONFIG_TEMPLATE`**

Add the new sections to the template:

```python
DEFAULT_CONFIG_TEMPLATE = """\
[project]
name = "{name}"
version = "0.1.0"
# targets = ["rust", "dart", "js", "python"]
# ui_target = "react"       # "react" | "vue" | "dioxus" | "flutter" | "omniview"

[aliases]
# "@components" = "./src/components"
# "@lib" = "./src/lib"

[source]
dir = "src"

[output]
dir = "build"

# ── Target-specific options ────────────────────

# ... (existing target sections unchanged) ...
"""
```

- [ ] **Step 4: Update `validate_config`**

Add validation for `ui_target`:

```python
valid_ui_targets = {"", "react", "vue", "dioxus", "flutter", "omniview"}
if config.ui_target not in valid_ui_targets:
    errors.append(
        f"Invalid ui_target '{config.ui_target}'. "
        f"Valid: {', '.join(t for t in valid_ui_targets if t)}"
    )
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/config.py
git commit -m "feat(config): add ui_target and aliases to .omnilang.toml schema"
```

---

### Task 5.2: Update `omnic build` command

**Files:**
- Modify: `omniview/cli.py`
- Modify: `omnilang/cli.py`

- [ ] **Step 1: Update `build_command` in `omniview/cli.py`**

Modify the build flow to:
1. Read `.omnilang.toml` for aliases and ui_target
2. Use `ModuleLoader.build_source_map()` to collect all sources
3. Pass source map to `compile_multi()` (Rust DLL when available) or Python fallback
4. Write output files to `build/` directory

```python
def build_command(args):
    """Build .omni project with multi-file compilation."""
    from omnilang.config import load_config
    from omnilang.modules.resolver import ModuleResolver
    from omniview.modules.loader import ModuleLoader
    
    # Load project config
    config = load_config()
    
    # Find entry file (uses [source] section per spec)
    source_dir = Path(config.source.dir)
    entry = Path(args.file or (source_dir / 'main.omni'))
    if not entry.exists():
        print(f"error: entry file '{entry}' not found")
        sys.exit(1)
    
    # Set up resolver with aliases from config
    project_root = entry.parent.parent if entry.parent.name == source_dir.name else Path.cwd()
    resolver = ModuleResolver(
        project_root=project_root,
        aliases=config.aliases,
    )
    
    # Load all transitive imports + path mapping
    loader = ModuleLoader(str(project_root))
    loader.resolver = resolver
    
    try:
        source_map, path_mapping = loader.build_source_map(str(entry))
    except ValueError as e:
        print(f"error: {e}")
        sys.exit(1)
    
    print(f"Validating {len(source_map)} file(s)...")
    
    # Phase 1: Rust cross-file validation (optional, graceful fallback)
    rust_ok = False
    try:
        from omniview.codegen.rust_bridge import RustBridge
        bridge = RustBridge()
        if bridge.available:
            result = bridge.validate_multi(source_map, path_mapping)
            if not result.get('success'):
                for err in result.get('errors', []):
                    print(f"  error: {err}", file=sys.stderr)
                sys.exit(1)
            rust_ok = True
            print("  cross-file validation passed (Rust)")
    except ImportError:
        print("  (Rust bridge not available, skipping cross-file validation)")
    except Exception as e:
        print(f"  warning: Rust bridge error: {e}")
    
    # Phase 2: Python codegen (always runs, handles all 8 targets)
    output_map = _python_multi_compile(source_map, config)
    
    # Write outputs (uses [output] section per spec)
    output_dir = Path(config.output.dir)
    for file_path, code in output_map.items():
        out_path = output_dir / Path(file_path).relative_to(project_root)
        out_path = out_path.with_suffix(_target_extension(config))
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(code, encoding='utf-8')
        print(f"  wrote: {out_path}")
    
    print("Build complete.")
```

- [ ] **Step 2: Add Python fallback compilation helper**

```python
def _python_multi_compile(source_map: dict, config) -> dict:
    """Python fallback: compile each file individually."""
    from omnilang.parser import parse as parse_omni
    from omnilang.codegen.javascript import JavaScriptGenerator
    from omnilang.codegen.python import PythonGenerator
    from omnilang.codegen.rust import RustGenerator
    from omnilang.codegen.dart import DartGenerator
    
    generators = {
        'js': JavaScriptGenerator(),
        'python': PythonGenerator(),
        'rust': RustGenerator(),
        'dart': DartGenerator(),
    }
    
    outputs = {}
    for path, source in source_map.items():
        program = parse_omni(source)
        for target in config.targets:
            if target in generators:
                code = generators[target].generate(program, {}, {})
                out_key = f"{path}.{_ext_for_target(target)}"
                outputs[out_key] = code
    
    return outputs

def _ext_for_target(target: str) -> str:
    return {'js': 'js', 'python': 'py', 'rust': 'rs', 'dart': 'dart'}.get(target, 'js')
```

- [ ] **Step 3: Update `omnilang/cli.py` `cmd_build`**

Apply the same changes to the older omnilang CLI for consistency.

- [ ] **Step 4: Commit**

```bash
git add omniview/cli.py omnilang/cli.py
git commit -m "feat(cli): update build command for multi-file compilation with config"
```

---

### Task 5.3: Update Rust FFI and Python RustBridge

**Files:**
- Modify: `omni-compiler/src/lib.rs`
- Verify/Create: `omniview/codegen/rust_bridge.py`

- [ ] **Step 0: Verify `RustBridge` base class exists**

Check if `omniview/codegen/rust_bridge.py` exists. If not, create it with the base class:

```python
"""Rust compiler DLL bridge via C FFI (cdylib) or WASM."""
import sys
import json
from ctypes import CDLL, c_char_p, byref

class RustBridge:
    """Bridge to the Rust compiler DLL. Gracefully degrades if DLL not available."""
    
    def __init__(self):
        self._lib = None
        self.available = False
        try:
            # Try loading the native DLL
            self._lib = CDLL('./omni_compiler.dll')  # or .so/.dylib per platform
            self.available = True
        except OSError:
            pass
    
    def _call(self, func_name: str, *args: str) -> str:
        """Call a C FFI function that takes C strings and returns a C string."""
        func = getattr(self._lib, func_name)
        func.restype = c_char_p
        c_args = [c_char_p(arg.encode('utf-8')) for arg in args]
        result = func(*c_args)
        return result.decode('utf-8') if isinstance(result, bytes) else str(result)
```

- [ ] **Step 1: Add multi-file C FFI export (validation only)**

In `omni-compiler/src/lib.rs`, add the C FFI wrapper:

```rust
/// Validate multiple .omni sources with cross-file checks.
/// Receives JSON:
///   `{ "sources": { "path": "source", ... }, "path_mapping": { "orig": "resolved", ... } }`
/// Returns JSON:
///   `{ "success": true/false, "errors": ["..."] }`
#[no_mangle]
pub extern "C" fn omni_validate_multi(
    input_json: *const c_char,
) -> *mut c_char {
    let input_str = unsafe { CStr::from_ptr(input_json) }.to_str().unwrap_or("{}");
    
    #[derive(Deserialize)]
    struct MultiInput {
        sources: HashMap<String, String>,
        path_mapping: HashMap<String, String>,
    }
    
    let input: MultiInput = serde_json::from_str(input_str).unwrap_or(MultiInput {
        sources: HashMap::new(),
        path_mapping: HashMap::new(),
    });
    
    let result = validate_multi(input.sources, input.path_mapping);
    let json = serde_json::to_string(&result).unwrap_or_else(|_| {
        r#"{"success":false,"programs":{},"errors":["serialization failed"]}"#.to_string()
    });
    
    CString::new(json).unwrap().into_raw()
}
```

- [ ] **Step 2: Update Python `RustBridge` with `validate_multi` method**

Add to `omniview/codegen/rust_bridge.py`:

```python
def validate_multi(self, source_map: dict[str, str], path_mapping: dict[str, str]) -> dict | None:
    """Call omni_validate_multi via FFI for cross-file validation.
    
    Returns the result dict with 'success' and 'errors' keys, or None if unavailable.
    """
    if not self.available:
        return None
    
    import json
    input_data = {
        'sources': source_map,
        'path_mapping': path_mapping,
    }
    input_json = json.dumps(input_data)
    
    result_json = self._call('omni_validate_multi', input_json)
    result = json.loads(result_json)
    
    if not result.get('success'):
        for err in result.get('errors', []):
            print(f"  error: {err}", file=sys.stderr)
    
    return result
```

- [ ] **Step 3: Commit**

```bash
git add omni-compiler/src/lib.rs omniview/codegen/rust_bridge.py
git commit -m "feat(rust-ffi): add omni_validate_multi C FFI, create Python RustBridge base class"
```

---

## Phase 6: Editor Integration

### Task 6.1: Update editor `.omni.json` schema docs

**Files:**
- Modify: `editor/server/omni_json/schema.py` (verify path exists; if `editor/server/omni_json/` doesn't exist, create the directory and `schema.py`)

- [ ] **Step 1: Document `.omni.json` role change**

Add a comment/docstring at the top of the schema module clarifying the new role:

```python
"""
.omni.json schema — Editor Runtime Snapshot

NOTE: As of 2026-06-16, .omni.json is the EDITOR RUNTIME SNAPSHOT, not the
primary project configuration. For project configuration, use .omnilang.toml.

.omni.json stores:
- Design canvas state (elements, wires, positions)
- Code canvas state (code_tabs, elements)
- Editor transient state (open tabs, viewport)

It can be compiled to an Omni project via `omnic compile <file>.omni.json`.
"""
```

- [ ] **Step 2: Commit**

```bash
git add editor/server/omni_json/schema.py
git commit -m "docs(editor): document .omni.json role as editor runtime snapshot"
```

---

## Verification Checklist

After all phases are complete, run the full verification:

- [ ] **Python grammar tests**: Parse all import/export syntax variants with both grammars
- [ ] **Rust parser tests**: `cargo test --test parser_tests` — all 16+ tests pass
- [ ] **Rust lexer tests**: `cargo test --test lexer_tests` — all 18+ tests pass
- [ ] **Module loader boundary test**: Logic file importing view file raises error
- [ ] **Module loader source map**: `build_source_map()` returns (source_map, path_mapping) tuple with correct mappings
- [ ] **Omniview double-check**: File with `import omniview;` but no UI declarations → error; file with UI but no `import omniview;` → error
- [ ] **Resolver .omni enforcement**: Missing `.omni` extension returns None
- [ ] **Resolver aliases**: `@alias/path.omni` resolves correctly from config
- [ ] **JS codegen**: All 5 import variants emit correct ES module syntax
- [ ] **Rust codegen**: `use` statements and `pub` visibility emit correct Rust
- [ ] **Python codegen**: `from X import Y` and `import X as Y` emit correct Python
- [ ] **Dart codegen**: `import '...' show X;` and `export '...' show X;` emit correct Dart
- [ ] **React/Vue codegen**: Import statements appear before component code
- [ ] **Dioxus codegen**: Rust-style `use` statements emit correctly
- [ ] **Flutter codegen**: Dart-style imports emit correctly
- [ ] **CLI build**: `omnic build` compiles multi-file project
- [ ] **CLI build entry**: `omnic build main.omni` compiles entry + transitive deps
- [ ] **Config validation**: Invalid `ui_target` produces clear error
- [ ] **Config template**: `omnic init` generates `.omnilang.toml` with aliases section
- [ ] **Config loading**: Aliases from `.omnilang.toml` flow to resolver
- [ ] **Rust semantic analysis**: Import variants processed in all 5 passes, no `todo!()` panics
- [ ] **Rust IR lowering**: Import statements skipped (no runtime representation)
- [ ] **Rust FFI**: `omni_validate_multi` accepts JSON `{sources, path_mapping}` and returns validation result
- [ ] **RustBridge**: Python `RustBridge` base class exists and `validate_multi` method calls FFI correctly
- [ ] **Backward compat**: Old-style `import { X } from "path"` still works
- [ ] **Backward compat**: `import omniview;` still works as mode marker
- [ ] **Backward compat**: `omni_compile_to_ir` / `omni_compile_to_omniview` single-file FFI still works (imports skipped gracefully)
- [ ] **No regressions**: All existing tests pass (`cargo test`, `pytest`)
