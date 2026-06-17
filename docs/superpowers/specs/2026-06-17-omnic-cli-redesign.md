# OmniLang CLI & Compiler Redesign

**Date:** 2026-06-17
**Status:** Approved
**Scope:** `omnic` CLI (Python) + `omni-compiler` (Rust DLL)

---

## 1. Motivation

The current `omnic` CLI cannot compile pure logic `.omni` code. The IR lowering layer in the Rust
compiler only processes `ComponentDef` and `StoreDef` AST nodes — top-level functions, structs,
enums, and type aliases are parsed but silently discarded. All 5 logic targets (Python, JS,
TypeScript, Rust, Dart) produce empty output.

Additional structural problems:

- Two parallel Rust→Python FFI paths (DLL ctypes + CLI subprocess) with inconsistent parameter
  conventions (`c_char_p` vs `ptr+len`).
- Error handling uses bare `try/except` + `traceback.print_exc()` — no structured error types
  and user-facing messages expose stack traces.
- No integration tests exercise the real project→build→run pipeline.
- IR was designed exclusively for UI reactivity (signals, bindings, mutations); adding logic
  constructs requires a unified design, not field-level patches.

## 2. Design Goals

1. **Unified IR** — all `.omni` language constructs (functions, structs, enums, type aliases,
   components, stores) are first-class citizens in a single IR.
2. **Single FFI path** — Rust compiler communicates with Python exclusively via DLL (ctypes),
   using a consistent `ptr+len` calling convention.
3. **Structured errors** — typed error hierarchy in Python; user-facing output is clean and
   never exposes a traceback.
4. **Real-project integration tests** — multiple logic and view test projects exercise the
   full build→run pipeline across all targets.
5. **Long-term maintainability** — clean separation of concerns, each codegen backend is
   independently understandable.

## 3. CLI Interface

### 3.1 Commands

| Command | Purpose |
|---------|---------|
| `omnic build [files...] [--target/-t] [--output/-o]` | Compile `.omni` → target language |
| `omnic analyse [file\|dir] [--format]` | Static analysis (parser + semantic) |
| `omnic create <name> -t logic\|view` | Scaffold a new project |
| `omnic run <file.omni> [--target/-t]` | Compile and execute |
| `omnic serve --dir <dir> [--port] [--vite-port]` | Start visual editor server |
| `omnic watch <file.omni> [--target/-t]` | Watch and auto-rebuild |

Removed: `init` (merged into `create`), `fmt` (not needed), `fix` (not needed), `lsp` (not needed).

### 3.2 Command Details

#### `omnic build`

```
omnic build                              # auto-discover src/main.omni
omnic build src/app.omni                 # single file
omnic build src/app.omni src/utils.omni  # multiple files
omnic build src/                         # directory
omnic build -t python                    # explicit target
omnic build -t react -o dist/            # custom output dir
```

Logic targets produce one output file per input file, preserving directory structure.
View targets (react, vue, dioxus, flutter) produce one output file per input file.
OmniView target produces exactly 3 files: `index.html`, `index.js`, `index.css`.

All compilation routes through `omni_compile_multi_to_target` DLL call.

#### `omnic analyse`

```
omnic analyse                  # project root: auto-discover and recursively analyse
omnic analyse src/app.omni     # single file
omnic analyse src/             # directory
omnic analyse --format json    # machine-readable output
```

Routes through `omni_analyse` DLL call. Outputs errors and warnings with file:line:col location.

#### `omnic create`

```
omnic create my-app -t logic   # pure logic: fn main, struct, enum, imports
omnic create my-app -t view    # UI: component, store, signals, import omniview
```

Generates `.omnilang.toml` and `src/main.omni` from templates. No compiler dependency.

#### `omnic run`

```
omnic run src/main.omni              # auto-detect target
omnic run src/main.omni -t python    # logic: compile → python
omnic run src/main.omni -t js        # logic: compile → node
```

**Logic run:** compile to `build/debug/` (preserving multi-file structure), execute entry point.

**View run:** compile to omniview → `build/debug/index.html + index.js + index.css` → open in browser.

#### `omnic serve`

Starts the visual editor (FastAPI + Vite). No changes from current behavior.

#### `omnic watch`

Watches `.omni` files and re-compiles on change. Routes through DLL, no subprocess per change.

### 3.3 Output Directory Convention

```
build/                   # omnic build (release)
├── src/
│   ├── main.py
│   └── utils/
│       └── math.py
└── components/
    └── TodoApp.tsx

build/debug/             # omnic run (temporary, not committed)
├── index.html           # view run
├── index.js
├── index.css
└── src/                 # logic run
    └── main.py
```

## 4. Unified IR Design

### 4.1 Top-Level Structure

```
IRProgram
├── version: "1.0"
├── modules: Map<FilePath, ModuleIR>
└── source_map: Map<String, SourceMapEntry>
```

Every `.omni` source file becomes one `ModuleIR`. Cross-file references are resolved through
the module map; the old top-level `components`/`stores` maps are removed.

### 4.2 ModuleIR

```rust
struct ModuleIR {
    name: String,
    functions: HashMap<String, FunctionIR>,
    structs: HashMap<String, StructIR>,
    enums: HashMap<String, EnumIR>,
    type_aliases: HashMap<String, TypeAliasIR>,
    components: HashMap<String, ComponentIR>,
    stores: HashMap<String, StoreIR>,
    imports: Vec<ImportIR>,
    exports: Vec<ExportIR>,
}
```

### 4.3 New IR Nodes (logic constructs)

```rust
struct FunctionIR {
    name: String,
    visibility: Visibility,          // Public | Private
    generics: Vec<String>,
    params: Vec<ParamIR>,
    return_type: Option<TypeIR>,
    body: BlockIR,
}

struct StructIR {
    name: String,
    visibility: Visibility,
    generics: Vec<String>,
    fields: Vec<FieldIR>,
}

struct FieldIR {
    name: String,
    type: TypeIR,
    visibility: Visibility,
}

struct EnumIR {
    name: String,
    visibility: Visibility,
    variants: Vec<VariantIR>,
}

struct VariantIR {
    name: String,
    payload: Option<TypeIR>,
}

struct TypeAliasIR {
    name: String,
    visibility: Visibility,
    target: TypeIR,
}

struct ImportIR {
    path: String,
    names: Vec<String>,
    alias: Option<String>,
}

struct ExportIR {
    names: Vec<String>,
}
```

### 4.4 Existing UI Constructs (unchanged)

`ComponentIR`, `StoreIR`, `StateIntent`, `ComputedIntent`, `EffectIntent`, `LifecycleIntent`,
`FunctionIR` (component methods), `ViewTree`, `ElementIR`, `StyleRuleIR`, `BlockIR`,
`StatementIR`, `ExprIR` — all kept as-is.

`ComponentIR.fns` and module-level `FunctionIR` are distinct: component methods have access to
`self` signals/states; module-level functions are standalone.

### 4.5 IR Lowering

The `lower()` function processes every AST construct in `Program`:

```rust
pub fn lower(program: &Program, semantics: &SemanticResult) -> IRProgram {
    let mut modules = HashMap::new();

    for (filepath, module) in &program.modules {
        let mut ir = ModuleIR::default();
        ir.name = module.name.clone();

        for func in &module.functions {
            ir.functions.insert(func.name.clone(), lower_function(func));
        }
        for strukt in &module.structs {
            ir.structs.insert(strukt.name.clone(), lower_struct(strukt));
        }
        for enum_def in &module.enums {
            ir.enums.insert(enum_def.name.clone(), lower_enum(enum_def));
        }
        for ta in &module.type_aliases {
            ir.type_aliases.insert(ta.name.clone(), lower_type_alias(ta));
        }
        // Existing UI lowering:
        for comp in &module.components {
            ir.components.insert(comp.name.clone(), lower_component(comp));
        }
        for store in &module.stores {
            ir.stores.insert(store.name.clone(), lower_store(store));
        }
        // Imports & exports
        ir.imports = lower_imports(&module.imports);
        ir.exports = lower_exports(&module.exports);

        modules.insert(filepath.clone(), ir);
    }

    IRProgram { version: "1.0".into(), modules, source_map: HashMap::new() }
}
```

## 5. Compiler Pipeline

```
.omni source
     │
     ▼
┌─────────┐
│  Lexer  │  tokenize → Token stream
└────┬────┘
     │
     ▼
┌─────────┐
│ Parser  │  tokens → AST (Program)
└────┬────┘     Program.modules: Map<FilePath, Module>
     │              Module: { functions, structs, enums, type_aliases,
     │                        components, stores, imports, exports }
     ▼
┌──────────┐
│ Semantic │  AST → validated AST + SymbolTable + TypeMap
└────┬─────┘     - Type checking for all constructs
     │           - Cross-file import resolution
     │           - Lint rules (dead code, unused imports, etc.)
     │           - Cycle detection across modules
     │           - Continues on error — accumulates diagnostics, does not early-exit
     ▼
┌──────────────┐
│ IR Lowering  │  validated AST → IRProgram
└──────┬───────┘     All constructs lowered; no discrimination between logic and view
     │
     ▼
┌────────────────┐
│ Target Codegen │  IRProgram → {filepath: content}
└────────────────┘
     │
     ├── python   → .py
     ├── js       → .js
     ├── ts       → .ts
     ├── rust     → .rs
     ├── dart     → .dart
     ├── react    → .tsx
     ├── vue      → .vue
     ├── dioxus   → .rs
     ├── flutter  → .dart
     └── omniview → .html + .js + .css
```

Each codegen backend is independent — no shared formatting infrastructure. Each backend
implements the `Codegen` trait and handles IR→target-language mapping internally.

Logic targets consume: `functions`, `structs`, `enums`, `type_aliases`.
View targets consume: `components`, `stores` + referenced `functions`.

## 6. Rust DLL FFI

### 6.1 Exported Functions

4 C functions, all using `ptr + len` convention:

```c
// Core compilation
char* omni_compile_multi_to_target(
    const uint8_t* sources_json,  uint32_t sources_json_len,
    const uint8_t* target,        uint32_t target_len,
    const uint8_t* path_mapping,  uint32_t path_mapping_len
);
// Returns JSON: { success: bool, files: {path: content}, errors: [...], warnings: [...] }

// Static analysis
char* omni_analyse(
    const uint8_t* request_json, uint32_t request_json_len
);
// Request: { sources: {path: source}, path_mapping: {import: resolved} }
// Response: { success: bool, diagnostics: [...] }

// Memory management
void omni_free_string(char* ptr);

// Version
char* omni_version(void);
```

### 6.2 Removed Functions

- `omni_compile_to_ir` — IR is internal; not exposed via FFI
- `omni_compile_to_omniview` — handled by `omni_compile_multi_to_target(target="omniview")`
- `omni_compile_to_target` — single-file convenience; Python wraps multi-target
- `omni_validate_multi` — merged into `omni_analyse`
- `omni_format` — removed
- `omni_*` functions using `*const c_char` — all migrated to `ptr+len`

### 6.3 Python `RustBridge`

```python
class RustBridge:
    _dll: ctypes.CDLL | None = None

    @classmethod
    def compile(cls, sources: dict[str, str], target: str,
                path_mapping: dict[str, str] | None = None) -> CompileResult: ...

    @classmethod
    def analyse(cls, sources: dict[str, str],
                path_mapping: dict[str, str] | None = None) -> AnalyseResult: ...
```

No CLI fallback. If DLL is not found, `RustBridge` raises `CompilerUnavailable` with a clear
message telling the user to build the Rust compiler.

## 7. Error Handling

### 7.1 Python Error Types

```python
@dataclass
class OmniError:
    severity: Literal["error", "warning", "info"]
    code: str                    # "E0001" | "W0001"
    message: str                 # human-readable
    file: str | None
    line: int | None
    col: int | None
    suggestion: str | None

class OmniLangError(Exception):
    exit_code: int
    errors: list[OmniError]

class CompilerUnavailable(OmniLangError): ...   # DLL not found
class CompileError(OmniLangError): ...          # Rust compilation failed
class AnalyseError(OmniLangError): ...          # Static analysis found issues
class ConfigError(OmniLangError): ...           # .omnilang.toml issues
class InputError(OmniLangError): ...            # Bad CLI args / file not found
```

### 7.2 Error Display

```
Error: could not resolve import './utils/math.omni'
  → src/main.omni:3:8 (E0201)
  Suggestion: check the file path and ensure the imported file is in the project

2 error(s), 1 warning(s)
```

### 7.3 Unexpected Errors

Outermost exception handler catches anything not `OmniLangError`:

```
Internal error: unexpected failure in Rust compiler.
Please report this at https://github.com/.../issues
Details: <one-line summary, no traceback>
```

## 8. Test Strategy

### 8.1 Unit Tests (Rust)

- Parser round-trip tests (`.omni` → AST → verify structure)
- Semantic analysis tests (type checking, symbol resolution, cycle detection)
- IR lowering tests (AST → IR JSON, verify all constructs present)
- Each codegen backend: input IR → expected output string (snapshot tests)

### 8.2 Unit Tests (Python)

- CLI argument parsing
- Error type formatting
- `RustBridge` DLL loading/unavailable error paths
- Config loading

### 8.3 Integration Test Projects

```
test/projects/
├── logic-counter/         # fn main, struct, enum, multi-file imports
├── logic-cli-tool/        # command-line args, file I/O patterns
├── view-todo/             # component, store, signals, computed
├── view-ecommerce/         # multi-component, multi-store, routing
└── mixed-blog/            # logic files + view files with cross-references
```

Each project:
1. Has complete `.omni` source files
2. `omnic build` → all applicable targets
3. Verify: compiled output passes language runtime (python/node/tsc/cargo check/dart analyze)
4. `omnic run` → exits successfully with expected output
5. `omnic analyse` → returns clean (no errors) or expected diagnostics

Tests run in CI as merge gate.

## 9. Implementation Notes

### 9.1 Parser Changes

The current `Program` struct may need adjustment to support a `modules` map.
If the parser currently produces a flat `Program { functions, structs, components, stores, ... }`,
it should be restructured to `Program { modules: HashMap<FilePath, Module> }` where each
`Module` holds its own constructs. The multi-file compilation path already parses files
independently; this formalizes the module boundary in the AST.

### 9.2 Semantic Analysis

The semantic analyzer currently returns early on first error with an empty `SymbolTable`.
It should accumulate diagnostics and continue — IR lowering can proceed with partial
information (warnings don't block codegen, errors are reported alongside best-effort output).

### 9.3 Backward Compatibility

This is a breaking redesign. The old `IRProgram { components, stores }` shape, the
`compile_to_ir` FFI function, and the CLI subprocess path are all removed. No migration
path is needed — the current system is non-functional for logic code and this is a
replacement.

### 9.4 Editor Integration

The editor (`omnic serve`) calls `omnilang.api` for `parse_view_tree` and `parse_code_blocks`.
These functions use `RustBridge.compile_to_ir()` internally. After the redesign, they should
call `RustBridge.analyse()` or a dedicated `RustBridge.compile_to_ir()` internal function
instead. The editor's API surface (`CompileResult`, `ViewTreeNode`, `CodeBlockNode`) remains
stable.

## 10. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Rewriting all 9 codegen backends is time-consuming | Phase: fix logic targets first (5 backends), then view targets (4 backends). Logic targets are simpler (no reactivity). |
| Semantic analysis changes could introduce regressions in view codegen | Keep existing semantic tests; add regression tests for known-good view projects before touching view codegen. |
| DLL-only means the Rust compiler must build on all platforms | CI builds DLL for Windows/macOS/Linux; Python package bundles or documents build steps. |
