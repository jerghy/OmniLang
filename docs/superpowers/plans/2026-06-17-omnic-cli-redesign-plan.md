# OmniLang CLI & Compiler Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the OmniLang compiler with a unified IR that treats all language constructs (functions, structs, enums, type aliases, components, stores) as first-class citizens, fix all 9 codegen backends, and restructure the Python CLI with structured errors and DLL-only FFI.

**Architecture:** Single pipeline: Lexer → Parser → Semantic (accumulates diagnostics, never early-exits) → IR Lowering (all AST nodes lowered) → Target Codegen (each backend independent). Rust communicates with Python exclusively via DLL ctypes (ptr+len convention, 4 exported functions).

**Tech Stack:** Rust (omni-compiler cdylib), Python 3.10+ (omnilang package with ctypes), Cargo for Rust builds, pytest for Python tests.

---

## File Structure

### Created Files

| File | Purpose |
|------|---------|
| `omni-compiler/src/ir/logic.rs` | New IR nodes: FunctionIR, StructIR, EnumIR, TypeAliasIR, ImportIR, ExportIR, ModuleIR |
| `omni-compiler/src/ir/lower_logic.rs` | AST → IR lowering for logic constructs |
| `omni-compiler/src/semantic/lints.rs` | Lint rules (unused imports, dead code, etc.) |
| `omni-compiler/tests/codegen_snapshots.rs` | Snapshot tests for codegen backends |
| `omnilang/errors.py` | Structured error types: OmniError, OmniLangError subclasses |
| `omnilang/result.py` | CompileResult and AnalyseResult dataclasses |
| `.github/workflows/ci.yml` | Cross-platform CI pipeline: Rust DLL + Python tests |
| `test/projects/logic-counter/` | Integration test project (logic) |
| `test/projects/view-todo/` | Integration test project (view) |
| `test/projects/logic-cli-tool/` | Integration test project (CLI patterns) |
| `test/projects/view-ecommerce/` | Integration test project (multi-component) |
| `test/projects/mixed-blog/` | Integration test project (logic + view) |
| `tests/test_errors.py` | Python error formatting tests |
| `tests/test_cli.py` | CLI argument parsing tests |
| `tests/test_rustbridge.py` | RustBridge DLL interaction tests |

### Modified Files

| File | Change Summary |
|------|---------------|
| `omni-compiler/src/ir/mod.rs` | Replace top-level `components`/`stores` with `modules: HashMap<String, ModuleIR>` |
| `omni-compiler/src/parser/ast.rs` | Restructure `Program` to `Program { modules: HashMap<String, Module> }` |
| `omni-compiler/src/parser/mod.rs` | Adapt parser to populate `modules` map |
| `omni-compiler/src/semantic/mod.rs` | Accumulate diagnostics, continue on error |
| `omni-compiler/src/lib.rs` | Remove old FFI functions, add `omni_analyse`, unify on ptr+len |
| `omni-compiler/src/codegen/python.rs` | Handle logic constructs from ModuleIR |
| `omni-compiler/src/codegen/javascript.rs` | Handle logic constructs + add TypeScript mode |
| `omni-compiler/src/codegen/rust.rs` | Handle logic constructs |
| `omni-compiler/src/codegen/dart.rs` | Handle logic constructs |
| `omni-compiler/src/codegen/react.rs` | Adapt to modules-based IR |
| `omni-compiler/src/codegen/vue.rs` | Adapt to modules-based IR |
| `omni-compiler/src/codegen/dioxus.rs` | Adapt to modules-based IR |
| `omni-compiler/src/codegen/flutter.rs` | Adapt to modules-based IR |
| `omni-compiler/src/codegen/omniview.rs` | Adapt to modules-based IR |
| `omni-compiler/src/codegen/mod.rs` | Update `compile()` dispatch |
| `omnilang/rust_bridge.py` | Remove CLI fallback, simplify to DLL-only, add `analyse()` |
| `omnilang/api.py` | Use new RustBridge interface, structured errors |
| `omnilang/cli.py` | Remove init/fmt/fix/lsp, rename check→analyse, structured errors |

### Removed Files

| File | Reason |
|------|--------|
| `omnilang/lsp/` directory | LSP command removed |
| `omnilang/analyze/` directory | Replaced by Rust `omni_analyse` |
| Old FFI functions in lib.rs | `omni_compile_to_ir`, `omni_compile_to_omniview`, `omni_compile_to_target` (single-file), `omni_validate_multi`, `omni_format`, old `omni_version` |
| WASM feature in Cargo.toml | No longer targeting WASM |
| CLI subprocess fallback in rust_bridge.py | DLL-only; `_try_cli()`, `_try_cli_omniview()` removed |

---

### Task 1: Restructure `Program` AST to module-based model

**Files:**
- Modify: `omni-compiler/src/parser/ast.rs`

- [ ] **Step 1: Add `Module` struct to AST**

In `omni-compiler/src/parser/ast.rs`, after the `Program` struct definition, add:

```rust
/// A single source module — corresponds to one .omni file.
#[derive(Debug, Clone, Default)]
pub struct Module {
    pub name: String,
    pub functions: Vec<FunctionDef>,
    pub structs: Vec<StructDef>,
    pub tagged_unions: Vec<TaggedUnionDef>,
    pub type_aliases: Vec<TypeAlias>,
    pub components: Vec<ComponentDef>,
    pub stores: Vec<StoreDef>,
    pub themes: Vec<ThemeDef>,
    pub imports: Vec<ImportStmt>,
    pub exports: Vec<ExportFrom>,
}
```

- [ ] **Step 2: Change `Program` to use modules map**

Replace the current `Program` struct with:

```rust
#[derive(Debug, Clone, Default)]
pub struct Program {
    pub modules: HashMap<String, Module>,
}
```

Add `use std::collections::HashMap;` at the top of the file.

- [ ] **Step 3: Build and fix all compilation errors**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Many "no field `functions` on `Program`" errors — these guide subsequent tasks.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/parser/ast.rs
git commit -m "refactor: restructure Program AST to module-based model"
```

---

### Task 2: Adapt parser to populate modules map

**Files:**
- Modify: `omni-compiler/src/parser/mod.rs`
- Modify: `omni-compiler/src/parser/top_level.rs`

- [ ] **Step 1: Update parser entry point**

In `omni-compiler/src/parser/mod.rs`, the `parse()` function currently returns a flat `Program`. Change it to return `Program` with a single `"input.omni"` module:

```rust
pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module()?;
    let mut modules = HashMap::new();
    modules.insert("input.omni".to_string(), module);
    Ok(Program { modules })
}
```

- [ ] **Step 2: Add `parse_module()` method**

In `omni-compiler/src/parser/top_level.rs`, rename the current top-level parsing function to `parse_module()` and have it return `Module` instead of `Program`:

```rust
pub fn parse_module(&mut self) -> Result<Module, ParseError> {
    let mut module = Module::default();
    module.name = "input".to_string();
    // ... existing parsing logic, populating module.functions, module.structs, etc.
    Ok(module)
}
```

- [ ] **Step 3: Build and fix**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Fewer errors than before. Fix remaining issues in parser sub-modules.

- [ ] **Step 4: Run parser tests**

Run: `cargo test --lib parser 2>&1`
Expected: Existing parser tests adapt or are updated.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/parser/
git commit -m "refactor: adapt parser to populate Module in modules map"
```

---

### Task 3: Adapt compile functions in lib.rs to new AST shape

**Files:**
- Modify: `omni-compiler/src/lib.rs`

- [ ] **Step 1: Update `compile()` to iterate modules**

Change all references from `program.functions` → iteration over `program.modules.values()`:

```rust
pub fn compile(source: &str) -> CompileResult {
    // ... lex + parse ...
    // Phase 3: Semantic analysis — iterate all modules
    for (filepath, module) in &program.modules {
        // collect symbols, check types per module
    }
    // Phase 4: IR lowering
    let ir_program = ir::lower(&program, &semantics);
    // ...
}

pub fn compile_multi_to_target(
    sources: HashMap<String, String>,
    target: codegen::Target,
    path_mapping: HashMap<String, String>,
) -> String {
    // Parse each source → Module, insert into program.modules with filepath as key
    for (path, source) in &sources {
        let tokens = lexer::tokenize(source);
        match parser::parse(&tokens) {
            Ok(program) => {
                // program has one module; extract it
                if let Some((_, module)) = program.modules.into_iter().next() {
                    programs.insert(path.clone(), program);
                    // store module reference
                }
            }
            // ...
        }
    }
    // ...
}
```

- [ ] **Step 2: Update `compile_omniview()` similarly**

Same pattern — extract module from parse result.

- [ ] **Step 3: Update `validate_multi()` and other helpers**

Update `build_pub_symbols()`, `validate_cross_file()`, `validate_file_consistency()` to iterate `program.modules`.

- [ ] **Step 4: Build**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Compile errors in lib.rs if any `program.functions` / `program.components` remain. Fix them all.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/lib.rs
git commit -m "refactor: adapt lib.rs compile functions to module-based AST"
```

---

### Task 4: Add logic IR nodes (new file)

**Files:**
- Create: `omni-compiler/src/ir/logic.rs`

- [ ] **Step 1: Create `omni-compiler/src/ir/logic.rs`**

```rust
//! IR nodes for logic constructs: functions, structs, enums, type aliases, imports, exports.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionIR {
    pub name: String,
    pub visibility: String,  // "public" | "private"
    // generics: Vec<String> — reserved for future parser support
    pub params: Vec<ParamIR>,
    pub return_type: Option<TypeIR>,
    pub body: BlockIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructIR {
    pub name: String,
    pub visibility: String,
    // generics: Vec<String> — reserved for future parser support
    pub fields: Vec<FieldIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldIR {
    pub name: String,
    pub r#type: TypeIR,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumIR {
    pub name: String,
    pub visibility: String,
    pub variants: Vec<VariantIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantIR {
    pub name: String,
    pub fields: Vec<FieldIR>,  // named fields with types
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasIR {
    pub name: String,
    pub visibility: String,
    pub target: TypeIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIR {
    pub path: String,
    pub names: Vec<String>,
    pub alias: Option<String>,
    pub kind: String,  // "named" | "namespace" | "type_only" | "lib"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportIR {
    pub names: Vec<String>,
    pub path: Option<String>,
}

/// A single module in the IR — corresponds to one .omni source file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleIR {
    pub name: String,
    #[serde(default)]
    pub functions: HashMap<String, FunctionIR>,
    #[serde(default)]
    pub structs: HashMap<String, StructIR>,
    #[serde(default)]
    pub enums: HashMap<String, EnumIR>,
    #[serde(default)]
    pub type_aliases: HashMap<String, TypeAliasIR>,
    #[serde(default)]
    pub components: HashMap<String, super::ComponentIR>,
    #[serde(default)]
    pub stores: HashMap<String, super::StoreIR>,
    #[serde(default)]
    pub themes: Vec<serde_json::Value>,  // ThemeIR kept as JSON for now
    #[serde(default)]
    pub imports: Vec<ImportIR>,
    #[serde(default)]
    pub exports: Vec<ExportIR>,
}
```

- [ ] **Step 2: Register module in `omni-compiler/src/ir/mod.rs`**

Add at the top:
```rust
pub mod logic;
pub use logic::*;
```

- [ ] **Step 3: Build**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Warnings about unused items (fine for now).

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/ir/logic.rs omni-compiler/src/ir/mod.rs
git commit -m "feat: add logic IR nodes (FunctionIR, StructIR, EnumIR, ModuleIR)"
```

---

### Task 5: Restructure `IRProgram` to use modules map

**Files:**
- Modify: `omni-compiler/src/ir/mod.rs`

- [ ] **Step 1: Replace top-level IRProgram fields**

Replace the current `IRProgram` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub modules: HashMap<String, ModuleIR>,
    pub source_map: HashMap<String, SourceMapEntry>,
}
```

Remove the old `components` and `stores` fields from `IRProgram`. `ComponentIR` and `StoreIR` remain as types, they now live inside `ModuleIR`.

- [ ] **Step 2: Add a helper for backward compat during migration**

```rust
impl IRProgram {
    /// Collect all components across all modules.
    pub fn all_components(&self) -> HashMap<&str, &ComponentIR> {
        self.modules.iter()
            .flat_map(|(_, m)| m.components.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Collect all stores across all modules.
    pub fn all_stores(&self) -> HashMap<&str, &StoreIR> {
        self.modules.iter()
            .flat_map(|(_, m)| m.stores.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}
```

- [ ] **Step 3: Build**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Many compile errors in codegen backends (they reference `ir.components`). These guide Tasks 12-20.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/ir/mod.rs
git commit -m "refactor: replace top-level IRProgram fields with modules map"
```

---

### Task 6: Implement IR lowering for logic constructs

**Files:**
- Create: `omni-compiler/src/ir/lower_logic.rs`

- [ ] **Step 1: Create `omni-compiler/src/ir/lower_logic.rs`**

```rust
//! AST → IR lowering for logic constructs (functions, structs, enums, type aliases).

use std::collections::HashMap;
use crate::parser::ast::*;
use super::*;
use super::logic::*;

pub fn lower_function(func: &FunctionDef) -> FunctionIR {
    FunctionIR {
        name: func.name.clone(),
        visibility: match func.visibility {
            Visibility::Public => "public".into(),
            Visibility::Private => "private".into(),
        },
        params: func.params.iter().map(|p| ParamIR {
            name: p.name.clone(),
            r#type: p.type_ann.as_ref().map(lower_type),
        }).collect(),
        return_type: func.return_type.as_ref().map(lower_type),
        body: lower_block(&func.body),
    }
}

pub fn lower_struct(s: &StructDef) -> StructIR {
    StructIR {
        name: s.name.clone(),
        visibility: match s.visibility {
            Visibility::Public => "public".into(),
            Visibility::Private => "private".into(),
        },
        fields: s.fields.iter().map(|f| FieldIR {
            name: f.name.clone(),
            r#type: lower_type(&f.type_ann),
            visibility: "public".into(),
        }).collect(),
    }
}

pub fn lower_enum(tu: &TaggedUnionDef) -> EnumIR {
    EnumIR {
        name: tu.name.clone(),
        visibility: "public".into(),
        variants: tu.variants.iter().map(|v| VariantIR {
            name: v.name.clone(),
            fields: v.fields.iter().map(|f| FieldIR {
                name: f.name.clone(),
                r#type: lower_type(&f.type_ann),
                visibility: "public".into(),
            }).collect(),
        }).collect(),
    }
}

pub fn lower_type_alias(ta: &TypeAlias) -> TypeAliasIR {
    TypeAliasIR {
        name: ta.name.clone(),
        visibility: match ta.visibility {
            Visibility::Public => "public".into(),
            Visibility::Private => "private".into(),
        },
        target: lower_type(&ta.type_ann),
    }
}

pub fn lower_import(imp: &ImportStmt) -> ImportIR {
    match imp {
        ImportStmt::Named(n) => ImportIR {
            path: n.path.clone(),
            names: n.names.clone(),
            alias: None,
            kind: "named".into(),
        },
        ImportStmt::Namespace(ns) => ImportIR {
            path: ns.path.clone(),
            names: vec![],
            alias: Some(ns.alias.clone()),
            kind: "namespace".into(),
        },
        ImportStmt::TypeOnly(t) => ImportIR {
            path: t.path.clone(),
            names: t.names.clone(),
            alias: None,
            kind: "type_only".into(),
        },
        ImportStmt::Lib(name) => ImportIR {
            path: name.clone(),
            names: vec![],
            alias: None,
            kind: "lib".into(),
        },
    }
}

pub fn lower_export(exp: &ExportFrom) -> ExportIR {
    ExportIR {
        names: exp.names.clone(),
        path: Some(exp.path.clone()),
    }
}
```

- [ ] **Step 2: Register module**

In `omni-compiler/src/ir/mod.rs`, add:
```rust
pub mod lower_logic;
```

- [ ] **Step 3: Update `lower()` to call logic lowering**

In `omni-compiler/src/ir/mod.rs`, replace the `lower()` function body to iterate `program.modules` and call the new lowering functions for each construct type:

```rust
pub fn lower(program: &Program, _semantics: &SemanticResult) -> IRProgram {
    let mut modules = HashMap::new();

    for (filepath, module) in &program.modules {
        let mut ir = ModuleIR {
            name: module.name.clone(),
            ..Default::default()
        };

        for func in &module.functions {
            ir.functions.insert(func.name.clone(), lower_logic::lower_function(func));
        }
        for strukt in &module.structs {
            ir.structs.insert(strukt.name.clone(), lower_logic::lower_struct(strukt));
        }
        for tu in &module.tagged_unions {
            ir.enums.insert(tu.name.clone(), lower_logic::lower_enum(tu));
        }
        for ta in &module.type_aliases {
            ir.type_aliases.insert(ta.name.clone(), lower_logic::lower_type_alias(ta));
        }
        for comp in &module.components {
            ir.components.insert(comp.name.clone(), lower_component(comp));
        }
        for store in &module.stores {
            ir.stores.insert(store.name.clone(), lower_store(store));
        }
        for theme in &module.themes {
            // Themes are passed through as serialized JSON for now
            if let Ok(json) = serde_json::to_value(theme) {
                ir.themes.push(json);
            }
        }
        for imp in &module.imports {
            ir.imports.push(lower_logic::lower_import(imp));
        }
        for exp in &module.exports {
            ir.exports.push(lower_logic::lower_export(exp));
        }

        modules.insert(filepath.clone(), ir);
    }

    IRProgram {
        version: "1.0".into(),
        modules,
        source_map: HashMap::new(),
    }
}
```

- [ ] **Step 4: Build**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Some errors in codegen backends (still referencing old IR fields). Ignore for now.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/ir/
git commit -m "feat: implement IR lowering for all logic constructs"
```

> **Note:** Lowering unit tests depend on Task 7's `SemanticResult.diagnostics` field.
> They are written in Task 7 Step 6 below.

---

### Task 7: Fix semantic analysis — accumulate diagnostics, continue on error

**Files:**
- Modify: `omni-compiler/src/semantic/mod.rs`

- [ ] **Step 1: Add `diagnostics` field to `SemanticResult`, change analyze to accumulate**

In `omni-compiler/src/semantic/mod.rs`, update the struct:

```rust
#[derive(Debug)]
pub struct SemanticResult {
    pub symbols: SymbolTable,
    pub dependencies: HashMap<String, Vec<String>>,
    pub diagnostics: Vec<Diagnostic>,  // accumulated errors + warnings
}
```

Then change `analyze()` to always return a result, never early-exit:

```rust
pub fn analyze(program: &Program) -> SemanticResult {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}

// In SemanticAnalyzer:
pub fn analyze(&mut self, program: &Program) -> SemanticResult {
    // Pass 1: Collect symbols from all modules
    for (_filepath, module) in &program.modules {
        self.collect_module_symbols(module);
    }
    // Pass 2: Resolve types
    for (_filepath, module) in &program.modules {
        self.resolve_module_types(module);
    }
    // Pass 3: Validate
    for (_filepath, module) in &program.modules {
        self.validate_module(module);
    }
    // Always return what we have — diagnostics accumulated in self.diagnostics
    SemanticResult {
        symbols: std::mem::take(&mut self.symbols),
        dependencies: std::mem::take(&mut self.dependencies),
        diagnostics: std::mem::take(&mut self.diagnostics),
    }
}
```

The existing `push_diagnostic()` calls that used to `Err(diags)` now push to `self.diagnostics` instead.

- [ ] **Step 2: Adapt existing methods to work per-module**

Rename `collect_symbols(program)` to `collect_module_symbols(module: &Module)`. Same for resolver and validator.

- [ ] **Step 3: Update callers in lib.rs**

Change `compile()` to always get a `SemanticResult`:
```rust
let semantics = semantic::analyze(&program);
// Check semantics.diagnostics for errors, but always proceed to lowering
```

- [ ] **Step 4: Run semantic tests**

Run: `cargo test --lib semantic 2>&1`
Expected: Existing semantic tests pass (adjust as needed).

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/semantic/
git commit -m "refactor: semantic analysis accumulates diagnostics, never early-exits"
```

- [ ] **Step 6: Write IR lowering unit tests (moved from Task 6)**

With `SemanticResult.diagnostics` now available, add to `omni-compiler/src/ir/lower_logic.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser;

    #[test]
    fn test_lower_fn_main_to_ir() {
        let source = "fn main() { return 42; }";
        let tokens = tokenize(source);
        let program = parser::parse(&tokens).unwrap();
        let semantics = crate::semantic::SemanticResult {
            symbols: crate::semantic::symbols::SymbolTable::new(),
            dependencies: HashMap::new(),
            diagnostics: Vec::new(),
        };
        let ir = super::super::lower(&program, &semantics);
        let main_module = ir.modules.get("input.omni").unwrap();
        assert!(main_module.functions.contains_key("main"));
        let main_fn = &main_module.functions["main"];
        assert_eq!(main_fn.name, "main");
        assert_eq!(main_fn.visibility, "private");
    }

    #[test]
    fn test_lower_struct_to_ir() {
        let source = "struct Point { x: i32, y: i32 }";
        let tokens = tokenize(source);
        let program = parser::parse(&tokens).unwrap();
        let semantics = crate::semantic::SemanticResult {
            symbols: crate::semantic::symbols::SymbolTable::new(),
            dependencies: HashMap::new(),
            diagnostics: Vec::new(),
        };
        let ir = super::super::lower(&program, &semantics);
        let main_module = ir.modules.get("input.omni").unwrap();
        assert!(main_module.structs.contains_key("Point"));
    }
}
```

Run: `cargo test lower_fn_main_to_ir lower_struct_to_ir`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/src/ir/lower_logic.rs
git commit -m "test: add IR lowering unit tests for functions and structs"
```

---

### Task 8: Clean up Rust FFI — remove old functions, add omni_analyse

**Files:**
- Modify: `omni-compiler/src/lib.rs`

- [ ] **Step 1: Remove old FFI functions**

Delete these `#[no_mangle]` functions from `lib.rs`:
- `omni_compile_to_ir` (and its `*const c_char` signature)
- `omni_compile_to_omniview` (and its `*const c_char` signature)
- `omni_compile_to_target` (single-file `*const u8` version)
- `omni_validate_multi`
- `omni_format`
- `omni_version` (old `*const c_char` signature)

Keep: `omni_compile_multi_to_target`, `omni_free_string`, `omni_version()` (void version).

- [ ] **Step 2: Remove old functions from WASM features too**

Remove `compile_to_ir` and `compile_to_omniview` WASM exports.

- [ ] **Step 3: Add `omni_analyse` FFI function**

```rust
#[no_mangle]
pub unsafe extern "C" fn omni_analyse(
    request_json_ptr: *const u8,
    request_json_len: u32,
) -> *mut c_char {
    let request_str = if request_json_ptr.is_null() {
        return rust_to_c_string(
            r#"{"success":false,"diagnostics":[{"message":"null pointer","severity":"error"}]}"#.into(),
        );
    } else {
        let slice = unsafe { std::slice::from_raw_parts(request_json_ptr, request_json_len as usize) };
        std::str::from_utf8(slice).unwrap_or("{}")
    };

    #[derive(Deserialize)]
    struct AnalyseRequest {
        sources: HashMap<String, String>,
        #[serde(default)]
        path_mapping: HashMap<String, String>,
    }

    let input: AnalyseRequest = serde_json::from_str(request_str).unwrap_or(AnalyseRequest {
        sources: HashMap::new(),
        path_mapping: HashMap::new(),
    });

    let mut diagnostics: Vec<serde_json::Value> = Vec::new();

    for (path, source) in &input.sources {
        let tokens = lexer::tokenize(source);
        match parser::parse(&tokens) {
            Ok(program) => {
                let semantics = semantic::analyze(&program);
                // Collect diagnostics from semantic analysis
                for diag in &semantics.diagnostics {
                    diagnostics.push(serde_json::json!({
                        "severity": match diag.level {
                            crate::error::DiagnosticLevel::Error => "error",
                            crate::error::DiagnosticLevel::Warning => "warning",
                            _ => "info",
                        },
                        "code": diag.code,
                        "message": diag.message.clone(),
                        "file": path,
                        "line": diag.span.line,
                        "col": diag.span.col,
                        "suggestion": diag.suggestion.clone(),
                    }));
                }
                // Run lint rules (module created in Task 8.5)
                // TODO: after Task 8.5, uncomment:
                // let lint_diags = crate::semantic::lints::run_all(&program, &semantics);
                // for lint in &lint_diags { ... }
            }
            Err(e) => {
                diagnostics.push(serde_json::json!({
                    "severity": "error",
                    "code": "PARSE_ERROR",
                    "message": e.message,
                    "file": path,
                    "line": e.span.line,
                    "col": e.span.col,
                    "suggestion": null
                }));
            }
        }
    }

    let response = serde_json::json!({
        "success": diagnostics.iter().all(|d| d["severity"] != "error"),
        "diagnostics": diagnostics,
    });

    rust_to_c_string(response.to_string())
}
```

- [ ] **Step 4: Remove WASM feature flags from Cargo.toml**

In `omni-compiler/Cargo.toml`, remove the `wasm` feature and related dependencies (`wasm-bindgen`, `serde-wasm-bindgen`) since we no longer target WASM.

- [ ] **Step 5: Build**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Clean build with `omni_compile_multi_to_target`, `omni_analyse`, `omni_free_string`, `omni_version`.

- [ ] **Step 6: Commit**

```bash
git add omni-compiler/src/lib.rs omni-compiler/Cargo.toml
git commit -m "refactor: clean up FFI — remove old functions, add omni_analyse, WASM removal"
```

---

### Task 8.5: Create semantic lints module

**Files:**
- Create: `omni-compiler/src/semantic/lints.rs`
- Modify: `omni-compiler/src/semantic/mod.rs`

- [ ] **Step 1: Create `omni-compiler/src/semantic/lints.rs`**

```rust
//! Lint rules that run during static analysis.
//! Each rule inspects the AST and returns zero or more diagnostic warnings.

use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LintDiagnostic {
    pub rule: String,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

/// Run all lint rules against the program and accumulated semantic info.
pub fn run_all(program: &Program, _semantics: &SemanticResult) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    for (filepath, module) in &program.modules {
        check_unused_imports(filepath, module, &mut diags);
        // Additional lint rules added here as needed
    }
    diags
}

fn check_unused_imports(filepath: &str, module: &Module, diags: &mut Vec<LintDiagnostic>) {
    // Stub: in the initial implementation this is a no-op.
    // Future: cross-reference import names with actual usage sites.
    let _ = (filepath, module, diags);
}
```

- [ ] **Step 2: Register module in `omni-compiler/src/semantic/mod.rs`**

Add at the top:
```rust
pub mod lints;
```

- [ ] **Step 3: Uncomment lint calls in `omni_analyse` (Task 8 Step 3)**

In `omni-compiler/src/lib.rs`, uncomment the lint block in `omni_analyse`:
```rust
// Run lint rules (module created in Task 8.5)
let lint_diags = crate::semantic::lints::run_all(&program, &semantics);
for lint in &lint_diags {
    diagnostics.push(serde_json::json!({
        "severity": "warning",
        "code": lint.rule.clone(),
        "message": lint.message.clone(),
        "file": path,
        "line": lint.line,
        "col": lint.col,
        "suggestion": null,
    }));
}
```

- [ ] **Step 4: Build**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Clean build with lints module.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/semantic/lints.rs omni-compiler/src/semantic/mod.rs omni-compiler/src/lib.rs
git commit -m "feat: add semantic lints module with unused-imports stub"
```

---

### Task 9: Rewrite logic codegen — Python backend

**Files:**
- Modify: `omni-compiler/src/codegen/python.rs`

- [ ] **Step 1: Rewrite `generate()` to iterate modules**

```rust
impl Codegen for PythonCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_path, module) in &ir.modules {
            let mut w = CodeWriter::new();
            w.emit(&format!("# Generated from {}", module_path));
            w.emit("");

            // Generate structs as dataclasses
            for (_, strukt) in &module.structs {
                generate_struct(&mut w, strukt);
                w.emit("");
            }

            // Generate enums
            for (_, enum_def) in &module.enums {
                generate_enum(&mut w, enum_def);
                w.emit("");
            }

            // Generate type aliases
            for (name, ta) in &module.type_aliases {
                w.emit(&format!("{} = {}", name, format_type_ir(&ta.target)));
                w.emit("");
            }

            // Generate standalone functions
            for (_, func) in &module.functions {
                generate_function(&mut w, func, false);
                w.emit("");
            }

            let out_path = module_path.replace(".omni", ".py");
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}
```

- [ ] **Step 2: Add `generate_struct()` and `generate_enum()`**

```rust
fn generate_struct(w: &mut CodeWriter, s: &StructIR) {
    w.emit("@dataclass");
    w.emit(&format!("class {}:", s.name));
    w.indent();
    for field in &s.fields {
        let type_str = format_type_ir(&field.r#type);
        w.emit(&format!("{}: {}", field.name, type_str));
    }
    w.dedent();
}

fn generate_enum(w: &mut CodeWriter, e: &EnumIR) {
    w.emit(&format!("class {}(Enum):", e.name));
    w.indent();
    for (i, variant) in e.variants.iter().enumerate() {
        let val = match &variant.payload {
            Some(_) => "None",
            None => {
                w.emit(&format!("{} = {}", variant.name, i));
                continue;
            }
        };
        w.emit(&format!("{} = {}", variant.name, val));
    }
    w.dedent();
}
```

- [ ] **Step 3: Update `generate_function()` to accept `is_standalone` param**

Modify the existing `generate_function(w, func)` to `generate_function(w, func, is_standalone: bool)`. When `is_standalone`, don't include `self` in params:

```rust
fn generate_function(w: &mut CodeWriter, func: &FunctionIR, is_standalone: bool) {
    let mut param_strs: Vec<String> = if is_standalone {
        vec![]
    } else {
        vec!["self".to_string()]
    };
    // ... rest unchanged
}
```

- [ ] **Step 4: Add `format_type_ir()` helper**

```rust
fn format_type_ir(ty: &TypeIR) -> String {
    match ty.kind.as_str() {
        "i32" | "i64" | "int" | "u32" | "u64" => "int".into(),
        "f64" | "f32" | "float" => "float".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "str".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = ty.type_args.first() {
                format!("list[{}]", format_type_ir(inner))
            } else {
                "list".into()
            }
        }
        other => {
            if other.chars().next().map_or(false, |c| c.is_uppercase()) {
                format!("'{}'", other)
            } else {
                "Any".into()
            }
        }
    }
}
```

- [ ] **Step 5: Run existing Python codegen tests**

Run: `cargo test --lib codegen::python 2>&1`
Expected: Tests need updating for module-based IR. Fix them.

- [ ] **Step 6: Add test for standalone function generation**

```rust
#[test]
fn test_generate_standalone_function() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "main".into(),
        visibility: "private".into(),
        generics: vec![],
        params: vec![],
        return_type: None,
        body: BlockIR { statements: vec![], execution_mode: "client-only".into() },
    };
    generate_function(&mut w, &func, true);
    let out = w.into_string();
    assert!(out.contains("def main():"));
    assert!(out.contains("pass"));
    assert!(!out.contains("self"));
}
```

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/src/codegen/python.rs
git commit -m "feat: rewrite Python codegen for logic constructs (functions, structs, enums)"
```

---

### Task 10: Rewrite logic codegen — JavaScript backend

**Files:**
- Modify: `omni-compiler/src/codegen/javascript.rs`

- [ ] **Step 1: Add standalone function generation**

Similar to Python task, add `generate_function(func, is_standalone)`, `generate_struct()`, `generate_enum()`, `generate_type_alias()` methods adapted for JavaScript output.

Standalone function:
```javascript
function main() {
  return 42;
}
```

Struct becomes a class:
```javascript
class Point {
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }
}
```

TypeScript mode (when `typescript_mode: true`): add type annotations:
```typescript
function main(): number {
  return 42;
}
```

- [ ] **Step 2: Ensure `JavaScriptCodegen` struct has `typescript_mode: bool` field**

```rust
pub struct JavaScriptCodegen {
    typescript_mode: bool,
}

impl JavaScriptCodegen {
    pub fn new(typescript_mode: bool) -> Self {
        Self { typescript_mode }
    }
}
```

- [ ] **Step 3: Run existing tests, add new ones**

Run: `cargo test --lib codegen::javascript 2>&1`

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/javascript.rs
git commit -m "feat: rewrite JS/TS codegen for logic constructs"
```

---

### Task 11: Rewrite logic codegen — Rust backend

**Files:**
- Modify: `omni-compiler/src/codegen/rust.rs`

- [ ] **Step 1: Rewrite `generate()` to iterate modules and produce Rust code**

```rust
impl Codegen for RustCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_path, module) in &ir.modules {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", module_path));
            w.emit("");

            // Structs
            for (_, s) in &module.structs {
                let vis = if s.visibility == "public" { "pub " } else { "" };
                w.emit(&format!("{}struct {} {{", vis, s.name));
                w.indent();
                for field in &s.fields {
                    let rust_ty = format_type_ir_rust(&field.r#type);
                    let fvis = if field.visibility == "public" { "pub " } else { "" };
                    w.emit(&format!("{}{}: {},", fvis, field.name, rust_ty));
                }
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            // Enums
            for (_, e) in &module.enums {
                let vis = if e.visibility == "public" { "pub " } else { "" };
                w.emit(&format!("{}enum {} {{", vis, e.name));
                w.indent();
                for variant in &e.variants {
                    if variant.fields.is_empty() {
                        w.emit(&format!("{},", variant.name));
                    } else {
                        let fields_str: Vec<String> = variant.fields.iter()
                            .map(|f| format!("{}: {}", f.name, format_type_ir_rust(&f.r#type)))
                            .collect();
                        w.emit(&format!("{} {{ {} }},", variant.name, fields_str.join(", ")));
                    }
                }
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            // Type aliases
            for (name, ta) in &module.type_aliases {
                let vis = if ta.visibility == "public" { "pub " } else { "" };
                w.emit(&format!("{}{} = {};", vis, name, format_type_ir_rust(&ta.target)));
                w.emit("");
            }

            // Standalone functions
            for (_, func) in &module.functions {
                let vis = if func.visibility == "public" { "pub " } else { "" };
                let params: Vec<String> = func.params.iter()
                    .map(|p| format!("{}: {}", p.name, format_type_ir_rust(p.r#type.as_ref().unwrap())))
                    .collect();
                let ret = match &func.return_type {
                    Some(t) => format!(" -> {}", format_type_ir_rust(t)),
                    None => String::new(),
                };
                w.emit(&format!("{}fn {}({}){} {{", vis, func.name, params.join(", "), ret));
                w.indent();
                for stmt in &func.body.statements {
                    generate_statement_rust(&mut w, stmt);
                }
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            let out_path = module_path.replace(".omni", ".rs");
            files.push(CompiledFile { path: out_path, content: w.into_string() });
        }

        CompileOutput::success(files)
    }
}

fn format_type_ir_rust(ty: &TypeIR) -> String {
    match ty.kind.as_str() {
        "i32" => "i32".into(),
        "i64" => "i64".into(),
        "u32" => "u32".into(),
        "u64" => "u64".into(),
        "f64" => "f64".into(),
        "f32" => "f32".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "String".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = ty.type_args.first() {
                format!("Vec<{}>", format_type_ir_rust(inner))
            } else {
                "Vec<_>".into()
            }
        }
        "HashMap" | "Map" => "std::collections::HashMap<String, _>".into(),
        other => other.to_string(),
    }
}
```

- [ ] **Step 2: Add test**

```rust
#[test]
fn test_generate_standalone_rust_function() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "main".into(),
        visibility: "private".into(),
        params: vec![],
        return_type: None,
        body: BlockIR { statements: vec![], execution_mode: "client-only".into() },
    };
    generate_function_rust(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("fn main()"));
}
```

- [ ] **Step 3: Build and test**

Run: `cargo test --lib codegen::rust 2>&1`

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/rust.rs
git commit -m "feat: rewrite Rust codegen for logic constructs"
```

---

### Task 12: Rewrite logic codegen — Dart backend

**Files:**
- Modify: `omni-compiler/src/codegen/dart.rs`

- [ ] **Step 1: Rewrite `generate()` to iterate modules and produce Dart code**

```rust
impl Codegen for DartCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_path, module) in &ir.modules {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", module_path));
            w.emit("");

            // Structs → Dart classes
            for (_, s) in &module.structs {
                w.emit(&format!("class {} {{", s.name));
                w.indent();
                for field in &s.fields {
                    let dart_ty = format_type_ir_dart(&field.r#type);
                    w.emit(&format!("final {} {};", dart_ty, field.name));
                }
                // Constructor
                let params: Vec<String> = s.fields.iter()
                    .map(|f| format!("required this.{}", f.name))
                    .collect();
                w.emit(&format!("{}({{{}}});", s.name, params.join(", ")));
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            // Enums → Dart enums
            for (_, e) in &module.enums {
                w.emit(&format!("enum {} {{", e.name));
                w.indent();
                for variant in &e.variants {
                    if variant.fields.is_empty() {
                        w.emit(&format!("{},", variant.name));
                    } else {
                        let fields_str: Vec<String> = variant.fields.iter()
                            .map(|f| format!("{} {}", format_type_ir_dart(&f.r#type), f.name))
                            .collect();
                        w.emit(&format!("{}({}),", variant.name, fields_str.join(", ")));
                    }
                }
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            // Type aliases → Dart typedef
            for (name, ta) in &module.type_aliases {
                w.emit(&format!("typedef {} = {};", name, format_type_ir_dart(&ta.target)));
                w.emit("");
            }

            // Standalone functions
            for (_, func) in &module.functions {
                let ret = match &func.return_type {
                    Some(t) => format_type_ir_dart(t),
                    None => "void".into(),
                };
                let params: Vec<String> = func.params.iter()
                    .map(|p| format!("{} {}", format_type_ir_dart(p.r#type.as_ref().unwrap()), p.name))
                    .collect();
                w.emit(&format!("{} {}({}) {{", ret, func.name, params.join(", ")));
                w.indent();
                for stmt in &func.body.statements {
                    generate_statement_dart(&mut w, stmt);
                }
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            let out_path = module_path.replace(".omni", ".dart");
            files.push(CompiledFile { path: out_path, content: w.into_string() });
        }

        CompileOutput::success(files)
    }
}

fn format_type_ir_dart(ty: &TypeIR) -> String {
    match ty.kind.as_str() {
        "i32" | "i64" | "u32" | "u64" | "int" => "int".into(),
        "f64" | "f32" | "float" => "double".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "String".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = ty.type_args.first() {
                format!("List<{}>", format_type_ir_dart(inner))
            } else {
                "List".into()
            }
        }
        "Map" | "HashMap" => "Map<String, dynamic>".into(),
        other => other.to_string(),
    }
}
```

- [ ] **Step 2: Add test**

```rust
#[test]
fn test_generate_dart_function() {
    let func = FunctionIR {
        name: "greet".into(), visibility: "public".into(),
        params: vec![ParamIR { name: "name".into(), r#type: Some(TypeIR { kind: "string".into(), type_args: vec![] }) }],
        return_type: Some(TypeIR { kind: "string".into(), type_args: vec![] }),
        body: BlockIR { statements: vec![], execution_mode: "client-only".into() },
    };
    // Verify output contains "String greet(String name)"
    // (actual test uses CodeWriter + generate_function_dart helper)
}
```

- [ ] **Step 3: Build and test**

Run: `cargo test --lib codegen::dart 2>&1`

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/dart.rs
git commit -m "feat: rewrite Dart codegen for logic constructs"
```

---

### Task 13: Adapt view codegen — React

**Files:**
- Modify: `omni-compiler/src/codegen/react.rs`

- [ ] **Step 1: Update `generate()` to iterate `ir.modules`**

Replace `ir.components` top-level access with module iteration:

```rust
fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
    let mut files = Vec::new();

    for (module_path, module) in &ir.modules {
        // Skip modules without components (logic-only files)
        if module.components.is_empty() {
            continue;
        }

        let mut w = CodeWriter::new();
        w.emit(&format!("// Generated from {}", module_path));
        w.emit("import React from 'react';");
        w.emit("");

        for (_, comp) in &module.components {
            generate_react_component(&mut w, comp, module);
        }

        let out_path = module_path.replace(".omni", ".tsx");
        files.push(CompiledFile { path: out_path, content: w.into_string() });
    }

    CompileOutput::success(files)
}
```

The `generate_react_component()` and existing view-tree helpers remain unchanged — only the dispatch loop changes.

- [ ] **Step 2: Build and test**

Run: `cargo test --lib codegen::react 2>&1`

- [ ] **Step 3: Commit**

```bash
git add omni-compiler/src/codegen/react.rs
git commit -m "refactor: adapt React codegen to modules-based IR"
```

---

### Task 14: Adapt view codegen — Vue

**Files:**
- Modify: `omni-compiler/src/codegen/vue.rs`

- [ ] **Step 1: Update to module-based IR iteration**

Same pattern as React — wrap in `for (module_path, module) in &ir.modules`, skip modules with empty `components`, iterate `module.components`/`module.stores` instead of `ir.components`/`ir.stores`.

```rust
fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
    let mut files = Vec::new();
    for (module_path, module) in &ir.modules {
        if module.components.is_empty() { continue; }
        let mut w = CodeWriter::new();
        for (_, comp) in &module.components {
            generate_vue_sfc(&mut w, comp, module);
        }
        let out_path = module_path.replace(".omni", ".vue");
        files.push(CompiledFile { path: out_path, content: w.into_string() });
    }
    CompileOutput::success(files)
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib codegen::vue 2>&1`

- [ ] **Step 3: Commit**

---

### Task 15: Adapt view codegen — Dioxus

**Files:**
- Modify: `omni-compiler/src/codegen/dioxus.rs`

- [ ] **Step 1: Update to module-based IR iteration**

```rust
fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
    let mut files = Vec::new();
    for (module_path, module) in &ir.modules {
        if module.components.is_empty() { continue; }
        let mut w = CodeWriter::new();
        for (_, comp) in &module.components {
            generate_dioxus_component(&mut w, comp, module);
        }
        let out_path = module_path.replace(".omni", ".rs");
        files.push(CompiledFile { path: out_path, content: w.into_string() });
    }
    CompileOutput::success(files)
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib codegen::dioxus 2>&1`

- [ ] **Step 3: Commit**

---

### Task 16: Adapt view codegen — Flutter

**Files:**
- Modify: `omni-compiler/src/codegen/flutter.rs`

- [ ] **Step 1: Update to module-based IR iteration**

```rust
fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
    let mut files = Vec::new();
    for (module_path, module) in &ir.modules {
        if module.components.is_empty() { continue; }
        let mut w = CodeWriter::new();
        for (_, comp) in &module.components {
            generate_flutter_widget(&mut w, comp, module);
        }
        let out_path = module_path.replace(".omni", ".dart");
        files.push(CompiledFile { path: out_path, content: w.into_string() });
    }
    CompileOutput::success(files)
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib codegen::flutter 2>&1`

- [ ] **Step 3: Commit**

---

### Task 17: Adapt view codegen — OmniView

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`

- [ ] **Step 1: Update `compile_files()` to iterate `ir.modules`**

```rust
fn compile_files(&self, ir: &IRProgram) -> CompileOutput {
    // OmniView always produces 3 files regardless of input count
    let mut html_body = String::new();
    let mut js_runtime = String::new();
    let mut css_styles = String::new();

    for (_module_path, module) in &ir.modules {
        for (_, comp) in &module.components {
            let (html, js, css) = generate_omniview_component(comp, module);
            html_body.push_str(&html);
            js_runtime.push_str(&js);
            css_styles.push_str(&css);
        }
    }

    let mut files = HashMap::new();
    files.insert("index.html".into(), html_body);
    files.insert("index.js".into(), js_runtime);
    files.insert("index.css".into(), css_styles);
    CompileOutput { success: true, files, errors: vec![], warnings: vec![] }
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib codegen::omniview 2>&1`

- [ ] **Step 3: Commit**

---

### Task 18: Update codegen dispatch in mod.rs

**Files:**
- Modify: `omni-compiler/src/codegen/mod.rs`

- [ ] **Step 1: Ensure `compile()` dispatch works with new IR shape**

No changes needed to the `compile()` function itself — it passes `ir: &IRProgram` to each backend unchanged. Verify the build passes.

- [ ] **Step 2: Add TypeScript target that wasn't a separate file**

The TypeScript case already routes to `JavaScriptCodegen::new(true)`. No new file needed.

- [ ] **Step 3: Build**

Run: `cd omni-compiler && cargo build 2>&1`
Expected: Clean build, all codegen backends compile.

- [ ] **Step 4: Run all codegen tests**

Run: `cargo test --lib codegen 2>&1`

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/codegen/
git commit -m "refactor: adapt all codegen backends to modules-based IR"
```

---

### Task 18.5: Add codegen snapshot test infrastructure

**Files:**
- Create: `omni-compiler/tests/snapshots/`

- [ ] **Step 1: Add snapshot test for Python codegen**

Create `omni-compiler/tests/codegen_snapshots.rs`:

```rust
//! Snapshot tests: verify that given IR input, codegen backends produce expected output.
//! Each test builds an IRProgram programmatically, runs it through a codegen backend,
//! and compares the output against a stored snapshot file.

use omni_compiler::ir::*;
use omni_compiler::codegen::*;
use std::collections::HashMap;

fn make_test_module() -> ModuleIR {
    ModuleIR {
        name: "test.omni".into(),
        functions: {
            let mut m = HashMap::new();
            m.insert("main".into(), FunctionIR {
                name: "main".into(),
                visibility: "private".into(),
                params: vec![],
                return_type: None,
                body: BlockIR { statements: vec![], execution_mode: "client-only".into() },
            });
            m
        },
        structs: {
            let mut m = HashMap::new();
            m.insert("Point".into(), StructIR {
                name: "Point".into(),
                visibility: "public".into(),
                fields: vec![
                    FieldIR { name: "x".into(), r#type: TypeIR { kind: "i32".into(), type_args: vec![] }, visibility: "public".into() },
                    FieldIR { name: "y".into(), r#type: TypeIR { kind: "i32".into(), type_args: vec![] }, visibility: "public".into() },
                ],
            });
            m
        },
        ..Default::default()
    }
}

fn make_test_ir() -> IRProgram {
    let mut modules = HashMap::new();
    modules.insert("test.omni".into(), make_test_module());
    IRProgram { version: "1.0".into(), modules, source_map: HashMap::new() }
}

#[test]
fn snapshot_python_basic() {
    let ir = make_test_ir();
    let codegen = PythonCodegen;
    let output = codegen.generate(&ir, &HashMap::new());
    assert!(output.success);
    let content = &output.files[0].content;
    // Verify key expected output patterns
    assert!(content.contains("def main():"));
    assert!(content.contains("class Point:"));
    assert!(content.contains("x: int"));
}

#[test]
fn snapshot_javascript_basic() {
    let ir = make_test_ir();
    let codegen = JavaScriptCodegen::new(false);
    let output = codegen.generate(&ir, &HashMap::new());
    assert!(output.success);
    let content = &output.files[0].content;
    assert!(content.contains("function main()"));
    assert!(content.contains("class Point"));
}

#[test]
fn snapshot_typescript_basic() {
    let ir = make_test_ir();
    let codegen = JavaScriptCodegen::new(true);
    let output = codegen.generate(&ir, &HashMap::new());
    assert!(output.success);
    let content = &output.files[0].content;
    assert!(content.contains(": number"));  // TypeScript type annotations
}
```

Run: `cargo test --test codegen_snapshots 2>&1`
Expected: 3 PASS

- [ ] **Step 2: Commit**

```bash
git add omni-compiler/tests/codegen_snapshots.rs
git commit -m "test: add codegen snapshot test infrastructure"
```

---

### Task 18.6: Add CI build configuration

**Files:**
- Create: `.github/workflows/ci.yml` (or update existing)

- [ ] **Step 1: Create CI workflow that builds DLL and runs all tests**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rust-build-and-test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build Rust compiler (release)
        run: cd omni-compiler && cargo build --release
      - name: Run Rust tests
        run: cd omni-compiler && cargo test
      - name: Upload DLL artifact
        uses: actions/upload-artifact@v4
        with:
          name: omni_compiler-${{ matrix.os }}
          path: omni-compiler/target/release/omni_compiler.*

  python-tests:
    runs-on: ${{ matrix.os }}
    needs: rust-build-and-test
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - name: Download DLL artifact
        uses: actions/download-artifact@v4
        with:
          name: omni_compiler-${{ matrix.os }}
          path: omni-compiler/target/release/
      - name: Install Python deps
        run: pip install pytest
      - name: Run Python tests
        run: pytest tests/ -v
      - name: Run integration test projects
        run: |
          cd test/projects/logic-counter && python -m omnilang.cli build
          cd test/projects/view-todo && python -m omnilang.cli build -t omniview
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add cross-platform CI pipeline for Rust DLL + Python tests"
```

---

> **Note on TypeIR:** The `TypeIR` struct used throughout codegen assumes the following shape
> (defined in `omni-compiler/src/ir/mod.rs`):
> ```rust
> #[derive(Debug, Clone, Serialize, Deserialize)]
> pub struct TypeIR {
>     pub kind: String,        // "i32", "string", "Vec", custom type name, etc.
>     pub type_args: Vec<TypeIR>,  // generic parameters, empty if none
> }
> ```
> The `format_type_ir_*()` helpers in each backend match on `kind` to produce the
> target language's native type. Custom/user types are passed through unchanged.

---

### Task 19: Create Python structured error types

**Files:**
- Create: `omnilang/errors.py`

- [ ] **Step 1: Create `omnilang/errors.py`**

```python
"""Structured error types for the OmniLang CLI."""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal


@dataclass
class OmniError:
    """A single diagnostic from compilation or analysis."""
    severity: Literal["error", "warning", "info"]
    code: str
    message: str
    file: str | None = None
    line: int | None = None
    col: int | None = None
    suggestion: str | None = None

    def format(self) -> str:
        loc = ""
        if self.file:
            loc = f"{self.file}"
            if self.line is not None:
                loc += f":{self.line}"
                if self.col is not None:
                    loc += f":{self.col}"
        sev = self.severity.capitalize()
        msg = f"{sev}: {self.message}"
        if loc:
            msg += f"\n  -> {loc} ({self.code})"
        if self.suggestion:
            msg += f"\n  Suggestion: {self.suggestion}"
        return msg


class OmniLangError(Exception):
    """Base exception for all omnic CLI errors."""
    exit_code: int = 1

    def __init__(self, errors: list[OmniError] | None = None):
        self.errors = errors or []
        super().__init__("")

    def format(self) -> str:
        if not self.errors:
            return "An unknown error occurred."
        lines = [e.format() for e in self.errors]
        error_count = sum(1 for e in self.errors if e.severity == "error")
        warn_count = sum(1 for e in self.errors if e.severity == "warning")
        summary_parts = []
        if error_count:
            summary_parts.append(f"{error_count} error(s)")
        if warn_count:
            summary_parts.append(f"{warn_count} warning(s)")
        if summary_parts:
            lines.append("")
            lines.append(", ".join(summary_parts))
        return "\n".join(lines)


class CompilerUnavailable(OmniLangError):
    """Rust compiler DLL is not found."""
    exit_code = 2

    def __init__(self):
        super().__init__([OmniError(
            severity="error",
            code="COMPILER_UNAVAILABLE",
            message="Rust compiler DLL (omni_compiler.dll) not found. "
                    "Build it with: cd omni-compiler && cargo build --release",
        )])


class CompileError(OmniLangError):
    """Compilation failed."""
    exit_code = 1


class AnalyseError(OmniLangError):
    """Static analysis found issues."""
    exit_code = 1


class ConfigError(OmniLangError):
    """Project configuration issue."""
    exit_code = 3

    @classmethod
    def not_found(cls) -> "ConfigError":
        return cls([OmniError(
            severity="error",
            code="CONFIG_NOT_FOUND",
            message="No .omnilang.toml found. Run 'omnic create <name>' to create a project.",
        )])


class InputError(OmniLangError):
    """Bad CLI arguments or file not found."""
    exit_code = 4

    @classmethod
    def file_not_found(cls, filepath: str) -> "InputError":
        return cls([OmniError(
            severity="error",
            code="FILE_NOT_FOUND",
            message=f"File not found: {filepath}",
            file=filepath,
        )])
```

- [ ] **Step 2: Write tests**

Create `tests/test_errors.py`:

```python
import pytest
from omnilang.errors import OmniError, CompilerUnavailable, InputError


def test_omni_error_format():
    err = OmniError(
        severity="error", code="E0001",
        message="cannot resolve import",
        file="src/main.omni", line=3, col=8,
        suggestion="check the import path"
    )
    output = err.format()
    assert "Error:" in output
    assert "src/main.omni:3:8" in output
    assert "E0001" in output
    assert "check the import path" in output


def test_compiler_unavailable():
    err = CompilerUnavailable()
    assert err.exit_code == 2
    assert "omni_compiler.dll" in err.format()


def test_input_error_file_not_found():
    err = InputError.file_not_found("missing.omni")
    assert err.exit_code == 4
    assert "missing.omni" in err.format()


def test_multiple_errors():
    err = CompileError([
        OmniError("error", "E0001", "first error", "a.omni", 1, 0, None),
        OmniError("warning", "W0001", "first warning", "a.omni", 2, 0, None),
    ])
    output = err.format()
    assert "1 error(s)" in output
    assert "1 warning(s)" in output


def test_config_error_not_found():
    err = ConfigError.not_found()
    assert err.exit_code == 3
    assert ".omnilang.toml" in err.format()


def test_config_error_malformed():
    from omnilang.errors import OmniError
    err = ConfigError([OmniError(
        severity="error", code="CONFIG_MALFORMED",
        message="Failed to parse .omnilang.toml: unexpected token at line 3",
        file=".omnilang.toml", line=3, col=1,
    )])
    assert err.exit_code == 3
    output = err.format()
    assert ".omnilang.toml" in output
    assert "line 3" in output.lower() or ":3" in output
```

Run: `pytest tests/test_errors.py -v`
Expected: 6 PASS

- [ ] **Step 3: Commit**

```bash
git add omnilang/errors.py tests/test_errors.py
git commit -m "feat: add structured Python error types with tests"
```

---

### Task 20: Create CompileResult and AnalyseResult dataclasses

**Files:**
- Create: `omnilang/result.py`

- [ ] **Step 1: Create `omnilang/result.py`**

```python
"""Result types for compilation and analysis operations."""
from __future__ import annotations

from dataclasses import dataclass, field
from omnilang.errors import OmniError


@dataclass
class CompileResult:
    success: bool
    files: dict[str, str] = field(default_factory=dict)
    errors: list[OmniError] = field(default_factory=list)
    warnings: list[OmniError] = field(default_factory=list)

    @classmethod
    def from_rust_response(cls, data: dict, default_file: str = "<input>") -> "CompileResult":
        errors: list[OmniError] = []
        warnings: list[OmniError] = []
        for item in data.get("errors", []):
            err = OmniError(
                severity=item.get("severity", "error"),
                code=item.get("code", "RUST_ERROR"),
                message=str(item.get("message", item)),
                file=item.get("file", default_file),
                line=item.get("line"),
                col=item.get("col"),
                suggestion=item.get("suggestion"),
            )
            if err.severity == "warning":
                warnings.append(err)
            else:
                errors.append(err)
        for item in data.get("warnings", []):
            if isinstance(item, dict):
                warnings.append(OmniError(
                    severity="warning",
                    code=item.get("code", "RUST_WARN"),
                    message=str(item.get("message", item)),
                    file=item.get("file", default_file),
                    line=item.get("line"),
                    col=item.get("col"),
                ))
        return cls(
            success=data.get("success", False),
            files=data.get("files", {}),
            errors=errors,
            warnings=warnings,
        )


@dataclass
class AnalyseResult:
    success: bool
    diagnostics: list[OmniError] = field(default_factory=list)

    @classmethod
    def from_rust_response(cls, data: dict, default_file: str = "<input>") -> "AnalyseResult":
        diagnostics: list[OmniError] = []
        for item in data.get("diagnostics", []):
            diagnostics.append(OmniError(
                severity=item.get("severity", "error"),
                code=item.get("code", "ANALYSE"),
                message=str(item.get("message", item)),
                file=item.get("file", default_file),
                line=item.get("line"),
                col=item.get("col"),
                suggestion=item.get("suggestion"),
            ))
        return cls(
            success=data.get("success", False),
            diagnostics=diagnostics,
        )
```

- [ ] **Step 2: Commit**

```bash
git add omnilang/result.py
git commit -m "feat: add CompileResult and AnalyseResult dataclasses"
```

---

### Task 21: Rewrite RustBridge — DLL-only, simplified

**Files:**
- Modify: `omnilang/rust_bridge.py`

- [ ] **Step 1: Rewrite `RustBridge` to remove CLI fallback**

```python
"""Rust bridge — ctypes DLL interface for omni-compiler."""
from __future__ import annotations

import ctypes
import json
from pathlib import Path

from omnilang.errors import CompilerUnavailable
from omnilang.result import CompileResult, AnalyseResult


class RustBridge:
    _dll: ctypes.CDLL | None = None
    _checked: bool = False
    _dll_search_paths: list[Path] = [
        Path(__file__).parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll",
    ]
    MIN_VERSION: str = "0.1.0"

    @classmethod
    def _load(cls) -> ctypes.CDLL:
        if cls._dll is not None:
            return cls._dll
        if cls._checked:
            raise CompilerUnavailable()

        for p in cls._dll_search_paths:
            if p.exists():
                cls._dll = ctypes.CDLL(str(p))
                # Verify ABI compatibility
                ver_func = cls._dll.omni_version
                ver_func.argtypes = []
                ver_func.restype = ctypes.c_void_p
                ptr = ver_func()
                if ptr:
                    version = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
                    cls._dll.omni_free_string(ptr)
                    # Parse and check version
                    # If incompatible, raise clear error
                return cls._dll

        cls._checked = True
        raise CompilerUnavailable()

    @classmethod
    def compile(cls, sources: dict[str, str], target: str,
                path_mapping: dict[str, str] | None = None) -> CompileResult:
        dll = cls._load()
        func = dll.omni_compile_multi_to_target
        func.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32]
        func.restype = ctypes.c_void_p

        sources_bytes = json.dumps(sources).encode("utf-8")
        target_bytes = target.encode("utf-8")
        mapping_bytes = json.dumps(path_mapping or {}).encode("utf-8")

        ptr = func(
            ctypes.cast(sources_bytes, ctypes.c_void_p), ctypes.c_uint32(len(sources_bytes)),
            ctypes.cast(target_bytes, ctypes.c_void_p), ctypes.c_uint32(len(target_bytes)),
            ctypes.cast(mapping_bytes, ctypes.c_void_p), ctypes.c_uint32(len(mapping_bytes)),
        )
        if ptr is None:
            return CompileResult(success=False, errors=[...])  # null pointer error

        result_json = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
        dll.omni_free_string.argtypes = [ctypes.c_void_p]
        dll.omni_free_string.restype = None
        dll.omni_free_string(ptr)

        data = json.loads(result_json)
        return CompileResult.from_rust_response(data)

    @classmethod
    def analyse(cls, sources: dict[str, str],
                path_mapping: dict[str, str] | None = None) -> AnalyseResult:
        dll = cls._load()
        func = dll.omni_analyse
        func.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
        func.restype = ctypes.c_void_p

        request = json.dumps({
            "sources": sources,
            "path_mapping": path_mapping or {},
        }).encode("utf-8")

        ptr = func(ctypes.cast(request, ctypes.c_void_p), ctypes.c_uint32(len(request)))
        if ptr is None:
            return AnalyseResult(success=False)

        result_json = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
        dll.omni_free_string(ptr)

        data = json.loads(result_json)
        return AnalyseResult.from_rust_response(data)
```

- [ ] **Step 2: Remove old methods**

Remove: `is_available()`, `compile_to_ir()`, `compile_to_omniview()`, `_try_dll()`, `_try_cli()`, `_call_dll()`, `_try_dll_omniview()`, `_try_cli_omniview()`, `format()`, `validate_multi()`.

- [ ] **Step 3: Write tests**

Create `tests/test_rustbridge.py`:

```python
import pytest
from omnilang.rust_bridge import RustBridge
from omnilang.errors import CompilerUnavailable


def test_compiler_unavailable_when_dll_missing():
    import omnilang.rust_bridge as rb
    old_paths = list(rb.RustBridge._dll_search_paths)
    try:
        # Point to a nonexistent path to trigger CompilerUnavailable
        rb.RustBridge._dll_search_paths = [Path("/nonexistent/omni_compiler.dll")]
        rb.RustBridge._checked = False
        rb.RustBridge._dll = None
        with pytest.raises(CompilerUnavailable) as exc:
            rb.RustBridge._load()
        assert "omni_compiler.dll" in exc.value.format()
    finally:
        rb.RustBridge._dll_search_paths = old_paths
        rb.RustBridge._dll = None
        rb.RustBridge._checked = False
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/rust_bridge.py tests/test_rustbridge.py
git commit -m "refactor: rewrite RustBridge — DLL-only, add analyse(), remove CLI fallback"
```

---

### Task 22: Update `omnilang/api.py` — use new RustBridge and error types

**Files:**
- Modify: `omnilang/api.py`

- [ ] **Step 1: Rewrite `compile_omni_multi()`**

Replace manual JSON parsing and `CompileError` construction with `CompileResult.from_rust_response()`. Remove `_detect_ui_mode()` — auto-detection now happens in Rust.

```python
def compile_omni_multi(
    sources: dict[str, str],
    target: str = "python",
    path_mapping: dict[str, str] | None = None,
    **_kwargs,
) -> CompileResult:
    from omnilang.rust_bridge import RustBridge
    from omnilang.errors import CompileError

    primary = next(iter(sources.keys()), "<input>")

    try:
        result = RustBridge.compile(sources, target, path_mapping)
    except CompilerUnavailable:
        raise
    except Exception as e:
        raise CompileError([OmniError(
            severity="error", code="BRIDGE_ERROR",
            message=f"Rust compiler call failed: {e}",
            file=primary,
        )])

    if not result.success and result.errors:
        raise CompileError(result.errors + result.warnings)
    return result
```

- [ ] **Step 2: Keep `parse_view_tree()` and `parse_code_blocks()` for editor compat**

These functions call `RustBridge.compile_to_ir()` internally — after the redesign, this function no longer exists. Add a private `_compile_to_ir()` method to `RustBridge` that calls `omni_compile_multi_to_target` with a special "ir" target or adds a new FFI function just for the editor.

For now, in `api.py`:

```python
def parse_view_tree(source: str, filepath: str = "<input>") -> list[ViewTreeNode]:
    """Parse .omni source -> extract view element tree (for canvas rendering).
    
    After the IR redesign, this is maintained for editor compatibility.
    Uses RustBridge to compile and extract view info from the IR.
    """
    try:
        from omnilang.rust_bridge import RustBridge
        result = RustBridge.compile({filepath: source}, "python")
        # View tree extraction from compiled result is not yet supported
        # in the new IR. Use regex fallback for editor compatibility.
        return _extract_view_tree_regex(source)
    except Exception:
        return _extract_view_tree_regex(source)
```

- [ ] **Step 3: Fix `cmd_serve` compatibility — remove `compile_to_ir` dependency**

The editor server (`omnic serve`) calls `RustBridge.compile_to_ir()` through `api.py`'s `parse_view_tree()` and `parse_code_blocks()`. After the redesign, `compile_to_ir` no longer exists as an FFI function. Add a private internal IR helper in `omnilang/api.py`:

```python
# In omnilang/api.py, add a private helper used by the editor server:

def _compile_ir_internal(sources: dict[str, str]) -> dict:
    """Compile sources to IR JSON (internal use — editor server).
    
    Calls omni_compile_multi_to_target with target="python" to get
    compiled output for view tree extraction. This is a stopgap;
    a dedicated omni_compile_to_ir FFI function should be added
    in a follow-up task for full IR fidelity.
    """
    from omnilang.rust_bridge import RustBridge
    result = RustBridge.compile(sources, "python")
    return {"files": result.files, "success": result.success}
```

Then in `omnilang/cli.py`, update `cmd_serve`:

```python
def cmd_serve(args):
    """Start the visual editor server (FastAPI + Vite)."""
    # The serve command uses api.py functions for parsing.
    # With the IR redesign, parse_view_tree() and parse_code_blocks()
    # now use _compile_ir_internal() or regex fallback internally.
    from omnilang.api import parse_view_tree, parse_code_blocks
    # ... existing serve startup logic unchanged ...
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/api.py
git commit -m "refactor: update api.py to use new RustBridge, structured errors, fix serve compat"
```

---

### Task 23: Rewrite CLI — remove commands, add analyse, structured errors

**Files:**
- Modify: `omnilang/cli.py`

- [ ] **Step 1: Define `ALL_TARGETS` constant and `_collect_sources()` helper**

At the top of `omnilang/cli.py`, add:

```python
ALL_TARGETS = [
    "python", "js", "ts", "rust", "dart",
    "react", "vue", "dioxus", "flutter", "omniview",
]

LOGIC_TARGETS = {"python", "js", "ts", "rust", "dart"}
VIEW_TARGETS = {"react", "vue", "dioxus", "flutter", "omniview"}


def _collect_sources(args) -> dict[str, str]:
    """Collect .omni source files from CLI args (file, files, or directory).
    Returns {filepath: source_content} dict."""
    import os

    sources: dict[str, str] = {}

    # Single file
    if hasattr(args, "file") and args.file:
        path = args.file
        if not os.path.exists(path):
            raise InputError.file_not_found(path)
        with open(path, encoding="utf-8") as f:
            sources[path] = f.read()
        return sources

    # Multiple files
    if hasattr(args, "files") and args.files:
        for path in args.files:
            if not os.path.exists(path):
                raise InputError.file_not_found(path)
            with open(path, encoding="utf-8") as f:
                sources[path] = f.read()
        return sources

    # Directory: auto-discover src/main.omni
    if hasattr(args, "directory") and args.directory:
        root = args.directory
    else:
        root = "."

    src_dir = os.path.join(root, "src")
    if os.path.isdir(src_dir):
        for dirpath, _dirnames, filenames in os.walk(src_dir):
            for fn in filenames:
                if fn.endswith(".omni"):
                    full = os.path.join(dirpath, fn)
                    with open(full, encoding="utf-8") as f:
                        sources[full] = f.read()

    if not sources:
        raise InputError([OmniError(
            severity="error", code="NO_SOURCES",
            message="No .omni source files found. Run 'omnic create <name>' to create a project.",
        )])

    return sources
```

- [ ] **Step 2: Remove commands and arguments**

Remove: `init` subparser, `fix` subparser, `fmt` subparser, `lsp` subparser, `cmd_init()`, `cmd_fix()`, `cmd_fmt()`, `cmd_lsp()`.

- [ ] **Step 3: Rename `check` → `analyse`**

Rename `check_parser` → `analyse_parser`, add `--format` argument. Rename `cmd_check()` → `cmd_analyse()`.

```python
def cmd_analyse(args):
    """Run static analysis on .omni source via Rust compiler."""
    # Collect sources
    sources = _collect_sources(args)
    from omnilang.rust_bridge import RustBridge
    from omnilang.errors import AnalyseError
    from omnilang.result import AnalyseResult

    try:
        result = RustBridge.analyse(sources)
    except CompilerUnavailable:
        raise
    except Exception as e:
        raise AnalyseError([OmniError(
            severity="error", code="ANALYSE_FAILED",
            message=str(e),
        )])

    if args.format == "json":
        import json
        print(json.dumps([d.__dict__ for d in result.diagnostics], indent=2))
    else:
        for d in result.diagnostics:
            print(d.format())
        if result.success:
            print("No issues found.")

    if not result.success:
        raise AnalyseError(result.diagnostics)
```

- [ ] **Step 4: Add `_write_standalone_omniview()` helper**

The function that stitches omniview output into browser-runnable HTML:

```python
def _write_standalone_omniview(files: dict[str, str], output_dir: str):
    """Assemble OmniView output (body snippet, JS runtime, CSS) into a
    standalone, browser-runnable HTML file."""
    import os

    body = files.get("index.html", "")
    js = files.get("index.js", "")
    css = files.get("index.css", "")

    standalone = (
        "<!DOCTYPE html>\n"
        '<html lang="en">\n'
        "<head>\n"
        '<meta charset="UTF-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1.0">\n'
        "<title>OmniView App</title>\n"
        "<style>\n"
        "*,*::before,*::after{box-sizing:border-box}\n"
        "body{margin:0;padding:20px;font-family:system-ui,sans-serif;background:#0a0a1a}\n"
        f"{css}\n"
        "</style>\n"
        "</head>\n"
        "<body>\n"
        f"{body}\n"
        "<script>\n"
        f"{js}\n"
        "</script>\n"
        "</body>\n"
        "</html>"
    )

    os.makedirs(output_dir, exist_ok=True)
    out_path = os.path.join(output_dir, "index.html")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(standalone)
    print(f"  -> {out_path}")
```

- [ ] **Step 5: Update `cmd_run()` for view and logic projects**

```python
def cmd_run(args):
    import os, subprocess, webbrowser

    # Compile
    sources = _collect_sources(args)
    target = args.target
    if target is None:
        # Auto-detect: if source imports omniview or has components, it's a view project
        primary = next(iter(sources.values()), "")
        is_view = "import omniview" in primary or "component " in primary
        target = "omniview" if is_view else "python"

    result = RustBridge.compile(sources, target)

    if not result.success:
        raise CompileError(result.errors + result.warnings)

    output_dir = os.path.join("build", "debug")

    if target == "omniview":
        # View run: produce build/debug/index.html + index.js + index.css → open browser
        _write_standalone_omniview(result.files, output_dir)
        webbrowser.open(f"file:///{os.path.abspath(os.path.join(output_dir, 'index.html'))}")
    else:
        # Logic run: write all files to build/debug/, execute entry point
        for filename, content in result.files.items():
            out = os.path.join(output_dir, filename)
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with open(out, "w", encoding="utf-8") as f:
                f.write(content)
        # Execute the main module
        entry = os.path.join(output_dir, "src", "main.py")
        if target == "python" and os.path.exists(entry):
            subprocess.run(["python", entry])
        elif target == "js" or target == "ts":
            js_entry = os.path.join(output_dir, "src", "main.js")
            if os.path.exists(js_entry):
                subprocess.run(["node", js_entry])
```

- [ ] **Step 6: Add `cmd_create()` — scaffold new projects**

```python
LOGIC_TEMPLATE = '''// {name} — OmniLang logic project
fn main() {{
    print("Hello from {name}!");
}}
'''

VIEW_TEMPLATE = '''// {name} — OmniLang view project
import omniview;

component App {{
    view {{
        <div class: app>
            <h1>{name}</h1>
            <p>Welcome to your OmniView app!</p>
        </div>
    }}
}}
'''


def cmd_create(args):
    """Scaffold a new OmniLang project (no compiler dependency)."""
    import os

    project_dir = os.path.abspath(args.name)
    if os.path.exists(project_dir):
        raise InputError([OmniError(
            severity="error", code="DIR_EXISTS",
            message=f"Directory already exists: {project_dir}",
        )])

    project_type = args.type  # "logic" or "view"
    os.makedirs(os.path.join(project_dir, "src"), exist_ok=True)

    # Write .omnilang.toml
    config = (
        '[project]\n'
        f'name = "{args.name}"\n'
        'version = "0.1.0"\n'
        f'type = "{project_type}"\n'
        '\n'
        '[source]\n'
        'dir = "src"\n'
        '\n'
        '[output]\n'
        'dir = "build"\n'
    )
    with open(os.path.join(project_dir, ".omnilang.toml"), "w", encoding="utf-8") as f:
        f.write(config)

    # Write src/main.omni from template
    template = LOGIC_TEMPLATE if project_type == "logic" else VIEW_TEMPLATE
    with open(os.path.join(project_dir, "src", "main.omni"), "w", encoding="utf-8") as f:
        f.write(template.format(name=args.name))

    print(f"Created {project_type} project '{args.name}' at {project_dir}")
    print("  cd " + args.name)
    print("  omnic run src/main.omni")


# In _main(), add the create subparser:
# create_parser = subparsers.add_parser("create", help="Scaffold a new project")
# create_parser.add_argument("name", help="Project name")
# create_parser.add_argument("--type", "-t", choices=["logic", "view"], required=True,
#                            help="Project type: logic or view")
```

- [ ] **Step 7: Wrap main() with structured error handler**

```python
def main():
    try:
        _main()
    except OmniLangError as e:
        print(e.format(), file=sys.stderr)
        sys.exit(e.exit_code)
    except Exception as e:
        print(f"Internal error: {e}", file=sys.stderr)
        print("Please report this at https://github.com/.../issues", file=sys.stderr)
        sys.exit(99)

def _main():
    # ... current main() body ...
```

- [ ] **Step 8: Write CLI tests**

Create `tests/test_cli.py`:

```python
import pytest
from omnilang.cli import main as cli_main


def test_build_help(capsys):
    with pytest.raises(SystemExit):
        import sys
        sys.argv = ["omnic", "build", "--help"]
        cli_main()
    captured = capsys.readouterr()
    assert "--target" in captured.out


def test_create_logic(capsys, tmp_path):
    import sys
    sys.argv = ["omnic", "create", str(tmp_path / "myapp"), "-t", "logic"]
    with pytest.raises(SystemExit):
        cli_main()
    assert (tmp_path / "myapp" / ".omnilang.toml").exists()
    assert (tmp_path / "myapp" / "src" / "main.omni").exists()
```

Run: `pytest tests/test_cli.py -v`

- [ ] **Step 9: Commit**

```bash
git add omnilang/cli.py tests/test_cli.py
git commit -m "refactor: rewrite CLI — remove init/fmt/fix/lsp, add analyse/create, structured errors"
```

---

### Task 23.5: Add `omnic watch` command

**Files:**
- Modify: `omnilang/cli.py`

- [ ] **Step 1: Add watch subparser and command function**

```python
def cmd_watch(args):
    """Watch .omni files and auto-rebuild on changes."""
    import os
    import time

    input_file = args.file
    target = args.target

    if not os.path.exists(input_file):
        raise InputError.file_not_found(input_file)

    print(f"Watching {input_file} (target={target})")
    print("Press Ctrl+C to stop.")

    last_mtime = os.path.getmtime(input_file)
    build_count = 0

    from omnilang.rust_bridge import RustBridge

    try:
        while True:
            time.sleep(0.5)
            try:
                current_mtime = os.path.getmtime(input_file)
            except OSError:
                continue

            if current_mtime != last_mtime:
                last_mtime = current_mtime
                build_count += 1
                timestamp = time.strftime("%H:%M:%S")

                try:
                    with open(input_file, encoding="utf-8") as f:
                        source = f.read()
                except Exception:
                    continue

                from omnilang.config import load_config
                config = None
                try:
                    config = load_config()
                except Exception:
                    pass

                output_dir = args.output
                if config is not None and args.output == "./build":
                    output_dir = config.get("output", {}).get("dir", "build")

                try:
                    result = RustBridge.compile({input_file: source}, target)
                    if result.success:
                        os.makedirs(output_dir, exist_ok=True)
                        for filename, content in result.files.items():
                            out_path = os.path.join(output_dir, filename)
                            os.makedirs(os.path.dirname(out_path), exist_ok=True)
                            with open(out_path, "w", encoding="utf-8") as f:
                                f.write(content)
                        print(f"[{timestamp}] #{build_count} OK")
                    else:
                        for err in result.errors:
                            print(err.format(), file=sys.stderr)
                        print(f"[{timestamp}] #{build_count} FAILED")
                except OmniLangError as e:
                    print(f"[{timestamp}] #{build_count} FAILED: {e}", file=sys.stderr)
    except KeyboardInterrupt:
        print(f"\nStopped. ({build_count} build(s))")
```

In `_main()`, add the subparser:

```python
watch_parser = subparsers.add_parser("watch", help="Watch .omni files and auto-rebuild")
watch_parser.add_argument("file", help="Source file (.omni)")
watch_parser.add_argument("--target", "-t", choices=ALL_TARGETS, default="python",
                          help="Target language/framework (default: python)")
watch_parser.add_argument("--output", "-o", default="./build",
                          help="Output directory (default: ./build)")
```

- [ ] **Step 2: Commit**

```bash
git add omnilang/cli.py
git commit -m "feat: add omnic watch command"
```

---

### Task 24: End-to-end smoke test — logic-counter project

**Files:**
- Create: `test/projects/logic-counter/`

- [ ] **Step 1: Create test project structure**

```
test/projects/logic-counter/
├── .omnilang.toml
└── src/
    ├── main.omni
    └── utils/
        └── math.omni
```

`src/main.omni`:
```
import { add } from "./utils/math.omni";

struct Counter {
    value: i32,
}

fn main() {
    let c = Counter { value: 0 };
    let result = add(c.value, 5);
    print(result);
}
```

`src/utils/math.omni`:
```
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

`.omnilang.toml`:
```toml
[project]
name = "logic-counter"
version = "0.1.0"
targets = ["python"]

[source]
dir = "src"

[output]
dir = "build"
```

- [ ] **Step 2: Build and verify Python output**

Run from test project dir:
```bash
omnic build
```
Expected: `build/src/main.py` and `build/src/utils/math.py` exist.

```bash
python -c "import sys; sys.path.insert(0, 'build'); from src import main"
```
Expected: prints "5" (0 + 5).

- [ ] **Step 3: Run `omnic analyse`**

```bash
omnic analyse
```
Expected: "No issues found."

- [ ] **Step 4: Run `omnic run`**

```bash
omnic run src/main.omni
```
Expected: prints "5".

- [ ] **Step 5: Commit**

```bash
git add test/projects/logic-counter/
git commit -m "test: add logic-counter integration test project"
```

---

### Task 25: Integration test — view-todo project

**Files:**
- Create: `test/projects/view-todo/`

- [ ] **Step 1: Create test project**

```
test/projects/view-todo/
├── .omnilang.toml
└── src/
    ├── main.omni
    └── components/
        └── TodoList.omni
```

`src/main.omni`:
```
import omniview;
import { TodoList } from "./components/TodoList.omni";

component App {
    view {
        <div class: app>
            <TodoList />
        </div>
    }
}
```

- [ ] **Step 2: Build to omniview and verify 3 output files**

```bash
omnic build -t omniview
```
Expected: `build/index.html`, `build/index.js`, `build/index.css`.

- [ ] **Step 3: Run and verify browser opens**

```bash
omnic run src/main.omni
```
Expected: `build/debug/index.html` + `.js` + `.css` created, browser opened.

- [ ] **Step 4: Build to react and verify N output files**

```bash
omnic build -t react
```
Expected: `build/src/main.tsx` and `build/src/components/TodoList.tsx`.

- [ ] **Step 5: Commit**

---

### Task 25b: Integration test — logic-cli-tool project

**Files:**
- Create: `test/projects/logic-cli-tool/`

- [ ] **Step 1: Create project with CLI-like patterns**

```
test/projects/logic-cli-tool/
├── .omnilang.toml
└── src/
    └── main.omni
```

`src/main.omni`:
```
struct Args {
    command: string,
    verbose: bool,
}

fn run(args: Args) -> string {
    if args.verbose {
        return "Running in verbose mode: " + args.command;
    }
    return args.command;
}

fn main() {
    let args = Args { command: "build", verbose: true };
    print(run(args));
}
```

- [ ] **Step 2: Build → verify multi-target output**

```bash
omnic build -t python && python build/src/main.py
omnic build -t js && node build/src/main.js
```
Expected: "Running in verbose mode: build" (both targets).

- [ ] **Step 3: Commit**

---

### Task 25c: Integration test — view-ecommerce project

**Files:**
- Create: `test/projects/view-ecommerce/`

- [ ] **Step 1: Create project with multiple components and stores**

```
test/projects/view-ecommerce/
├── .omnilang.toml
└── src/
    ├── main.omni
    └── components/
        ├── ProductList.omni
        └── CartIcon.omni
```

`src/main.omni`:
```
import omniview;
import { ProductList } from "./components/ProductList.omni";
import { CartIcon } from "./components/CartIcon.omni";

store CartStore {
    signal items: Vec<string> = [];
    signal total: f64 = 0.0;

    get count: i32 = items.len();
}

component App {
    store_ref CartStore as cart;

    view {
        <div class: app>
            <header>
                <CartIcon count={cart.count} />
            </header>
            <main>
                <ProductList />
            </main>
        </div>
    }
}
```

- [ ] **Step 2: Build to react (multi-file) and verify output count matches input count**

```bash
omnic build -t react
```
Expected: 3 `.tsx` files (main.tsx, ProductList.tsx, CartIcon.tsx).

- [ ] **Step 3: Commit**

---

### Task 25d: Integration test — mixed-blog project

**Files:**
- Create: `test/projects/mixed-blog/`

- [ ] **Step 1: Create project with logic + view cross-references**

```
test/projects/mixed-blog/
├── .omnilang.toml
└── src/
    ├── main.omni
    ├── models/
    │   └── post.omni
    └── components/
        └── PostCard.omni
```

`src/models/post.omni` (logic file):
```
pub struct Post {
    title: string,
    body: string,
    author: string,
}

pub fn format_date(timestamp: i64) -> string {
    return "2026-06-17";  // simplified
}
```

`src/components/PostCard.omni` (view file):
```
import omniview;
import { Post } from "../models/post.omni";
import { format_date } from "../models/post.omni";

component PostCard {
    signal post: Post = Post { title: "", body: "", author: "" };

    view {
        <div class: card>
            <h2>{post.title}</h2>
            <p>{post.body}</p>
            <span>By {post.author}</span>
        </div>
    }
}
```

- [ ] **Step 2: Build → verify cross-file imports resolve correctly**

```bash
omnic build -t react
omnic analyse
```
Expected: No import resolution errors.

- [ ] **Step 3: Commit**

---

### Task 26: Build and run all tests end-to-end

- [ ] **Step 1: Build Rust DLL**

```bash
cd omni-compiler && cargo build --release
```
Expected: Clean build.

- [ ] **Step 2: Run all Rust tests**

```bash
cargo test
```
Expected: All tests pass.

- [ ] **Step 3: Run all Python tests**

```bash
pytest tests/ -v
```
Expected: All tests pass.

- [ ] **Step 4: Run integration test projects**

```bash
cd test/projects/logic-counter && omnic build && omnic analyse
cd test/projects/view-todo && omnic build -t omniview && omnic build -t react
```

- [ ] **Step 5: Commit any remaining changes**

```bash
git add omni-compiler/ omnilang/ tests/ test/projects/ .github/workflows/ci.yml
git commit -m "chore: final integration verification, all tests passing"
```

---

## Implementation Order

Tasks are numbered for dependency ordering:

```
Task 1 (AST Module) ──→ Task 2 (Parser adapt) ──→ Task 3 (lib.rs adapt)
                                                       │
                          Task 4 (IR logic.rs) ←────────┘
                                 │
                          Task 5 (IRProgram modules)
                                 │
                          Task 6 (IR lowering logic)
                                 │
                          Task 7 (Semantic fix + lowering tests)
                                 │
                          Task 8 (FFI cleanup)
                                 │
                          Task 8.5 (Semantic lints module)
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                  │
         Tasks 9-12          Tasks 13-17        Tasks 19-20
         (Logic codegen)     (View codegen)     (Python errors/result)
              │                  │                  │
              └──────────────────┼──────────────────┘
                                 │
                          Task 18 (codegen dispatch)
                                 │
                    ┌────────────┼────────────┐
                    │            │            │
              Task 18.5    Task 18.6     Task 21
          (snapshot tests)  (CI config)  (RustBridge)
                                             │
                                      Task 22 (api.py + serve fix)
                                             │
                                      Task 23 (CLI rewrite)
                                      Task 23.5 (watch)
                                             │
                     ┌───────────────────────┼───────────────────────┐
                     │                       │                       │
                Task 24 (logic-counter)  Task 25 (view-todo)    Tasks 25b-25d
                     │                       │                 (more integration)
                     └───────────────────────┬───────────────────────┘
                                             │
                                      Task 26 (final verification)
```

Tasks 9-12 can run in parallel. Tasks 13-17 can run in parallel. Tasks 19-20 can run in parallel.
Tasks 18.5 and 18.6 can run in parallel. Tasks 24, 25, 25b, 25c, 25d can run in parallel.

---

## Additional Risks (from plan review)

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Editor (`omnic serve`) view tree rendering uses `_compile_ir_internal()` stopgap — full IR fidelity requires dedicated FFI function | Low | Task 22 Step 3.5 adds `_compile_ir_internal()` that calls `omni_compile_multi_to_target`. Follow-up task should add dedicated `omni_compile_to_ir` FFI for the editor. |
| FFI `ptr+len` convention: malicious or buggy callers can pass wrong length → OOB read (unsafe) | Low | Document in FFI function comments that caller must provide honest lengths. Debug builds could add canary markers. Same pattern exists in current code. |
| OmniLang `print()` vs `println()` syntax not fully confirmed — integration test examples may need adjustment | Low | Verify against actual parser grammar during implementation. Adjust test source if needed. |
| `Visibility` enum used in AST but IR stores as `String` — loss of compile-time safety for visibility values | Low | Acceptable trade-off for Serde serialization simplicity. Codegen backends match on known strings ("public" / "private"). Future: add custom Serde for Visibility enum. |
| Cross-platform DLL building in CI — matrix covers Windows/macOS/Linux but first-time setup may need adjustments | Medium | Task 18.6 adds CI workflow. Expect path/extension differences (`.dll` vs `.so` vs `.dylib`) to need tweaking in first CI run. |
