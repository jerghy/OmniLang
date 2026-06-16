# VSCode Extension for OmniLang — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the minimal VSCode extension to a full IDE experience with 11 LSP features powered by a Rust analyze() library, Python LSP server, and thin VSCode LanguageClient.

**Architecture:** Three-layer design — Rust `lsp.rs` provides pure-computation `analyze()` and `format()` functions with all semantic logic; Python LSP server manages document state and protocol conversion; VSCode extension is a minimal LanguageClient launcher. Single `AnalysisRequest` → `AnalysisResult` JSON contract across the FFI boundary.

**Tech Stack:** Rust (omni-compiler crate), Python 3.12+ (ctypes FFI, hand-rolled JSON-RPC stdio), Node.js (vscode-languageclient), TextMate grammar JSON.

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `omni-compiler/src/lsp.rs` | **Create** | `analyze()` + all 11 operation implementations, cross-file symbol table, cursor-targeted AST queries |
| `omni-compiler/src/lib.rs` | **Modify** | Add `pub mod lsp;`, add `format()` function, add `omni_analyze` + `omni_format` + `omni_free_string` FFI exports |
| `omnilang/rust_bridge.py` | **Modify** | Add `analyze()` and `format()` methods, call `omni_analyze` / `omni_format` FFI |
| `omnilang/cli.py` | **Modify** | Add `lsp` subcommand + `cmd_lsp()` function |
| `omnilang/lsp/server.py` | **Modify** | Remove all AST-dependent code, rewrite all handlers to call `rust_bridge.analyze()`, add 5 new capability handlers, workspace+debounce management |
| `vscode-extension/extension.js` | **Modify** | Rewrite as LanguageClient launcher (47→30 lines) |
| `vscode-extension/package.json` | **Modify** | Add `snippets` contribution |
| `vscode-extension/syntaxes/omnilang.tmLanguage.json` | **Create** | TextMate grammar converted from Monarch tokenizer |
| `vscode-extension/snippets/omnilang.json` | **Create** | 7 code snippets (comp, store, fn, view, if, for, sig) |

---

### Task 1: Rust — lsp.rs data structures + multi-file parser

**Files:**
- Create: `omni-compiler/src/lsp.rs`
- Modify: `omni-compiler/src/lib.rs`

- [ ] **Step 1: Create `omni-compiler/src/lsp.rs` with all data structures**

```rust
//! LSP support module — stateless analyze() for all semantic IDE operations.
//!
//! Receives AnalysisRequest (JSON), returns AnalysisResult (JSON).
//! Pure computation — no file I/O, no caching across calls.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::error::Diagnostic;
use crate::lexer;
use crate::parser;
use crate::parser::ast::*;
use crate::semantic;

// ── Request types ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalysisRequest {
    pub files: Vec<SourceFile>,
    pub primary_file: String,
    pub operation: Operation,
    pub cursor_offset: usize,
    #[serde(default)]
    pub extra: Extra,
}

#[derive(Debug, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub is_open: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Diagnostics,
    Completion,
    Hover,
    Definition,
    References,
    Rename,
    PrepareRename,
    SemanticTokens,
    SignatureHelp,
    DocumentSymbol,
    FoldingRanges,
}

#[derive(Debug, Default, Deserialize)]
pub struct Extra {
    #[serde(default)]
    pub new_name: Option<String>,
}

// ── Response types ─────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub success: bool,
    pub primary_result: PerFileResult,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_diagnostics: Vec<FileDiagnostics>,
}

#[derive(Debug, Default, Serialize)]
pub struct PerFileResult {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<LspDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_imports: Vec<ResolvedImport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<CompletionItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<HoverInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub definitions: Vec<Location>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Location>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<TextEdit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_range: Option<LspRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_tokens: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignatureInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<SymbolInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folds: Vec<FoldingRange>,
}

#[derive(Debug, Serialize)]
pub struct LspDiagnostic {
    pub severity: i32,        // 1=Error, 2=Warning, 3=Info
    pub range: LspRange,      // 1-based
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LspRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Serialize)]
pub struct ResolvedImport {
    pub source_path: String,
    pub resolved_path: String,
}

#[derive(Debug, Serialize)]
pub struct CompletionItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HoverInfo {
    pub contents: String,     // markdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<LspRange>,
}

#[derive(Debug, Serialize)]
pub struct Location {
    pub file_path: String,
    pub range: LspRange,
}

#[derive(Debug, Serialize)]
pub struct TextEdit {
    pub range: LspRange,
    pub new_text: String,
}

#[derive(Debug, Serialize)]
pub struct SignatureInfo {
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_parameter: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ParameterInfo {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: i32,             // LSP SymbolKind
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<LspRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_range: Option<LspRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SymbolInfo>,
}

#[derive(Debug, Serialize)]
pub struct FoldingRange {
    pub start_line: u32,
    pub end_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileDiagnostics {
    pub file_path: String,
    pub diagnostics: Vec<LspDiagnostic>,
}

// ── Internal parse state ───────────────────────────────

struct ParsedFile {
    path: String,
    content: String,    // original source — needed for cursor offset calculations
    program: Program,
    tokens: Vec<lexer::Token>,
}

struct AnalysisContext {
    files: HashMap<String, ParsedFile>,
    primary_file: String,
    cursor_offset: usize,
    /// Cross-file symbol table: file_path -> [(name, kind, span)]
    cross_symbols: HashMap<String, Vec<(String, SymbolKind, Span)>>,
}

#[derive(Debug, Clone)]
enum SymbolKind {
    Function,
    Variable,
    Struct,
    TypeAlias,
    Component,
    Store,
    Signal,
    Computed,
    Prop,
    Action,
    Event,
    Theme,
    Import,
    Export,
    Param,
}

// ── Helpers ────────────────────────────────────────────

fn span_to_lsp_range(span: &Span) -> LspRange {
    LspRange {
        start_line: span.line,
        start_col: span.col,
        end_line: span.end_line,
        end_col: span.end_col,
    }
}

fn diagnostic_to_lsp(d: &Diagnostic) -> LspDiagnostic {
    LspDiagnostic {
        severity: match d.severity {
            crate::error::Severity::Error => 1,
            crate::error::Severity::Warning => 2,
            crate::error::Severity::Info => 3,
        },
        range: LspRange {
            start_line: d.line,
            start_col: d.col,
            end_line: d.end_line,
            end_col: d.end_col,
        },
        message: d.message.clone(),
        code: d.code.clone(),
        source: Some("omnilang".to_string()),
    }
}

fn byte_offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let offset = offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.chars().filter(|&c| c == '\n').count() as u32 + 1;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = (offset - last_newline) as u32 + 1;
    (line, col)
}
```

- [ ] **Step 2: Add `pub mod lsp;` to `omni-compiler/src/lib.rs`**

```rust
// After the existing module declarations (line 16):
pub mod lsp;
```

- [ ] **Step 3: Implement multi-file parsing + cross-file symbol table in lsp.rs**

Append to `omni-compiler/src/lsp.rs`:

```rust
impl AnalysisContext {
    fn new(request: &AnalysisRequest) -> Self {
        let mut files: HashMap<String, ParsedFile> = HashMap::new();
        let mut cross_symbols: HashMap<String, Vec<(String, SymbolKind, Span)>> = HashMap::new();

        for sf in &request.files {
            let tokens = lexer::tokenize(&sf.content);
            match parser::parse(&tokens) {
                Ok(program) => {
                    let symbols = collect_file_symbols(&sf.path, &program);
                    cross_symbols.insert(sf.path.clone(), symbols);
                    files.insert(sf.path.clone(), ParsedFile {
                        path: sf.path.clone(),
                        content: sf.content.clone(),
                        program,
                        tokens,
                    });
                }
                Err(_) => {
                    // Store empty program — parse error will be in diagnostics
                    files.insert(sf.path.clone(), ParsedFile {
                        path: sf.path.clone(),
                        content: sf.content.clone(),
                        program: Program::default(),
                        tokens,
                    });
                }
            }
        }

        AnalysisContext {
            files,
            primary_file: request.primary_file.clone(),
            cursor_offset: request.cursor_offset,
            cross_symbols,
        }
    }
}

fn collect_file_symbols(
    file_path: &str,
    program: &Program,
) -> Vec<(String, SymbolKind, Span)> {
    let mut symbols = Vec::new();

    for f in &program.functions {
        symbols.push((f.name.clone(), SymbolKind::Function, f.name_span.clone()));
        for p in &f.params {
            symbols.push((p.name.clone(), SymbolKind::Param, p.name_span));
        }
    }
    for s in &program.structs {
        symbols.push((s.name.clone(), SymbolKind::Struct, s.name_span.clone()));
    }
    for ta in &program.type_aliases {
        symbols.push((ta.name.clone(), SymbolKind::TypeAlias, ta.name_span.clone()));
    }
    for c in &program.components {
        symbols.push((c.name.clone(), SymbolKind::Component, c.name_span.clone()));
        for sig in &c.signals {
            symbols.push((sig.name.clone(), SymbolKind::Signal, sig.name_span.clone()));
        }
        for prop in &c.props {
            symbols.push((prop.name.clone(), SymbolKind::Prop, prop.name_span.clone()));
        }
        for comp in &c.computed {
            symbols.push((comp.name.clone(), SymbolKind::Computed, comp.name_span.clone()));
        }
    }
    for st in &program.stores {
        symbols.push((st.name.clone(), SymbolKind::Store, st.name_span.clone()));
    }
    for imp in &program.imports {
        match imp {
            ImportStmt::Named(n) => {
                for name in &n.names {
                    symbols.push((name.clone(), SymbolKind::Import, n.span));
                }
            }
            _ => {}
        }
    }

    symbols
}
```

- [ ] **Step 4: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: no errors. Note: `name_span` field may not exist on all AST nodes — add `#[serde(skip)] name_span: Span` fields where needed during implementation.

---

### Task 2: Rust — Diagnostics operation

**Files:**
- Modify: `omni-compiler/src/lsp.rs`

- [ ] **Step 1: Implement diagnostics collection**

Append to `omni-compiler/src/lsp.rs`:

```rust
fn compute_diagnostics(ctx: &AnalysisContext) -> Vec<LspDiagnostic> {
    let mut diags = Vec::new();

    // Pass 1: collect parse + semantic errors per file
    let mut programs: HashMap<String, Program> = HashMap::new();
    for (path, pf) in &ctx.files {
        match parser::parse(&pf.tokens) {
            Ok(program) => {
                match semantic::analyze(&program) {
                    Ok(_) => {}
                    Err(sem_diags) => {
                        for d in &sem_diags {
                            diags.push(diagnostic_to_lsp(d));
                        }
                    }
                }
                programs.insert(path.clone(), program);
            }
            Err(e) => {
                let d: Diagnostic = e.into();
                diags.push(diagnostic_to_lsp(&d));
            }
        }
    }

    // Pass 2: run cross-file lints on all successfully-parsed programs
    // Build merged IR for lint context
    let mut merged_ir = crate::ir::IRProgram {
        version: "1.0".to_string(),
        components: HashMap::new(),
        stores: HashMap::new(),
        source_map: HashMap::new(),
    };
    for (path, program) in &programs {
        if let Ok(sem) = semantic::analyze(program) {
            let ir = crate::ir::lower(program, &sem);
            for (name, comp) in ir.components {
                merged_ir.components.entry(name).or_insert(comp);
            }
            for (name, store) in ir.stores {
                merged_ir.stores.entry(name).or_insert(store);
            }
        }
    }
    let lint_diags = crate::semantic::lints::run_all(&programs, &merged_ir);
    for d in &lint_diags {
        diags.push(lint_diag_to_lsp(d));
    }

    diags
}

fn lint_diag_to_lsp(d: &crate::semantic::lints::LintDiagnostic) -> LspDiagnostic {
    LspDiagnostic {
        severity: match d.severity {
            crate::semantic::lints::LintSeverity::Error => 1,
            crate::semantic::lints::LintSeverity::Warning => 2,
            crate::semantic::lints::LintSeverity::Info => 3,
        },
        range: LspRange {
            start_line: d.line,
            start_col: d.col,
            end_line: d.end_line,
            end_col: d.end_col,
        },
        message: format!("[{}] {}", d.rule, d.message),
        code: d.rule.clone(),
        source: Some("omnilang-lint".to_string()),
    }
}
```

- [ ] **Step 2: Implement resolved_imports extraction**

Append to `omni-compiler/src/lsp.rs`:

```rust
fn compute_resolved_imports(ctx: &AnalysisContext) -> Vec<ResolvedImport> {
    let mut imports = Vec::new();
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return imports,
    };

    for imp in &primary.program.imports {
        match imp {
            ImportStmt::Named(n) => {
                let resolved = resolve_import_path(&ctx.primary_file, &n.path, &ctx.files);
                imports.push(ResolvedImport {
                    source_path: n.path.clone(),
                    resolved_path: resolved,
                });
            }
            ImportStmt::Namespace(ns) => {
                let resolved = resolve_import_path(&ctx.primary_file, &ns.path, &ctx.files);
                imports.push(ResolvedImport {
                    source_path: ns.path.clone(),
                    resolved_path: resolved,
                });
            }
            ImportStmt::TypeOnly(t) => {
                let resolved = resolve_import_path(&ctx.primary_file, &t.path, &ctx.files);
                imports.push(ResolvedImport {
                    source_path: t.path.clone(),
                    resolved_path: resolved,
                });
            }
            ImportStmt::Lib(_) => {}
        }
    }

    imports
}

fn resolve_import_path(
    from_file: &str,
    import_path: &str,
    files: &HashMap<String, ParsedFile>,
) -> String {
    // Simple resolution: relative path from the importing file
    // Python pre-resolves all paths to absolute before calling analyze(),
    // so we can do basic path normalization here.
    use std::path::Path;

    let parent = Path::new(from_file).parent().unwrap_or(Path::new(""));
    let resolved = parent.join(import_path).with_extension("omni");

    // Normalize and check against known files
    let normalized = resolved.to_string_lossy().replace('\\', "/");
    for key in files.keys() {
        if key.ends_with(&normalized) || normalized.ends_with(key) {
            return key.clone();
        }
    }
    normalized
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: no errors. Note: `lints::run_all_file` may not exist — adapt to existing lint API (e.g. `lints::run_all` which takes `&HashMap<String, Program>`). The `LintDiagnostic` fields (line, col, end_line, end_col) may differ — adapt to actual struct.

---

### Task 3: Rust — Hover + Definition + References operations

**Files:**
- Modify: `omni-compiler/src/lsp.rs`

- [ ] **Step 1: Implement hover — find symbol at cursor and return markdown info**

Append to `omni-compiler/src/lsp.rs`:

```rust
fn compute_hover(ctx: &AnalysisContext) -> Option<HoverInfo> {
    let (symbol_name, symbol_kind, symbol_span) = find_symbol_at_cursor(ctx)?;

    let contents = match symbol_kind {
        SymbolKind::Function => {
            format!("**Function `{}`**\n\n```omnilang\nfn {}(...) {{ ... }}\n```", symbol_name, symbol_name)
        }
        SymbolKind::Variable | SymbolKind::Param => {
            format!("**Variable `{}`**", symbol_name)
        }
        SymbolKind::Struct => {
            format!("**Struct `{}`**", symbol_name)
        }
        SymbolKind::Component => {
            format!("**Component `{}`**", symbol_name)
        }
        SymbolKind::Store => {
            format!("**Store `{}`**", symbol_name)
        }
        SymbolKind::Signal => {
            format!("**Signal `{}`**\n\nReactive state variable — changes trigger re-render.", symbol_name)
        }
        SymbolKind::Computed => {
            format!("**Computed `{}`**\n\nDerived reactive value — auto-updates when dependencies change.", symbol_name)
        }
        _ => format!("**`{}`**", symbol_name),
    };

    Some(HoverInfo {
        contents,
        range: Some(span_to_lsp_range(&symbol_span)),
    })
}

fn find_symbol_at_cursor(ctx: &AnalysisContext) -> Option<(String, SymbolKind, Span)> {
    let primary = ctx.files.get(&ctx.primary_file)?;
    let (cursor_line, cursor_col) = byte_offset_to_line_col(&primary.content, ctx.cursor_offset);

    let symbols = ctx.cross_symbols.get(&ctx.primary_file)?;
    // Find the narrowest symbol span that contains the cursor
    symbols.iter()
        .filter(|(_, _, span)| {
            span.line <= cursor_line && span.end_line >= cursor_line
                && (span.line < cursor_line || span.col <= cursor_col)
                && (span.end_line > cursor_line || span.end_col >= cursor_col)
        })
        .min_by_key(|(_, _, span)| {
            // Narrowest span = longest match
            let len = (span.end_line as i64 - span.line as i64) * 1000
                + (span.end_col as i64 - span.col as i64);
            -len
        })
        .map(|(name, kind, span)| (name.clone(), kind.clone(), span.clone()))
}
```

- [ ] **Step 2: Implement definition — return definition location**

Append to `omni-compiler/src/lsp.rs`:

```rust
fn compute_definition(ctx: &AnalysisContext) -> Vec<Location> {
    let (symbol_name, _, symbol_span) = match find_symbol_at_cursor(ctx) {
        Some(s) => s,
        None => return vec![],
    };

    // Check if the symbol is an import — if so, resolve to the source file
    let primary = ctx.files.get(&ctx.primary_file).unwrap();
    for imp in &primary.program.imports {
        match imp {
            ImportStmt::Named(n) if n.names.contains(&symbol_name) => {
                let resolved = resolve_import_path(&ctx.primary_file, &n.path, &ctx.files);
                if let Some(target_symbols) = ctx.cross_symbols.get(&resolved) {
                    for (name, _, span) in target_symbols {
                        if name == &symbol_name {
                            return vec![Location {
                                file_path: resolved.clone(),
                                range: span_to_lsp_range(span),
                            }];
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Search all files for the definition of this symbol
    for (file_path, symbols) in &ctx.cross_symbols {
        for (name, kind, span) in symbols {
            if name == &symbol_name && is_definition_kind(kind) {
                return vec![Location {
                    file_path: file_path.clone(),
                    range: span_to_lsp_range(span),
                }];
            }
        }
    }

    vec![]
}

fn is_definition_kind(kind: &SymbolKind) -> bool {
    matches!(kind,
        SymbolKind::Function | SymbolKind::Struct | SymbolKind::TypeAlias
        | SymbolKind::Component | SymbolKind::Store | SymbolKind::Signal
        | SymbolKind::Computed | SymbolKind::Prop | SymbolKind::Action
        | SymbolKind::Variable | SymbolKind::Param
    )
}
```

- [ ] **Step 3: Implement references — find all references to the symbol at cursor**

Append to `omni-compiler/src/lsp.rs`:

```rust
fn compute_references(ctx: &AnalysisContext) -> Vec<Location> {
    let (symbol_name, _, _) = match find_symbol_at_cursor(ctx) {
        Some(s) => s,
        None => return vec![],
    };

    let mut refs = Vec::new();

    // Search all files for references to this symbol name
    for (file_path, pf) in &ctx.files {
        // Walk the AST looking for VarRef nodes matching symbol_name
        find_var_refs_in_program(&pf.program, &symbol_name, file_path, &mut refs);
    }

    refs
}

fn find_var_refs_in_program(
    program: &Program,
    target: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    // Collect from all expression-containing locations
    for func in &program.functions {
        find_var_refs_in_exprs(&func.body, target, file_path, refs);
    }
    for comp in &program.components {
        find_var_refs_in_exprs(&comp.view_body, target, file_path, refs);
    }
    // Add more traversals as needed
}

fn find_var_refs_in_exprs(
    body: &crate::parser::ast::Block,
    target: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    for stmt in &body.stmts {
        collect_refs_from_stmt(stmt, target, file_path, refs);
    }
    if let Some(ref trailing) = body.trailing_expr {
        collect_refs_from_expr(trailing, target, file_path, refs);
    }
}

fn collect_refs_from_stmt(
    stmt: &Stmt,
    target: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    match stmt {
        Stmt::VarDecl(vd) => {
            if vd.name == target {
                refs.push(Location {
                    file_path: file_path.to_string(),
                    range: span_to_lsp_range(&vd.name_span),
                });
            }
            if let Some(init) = &vd.init {
                collect_refs_from_expr(init, target, file_path, refs);
            }
        }
        Stmt::Assign(a) => {
            if a.target == target {
                refs.push(Location {
                    file_path: file_path.to_string(),
                    range: span_to_lsp_range(&a.span),
                });
            }
            collect_refs_from_expr(&a.value, target, file_path, refs);
        }
        Stmt::Expr(e) | Stmt::Return(Some(e)) | Stmt::Print(Some(e)) => {
            collect_refs_from_expr(e, target, file_path, refs);
        }
        Stmt::If(i) => {
            collect_refs_from_expr(&i.condition, target, file_path, refs);
            find_var_refs_in_block(&i.then_body, target, file_path, refs);
            if let Some(ref else_b) = i.else_body {
                find_var_refs_in_block(else_b, target, file_path, refs);
            }
        }
        Stmt::While(w) => {
            collect_refs_from_expr(&w.condition, target, file_path, refs);
            find_var_refs_in_block(&w.body, target, file_path, refs);
        }
        Stmt::ForIn(fi) => {
            collect_refs_from_expr(&fi.iterable, target, file_path, refs);
            find_var_refs_in_block(&fi.body, target, file_path, refs);
        }
        _ => {}
    }
}

fn collect_refs_from_expr(
    expr: &Expr,
    target: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    match expr {
        Expr::VarRef(name, span) if name == target => {
            refs.push(Location {
                file_path: file_path.to_string(),
                range: span_to_lsp_range(span),
            });
        }
        Expr::FuncCall { func, args, .. } => {
            collect_refs_from_expr(func, target, file_path, refs);
            for arg in args {
                collect_refs_from_expr(arg, target, file_path, refs);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_refs_from_expr(object, target, file_path, refs);
            for arg in args {
                collect_refs_from_expr(arg, target, file_path, refs);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_refs_from_expr(left, target, file_path, refs);
            collect_refs_from_expr(right, target, file_path, refs);
        }
        Expr::UnaryOp { operand, .. } => {
            collect_refs_from_expr(operand, target, file_path, refs);
        }
        Expr::FieldAccess { object, .. } => {
            collect_refs_from_expr(object, target, file_path, refs);
        }
        Expr::IndexGet { object, index, .. } => {
            collect_refs_from_expr(object, target, file_path, refs);
            collect_refs_from_expr(index, target, file_path, refs);
        }
        Expr::IfExpr { condition, then_expr, else_expr, .. } => {
            collect_refs_from_expr(condition, target, file_path, refs);
            collect_refs_from_expr(then_expr, target, file_path, refs);
            collect_refs_from_expr(else_expr, target, file_path, refs);
        }
        Expr::ArrayLiteral { elements, .. } => {
            for el in elements {
                collect_refs_from_expr(el, target, file_path, refs);
            }
        }
        Expr::MapLiteral { entries, .. } => {
            for (k, v) in entries {
                collect_refs_from_expr(k, target, file_path, refs);
                collect_refs_from_expr(v, target, file_path, refs);
            }
        }
        _ => {}
    }
}

fn find_var_refs_in_block(
    block: &Block,
    target: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    for stmt in &block.stmts {
        collect_refs_from_stmt(stmt, target, file_path, refs);
    }
    if let Some(ref trailing) = block.trailing_expr {
        collect_refs_from_expr(trailing, target, file_path, refs);
    }
}
```

- [ ] **Step 4: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: no errors. Note: `program.source_text()` may not exist — use `primary.content` via the AnalysisContext's KnownFiles. The body/exhaustive traversal implementations in step 3 will need to match actual AST node shapes; complete them during implementation.

---

### Task 4: Rust — Completion operation

**Files:**
- Modify: `omni-compiler/src/lsp.rs`

- [ ] **Step 1: Implement keyword + symbol completion**

Append to `omni-compiler/src/lsp.rs`:

```rust
// LSP CompletionItemKind constants
const KIND_KEYWORD: i32 = 14;
const KIND_FUNCTION: i32 = 3;
const KIND_VARIABLE: i32 = 6;
const KIND_STRUCT: i32 = 22;
const KIND_CLASS: i32 = 7;
const KIND_FIELD: i32 = 5;
const KIND_MODULE: i32 = 9;

const OMNILANG_KEYWORDS: &[(&str, &str)] = &[
    ("fn", "fn name(params) -> Type { ... }"),
    ("let", "let name: Type = value;"),
    ("mut", "mut — mutable binding modifier"),
    ("if", "if condition { ... }"),
    ("else", "else { ... }"),
    ("for", "for var in iterable { ... }"),
    ("while", "while condition { ... }"),
    ("match", "match value { pattern => body, ... }"),
    ("return", "return value;"),
    ("struct", "struct Name { field: Type, ... }"),
    ("import", "import { Name } from \"./path\";"),
    ("export", "export fn name(...) -> Type { ... }"),
    ("type", "type Name<T> = Variant1 | Variant2"),
    ("async", "async fn name(...) -> Type { ... }"),
    ("await", "await expr"),
    ("true", "true — boolean literal"),
    ("false", "false — boolean literal"),
    ("null", "null — absence of value"),
    ("break", "break;"),
    ("continue", "continue;"),
];

const OMNIVIEW_KEYWORDS: &[(&str, &str)] = &[
    ("component", "component Name { ... }"),
    ("store", "store Name { ... }"),
    ("signal", "signal name: Type = value;"),
    ("prop", "prop name: Type;"),
    ("computed", "computed name: Type = expr;"),
    ("action", "action name(params) { ... }"),
    ("view", "view { ... }"),
    ("style", "style { ... }"),
    ("ref", "ref name: Type;"),
];

fn compute_completion(ctx: &AnalysisContext) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // 1. Keywords (always available)
    for (kw, detail) in OMNILANG_KEYWORDS {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(KIND_KEYWORD),
            detail: Some(detail.to_string()),
            documentation: None,
            insert_text: Some(kw.to_string()),
            sort_text: Some(format!("01_{}", kw)),
        });
    }

    // 2. OmniView keywords (if file has omniview imports or UI constructs)
    let primary = ctx.files.get(&ctx.primary_file);
    let is_omniview = primary.map_or(false, |pf| {
        pf.program.imports.iter().any(|imp| {
            matches!(imp, ImportStmt::Lib(name) if name == "omniview")
        }) || !pf.program.components.is_empty()
            || !pf.program.stores.is_empty()
    });

    if is_omniview {
        for (kw, detail) in OMNIVIEW_KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(KIND_KEYWORD),
                detail: Some(detail.to_string()),
                documentation: None,
                insert_text: Some(kw.to_string()),
                sort_text: Some(format!("00_{}", kw)),
            });
        }
    }

    // 3. Cross-file symbols
    for (file_path, symbols) in &ctx.cross_symbols {
        for (name, kind, _span) in symbols {
            let lsp_kind = symbol_kind_to_completion_kind(kind);
            let detail = format!("{} (from {})",
                symbol_kind_display(kind),
                short_file_name(file_path),
            );
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(lsp_kind),
                detail: Some(detail),
                documentation: None,
                insert_text: Some(name.clone()),
                sort_text: Some(format!("10_{}", name)),
            });
        }
    }

    items
}

fn symbol_kind_to_completion_kind(kind: &SymbolKind) -> i32 {
    match kind {
        SymbolKind::Function | SymbolKind::Action => KIND_FUNCTION,
        SymbolKind::Variable | SymbolKind::Param | SymbolKind::Signal
            | SymbolKind::Computed | SymbolKind::Prop => KIND_VARIABLE,
        SymbolKind::Struct | SymbolKind::TypeAlias => KIND_STRUCT,
        SymbolKind::Component => KIND_CLASS,
        SymbolKind::Store => KIND_MODULE,
        SymbolKind::Import | SymbolKind::Export => KIND_MODULE,
        SymbolKind::Event => KIND_FIELD,
        SymbolKind::Theme => KIND_CLASS,
    }
}

fn symbol_kind_display(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "fn",
        SymbolKind::Variable => "var",
        SymbolKind::Struct => "struct",
        SymbolKind::TypeAlias => "type",
        SymbolKind::Component => "component",
        SymbolKind::Store => "store",
        SymbolKind::Signal => "signal",
        SymbolKind::Computed => "computed",
        SymbolKind::Prop => "prop",
        SymbolKind::Action => "action",
        SymbolKind::Event => "event",
        SymbolKind::Theme => "theme",
        SymbolKind::Import => "import",
        SymbolKind::Export => "export",
        SymbolKind::Param => "param",
    }
}

fn short_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: no errors.

---

### Task 5: Rust — Remaining operations (SemanticTokens, DocumentSymbol, FoldingRanges, SignatureHelp, Rename)

**Files:**
- Modify: `omni-compiler/src/lsp.rs`

- [ ] **Step 1: Implement semantic tokens**

Append to `omni-compiler/src/lsp.rs`:

```rust
// Semantic token types (index into tokenTypes array)
const TOK_KEYWORD: u32 = 0;
const TOK_TYPE: u32 = 1;
const TOK_FUNCTION: u32 = 2;
const TOK_VARIABLE: u32 = 3;
const TOK_STRING: u32 = 4;
const TOK_NUMBER: u32 = 5;
const TOK_COMMENT: u32 = 6;
const TOK_OPERATOR: u32 = 7;

const TOKEN_TYPES: &[&str] = &[
    "keyword", "type", "function", "variable",
    "string", "number", "comment", "operator",
];

fn compute_semantic_tokens(ctx: &AnalysisContext) -> Vec<u32> {
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return vec![],
    };

    let mut tokens = Vec::new();
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;

    // Walk tokens from the lexer and classify each one
    for tok in &primary.tokens {
        use crate::lexer::TokenKind;
        let (token_type, modifiers) = match &tok.kind {
            // Keywords
            TokenKind::Fn | TokenKind::Let | TokenKind::Mut | TokenKind::If
            | TokenKind::Else | TokenKind::While | TokenKind::For | TokenKind::In
            | TokenKind::Return | TokenKind::Break | TokenKind::Continue
            | TokenKind::Match | TokenKind::Async | TokenKind::Await
            | TokenKind::Import | TokenKind::Export | TokenKind::As
            | TokenKind::True | TokenKind::False | TokenKind::Null
            | TokenKind::Component | TokenKind::Store | TokenKind::Signal
            | TokenKind::Computed | TokenKind::View | TokenKind::Style
            | TokenKind::Struct | TokenKind::Type => (TOK_KEYWORD, 0),

            // Types
            TokenKind::String | TokenKind::Int | TokenKind::Float
            | TokenKind::Double | TokenKind::Bool | TokenKind::Void
            | TokenKind::List | TokenKind::Map | TokenKind::Set => (TOK_TYPE, 0),

            // Literals
            TokenKind::StringLit(_) | TokenKind::TemplateString(_) => (TOK_STRING, 0),
            TokenKind::IntLit(_) | TokenKind::FloatLit(_) => (TOK_NUMBER, 0),

            // Identifiers (classified by context later if needed)
            TokenKind::Ident(_) => (TOK_VARIABLE, 0),

            // Skip whitespace and comments for now
            _ => continue,
        };

        let line = tok.span.line - 1; // 0-based for LSP
        let col = tok.span.col - 1;

        let delta_line = line.saturating_sub(prev_line);
        let delta_col = if delta_line == 0 {
            col.saturating_sub(prev_col)
        } else {
            col
        };
        let length = (tok.span.end - tok.span.start) as u32;

        tokens.push(delta_line);
        tokens.push(delta_col);
        tokens.push(length);
        tokens.push(token_type);
        tokens.push(modifiers);

        prev_line = line;
        prev_col = col;
    }

    tokens
}
```

- [ ] **Step 2: Implement document symbols**

Append to `omni-compiler/src/lsp.rs`:

```rust
// LSP SymbolKind
const SYM_FILE: i32 = 1;
const SYM_FUNCTION: i32 = 12;
const SYM_STRUCT: i32 = 23;
const SYM_VARIABLE: i32 = 13;
const SYM_CLASS: i32 = 5;
const SYM_NAMESPACE: i32 = 3;
const SYM_FIELD: i32 = 14;

fn compute_document_symbols(ctx: &AnalysisContext) -> Vec<SymbolInfo> {
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return vec![],
    };

    let mut symbols = Vec::new();

    // Components
    for comp in &primary.program.components {
        let mut children = Vec::new();
        for sig in &comp.signals {
            children.push(SymbolInfo {
                name: sig.name.clone(),
                kind: SYM_FIELD,
                range: Some(span_to_lsp_range(&sig.name_span)),
                selection_range: Some(span_to_lsp_range(&sig.name_span)),
                children: vec![],
            });
        }
        for prop in &comp.props {
            children.push(SymbolInfo {
                name: prop.name.clone(),
                kind: SYM_FIELD,
                range: Some(span_to_lsp_range(&prop.name_span)),
                selection_range: Some(span_to_lsp_range(&prop.name_span)),
                children: vec![],
            });
        }
        symbols.push(SymbolInfo {
            name: comp.name.clone(),
            kind: SYM_CLASS,
            range: Some(span_to_lsp_range(&comp.name_span)),
            selection_range: Some(span_to_lsp_range(&comp.name_span)),
            children,
        });
    }

    // Stores
    for st in &primary.program.stores {
        symbols.push(SymbolInfo {
            name: st.name.clone(),
            kind: SYM_NAMESPACE,
            range: Some(span_to_lsp_range(&st.name_span)),
            selection_range: Some(span_to_lsp_range(&st.name_span)),
            children: vec![],
        });
    }

    // Functions
    for func in &primary.program.functions {
        symbols.push(SymbolInfo {
            name: func.name.clone(),
            kind: SYM_FUNCTION,
            range: Some(span_to_lsp_range(&func.name_span)),
            selection_range: Some(span_to_lsp_range(&func.name_span)),
            children: vec![],
        });
    }

    // Structs
    for s in &primary.program.structs {
        symbols.push(SymbolInfo {
            name: s.name.clone(),
            kind: SYM_STRUCT,
            range: Some(span_to_lsp_range(&s.name_span)),
            selection_range: Some(span_to_lsp_range(&s.name_span)),
            children: vec![],
        });
    }

    symbols
}
```

- [ ] **Step 3: Implement folding ranges + signature help + rename**

Append to `omni-compiler/src/lsp.rs`:

```rust
fn compute_folding_ranges(ctx: &AnalysisContext) -> Vec<FoldingRange> {
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return vec![],
    };

    let mut folds = Vec::new();
    let source = &primary.content;
    let lines: Vec<&str> = source.lines().collect();

    // Find matching brace pairs and create folding ranges
    let mut brace_stack: Vec<(usize, u32)> = Vec::new(); // (char_pos, line)
    for (char_pos, ch) in source.char_indices() {
        let line = source[..char_pos].chars().filter(|&c| c == '\n').count() as u32 + 1;
        match ch {
            '{' => brace_stack.push((char_pos, line)),
            '}' => {
                if let Some((_open_pos, open_line)) = brace_stack.pop() {
                    if open_line < line {
                        folds.push(FoldingRange {
                            start_line: open_line,
                            end_line: line,
                            kind: Some("region".to_string()),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    folds
}

fn compute_signature_help(ctx: &AnalysisContext) -> Option<SignatureInfo> {
    let primary = ctx.files.get(&ctx.primary_file)?;
    let (cursor_line, cursor_col) = byte_offset_to_line_col(
        &primary.content, ctx.cursor_offset,
    );

    // Find function call at cursor position by walking AST
    find_func_call_at(&primary.program, cursor_line, cursor_col)
}

fn find_func_call_at(program: &Program, line: u32, col: u32) -> Option<SignatureInfo> {
    // Walk all functions looking for FuncCall nodes at cursor position
    for func in &program.functions {
        if let Some(sig) = find_call_in_block(&func.body, line, col) {
            return Some(sig);
        }
    }
    for comp in &program.components {
        if let Some(sig) = find_call_in_block(&comp.view_body, line, col) {
            return Some(sig);
        }
    }
    None
}

fn find_call_in_block(block: &Block, line: u32, col: u32) -> Option<SignatureInfo> {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Expr(expr) | Stmt::Return(Some(expr)) => {
                if let Some(sig) = find_call_in_expr(expr, line, col) {
                    return Some(sig);
                }
            }
            Stmt::VarDecl(vd) => {
                if let Some(init) = &vd.init {
                    if let Some(sig) = find_call_in_expr(init, line, col) {
                        return Some(sig);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(ref trailing) = block.trailing_expr {
        return find_call_in_expr(trailing, line, col);
    }
    None
}

fn find_call_in_expr(expr: &Expr, line: u32, col: u32) -> Option<SignatureInfo> {
    match expr {
        Expr::FuncCall { func, args, span } => {
            if span_contains(span, line, col) {
                // Extract function name and build signature
                let func_name = match func.as_ref() {
                    Expr::VarRef(name, _) => name.clone(),
                    _ => "unknown".to_string(),
                };
                let params: Vec<ParameterInfo> = args.iter().enumerate().map(|(i, _)| {
                    ParameterInfo {
                        label: format!("arg{}", i),
                        documentation: None,
                    }
                }).collect();
                return Some(SignatureInfo {
                    label: format!("{}({})", func_name,
                        args.iter().enumerate()
                            .map(|(i, _)| format!("arg{}", i))
                            .collect::<Vec<_>>().join(", ")),
                    parameters: params,
                    active_parameter: Some(args.len().saturating_sub(1) as i32),
                });
            }
            // Also check nested calls in args
            for arg in args {
                if let Some(sig) = find_call_in_expr(arg, line, col) {
                    return Some(sig);
                }
            }
            None
        }
        Expr::MethodCall { object, method, args, span } => {
            if span_contains(span, line, col) {
                let params: Vec<ParameterInfo> = args.iter().enumerate().map(|(i, _)| {
                    ParameterInfo { label: format!("arg{}", i), documentation: None }
                }).collect();
                return Some(SignatureInfo {
                    label: format!(".{}({})", method,
                        args.iter().enumerate()
                            .map(|(i, _)| format!("arg{}", i))
                            .collect::<Vec<_>>().join(", ")),
                    parameters: params,
                    active_parameter: Some(args.len().saturating_sub(1) as i32),
                });
            }
            None
        }
        Expr::BinaryOp { left, right, span, .. } => {
            if span_contains(span, line, col) {
                return find_call_in_expr(left, line, col)
                    .or_else(|| find_call_in_expr(right, line, col));
            }
            None
        }
        _ => None,
    }
}

fn span_contains(span: &Span, line: u32, col: u32) -> bool {
    span.line <= line && span.end_line >= line
        && (span.line < line || span.col <= col)
        && (span.end_line > line || span.end_col >= col)
}

fn compute_rename(ctx: &AnalysisContext) -> (Vec<TextEdit>, Option<LspRange>) {
    let refs = compute_references(ctx);
    let rename_range = find_symbol_at_cursor(ctx).map(|(_, _, span)| span_to_lsp_range(&span));

    let edits = refs.into_iter().map(|loc| TextEdit {
        range: loc.range,
        new_text: String::new(), // caller fills from Extra.new_name
    }).collect();

    (edits, rename_range)
}
```

- [ ] **Step 4: Wire the dispatch function**

Append to `omni-compiler/src/lsp.rs`:

```rust
/// Main entry point — stateless analysis for all LSP operations.
pub fn analyze(request: AnalysisRequest) -> AnalysisResult {
    let ctx = AnalysisContext::new(&request);

    let (diagnostics, resolved_imports) = if matches!(request.operation, Operation::Diagnostics) {
        (compute_diagnostics(&ctx), compute_resolved_imports(&ctx))
    } else {
        (vec![], vec![])
    };

    let mut result = PerFileResult {
        diagnostics,
        resolved_imports,
        ..Default::default()
    };

    match request.operation {
        Operation::Diagnostics => {} // already populated above

        Operation::Hover => {
            result.hover = compute_hover(&ctx);
        }

        Operation::Definition => {
            result.definitions = compute_definition(&ctx);
        }

        Operation::References => {
            result.references = compute_references(&ctx);
        }

        Operation::Completion => {
            result.items = compute_completion(&ctx);
        }

        Operation::SemanticTokens => {
            result.semantic_tokens = compute_semantic_tokens(&ctx);
        }

        Operation::DocumentSymbol => {
            result.symbols = compute_document_symbols(&ctx);
        }

        Operation::FoldingRanges => {
            result.folds = compute_folding_ranges(&ctx);
        }

        Operation::SignatureHelp => {
            result.signature = compute_signature_help(&ctx);
        }

        Operation::Rename => {
            let (edits, range) = compute_rename(&ctx);
            let new_name = request.extra.new_name.unwrap_or_default();
            result.edits = edits.into_iter().map(|mut e| {
                e.new_text = new_name.clone();
                e
            }).collect();
            result.rename_range = range;
        }

        Operation::PrepareRename => {
            result.rename_range = find_symbol_at_cursor(&ctx)
                .map(|(_, _, span)| span_to_lsp_range(&span));
        }
    }

    AnalysisResult {
        success: true,
        primary_result: result,
        other_diagnostics: vec![],
    }
}
```

- [ ] **Step 5: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: no errors. Some AST node fields (`name_span`, `view_body`, `body` on components/stores) may not match actual struct shapes — adapt to the real AST definition during implementation. Source text is accessed via `ParsedFile.content`, not a method on Program.

---

### Task 6: Rust — format() function + FFI exports

**Files:**
- Modify: `omni-compiler/src/lsp.rs` (append format())
- Modify: `omni-compiler/src/lib.rs` (add FFI exports)

- [ ] **Step 1: Add format() function to lsp.rs**

Append to `omni-compiler/src/lsp.rs`:

```rust
/// Format .omni source code — used by both LSP and CLI.
/// Performs consistent indentation, trailing whitespace removal, brace normalization.
pub fn format_source(source: &str) -> String {
    let tokens = lexer::tokenize(source);
    let lines: Vec<&str> = source.lines().collect();
    let mut formatted: Vec<String> = Vec::new();
    let mut indent_level: usize = 0;
    let indent_str = "    "; // 4 spaces

    // Simple token-based formatter:
    // - Track brace depth for indentation
    // - Remove trailing whitespace
    // - Ensure single trailing newline

    let source_chars: Vec<char> = source.chars().collect();
    let mut last_pos = 0usize;

    for tok in &tokens {
        use crate::lexer::TokenKind;
        let text = &source[tok.span.start..tok.span.end];

        // Emit any whitespace/newlines between tokens
        let between = &source[last_pos..tok.span.start];
        for ch in between.chars() {
            if ch == '\n' {
                formatted.push(String::new()); // empty = newline after trim
            }
        }

        match &tok.kind {
            TokenKind::LBrace => {
                // Opening brace on same line
                if let Some(last) = formatted.last_mut() {
                    last.push_str(" {");
                } else {
                    formatted.push(format!("{} {{", indent_str.repeat(indent_level)));
                }
                indent_level += 1;
            }
            TokenKind::RBrace => {
                indent_level = indent_level.saturating_sub(1);
                formatted.push(format!("{}}}", indent_str.repeat(indent_level)));
            }
            TokenKind::Semi => {
                if let Some(last) = formatted.last_mut() {
                    last.push(';');
                }
            }
            _ => {
                // Regular token — emit with current indentation
                if formatted.is_empty() || formatted.last().map_or(true, |s| s.is_empty()) {
                    formatted.push(format!("{}{}", indent_str.repeat(indent_level), text));
                } else if let Some(last) = formatted.last_mut() {
                    last.push(' ');
                    last.push_str(text);
                }
            }
        }

        last_pos = tok.span.end;
    }

    // Join lines, trim trailing whitespace, ensure single trailing newline
    let result = formatted.join("\n");
    let result = result.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    result.trim_end().to_string() + "\n"
}
```

- [ ] **Step 2: Add omni_analyze + omni_format FFI exports to lib.rs**

Replace the existing FFI section's `omni_analyze` placeholder if any, and append after the existing `omni_compile_to_target` function in `omni-compiler/src/lib.rs`:

```rust
/// Analyze .omni sources for LSP operations. Returns JSON (caller must free with omni_free_string).
///
/// # Safety
/// `request_json` must be a valid, null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn omni_analyze(request_json: *const c_char) -> *mut c_char {
    let request_str = unsafe {
        if request_json.is_null() {
            return rust_to_c_string(
                r#"{"success":false,"primary_result":{"diagnostics":[]},"other_diagnostics":[]}"#
                    .into(),
            );
        }
        CStr::from_ptr(request_json).to_str().unwrap_or("{}")
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let request: lsp::AnalysisRequest = match serde_json::from_str(request_str) {
            Ok(r) => r,
            Err(e) => {
                return serde_json::to_string(&lsp::AnalysisResult {
                    success: false,
                    primary_result: lsp::PerFileResult::default(),
                    other_diagnostics: vec![],
                })
            }
        };
        let analysis = lsp::analyze(request);
        serde_json::to_string(&analysis)
    }));

    match result {
        Ok(Ok(json)) => rust_to_c_string(json),
        _ => rust_to_c_string(
            r#"{"success":false,"primary_result":{"diagnostics":[]},"other_diagnostics":[]}"#
                .into(),
        ),
    }
}

/// Format .omni source code. Returns formatted source (caller must free with omni_free_string).
///
/// # Safety
/// `source` must be a valid, null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn omni_format(source: *const c_char) -> *mut c_char {
    let source_str = unsafe {
        if source.is_null() {
            return rust_to_c_string(String::new());
        }
        CStr::from_ptr(source).to_str().unwrap_or("")
    };
    let formatted = lsp::format_source(source_str);
    rust_to_c_string(formatted)
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: no errors.

---

### Task 7: Python — rust_bridge.py analyze() + format() methods

**Files:**
- Modify: `omnilang/rust_bridge.py`

- [ ] **Step 1: Add analyze() and format() methods to RustBridge**

Insert after the existing `compile_to_target()` method (after line 168) in `omnilang/rust_bridge.py`:

```python
@classmethod
def analyze(cls, request: dict) -> dict | None:
    """Call omni_analyze via FFI for LSP semantic operations.

    Args:
        request: AnalysisRequest dict matching the Rust struct schema.

    Returns:
        AnalysisResult dict with primary_result and other_diagnostics,
        or None if Rust compiler is not available.
    """
    if not cls.is_available():
        return None

    import json
    request_json = json.dumps(request)
    result = cls._call_dll("omni_analyze", request_json)
    return result


@classmethod
def format(cls, source: str) -> str | None:
    """Format .omni source via Rust FFI.

    Returns formatted source, or None if Rust compiler is not available.
    """
    if not cls.is_available():
        return None

    result = cls._call_dll("omni_format", source)
    if result is None:
        return None
    # omni_format returns the formatted string directly (not JSON-wrapped)
    # But _call_dll parses as JSON. Handle both cases.
    if isinstance(result, str):
        return result
    # If it parsed as dict (JSON), it's an error — fall through
    return None
```

- [ ] **Step 2: Handle the omni_format return type correctly**

The `_call_dll` method tries `json.loads()` on the result — but `omni_format` returns plain text (formatted source), not JSON. Modify `_call_dll` to handle non-JSON returns gracefully:

In `omnilang/rust_bridge.py`, inside `_call_dll` method (around line 207), change:

```python
if result_json:
    parsed = json.loads(result_json)
    # ... existing unwrap logic ...
```

To add a try-except before the json.loads:

```python
if result_json:
    try:
        parsed = json.loads(result_json)
    except json.JSONDecodeError:
        # Non-JSON return (e.g., formatted source from omni_format)
        return result_json
```

- [ ] **Step 3: Verify Python import**

```bash
cd omnilang && python -c "from omnilang.rust_bridge import RustBridge; print('import OK')"
```

Expected: `import OK`

---

### Task 8: Python — `omnic lsp` CLI subcommand

**Files:**
- Modify: `omnilang/cli.py`

- [ ] **Step 1: Add lsp subcommand to the argparse setup**

Insert after the `watch_parser` configuration (after line 650) in `omnilang/cli.py`:

```python
# ── lsp ──
lsp_parser = subparsers.add_parser("lsp", help="Start the OmniLang LSP server (stdio JSON-RPC)")
```

- [ ] **Step 2: Add cmd_lsp function**

Insert before the `# ═══ Main ═══` section (before line 560) in `omnilang/cli.py`:

```python
# ════════════════════════════════════════════════════════════
# LSP
# ════════════════════════════════════════════════════════════

def cmd_lsp(args):
    """Start the OmniLang Language Server Protocol server on stdio."""
    from omnilang.lsp.server import OmniLangServer
    server = OmniLangServer()
    server.start()
```

- [ ] **Step 3: Wire the lsp command in main()**

In the command dispatch section (after `elif args.command == "watch":` near line 669), add:

```python
elif args.command == "lsp":
    cmd_lsp(args)
```

- [ ] **Step 4: Verify CLI registration**

```bash
python -m omnilang.cli lsp --help 2>&1 || python omnilang/cli.py lsp --help 2>&1
```

Expected: no error about unknown command. (Server will start and block waiting for stdin — kill with Ctrl+C.)

---

### Task 9: Python — Rewrite LSP server

**Files:**
- Modify: `omnilang/lsp/server.py`

This is the largest change. The file is ~1047 lines and needs to be substantially rewritten.

- [ ] **Step 1: Remove all AST-dependent code but preserve keyword lists**

Delete these from the existing `omnilang/lsp/server.py`:
- Lines 1-28: All AST imports and `_HAS_AST` flag
- Lines 179-337: `_find_*()` family (7 functions), `_get_type_doc()`
- Lines 459-547: `_get_or_parse()`, `_wrap_omniview_ast()`, `_invalidate_ast()`
- Lines 1010-1043: `_find_var_decl_in_block()`, `_parse_via_rust()`

**Preserve these data blocks** (needed by fallback handlers):
- `KEYWORDS` (line 33-39)
- `KEYWORD_DETAILS` (line 42-71)
- `BUILTIN_TYPE_DOCS` (line 76-85)
- `OMNIVIEW_KEYWORDS` (line 89-107)
- `OMNIVIEW_DIRECTIVES` (line 109-122)
- `OMNIVIEW_ELEMENT_TAGS` (line 124-147)
- `OMNIVIEW_ELEMENT_PROPS` (line 149-170)
- `_pos_in_span()` helper (line 179-189) — possibly still useful
- `_make_lsp_position()`, `_make_lsp_range()`, `_make_lsp_location()` helpers (line 362-381)

Replace the imports section with the clean header:

```python
"""OmniLang Language Server Protocol server.

Provides IDE features via stdio LSP protocol using the Rust analyze() API.
Supports 11 LSP features: diagnostics, completion, hover, definition,
references, rename, semantic tokens, signature help, document symbols,
folding ranges, and formatting.
"""

import json
import os
import sys
import time
from pathlib import Path
from typing import Any
```

Then insert the keyword data blocks (copy-paste from old server.py, lines 33-170) after the imports.

- [ ] **Step 2: Rewrite the OmniLangServer class — constructor + workspace**

Keep the class structure but replace the constructor and add workspace management:

```python
class OmniLangServer:
    """LSP server for OmniLang language support, backed by Rust analyze()."""

    def __init__(self):
        self.workspace_roots: list[str] = []
        self.documents: dict[str, str] = {}          # uri -> source text
        self.document_versions: dict[str, int] = {}   # uri -> version
        self.document_open: dict[str, bool] = {}      # uri -> is_open
        self.import_graph: dict[str, list[str]] = {}   # uri -> [resolved_import_uris]
        self._pending_diagnostics: dict[str, float] = {}  # uri -> last_change_time
        self._debounce_seconds: float = 0.3
```

- [ ] **Step 3: Rewrite start() + _handle_message() to add new method routing**

Replace the existing `start()` (line 415) and `_handle_message()` (line 428) methods:

```python
def start(self):
    """Start the LSP server on stdio."""
    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                break
            self._handle_message(line.strip())
        except EOFError:
            break
        except Exception as e:
            self._send_error(str(e))

def _handle_message(self, raw: str):
    """Parse and dispatch an LSP message."""
    # Run any pending diagnostics first — ensures diagnostics fire
    # even if the user stops typing and no further requests arrive.
    self._run_pending_diagnostics()

    try:
        msg = json.loads(raw)
    except json.JSONDecodeError:
        return
    method = msg.get("method", "")

    if method == "initialize":
        self._handle_initialize(msg)
    elif method == "textDocument/didOpen":
        self._handle_did_open(msg)
    elif method == "textDocument/didChange":
        self._handle_did_change(msg)
    elif method == "textDocument/didClose":
        self._handle_did_close(msg)
    elif method == "textDocument/hover":
        self._handle_hover(msg)
    elif method == "textDocument/definition":
        self._handle_definition(msg)
    elif method == "textDocument/completion":
        self._handle_completion(msg)
    elif method == "textDocument/references":
        self._handle_references(msg)
    elif method == "textDocument/codeAction":
        self._handle_code_action(msg)
    elif method == "textDocument/rename":
        self._handle_rename(msg)
    elif method == "textDocument/prepareRename":
        self._handle_prepare_rename(msg)
    elif method == "textDocument/semanticTokens/full":
        self._handle_semantic_tokens(msg)
    elif method == "textDocument/signatureHelp":
        self._handle_signature_help(msg)
    elif method == "textDocument/documentSymbol":
        self._handle_document_symbol(msg)
    elif method == "textDocument/foldingRange":
        self._handle_folding_range(msg)
    elif method == "textDocument/formatting":
        self._handle_formatting(msg)
    # Unknown methods — ignore silently
```

- [ ] **Step 4: Rewrite initialize() with all new capabilities**

Replace `_handle_initialize()` (line 572):

```python
def _handle_initialize(self, msg: dict):
    self.workspace_roots = []
    params = msg.get("params", {})
    root_uri = params.get("rootUri", "")
    if root_uri:
        import urllib.parse
        root_path = urllib.parse.urlparse(root_uri).path
        # Windows: /C:/... -> C:/...
        if root_path.startswith('/') and len(root_path) > 2 and root_path[2] == ':':
            root_path = root_path[1:]
        self.workspace_roots = [root_path]
        self._scan_workspace(root_path)

    self._send({
        "jsonrpc": "2.0",
        "id": msg.get("id"),
        "result": {
            "capabilities": {
                "textDocumentSync": {
                    "openClose": True,
                    "change": 1,  # Full text sync
                },
                "hoverProvider": True,
                "definitionProvider": True,
                "completionProvider": {
                    "triggerCharacters": [".", ":", "@"],
                    "resolveProvider": False,
                },
                "referencesProvider": True,
                "codeActionProvider": {
                    "codeActionKinds": ["quickfix"],
                },
                "renameProvider": {
                    "prepareProvider": True,
                },
                "semanticTokensProvider": {
                    "full": True,
                    "range": False,
                    "legend": {
                        "tokenTypes": [
                            "keyword", "type", "function", "variable",
                            "string", "number", "comment", "operator",
                        ],
                        "tokenModifiers": [],
                    },
                },
                "foldingRangeProvider": True,
                "signatureHelpProvider": {
                    "triggerCharacters": ["(", ","],
                },
                "documentSymbolProvider": True,
                "documentFormattingProvider": True,
            },
            "serverInfo": {
                "name": "omnilang-lsp",
                "version": "0.3.0",
            },
        }
    })
```

- [ ] **Step 5: Add workspace scanning + build_analysis_request helper**

```python
def _scan_workspace(self, root_path: str):
    """Scan workspace for .omni files to pre-load for import resolution.

    Respects .gitignore and .omnilang.toml exclude patterns.
    """
    if not root_path or not os.path.isdir(root_path):
        return

    # Parse .gitignore patterns
    ignore_patterns = self._load_gitignore_patterns(root_path)

    # Parse .omnilang.toml exclude patterns
    try:
        from omnilang.config import load_config
        config = load_config(root_path)
        if config and config.source.exclude:
            ignore_patterns.extend(config.source.exclude)
    except Exception:
        pass

    for dirpath, dirnames, filenames in os.walk(root_path):
        # Filter directories in-place to skip ignored dirs
        dirnames[:] = [
            d for d in dirnames
            if not self._is_ignored(os.path.join(dirpath, d), root_path, ignore_patterns)
        ]
        for fname in filenames:
            full_path = os.path.join(dirpath, fname)
            if self._is_ignored(full_path, root_path, ignore_patterns):
                continue
            if fname.endswith('.omni'):
                uri = Path(full_path).as_uri()
                if uri not in self.documents:
                    try:
                        with open(full_path, 'r', encoding='utf-8') as f:
                            self.documents[uri] = f.read()
                        self.document_open[uri] = False
                    except Exception:
                        pass

@staticmethod
def _load_gitignore_patterns(root: str) -> list[str]:
    """Load .gitignore patterns as simple glob strings."""
    patterns = ['.git', 'node_modules', '__pycache__', '*.pyc', '.DS_Store']
    gitignore = os.path.join(root, '.gitignore')
    if os.path.exists(gitignore):
        try:
            with open(gitignore, 'r', encoding='utf-8') as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#'):
                        patterns.append(line.rstrip('/'))
        except Exception:
            pass
    return patterns

@staticmethod
def _is_ignored(path: str, root: str, patterns: list[str]) -> bool:
    """Check if a path matches any ignore pattern."""
    import fnmatch
    rel = os.path.relpath(path, root).replace('\\', '/')
    name = os.path.basename(path)
    for pat in patterns:
        if pat.endswith('/'):
            pat = pat[:-1]
        if fnmatch.fnmatch(rel, pat) or fnmatch.fnmatch(name, pat):
            return True
        # Pattern like "dir/" matches directory contents
        if '/' not in pat and fnmatch.fnmatch(rel, pat + '/*'):
            return True
    return False

def _build_analysis_request(self, primary_uri: str, operation: str,
                           cursor_line: int = 1, cursor_col: int = 1,
                           extra: dict | None = None) -> dict:
    """Build an AnalysisRequest dict for a given operation."""
    files = []
    primary_path = self._uri_to_path(primary_uri)

    # Include primary file
    if primary_uri in self.documents:
        files.append({
            "path": primary_path,
            "content": self.documents[primary_uri],
            "is_open": self.document_open.get(primary_uri, False),
        })

    # Include imported files (from import graph)
    imported_uris = self.import_graph.get(primary_uri, [])
    for imp_uri in imported_uris:
        if imp_uri in self.documents and imp_uri != primary_uri:
            files.append({
                "path": self._uri_to_path(imp_uri),
                "content": self.documents[imp_uri],
                "is_open": self.document_open.get(imp_uri, False),
            })

    # Calculate byte offset from line/col
    source = self.documents.get(primary_uri, "")
    cursor_offset = self._line_col_to_byte_offset(source, cursor_line, cursor_col)

    return {
        "files": files,
        "primary_file": primary_path,
        "operation": operation,
        "cursor_offset": cursor_offset,
        "extra": extra or {},
    }

@staticmethod
def _uri_to_path(uri: str) -> str:
    """Convert file:// URI to filesystem path."""
    import urllib.parse
    path = urllib.parse.urlparse(uri).path
    # Windows drive letter normalization
    if path.startswith('/') and len(path) > 2 and path[2] == ':':
        path = path[1:]
    return path

@staticmethod
def _line_col_to_byte_offset(source: str, line: int, col: int) -> int:
    """Convert 1-based line/col to byte offset (UTF-8)."""
    source_bytes = source.encode('utf-8')
    lines = source.split('\n')
    offset = 0
    for i in range(min(line - 1, len(lines))):
        offset += len(lines[i].encode('utf-8')) + 1  # +1 for '\n' byte
    # col is character-based; for multi-byte chars, count bytes up to col
    if line <= len(lines):
        chars_before = lines[line - 1][:col - 1]
        offset += len(chars_before.encode('utf-8'))
    return min(offset, len(source_bytes))
```

- [ ] **Step 6: Rewrite didOpen / didChange / didClose**

Replace existing handlers (lines 604-623):

```python
def _handle_did_open(self, msg: dict):
    doc = msg.get("params", {}).get("textDocument", {})
    uri = doc.get("uri", "")
    text = doc.get("text", "")
    version = doc.get("version", 0)
    self.documents[uri] = text
    self.document_versions[uri] = version
    self.document_open[uri] = True
    self._schedule_diagnostics(uri)

def _handle_did_change(self, msg: dict):
    doc = msg.get("params", {}).get("textDocument", {})
    changes = msg.get("params", {}).get("contentChanges", [])
    uri = doc.get("uri", "")
    version = doc.get("version", 0)
    if changes:
        self.documents[uri] = changes[-1].get("text", "")
    self.document_versions[uri] = version
    self._schedule_diagnostics(uri)

def _handle_did_close(self, msg: dict):
    doc = msg.get("params", {}).get("textDocument", {})
    uri = doc.get("uri", "")
    self.document_open[uri] = False
    # Keep content for import resolution — don't remove from self.documents
```

- [ ] **Step 7: Add debounced diagnostics + publish**

```python
def _schedule_diagnostics(self, uri: str):
    """Schedule a debounced diagnostics run for the given URI."""
    self._pending_diagnostics[uri] = time.time()

def _run_pending_diagnostics(self):
    """Process all pending diagnostics requests."""
    now = time.time()
    ready_uris = [
        uri for uri, t in self._pending_diagnostics.items()
        if now - t >= self._debounce_seconds
    ]
    for uri in ready_uris:
        self._pending_diagnostics.pop(uri, None)
        self._publish_diagnostics(uri)

def _publish_diagnostics(self, uri: str):
    """Call Rust analyze() for diagnostics and publish results."""
    from omnilang.rust_bridge import RustBridge

    # Run pending diagnostics check before this one
    self._run_pending_diagnostics()

    source = self.documents.get(uri, "")
    if not source:
        return

    # Build request
    request = self._build_analysis_request(uri, "diagnostics")

    result = RustBridge.analyze(request)
    if result is None:
        return  # Rust not available — fallback to keyword diagnostics

    # Parse and publish
    lsp_diags = []
    primary = result.get("primary_result", {})
    for d in primary.get("diagnostics", []):
        lsp_diags.append({
            "range": self._convert_range(d.get("range", {})),
            "severity": d.get("severity", 2),
            "message": d.get("message", ""),
            "code": d.get("code", ""),
            "source": d.get("source", "omnilang"),
        })

    self._send_notification("textDocument/publishDiagnostics", {
        "uri": uri,
        "diagnostics": lsp_diags,
    })

    # Update import graph from resolved_imports
    resolved = primary.get("resolved_imports", [])
    if resolved:
        imported_uris = []
        for imp in resolved:
            resolved_path = imp.get("resolved_path", "")
            resolved_uri = Path(resolved_path).as_uri()
            imported_uris.append(resolved_uri)
            # Load the imported file if not already known
            if resolved_uri not in self.documents and os.path.exists(resolved_path):
                try:
                    with open(resolved_path, 'r', encoding='utf-8') as f:
                        self.documents[resolved_uri] = f.read()
                    self.document_open[resolved_uri] = False
                except Exception:
                    pass
        self.import_graph[uri] = imported_uris

    # Publish cascade diagnostics to other open files
    for other in result.get("other_diagnostics", []):
        other_uri = Path(other.get("file_path", "")).as_uri()
        other_diags = []
        for d in other.get("diagnostics", []):
            other_diags.append({
                "range": self._convert_range(d.get("range", {})),
                "severity": d.get("severity", 2),
                "message": d.get("message", ""),
                "code": d.get("code", ""),
                "source": d.get("source", "omnilang"),
            })
        if other_diags:
            self._send_notification("textDocument/publishDiagnostics", {
                "uri": other_uri,
                "diagnostics": other_diags,
            })
```

- [ ] **Step 8: Rewrite all semantic handlers to use analyze()**

Replace `_handle_hover`, `_handle_definition`, `_handle_completion`, `_handle_references`, `_handle_code_action`, `_handle_rename` with unified analyze()-based versions. Remove all old implementations entirely.

```python
# ── Unified request helpers ──────────────────────────

def _extract_position(self, msg: dict) -> tuple[str, int, int]:
    """Extract (uri, 1-based-line, 1-based-col) from an LSP message."""
    params = msg.get("params", {})
    td = params.get("textDocument", {})
    pos = params.get("position", {})
    uri = td.get("uri", "")
    line = pos.get("line", 0) + 1
    col = pos.get("character", 0) + 1
    return uri, line, col

def _call_analyze(self, msg: dict, operation: str, extra: dict | None = None):
    """Call Rust analyze() for a semantic operation, return parsed result."""
    from omnilang.rust_bridge import RustBridge
    uri, line, col = self._extract_position(msg)
    request = self._build_analysis_request(uri, operation, line, col, extra)
    return RustBridge.analyze(request), uri

@staticmethod
def _convert_range(rs_range: dict) -> dict:
    """Convert 1-based Rust range to 0-based LSP range."""
    return {
        "start": {
            "line": rs_range.get("start_line", 1) - 1,
            "character": rs_range.get("start_col", 1) - 1,
        },
        "end": {
            "line": rs_range.get("end_line", 1) - 1,
            "character": rs_range.get("end_col", 1) - 1,
        },
    }

def _convert_location(self, loc: dict) -> dict:
    """Convert Rust Location to LSP Location."""
    import urllib.parse
    path = loc.get("file_path", "")
    return {
        "uri": Path(path).as_uri(),
        "range": self._convert_range(loc.get("range", {})),
    }

# ── textDocument/hover ──────────────────────────────

def _handle_hover(self, msg: dict):
    result, uri = self._call_analyze(msg, "hover")
    if result is None:
        # Fallback: keyword hover
        self._handle_hover_fallback(msg, uri)
        return

    hover = result.get("primary_result", {}).get("hover")
    if hover:
        lsp_hover = {"contents": {"kind": "markdown", "value": hover.get("contents", "")}}
        if hover.get("range"):
            lsp_hover["range"] = self._convert_range(hover["range"])
        self._send_result(msg.get("id"), lsp_hover)
    else:
        self._handle_hover_fallback(msg, uri)

def _handle_hover_fallback(self, msg: dict, uri: str):
    """Keyword/builtin hover fallback when Rust produces no result."""
    _, line, col = self._extract_position(msg)
    word = self._extract_word_at(uri, line, col)
    if word in KEYWORD_DETAILS:
        self._send_result(msg.get("id"), {
            "contents": {
                "kind": "markdown",
                "value": f"**`{word}`** keyword\n\n```omnilang\n{KEYWORD_DETAILS[word]}\n```",
            },
        })
    elif word in BUILTIN_TYPE_DOCS:
        self._send_result(msg.get("id"), {
            "contents": {
                "kind": "markdown",
                "value": f"**{word}** — {BUILTIN_TYPE_DOCS[word]}",
            },
        })
    else:
        self._send_result(msg.get("id"), None)

# ── textDocument/definition ─────────────────────────

def _handle_definition(self, msg: dict):
    result, _ = self._call_analyze(msg, "definition")
    if result is None:
        self._send_result(msg.get("id"), None)
        return

    defs = result.get("primary_result", {}).get("definitions", [])
    if defs:
        locations = [self._convert_location(d) for d in defs]
        self._send_result(msg.get("id"), locations if len(locations) > 1 else locations[0])
    else:
        self._send_result(msg.get("id"), None)

# ── textDocument/completion ─────────────────────────

def _handle_completion(self, msg: dict):
    result, uri = self._call_analyze(msg, "completion")
    if result is None:
        # Fallback: static keyword completion
        self._handle_completion_fallback(msg, uri)
        return

    items = result.get("primary_result", {}).get("items", [])
    if items:
        self._send_result(msg.get("id"), {"isIncomplete": False, "items": items})
    else:
        self._handle_completion_fallback(msg, uri)

def _handle_completion_fallback(self, msg: dict, uri: str):
    """Static keyword completion fallback — keyword + type completions."""
    is_omniview = self._is_omniview_file(uri)

    items = []
    if is_omniview:
        for kw, detail, kind in OMNIVIEW_KEYWORDS:
            items.append({"label": kw, "kind": kind, "detail": detail, "insertText": kw})
        for kw, detail, kind in OMNIVIEW_DIRECTIVES:
            items.append({"label": kw, "kind": kind, "detail": detail,
                          "insertText": kw[1:] if kw.startswith("@") else kw})
        for tag, detail, kind in OMNIVIEW_ELEMENT_TAGS:
            items.append({"label": tag, "kind": kind, "detail": detail, "insertText": tag})
        for prop, detail, kind in OMNIVIEW_ELEMENT_PROPS:
            items.append({"label": prop, "kind": kind, "detail": detail, "insertText": prop})
        for kw in KEYWORDS:
            items.append({"label": kw, "kind": 14,
                          "detail": KEYWORD_DETAILS.get(kw, ""), "insertText": kw})
    else:
        for kw in KEYWORDS:
            items.append({"label": kw, "kind": 14,
                          "detail": KEYWORD_DETAILS.get(kw, ""), "insertText": kw})
        for type_name in BUILTIN_TYPE_DOCS:
            items.append({"label": type_name, "kind": 9,
                          "detail": BUILTIN_TYPE_DOCS[type_name], "insertText": type_name})

    self._send_result(msg.get("id"), {"isIncomplete": False, "items": items})

# ── textDocument/references ─────────────────────────

def _handle_references(self, msg: dict):
    result, _ = self._call_analyze(msg, "references")
    if result is None:
        self._send_result(msg.get("id"), [])
        return

    refs = result.get("primary_result", {}).get("references", [])
    locations = [self._convert_location(r) for r in refs]
    self._send_result(msg.get("id"), locations)

# ── textDocument/rename ─────────────────────────────

def _handle_rename(self, msg: dict):
    params = msg.get("params", {})
    new_name = params.get("newName", "")
    result, uri = self._call_analyze(msg, "rename", {"new_name": new_name})
    if result is None:
        self._send_result(msg.get("id"), {"changes": {}})
        return

    edits = result.get("primary_result", {}).get("edits", [])
    changes: dict[str, list] = {}
    for edit in edits:
        lsp_edit = {
            "range": self._convert_range(edit.get("range", {})),
            "newText": edit.get("new_text", ""),
        }
        file_path = edit.get("file_path", self._uri_to_path(uri))
        file_uri = Path(file_path).as_uri()
        if file_uri not in changes:
            changes[file_uri] = []
        changes[file_uri].append(lsp_edit)
    self._send_result(msg.get("id"), {"changes": changes})

def _handle_prepare_rename(self, msg: dict):
    result, _ = self._call_analyze(msg, "prepare_rename")
    if result is None:
        self._send_result(msg.get("id"), None)
        return

    rename_range = result.get("primary_result", {}).get("rename_range")
    if rename_range:
        self._send_result(msg.get("id"), self._convert_range(rename_range))
    else:
        self._send_result(msg.get("id"), None)
```

- [ ] **Step 9: Add new handlers for semantic tokens, signature help, document symbols, folding, formatting**

```python
# ── textDocument/semanticTokens/full ────────────────

def _handle_semantic_tokens(self, msg: dict):
    result, _ = self._call_analyze(msg, "semantic_tokens")
    if result is None:
        self._send_result(msg.get("id"), {"data": []})
        return

    tokens = result.get("primary_result", {}).get("semantic_tokens", [])
    self._send_result(msg.get("id"), {"data": tokens})

# ── textDocument/signatureHelp ──────────────────────

def _handle_signature_help(self, msg: dict):
    result, _ = self._call_analyze(msg, "signature_help")
    if result is None:
        self._send_result(msg.get("id"), None)
        return

    sig = result.get("primary_result", {}).get("signature")
    if sig:
        self._send_result(msg.get("id"), {
            "signatures": [sig],
            "activeSignature": 0,
            "activeParameter": sig.get("active_parameter", 0),
        })
    else:
        self._send_result(msg.get("id"), None)

# ── textDocument/documentSymbol ─────────────────────

def _handle_document_symbol(self, msg: dict):
    result, _ = self._call_analyze(msg, "document_symbol")
    if result is None:
        self._send_result(msg.get("id"), [])
        return

    symbols = result.get("primary_result", {}).get("symbols", [])
    self._send_result(msg.get("id"), symbols)

# ── textDocument/foldingRange ───────────────────────

def _handle_folding_range(self, msg: dict):
    result, _ = self._call_analyze(msg, "folding_ranges")
    if result is None:
        self._send_result(msg.get("id"), [])
        return

    folds = result.get("primary_result", {}).get("folds", [])
    # Convert 1-based to 0-based line numbers
    for f in folds:
        f["startLine"] = f.get("start_line", 1) - 1
        f["endLine"] = f.get("end_line", 1) - 1
        if "start_line" in f: del f["start_line"]
        if "end_line" in f: del f["end_line"]
    self._send_result(msg.get("id"), folds)

# ── textDocument/formatting ─────────────────────────

def _handle_formatting(self, msg: dict):
    from omnilang.rust_bridge import RustBridge

    params = msg.get("params", {})
    td = params.get("textDocument", {})
    uri = td.get("uri", "")
    source = self.documents.get(uri, "")

    formatted = RustBridge.format(source)
    if formatted is None or formatted == source:
        self._send_result(msg.get("id"), [])
        return

    lines = source.split("\n")
    full_range = {
        "start": {"line": 0, "character": 0},
        "end": {"line": len(lines), "character": len(lines[-1]) if lines else 0},
    }
    self._send_result(msg.get("id"), [{"range": full_range, "newText": formatted}])

# ── textDocument/codeAction (keep existing static rules) ─────────

def _handle_code_action(self, msg: dict):
    """Return quick-fix suggestions based on diagnostics — static rules, no AST needed."""
    params = msg.get("params", {})
    td = params.get("textDocument", {})
    uri = td.get("uri", "")
    context = params.get("context", {})
    diags = context.get("diagnostics", [])

    actions = []
    for diag in diags:
        code = diag.get("code", "")
        range_val = diag.get("range", {})

        if code == "P0101":
            actions.append({
                "title": "Convert to float division (f64)",
                "kind": "quickfix", "diagnostics": [diag],
                "edit": {"changes": {uri: [{"range": range_val,
                    "newText": "(a as f64) / (b as f64)"}]}},
                "isPreferred": False,
            })
            actions.append({
                "title": "Add integer division helper (_idiv)",
                "kind": "quickfix", "diagnostics": [diag],
                "edit": {"changes": {uri: [{"range": range_val,
                    "newText": "_idiv(a, b)"}]}},
                "isPreferred": True,
            })
        elif code == "S0101":
            actions.append({
                "title": "Prefix unused variable with _",
                "kind": "quickfix", "diagnostics": [diag],
                "isPreferred": True,
            })
        elif code == "E0202":
            actions.append({
                "title": "Add _ = await expression to use async result",
                "kind": "quickfix", "diagnostics": [diag],
                "isPreferred": True,
            })
        elif code == "P0110":
            actions.append({
                "title": "Add zero-division guard",
                "kind": "quickfix", "diagnostics": [diag],
                "isPreferred": True,
            })

    self._send_result(msg.get("id"), actions)
```

- [ ] **Step 10: Verify Python LSP starts**

```bash
python -c "from omnilang.lsp.server import OmniLangServer; print('OK')"
```

Expected: `OK` (no ImportError).

**Note:** The keyword data blocks and helper methods `_is_omniview_file()` (line 394-405 of old server.py) and `_extract_word_at()` (line 661-680) must be preserved from the old server.py — they're needed by the fallback hover/completion handlers. Copy them into the rewritten class verbatim.

---

### Task 10: VSCode — extension.js rewrite

**Files:**
- Modify: `vscode-extension/extension.js`

- [ ] **Step 1: Replace extension.js with LanguageClient launcher**

Replace the entire content of `vscode-extension/extension.js`:

```js
const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

function activate(context) {
    client = new LanguageClient(
        'omnilang',
        'OmniLang Language Server',
        { command: 'omnic', args: ['lsp'] },
        { documentSelector: [{ language: 'omnilang' }] }
    );

    context.subscriptions.push(client.start());
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
```

- [ ] **Step 2: Verify syntax**

```bash
node -c vscode-extension/extension.js
```

Expected: no syntax errors.

---

### Task 11: VSCode — TextMate grammar

**Files:**
- Create: `vscode-extension/syntaxes/omnilang.tmLanguage.json`

- [ ] **Step 1: Create TextMate grammar from Monarch tokenizer**

Create `vscode-extension/syntaxes/omnilang.tmLanguage.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  "name": "OmniLang",
  "scopeName": "source.omni",
  "patterns": [
    { "include": "#comments" },
    { "include": "#strings" },
    { "include": "#numbers" },
    { "include": "#directives" },
    { "include": "#keywords" },
    { "include": "#types" },
    { "include": "#operators" },
    { "include": "#identifiers" }
  ],
  "repository": {
    "comments": {
      "patterns": [
        { "name": "comment.line.double-slash.omni", "match": "//.*$" },
        { "name": "comment.block.omni", "begin": "/\\*", "end": "\\*/" }
      ]
    },
    "strings": {
      "patterns": [
        {
          "name": "string.quoted.double.omni",
          "begin": "\"", "end": "\"",
          "patterns": [{ "name": "constant.character.escape.omni", "match": "\\\\." }]
        },
        {
          "name": "string.quoted.single.omni",
          "begin": "'", "end": "'",
          "patterns": [{ "name": "constant.character.escape.omni", "match": "\\\\." }]
        },
        {
          "name": "string.quoted.template.omni",
          "begin": "`", "end": "`",
          "patterns": [
            { "name": "constant.character.escape.omni", "match": "\\\\." },
            { "name": "string.interpolated.omni", "begin": "\\$\\{", "end": "\\}" }
          ]
        }
      ]
    },
    "numbers": {
      "patterns": [
        { "name": "constant.numeric.hex.omni", "match": "0[xX][0-9a-fA-F]+" },
        { "name": "constant.numeric.float.omni", "match": "\\d*\\.\\d+([eE][-+]?\\d+)?" },
        { "name": "constant.numeric.integer.omni", "match": "\\d+" }
      ]
    },
    "directives": {
      "patterns": [
        { "name": "keyword.directive.omni", "match": "@(if|for|else|mount|unmount|effect|slot|native|fallback|error|store|on:\\w+|bind:\\w+)" }
      ]
    },
    "keywords": {
      "patterns": [
        {
          "name": "keyword.control.omni",
          "match": "\\b(component|store|signal|computed|fn|view|style|import|export|as|from|if|else|for|while|match|case|default|return|yield|await|async|let|var|const|final|late|class|enum|struct|trait|impl|extends|super|this|self|new|throw|try|catch|finally|break|continue|in|is|pub|static|override|required|mixin|with|on|use|namespace|module|true|false|null|undefined|type|interface|nominal|private|print|println|action|prop|ref|getter|event|theme|mut|nominal|abstract|where)\\b"
        }
      ]
    },
    "types": {
      "patterns": [
        {
          "name": "storage.type.omni",
          "match": "\\b(String|int|double|float|bool|void|num|dynamic|List|Map|Set|Future|Stream|Option|Result|Widget|BuildContext|Element|i32|i64|f32|f64|u32|u64|Array|Any)\\b"
        }
      ]
    },
    "operators": {
      "patterns": [
        {
          "name": "keyword.operator.omni",
          "match": "==|!=|>=|<=|&&|\\|\\||=>|->|\\?\\?|\\?\\.|\\.\\.|\\+=|-=|\\*=|/=|%=|\\+|-|\\*|/|%|\\*\\*|~|!|\\||&|\\^|<<|>>|=|>|<|\\?"
        }
      ]
    },
    "identifiers": {
      "patterns": [
        { "name": "entity.name.function.omni", "match": "\\b[a-zA-Z_$][\\w$]*\\s*(?=\\()" },
        { "match": "\\b[a-zA-Z_$][\\w$]*\\b" }
      ]
    }
  }
}
```

- [ ] **Step 2: Verify JSON validity**

```bash
python -c "import json; json.load(open('vscode-extension/syntaxes/omnilang.tmLanguage.json')); print('JSON valid')"
```

Expected: `JSON valid`

---

### Task 12: VSCode — Snippets

**Files:**
- Create: `vscode-extension/snippets/omnilang.json`
- Modify: `vscode-extension/package.json`

- [ ] **Step 1: Create snippets file**

Create `vscode-extension/snippets/omnilang.json`:

```json
{
  "component": {
    "prefix": "comp",
    "body": [
      "component ${1:Name} {",
      "    ${2:signal count: int = 0;}",
      "",
      "    view {",
      "        ${3:// UI elements}",
      "    }",
      "}",
      ""
    ],
    "description": "OmniLang component with view block"
  },
  "store": {
    "prefix": "store",
    "body": [
      "store ${1:Name} {",
      "    signal ${2:field}: ${3:Type} = ${4:default};",
      "}",
      ""
    ],
    "description": "OmniLang store with signal"
  },
  "function": {
    "prefix": "fn",
    "body": [
      "fn ${1:name}(${2:params}) -> ${3:Type} {",
      "    ${4:// body}",
      "}",
      ""
    ],
    "description": "OmniLang function with return type"
  },
  "view": {
    "prefix": "view",
    "body": [
      "view {",
      "    ${1:// UI elements}",
      "}",
      ""
    ],
    "description": "OmniLang view block"
  },
  "ifDirective": {
    "prefix": "if",
    "body": [
      "@if ${1:condition} {",
      "    ${2:// content}",
      "}"
    ],
    "description": "@if directive"
  },
  "forDirective": {
    "prefix": "for",
    "body": [
      "@for ${1:item} in ${2:items} {",
      "    ${3:// content}",
      "}"
    ],
    "description": "@for directive"
  },
  "signal": {
    "prefix": "sig",
    "body": [
      "signal ${1:name}: ${2:Type} = ${3:value};"
    ],
    "description": "Signal declaration"
  }
}
```

- [ ] **Step 2: Add snippets contribution to package.json**

In `vscode-extension/package.json`, add to the `contributes` section after `grammars`:

```json
"snippets": [
  { "language": "omnilang", "path": "./snippets/omnilang.json" }
]
```

The full `contributes` section becomes:

```json
"contributes": {
  "languages": [{ "id": "omnilang", "extensions": [".omni"], "configuration": "./language-configuration.json" }],
  "grammars": [{ "language": "omnilang", "scopeName": "source.omni", "path": "./syntaxes/omnilang.tmLanguage.json" }],
  "snippets": [{ "language": "omnilang", "scopeName": "source.omni", "path": "./snippets/omnilang.json" }]
}
```

- [ ] **Step 3: Verify JSON validity**

```bash
python -c "import json; json.load(open('vscode-extension/package.json')); json.load(open('vscode-extension/snippets/omnilang.json')); print('JSON valid')"
```

Expected: `JSON valid`

---

### Task 13: Rust — Build DLL and integration test

**Files:**
- Build: `omni-compiler/`

- [ ] **Step 1: Build the Rust DLL**

```bash
cd omni-compiler && cargo build --release 2>&1
```

Expected: `Finished release [optimized] target(s)` with no errors.

- [ ] **Step 2: Verify DLL exports exist**

```bash
python -c "
import ctypes
dll = ctypes.CDLL('omni-compiler/target/release/omni_compiler.dll')
# Check new exports
for name in ['omni_analyze', 'omni_format', 'omni_free_string']:
    try:
        getattr(dll, name)
        print(f'  {name}: found')
    except AttributeError:
        print(f'  {name}: MISSING')
"
```

Expected: all three exports found.

- [ ] **Step 3: End-to-end test — analyze() round-trip**

```bash
python -c "
import json
from omnilang.rust_bridge import RustBridge

request = {
    'files': [{
        'path': 'test.omni',
        'content': 'fn main() { return 42; }',
        'is_open': True,
    }],
    'primary_file': 'test.omni',
    'operation': 'diagnostics',
    'cursor_offset': 0,
    'extra': {},
}
result = RustBridge.analyze(request)
print(json.dumps(result, indent=2))
"
```

Expected: JSON with `{"success": true, "primary_result": {"diagnostics": [...]}}`

- [ ] **Step 4: End-to-end test — format() round-trip**

```bash
python -c "
from omnilang.rust_bridge import RustBridge
source = 'fn main(){\nreturn 42;\n}\n'
formatted = RustBridge.format(source)
print(repr(formatted))
"
```

Expected: formatted source with proper indentation.

---

## Implementation Order

| # | Task | Dependencies | Can Parallel? |
|---|------|-------------|---------------|
| 1 | Rust lsp.rs data structures | None | — |
| 2 | Rust Diagnostics | 1 | — |
| 3 | Rust Hover + Definition + References | 1 | — |
| 4 | Rust Completion | 1 | — |
| 5 | Rust Remaining Ops + dispatch | 1-4 | — |
| 6 | Rust format() + FFI | 5 | — |
| 7 | Python rust_bridge.py | 6 (DLL built) | — |
| 8 | Python CLI lsp subcommand | 7 | — |
| 9 | Python LSP server rewrite | 7 | — |
| 10 | VSCode extension.js | 8, 9 | — |
| 11 | VSCode TextMate grammar | None | Parallel with 1-10 |
| 12 | VSCode Snippets | None | Parallel with 1-10 |
| 13 | Build + integration test | All | — |

Tasks 11 and 12 are independent of the Rust/Python chain and can be done in parallel with tasks 1-10.
