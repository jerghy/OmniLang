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
    resolved_imports: Vec<ResolvedImport>, // Diagnostics fills this for graph building
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

struct ResolvedImport { source_path: String, resolved_path: String }

// Shared sub-structures
struct Diagnostic { severity, range, message, code, related_info }
struct CompletionItem { label, kind, detail, documentation, insert_text, sort_text }
struct Location { file_path, range }
struct Range { start_line, start_col, end_line, end_col }   // 1-based, same as Span
struct HoverInfo { contents, range }
struct SignatureInfo { label, parameters, active_parameter }
struct SymbolInfo { name, kind, range, children }
struct FoldingRange { start_line, end_line, kind }
struct TextEdit { range, new_text }
struct FileDiagnostics { file_path, diagnostics: Vec<Diagnostic> }
```

### Coordinate convention

- Rust `Span` uses **1-based** line/col (existing convention in `lexer::token::Span`)
- Rust returns all `Range` values as **1-based** — matches internal Span
- Python converts to **0-based** LSP positions before sending responses
- `cursor_offset` uses **byte offset** (UTF-8), matching Span's `start`/`end` fields

### Multi-file behavior

- Rust parses all `files` and builds a cross-file symbol table
- Uses `primary_file` + `cursor_offset` as the operation anchor
- `other_diagnostics` carries cascade diagnostics when a change in `primary_file` affects imported files
- No caching across `analyze()` calls — each call is independent

### FFI signature

```rust
// Returns JSON, caller must free with omni_free_string
// Panic-safe: catch_unwind ensures no unwind across FFI boundary
#[no_mangle]
pub extern "C" fn omni_analyze(request_json: *const c_char) -> *mut c_char {
    let request_str = unsafe { CStr::from_ptr(request_json) }.to_str().unwrap_or("");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let request: AnalysisRequest = serde_json::from_str(request_str)?;
        let result = analyze(request);
        Ok::<_, serde_json::Error>(serde_json::to_string(&result)?)
    }));
    match result {
        Ok(Ok(json)) => CString::new(json).unwrap().into_raw(),
        _ => CString::new(r#"{"success":false,"primary_result":{"diagnostics":[]},"other_diagnostics":[]}"#).unwrap().into_raw(),
    }
}

#[no_mangle]
pub extern "C" fn omni_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)); }
    }
}
```

## Python LSP Server

### Workspace management

```python
class DocumentState:
    path: str          # absolute path, e.g. "D:/project/src/button.omni"
    content: str       # current source text
    version: int       # LSP document version
    is_open: bool      # editor has this open
```

- `didOpen` / `didChange`: update content, set is_open=true, trigger debounced diagnostics
- `didClose`: set is_open=false, retain content (still needed for import chains)
- All paths are **absolute** — Python normalizes URIs before calling Rust
- Workspace tracks an **import graph** — which files depend on which others

### Import graph discovery

Python needs to know which files to include in `files: Vec<SourceFile>`. Two mechanisms:

1. **Rust returns imports from `analyze()`**: Each `Diagnostics` operation also populates a
   `resolved_imports: Vec<ResolvedImport>` field in `PerFileResult`, listing every
   `import { X } from "./path"` with the resolved absolute path. Python persists these
   across calls to build the dependency graph.

2. **Project scan on startup**: Python scans the workspace root for all `.omni` files
   (respecting `.gitignore` and any `exclude` patterns in `.omnilang.toml`) and
   pre-loads their content for import resolution.

When `primary_file` changes, Python follows the import graph to decide which
additional files to include — both upstream (files that `primary_file` imports) and
downstream (files that import `primary_file`, for cascade diagnostics).

### Diagnostic flow

1. `didOpen` / `didChange` → 300ms debounce
2. Collect workspace files (open + import dependencies)
3. Call `rust_bridge.analyze(request)` with `Operation::Diagnostics`
4. Publish diagnostics to primary file + cascade to other open editors

### LSP capabilities (updated from current)

The existing server advertises: hover, definition, completion, references, codeAction, rename.
The redesign adds:

| Capability | Config |
|-----------|--------|
| `semanticTokensProvider` | `{ full: true, range: false }` |
| `foldingRangeProvider` | `true` |
| `signatureHelpProvider` | `{ triggerCharacters: ["(", ","] }` |
| `documentSymbolProvider` | `true` |
| `documentFormattingProvider` | `true` |

### AST removal

The current server has `_HAS_AST = False` with all AST-dependent code stubbed out.
In the redesign:
- `_HAS_AST`, `_get_or_parse()`, `_find_symbol_at()`, `_find_function_def()`,
  `_find_symbol_in_block()`, `_find_symbol_in_expr()`, `_find_symbol_in_trailing()`,
  `_find_var_decl_in_block()`, `_wrap_omniview_ast()`, `_parse_via_rust()` — all removed
- All semantic handlers call `rust_bridge.analyze()` directly instead
- `_is_omniview_file()` detection stays — needed for keyword completion fallback

### Formatting handler

```python
def _handle_formatting(self, msg: dict):
    uri = msg["params"]["textDocument"]["uri"]
    source = self.documents.get(uri, "")
    formatted = rust_bridge.format(source)  # standalone, not via analyze()
    if formatted != source:
        # Return a single edit replacing the entire document
        lines = source.split("\n")
        self._send_result(msg["id"], [{
            "range": _make_lsp_range(1, 1, len(lines), len(lines[-1]) + 1),
            "newText": formatted,
        }])
    else:
        self._send_result(msg["id"], [])
```

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
const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

function activate(context) {
    // Let LanguageClient handle the server process lifecycle
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

Key points:
- LanguageClient spawns `omnic lsp` itself — no manual spawn/stream wiring
- `deactivate()` cleans up the server process on extension unload
- If `omnic` is not on PATH, the LanguageClient will show a standard error notification

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
