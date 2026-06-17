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
| `omni-compiler/src/codegen/typescript.rs` | TypeScript codegen backend |
| `omnilang/errors.py` | Structured error types: OmniError, OmniLangError subclasses |
| `omnilang/result.py` | CompileResult and AnalyseResult dataclasses |
| `test/projects/logic-counter/` | Integration test project |
| `test/projects/view-todo/` | Integration test project |
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
| `omni-compiler/src/codegen/typescript.rs` (if exists as separate) | Merged into javascript.rs with mode flag |
| `omnilang/lsp/server.py` | LSP command removed |
| `omnilang/analyze/` directory | Replaced by Rust `omni_analyse` |

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
    pub generics: Vec<String>,
    pub params: Vec<ParamIR>,
    pub return_type: Option<TypeIR>,
    pub body: BlockIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructIR {
    pub name: String,
    pub visibility: String,
    pub generics: Vec<String>,
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
    pub payload: Option<TypeIR>,
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
        generics: vec![],
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
        generics: vec![],
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
            payload: v.fields.first().map(|f| lower_type(&f.type_ann)),
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

- [ ] **Step 5: Write lowering unit test**

Add to `omni-compiler/src/ir/lower_logic.rs`:

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
        };
        let ir = super::super::lower(&program, &semantics);
        let main_module = ir.modules.get("input.omni").unwrap();
        assert!(main_module.structs.contains_key("Point"));
    }
}
```

Run: `cargo test lower_fn_main_to_ir lower_struct_to_ir`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add omni-compiler/src/ir/
git commit -m "feat: implement IR lowering for all logic constructs"
```

---

### Task 7: Fix semantic analysis — accumulate diagnostics, continue on error

**Files:**
- Modify: `omni-compiler/src/semantic/mod.rs`

- [ ] **Step 1: Change `analyze()` to always return a result**

Change the return type from `Result<SemanticResult, Vec<Diagnostic>>` to just `SemanticResult` with accumulated diagnostics:

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
    // Always return what we have
    SemanticResult {
        symbols: std::mem::take(&mut self.symbols),
        dependencies: std::mem::take(&mut self.dependencies),
    }
}
```

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
                // Also run lint rules
                for (_filepath, module) in &program.modules {
                    // Collect lint diagnostics
                }
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

- [ ] **Step 1: Rewrite `generate()` to produce Rust code from logic IR**

Standalone function → `fn main() { ... }`
Struct → `struct Point { x: i32, y: i32 }`
Enum → Rust enum with variants
Type alias → `type ID = String;`

- [ ] **Step 2: Add tests**

- [ ] **Step 3: Commit**

---

### Task 12: Rewrite logic codegen — Dart backend

**Files:**
- Modify: `omni-compiler/src/codegen/dart.rs`

- [ ] **Step 1: Rewrite `generate()` to produce Dart code from logic IR**

Standalone function → `void main() { ... }`
Struct → Dart class
Enum → Dart enum

- [ ] **Step 2: Add tests**

- [ ] **Step 3: Commit**

---

### Task 13: Adapt view codegen — React

**Files:**
- Modify: `omni-compiler/src/codegen/react.rs`

- [ ] **Step 1: Update `generate()` to iterate `ir.modules`**

Replace `ir.components` with `ir.modules.values().flat_map(|m| m.components.iter())`.

- [ ] **Step 2: Build and test**

Run: `cargo test --lib codegen::react 2>&1`

- [ ] **Step 3: Commit**

---

### Task 14: Adapt view codegen — Vue

**Files:**
- Modify: `omni-compiler/src/codegen/vue.rs`

- [ ] **Step 1: Update to module-based IR iteration**

Same pattern as React task.

- [ ] **Step 2: Build and test**

- [ ] **Step 3: Commit**

---

### Task 15: Adapt view codegen — Dioxus

**Files:**
- Modify: `omni-compiler/src/codegen/dioxus.rs`

- [ ] **Step 1: Update to module-based IR iteration**

- [ ] **Step 2: Build and test**

- [ ] **Step 3: Commit**

---

### Task 16: Adapt view codegen — Flutter

**Files:**
- Modify: `omni-compiler/src/codegen/flutter.rs`

- [ ] **Step 1: Update to module-based IR iteration**

- [ ] **Step 2: Build and test**

- [ ] **Step 3: Commit**

---

### Task 17: Adapt view codegen — OmniView

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`

- [ ] **Step 1: Update `compile_files()` to iterate `ir.modules`**

Replace `ir.components` / `ir.stores` top-level access with module iteration.

- [ ] **Step 2: Build and test**

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
```

Run: `pytest tests/test_errors.py -v`
Expected: 4 PASS

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

    @classmethod
    def _load(cls) -> ctypes.CDLL:
        if cls._dll is not None:
            return cls._dll
        if cls._checked:
            raise CompilerUnavailable()

        dll_paths = [
            Path(__file__).parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll",
        ]
        for p in dll_paths:
            if p.exists():
                cls._dll = ctypes.CDLL(str(p))
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
    # Temporarily break DLL path to test error
    import omnilang.rust_bridge as rb
    old_paths = rb.RustBridge._dll_paths if hasattr(rb.RustBridge, '_dll_paths') else None
    try:
        with pytest.raises(CompilerUnavailable) as exc:
            rb.RustBridge._checked = False
            rb.RustBridge._dll = None
            # Force check with no valid paths
            rb.RustBridge._load()
        assert "omni_compiler.dll" in str(exc.value)
    finally:
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

- [ ] **Step 3: Commit**

```bash
git add omnilang/api.py
git commit -m "refactor: update api.py to use new RustBridge and structured errors"
```

---

### Task 23: Rewrite CLI — remove commands, add analyse, structured errors

**Files:**
- Modify: `omnilang/cli.py`

- [ ] **Step 1: Remove commands and arguments**

Remove: `init` subparser, `fix` subparser, `fmt` subparser, `lsp` subparser, `cmd_init()`, `cmd_fix()`, `cmd_fmt()`, `cmd_lsp()`.

- [ ] **Step 2: Rename `check` → `analyse`**

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

- [ ] **Step 3: Update `cmd_run()` for view projects**

```python
def cmd_run(args):
    # ... compile ...
    if is_view:
        # omniview target produces index.html + index.js + index.css
        output_dir = os.path.join("build", "debug")
        _write_standalone_omniview(result.files, output_dir)
        import webbrowser
        webbrowser.open(f"file:///{os.path.abspath(os.path.join(output_dir, 'index.html'))}")
    else:
        # Logic: write all files, execute entry point
        import tempfile, subprocess
        tmpdir = tempfile.mkdtemp(prefix="omni_run_")
        for filename, content in result.files.items():
            out = os.path.join(tmpdir, filename)
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with open(out, "w") as f:
                f.write(content)
        entry = os.path.join(tmpdir, "src", "main.py")
        subprocess.run(["python", entry])
```

- [ ] **Step 4: Wrap main() with structured error handler**

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

- [ ] **Step 5: Write CLI tests**

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

- [ ] **Step 6: Commit**

```bash
git add omnilang/cli.py tests/test_cli.py
git commit -m "refactor: rewrite CLI — remove init/fmt/fix/lsp, add analyse, structured errors"
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
    println(result);
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

- [ ] **Step 5: Commit any remaining changes and push**

```bash
git add -A
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
                          Task 7 (Semantic fix)
                                 │
                          Task 8 (FFI cleanup)
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
                          Task 21 (RustBridge)
                                 │
                          Task 22 (api.py)
                                 │
                          Task 23 (CLI rewrite)
                                 │
                     ┌───────────┴───────────┐
                     │                       │
                Task 24 (logic-counter)  Task 25 (view-todo)
                     │                       │
                     └───────────┬───────────┘
                                 │
                          Task 26 (final verification)
```

Tasks 9-12 can run in parallel. Tasks 13-17 can run in parallel. Tasks 19-20 can run in parallel.
