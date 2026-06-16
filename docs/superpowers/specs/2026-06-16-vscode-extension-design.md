# VSCode Extension for OmniLang — Design Spec

**Date:** 2026-06-16  
**Status:** Approved  

## Overview

Upgrade the existing minimal VSCode extension to a full IDE experience: LSP-first architecture with a Rust library providing all semantic features, a Python LSP server wrapping the protocol, and a thin VSCode client.

## Architecture

```
┌─ VSCode Extension (TypeScript/JS) ──────────────────────────────┐
│  • extension.js — launches omnic lsp, registers LanguageClient  │
│  • syntaxes/omnilang.tmLanguage.json — TextMate grammar         │
│  • snippets/omnilang.json — component/store/view snippets       │
│  • package.json — contributes languages, grammars, snippets     │
└──────────────────────────┬──────────────────────────────────────┘
                           │ stdio (JSON-RPC)
┌──────────────────────────▼──────────────────────────────────────┐
│  Python LSP Server (omnilang/lsp/server.py)                     │
│  • OmniLangServer — hand-rolled JSON-RPC, zero dependencies     │
│  • Workspace: HashMap<path, DocumentState> — open + disk files  │
│  • Debounce 300ms on didOpen/didChange → call Rust analyze()    │
│  • All semantic requests routed through RustBridge              │
│  • CLI entry: omnic lsp (new subcommand)                        │
└──────────────────────────┬──────────────────────────────────────┘
                           │ FFI (ctypes)
┌──────────────────────────▼──────────────────────────────────────┐
│  Rust Library (omni-compiler) — pure computation, no file I/O   │
│                                                                 │
│  ★ analyze(request_json: &str) -> result_json: &str            │
│     Unified semantic endpoint for all LSP operations            │
│  ★ format(source: &str) -> &str           (CLI reuse)           │
│  ★ compile(source, config)                (CLI reuse)           │
│                                                                 │
│  Internal: Lexer → Parser → AST → Semantic → structured output │
└─────────────────────────────────────────────────────────────────┘
```

## Rust: `analyze()` API

### Core principle

One function, one request type, one response type. Rust side is stateless — it receives all source files, does all computation, returns all results. Python manages document state.

### Request

```rust
struct AnalysisRequest {
    files: Vec<SourceFile>,      // all source files in scope
    primary_file: String,        // file path the cursor is in
    operation: Operation,        // which LSP operation to perform
    cursor_offset: usize,        // byte offset within primary_file
    extra: Extra,                // operation-specific parameters
}

struct SourceFile {
    path: String,                // file identifier for import resolution + Location
    content: String,             // source text
    is_open: bool,               // true = editor buffer, false = disk file
}

enum Operation {
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

struct Extra {
    new_name: Option<String>,    // target name for Rename
}
```

### Response

```rust
struct AnalysisResult {
    success: bool,
    primary_result: PerFileResult,           // primary_file results
    other_diagnostics: Vec<FileDiagnostics>,  // cascade diagnostics for other files
}

struct PerFileResult {
    diagnostics: Vec<Diagnostic>,       // always populated
    items: Vec<CompletionItem>,         // Completion
    hover: Option<HoverInfo>,           // Hover
    definitions: Vec<Location>,         // Definition
    references: Vec<Location>,          // References
    edits: Vec<TextEdit>,               // Rename
    rename_range: Option<Range>,        // PrepareRename
    semantic_tokens: Vec<u32>,         // SemanticTokens
    signature: Option<SignatureInfo>,  // SignatureHelp
    symbols: Vec<SymbolInfo>,          // DocumentSymbol
    folds: Vec<FoldingRange>,          // FoldingRanges
}

// Shared sub-structures
struct Diagnostic { severity, range, message, code, related_info }
struct CompletionItem { label, kind, detail, documentation, insert_text, sort_text }
struct Location { file_path, range }
struct Range { start_line, start_col, end_line, end_col }
struct HoverInfo { contents, range }
struct SignatureInfo { label, parameters, active_parameter }
struct SymbolInfo { name, kind, range, children }
struct FoldingRange { start_line, end_line, kind }
struct TextEdit { range, new_text }
```

### Multi-file behavior

- Rust parses all `files` and builds a cross-file symbol table
- Uses `primary_file` + `cursor_offset` as the operation anchor
- `other_diagnostics` carries cascade diagnostics when a change in `primary_file` affects imported files
- No caching across `analyze()` calls — each call is independent

### FFI signature

```rust
// Returns JSON, caller must free with omni_free_string
#[no_mangle]
pub extern "C" fn omni_analyze(request_json: *const c_char) -> *mut c_char;

#[no_mangle]
pub extern "C" fn omni_free_string(ptr: *mut c_char);
```

## Python LSP Server

### Workspace management

```python
class DocumentState:
    path: str          # "src/button.omni"
    content: str       # current source text
    version: int       # LSP document version
    is_open: bool      # editor has this open
```

- `didOpen` / `didChange`: update content, set is_open=true, trigger debounced diagnostics
- `didClose`: set is_open=false, retain content (still needed for import chains)
- Workspace also tracks an import graph to know which files to include in `files` param

### Diagnostic flow

1. `didOpen` / `didChange` → 300ms debounce
2. Collect workspace files (open + import dependencies)
3. Call `rust_bridge.analyze(request)` with `Operation::Diagnostics`
4. Publish diagnostics to primary file + cascade to other open editors

### CLI integration

```bash
omnic lsp   # new subcommand — starts stdio-based LSP server
```

Implementation: `omnilang/cli.py` adds `lsp` command that imports and runs `omnilang.lsp.server.main()`.

## VSCode Extension

### File structure

```
vscode-extension/
├── package.json                    # extension manifest
├── extension.js                    # entry: spawn omnic lsp, start LanguageClient
├── language-configuration.json     # exists — comments, brackets, auto-closing
├── syntaxes/
│   └── omnilang.tmLanguage.json   # NEW — TextMate grammar
├── snippets/
│   └── omnilang.json              # NEW — code snippets
└── .vscodeignore                  # publish exclusions
```

### extension.js (rewrite from current 47-line diagnostic-only version)

```js
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

function activate(context) {
    const server = spawn('omnic', ['lsp'], { cwd: workspaceFolder });
    const client = new LanguageClient(
        'omnilang',
        'OmniLang Language Server',
        () => ({ writer: server.stdin, reader: server.stdout }),
        { documentSelector: [{ language: 'omnilang' }] }
    );
    context.subscriptions.push(client.start());
}
```

### Features handled automatically by LSP client

| Feature | LSP Method | Rust Operation |
|---------|-----------|----------------|
| Diagnostics | textDocument/publishDiagnostics | Diagnostics |
| Completion | textDocument/completion | Completion |
| Hover | textDocument/hover | Hover |
| Go to Definition | textDocument/definition | Definition |
| Find References | textDocument/references | References |
| Rename | textDocument/rename + prepareRename | Rename / PrepareRename |
| Semantic Tokens | textDocument/semanticTokens/full | SemanticTokens |
| Signature Help | textDocument/signatureHelp | SignatureHelp |
| Document Symbols | textDocument/documentSymbol | DocumentSymbol |
| Folding Ranges | textDocument/foldingRange | FoldingRanges |
| Formatting | textDocument/formatting | (via format() standalone) |

### Static files maintained by extension

- TextMate grammar — static, no logic needed
- Snippets — static JSON, no logic needed
- Language configuration — already exists

### TextMate grammar coverage

Derived from the existing Monarch tokenizer (`editor/client/src/editor/omniLangTokens.ts`):

| Token category | Examples | Scope |
|---------------|----------|-------|
| Keywords | component, store, signal, computed, fn, view, if, else, for, import, export, struct, enum, type, return, let, mut, action, lifecycle, style, ref, async, await, match, true, false, null | keyword.control.omni |
| Types | String, int, float, double, bool, void, List, Map, Set, Any | storage.type.omni |
| Strings | "…", '…', `…` (template) | string.quoted.omni |
| Interpolation | ${…} inside template strings | string.interpolated.omni |
| Comments | // line, /* block */ | comment.line.omni, comment.block.omni |
| Numbers | 42, 3.14, 0xFF | constant.numeric.omni |
| Directives | @if, @for, @on:click, @bind:value | keyword.directive.omni |
| Operators | +, -, *, /, ==, !=, &&, \|\|, => | keyword.operator.omni |

### Snippets

| Prefix | Template |
|--------|----------|
| `comp` | component + view block |
| `store` | store + signal field |
| `fn` | function with return type |
| `view` | view block |
| `if` | @if directive |
| `for` | @for directive |
| `sig` | signal declaration |

## Implementation Order

| # | Module | What | Dependencies |
|---|--------|------|-------------|
| 1 | Rust: `lsp.rs` | `analyze()` function — all Operations, multi-file | None (uses existing parser/semantic) |
| 2 | Rust: FFI | `omni_analyze()` + `omni_free_string()` C exports | 1 |
| 3 | Python: `omnic lsp` | New CLI subcommand to launch server | 2 |
| 4 | Python: rewrite LSP server | Replace AST-dependent code with `analyze()` calls | 2, 3 |
| 5 | VSCode: `extension.js` | Rewrite as LSP client launcher | 3 |
| 6 | VSCode: TextMate grammar | Convert Monarch tokenizer → tmLanguage.json | (standalone) |
| 7 | VSCode: snippets | Create `snippets/omnilang.json` | (standalone) |

Items 6 and 7 are independent and can be done in parallel with the Rust/Python work.
