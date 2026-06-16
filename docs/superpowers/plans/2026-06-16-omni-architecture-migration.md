# OmniLang Architecture Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate to single Rust compiler — all parse/semantic/IR/codegen in Rust, Python as thin wrapper.

**Architecture:** 5 phases. Phase 1 audits import system. Phase 2 adds pure-logic codegen. Phase 3 refines OmniView native. Phase 4 adds framework codegen. Phase 5 cleanup.

**Tech Stack:** Rust 2021 (serde, thiserror), Python 3.10+ (FastAPI, ctypes FFI), TypeScript (React SPA)

---

## File Map

```
omni-compiler/src/codegen/
├── mod.rs                    MODIFY — Codegen trait + CodeWriter + Target + dispatch
├── python.rs                 CREATE — Python codegen (Phase 2)
├── javascript.rs             CREATE — JS/TS codegen (Phase 2)
├── rust.rs                   CREATE — Rust codegen (Phase 2)
├── dart.rs                   CREATE — Dart codegen (Phase 2)
├── omniview.rs               MODIFY — add generate_to_files (Phase 2), refine (Phase 3)
├── react.rs                  CREATE — (Phase 4)
├── vue.rs                    CREATE — (Phase 4)
├── dioxus.rs                 CREATE — (Phase 4)
└── flutter.rs                CREATE — (Phase 4)

omni-compiler/tests/
├── logic_codegen_tests.rs    CREATE — cross-target codegen tests (Phase 2)
└── regression/               CREATE — Rust vs Python output regression (Phase 2, 4)

omni-compiler/src/
├── lib.rs                    MODIFY — add compile_to_target + compile_multi_to_target FFI (Phase 2)
├── parser/top_level.rs       MODIFY — integrate preprocessor (Phase 5)
└── semantic/lints.rs         CREATE — lint rule framework + 40+ rules (Phase 5)

tests/regression/
└── test_rust_vs_python_codegen.py  CREATE — framework regression cases (Phase 4)

omnilang/cli.py               MODIFY — merge (Phase 5)
omniview/cli.py               DELETE — merged (Phase 5)
omniview/preprocessor.py      DELETE — migrated to Rust (Phase 5)
omniview/api.py               MODIFY — route to Rust codegen (Phase 2.5+)
docs/superpowers/specs/
└── 2026-06-16-omni-grammar-spec.md  CREATE — unified spec (Phase 5)
```

---

## Phase 1: Import System Audit

> The import system (`docs/superpowers/plans/2026-06-16-omni-import-system.md`) is ~95% done. Rust parser/AST/semantic/validation complete. Python grammar/AST/parser/resolver complete. Remaining: verify, decide what to skip.

### Task 1.1: Audit and decide

- [ ] **Step 1: Identify what to skip from import plan**

Read `docs/superpowers/plans/2026-06-16-omni-import-system.md`. The remaining tasks are:

| Import Plan Task | Decision |
|------------------|----------|
| 4.1-4.5: Update Python codegen for import/export | **Skip** — Rust codegen (Phase 2-4) handles this from scratch |
| 5.1: Unify .omnilang.toml config | **Skip** — defer to Phase 5 CLI merge |
| 5.2: Update omnic build for project mode | **Skip** — defer to Phase 5 |
| 5.3: Add omnic compile for .omni.json | **Skip** — defer to editor cycle |
| 6.1-6.2: Editor integration | **Skip** — defer to editor cycle |

- [ ] **Step 2: Run full test suite to verify current state**

```bash
cd omni-compiler && cargo test 2>&1
```

Expected: 58 pass (3 lib + 24 codegen + 13 lexer + 15 parser + 3 semantic), 0 fail.

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/ -x --timeout=60 2>&1 | tail -5
```

Expected: 360 collected (all pass).

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "chore: import system audit — defer remaining Python codegen tasks"
```

---

## Phase 2: Rust Pure-Logic Codegen

> Replace `omnilang/codegen/` (python/js/rust/dart). One `Codegen` trait, one file per target, shared `CodeWriter` helper.

### Task 2.0: Codegen Foundation

**Files:**
- Modify: `omni-compiler/src/codegen/mod.rs`

- [ ] **Step 1: Rewrite `mod.rs` with trait, writer, target enum, dispatch**

```rust
// omni-compiler/src/codegen/mod.rs

pub mod omniview;
pub mod python;
pub mod javascript;
pub mod rust;
pub mod dart;

use std::collections::HashMap;
use crate::ir::IRProgram;

// ── CodeWriter ───────────────────────────────

/// Simple buffered code writer with indentation.
pub struct CodeWriter {
    lines: Vec<String>,
    indent_level: usize,
    indent_str: &'static str,
}

impl CodeWriter {
    pub fn new() -> Self {
        Self { lines: vec![], indent_level: 0, indent_str: "    " }
    }

    pub fn emit(&mut self, line: &str) {
        if line.is_empty() {
            self.lines.push(String::new());
        } else {
            let pad = self.indent_str.repeat(self.indent_level);
            self.lines.push(format!("{}{}", pad, line));
        }
    }

    pub fn indent(&mut self) { self.indent_level += 1; }
    pub fn dedent(&mut self) { self.indent_level = self.indent_level.saturating_sub(1); }

    pub fn into_string(self) -> String {
        self.lines.join("\n") + "\n"
    }
}

// ── Output Types ─────────────────────────────

#[derive(Debug, Clone)]
pub struct CompiledFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug)]
pub struct CompileOutput {
    pub files: Vec<CompiledFile>,
    pub errors: Vec<String>,
}

impl CompileOutput {
    pub fn success(files: Vec<CompiledFile>) -> Self {
        Self { files, errors: vec![] }
    }

    pub fn error(msg: String) -> Self {
        Self { files: vec![], errors: vec![msg] }
    }
}

// ── Target ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Python, JavaScript, TypeScript, Rust, Dart,
    React, Vue, Dioxus, Flutter, OmniView,
}

impl Target {
    pub fn from_str(s: &str) -> Option<Target> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Some(Target::Python),
            "javascript" | "js" => Some(Target::JavaScript),
            "typescript" | "ts" => Some(Target::TypeScript),
            "rust" | "rs" => Some(Target::Rust),
            "dart" => Some(Target::Dart),
            "react" => Some(Target::React),
            "vue" => Some(Target::Vue),
            "dioxus" => Some(Target::Dioxus),
            "flutter" => Some(Target::Flutter),
            "omniview" => Some(Target::OmniView),
            _ => None,
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            Target::Python => ".py", Target::JavaScript => ".js",
            Target::TypeScript => ".ts", Target::Rust => ".rs",
            Target::Dart => ".dart", Target::React => ".tsx",
            Target::Vue => ".vue", Target::Dioxus => ".rs",
            Target::Flutter => ".dart", Target::OmniView => ".js",
        }
    }
}

// ── Codegen Trait ────────────────────────────

pub trait Codegen {
    /// `source_files` is {file_path → source_text} — NOT to be confused with
    /// `IRProgram.source_map` which tracks line/col mapping entries.
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput;
}

// ── Dispatch ─────────────────────────────────

pub fn compile(ir: &IRProgram, target: Target, source_files: &HashMap<String, String>) -> CompileOutput {
    match target {
        Target::Python => python::PythonCodegen.generate(ir, source_files),
        Target::JavaScript => javascript::JavaScriptCodegen::new(false).generate(ir, source_files),
        Target::TypeScript => javascript::JavaScriptCodegen::new(true).generate(ir, source_files),
        Target::Rust => rust::RustCodegen.generate(ir, source_files),
        Target::Dart => dart::DartCodegen.generate(ir, source_files),
        Target::OmniView => omniview::compile_files(ir, source_files),
        Target::React => CompileOutput::error("React codegen: not yet implemented in Rust".into()),
        Target::Vue => CompileOutput::error("Vue codegen: not yet implemented in Rust".into()),
        Target::Dioxus => CompileOutput::error("Dioxus codegen: not yet implemented in Rust".into()),
        Target::Flutter => CompileOutput::error("Flutter codegen: not yet implemented in Rust".into()),
    }
}

// ── Tests ────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_parse_all() {
        assert_eq!(Target::from_str("python"), Some(Target::Python));
        assert_eq!(Target::from_str("js"), Some(Target::JavaScript));
        assert_eq!(Target::from_str("ts"), Some(Target::TypeScript));
        assert_eq!(Target::from_str("rust"), Some(Target::Rust));
        assert_eq!(Target::from_str("dart"), Some(Target::Dart));
        assert_eq!(Target::from_str("react"), Some(Target::React));
        assert_eq!(Target::from_str("vue"), Some(Target::Vue));
        assert_eq!(Target::from_str("dioxus"), Some(Target::Dioxus));
        assert_eq!(Target::from_str("flutter"), Some(Target::Flutter));
        assert_eq!(Target::from_str("omniview"), Some(Target::OmniView));
    }

    #[test]
    fn test_target_extension() {
        assert_eq!(Target::Python.extension(), ".py");
        assert_eq!(Target::JavaScript.extension(), ".js");
        assert_eq!(Target::Rust.extension(), ".rs");
        assert_eq!(Target::React.extension(), ".tsx");
    }

    #[test]
    fn test_code_writer() {
        let mut w = CodeWriter::new();
        w.emit("// header");
        w.emit("");
        w.emit("fn main() {");
        w.indent();
        w.emit("return 0;");
        w.dedent();
        w.emit("}");
        let out = w.into_string();
        assert!(out.contains("    return 0;"));
        assert!(out.starts_with("// header"));
    }
}
```

- [ ] **Step 2: Add stub `compile_files` to `omniview.rs`**

```rust
// Add at end of omni-compiler/src/codegen/omniview.rs:

use std::collections::HashMap;
use super::{CompileOutput, CompiledFile};

pub fn compile_files(ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
    let output = generate(ir);
    CompileOutput::success(vec![
        CompiledFile { path: "index.js".into(), content: output.js },
        CompiledFile { path: "index.html".into(), content: output.html },
        CompiledFile { path: "index.css".into(), content: output.css },
    ])
}
```

- [ ] **Step 3: Create stub files for 4 new targets**

```bash
cd D:/space/OmniLang/omni-compiler/src/codegen
for f in python.rs javascript.rs rust.rs dart.rs; do
  echo "// TODO: implement" > "$f"
done
```

- [ ] **Step 4: Verify compilation**

```bash
cd omni-compiler && cargo test --lib codegen::tests 2>&1
```

Expected: 3 tests pass (target_parse_all, target_extension, test_code_writer).

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/codegen/
git commit -m "feat(rust-codegen): CodeWriter, Codegen trait, Target enum, compile dispatch, stub targets"
```

---

### Task 2.1: Python Codegen

**Files:**
- Modify: `omni-compiler/src/codegen/python.rs`
- Create: `omni-compiler/tests/logic_codegen_tests.rs`

- [ ] **Step 1: Write the failing test**

```rust
// omni-compiler/tests/logic_codegen_tests.rs

use omni_compiler::codegen::{Target, compile};
use omni_compiler::ir::IRProgram;
use std::collections::HashMap;

fn empty_ir() -> IRProgram {
    IRProgram {
        version: "0.1".into(),
        components: HashMap::new(),
        stores: HashMap::new(),
        source_map: HashMap::new(),
    }
}

#[test]
fn test_python_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Python, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, "main.py");
    assert!(result.files[0].content.contains("# Generated from"));
}

#[test]
fn test_python_simple_function() {
    // Test with actual IR containing a FunctionIR
    let mut components = HashMap::new();
    components.insert("App".into(), ComponentIR {
        name: "App".into(),
        generic_params: vec![],
        props: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![FunctionIR {
            name: "greet".into(),
            params: vec![],
            return_type: Some(TypeIR { kind: "string".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false }),
            body: BlockIR { statements: vec![], execution_mode: "sync".into() },
        }],
        store_refs: vec![],
        refs: vec![],
        view: None,
        styles: vec![],
    });
    let ir = IRProgram {
        version: "1.0".into(),
        components,
        stores: HashMap::new(),
        source_map: HashMap::new(),
    };
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Python, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    let content = &result.files[0].content;
    assert!(content.contains("def greet()"), "expected 'def greet():' in:\n{}", content);
}

#[test]
fn test_javascript_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::JavaScript, &sources);
    assert!(result.errors.is_empty());
    assert_eq!(result.files[0].path, "main.js");
}

#[test]
fn test_rust_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Rust, &sources);
    assert!(result.errors.is_empty());
    assert_eq!(result.files[0].path, "main.rs");
}

#[test]
fn test_dart_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Dart, &sources);
    assert!(result.errors.is_empty());
    assert_eq!(result.files[0].path, "main.dart");
}
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd omni-compiler && cargo test test_python_empty_program 2>&1 | tail -15
```

Expected: FAIL — PythonCodegen not found.

- [ ] **Step 3: Implement Python codegen**

```rust
// omni-compiler/src/codegen/python.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct PythonCodegen;

impl Codegen for PythonCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("# Generated from {}", source_path));
            w.emit("");

            // Type aliases
            for (name, _comp) in &ir.components {
                // Component fns are stored in ComponentIR.fns
                // w.emit(&format!("# component: {}", name));
            }

            // TODO: iterate IR functions/structs from IR once they carry
            // pure-logic declarations. Currently IR only has components + stores.

            let out_path = source_path.replace(".omni", ".py");
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

// ── Type Mapping ─────────────────────────────

pub fn py_type(omni_type: &str) -> String {
    match omni_type {
        "int" | "i32" | "i64" => "int".into(),
        "float" | "f64" => "float".into(),
        "bool" => "bool".into(),
        "string" => "str".into(),
        "void" | "()" => "None".into(),
        other if other.starts_with("Vec<") => {
            let inner = &other[4..other.len()-1];
            format!("list[{}]", py_type(inner))
        }
        other if other.starts_with("HashMap<") => {
            let parts: Vec<&str> = other[8..other.len()-1].split(',').collect();
            if parts.len() == 2 {
                format!("dict[{}, {}]", py_type(parts[0].trim()), py_type(parts[1].trim()))
            } else { "dict".into() }
        }
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_type_primitives() {
        assert_eq!(py_type("int"), "int");
        assert_eq!(py_type("f64"), "float");
        assert_eq!(py_type("bool"), "bool");
        assert_eq!(py_type("string"), "str");
        assert_eq!(py_type("void"), "None");
    }

    #[test]
    fn test_py_type_generics() {
        assert_eq!(py_type("Vec<i32>"), "list[int]");
        assert_eq!(py_type("HashMap<string, i32>"), "dict[str, int]");
    }
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd omni-compiler && cargo test test_python_empty_program 2>&1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/codegen/python.rs omni-compiler/tests/logic_codegen_tests.rs
git commit -m "feat(rust-codegen): Python codegen with Codegen trait, type mapping"
```

---

### Task 2.2: JavaScript/TypeScript Codegen

**Files:**
- Modify: `omni-compiler/src/codegen/javascript.rs`

- [ ] **Step 1: Implement JavaScript codegen**

```rust
// omni-compiler/src/codegen/javascript.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct JavaScriptCodegen {
    ts_mode: bool,
}

impl JavaScriptCodegen {
    pub fn new(ts_mode: bool) -> Self { Self { ts_mode } }
}

impl Codegen for JavaScriptCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let ext = if self.ts_mode { ".ts" } else { ".js" };
        let mut files = Vec::new();

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            let out_path = source_path.replace(".omni", ext);
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

// ── Type Mapping ─────────────────────────────

pub fn js_type(t: &str, ts_mode: bool) -> String {
    if !ts_mode { return String::new(); }  // JS: no type annotations
    match t {
        "int" | "i32" | "i64" | "float" | "f64" => "number".into(),
        "bool" => "boolean".into(),
        "string" => "string".into(),
        "void" => "void".into(),
        other if other.starts_with("Vec<") => {
            let inner = &other[4..other.len()-1];
            format!("{}[]", js_type(inner, ts_mode))
        }
        other if other.starts_with("HashMap<") => {
            let parts: Vec<&str> = other[8..other.len()-1].split(',').collect();
            if parts.len() == 2 {
                format!("Record<{}, {}>", js_type(parts[0].trim(), ts_mode), js_type(parts[1].trim(), ts_mode))
            } else { "object".into() }
        }
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_type_no_ts() {
        assert_eq!(js_type("int", false), "");
        assert_eq!(js_type("string", false), "");
    }

    #[test]
    fn test_ts_type_primitives() {
        assert_eq!(js_type("int", true), "number");
        assert_eq!(js_type("bool", true), "boolean");
        assert_eq!(js_type("string", true), "string");
        assert_eq!(js_type("void", true), "void");
    }

    #[test]
    fn test_ts_type_generics() {
        assert_eq!(js_type("Vec<i32>", true), "number[]");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd omni-compiler && cargo test test_javascript_empty_program 2>&1
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add omni-compiler/src/codegen/javascript.rs
git commit -m "feat(rust-codegen): JavaScript/TypeScript codegen with TS mode"
```

---

### Task 2.3: Rust & Dart Codegen

**Files:**
- Modify: `omni-compiler/src/codegen/rust.rs`
- Modify: `omni-compiler/src/codegen/dart.rs`

- [ ] **Step 1: Implement Rust codegen**

```rust
// omni-compiler/src/codegen/rust.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct RustCodegen;

impl Codegen for RustCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            let out_path = source_path.replace(".omni", ".rs");
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

pub fn rs_type(t: &str) -> String {
    match t {
        "int" | "i32" => "i32".into(),
        "i64" => "i64".into(),
        "float" | "f64" => "f64".into(),
        "bool" => "bool".into(),
        "string" => "String".into(),
        "void" | "()" => "()".into(),
        other if other.starts_with("Vec<") => {
            let inner = &other[4..other.len()-1];
            format!("Vec<{}>", rs_type(inner))
        }
        other if other.starts_with("HashMap<") => {
            let parts: Vec<&str> = other[8..other.len()-1].split(',').collect();
            if parts.len() == 2 {
                format!("HashMap<{}, {}>", rs_type(parts[0].trim()), rs_type(parts[1].trim()))
            } else { "HashMap".into() }
        }
        other => other.into(),
    }
}
```

- [ ] **Step 2: Implement Dart codegen**

```rust
// omni-compiler/src/codegen/dart.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct DartCodegen;

impl Codegen for DartCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            let out_path = source_path.replace(".omni", ".dart");
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

pub fn dart_type(t: &str) -> String {
    match t {
        "int" | "i32" | "i64" => "int".into(),
        "float" | "f64" => "double".into(),
        "bool" => "bool".into(),
        "string" => "String".into(),
        "void" | "()" => "void".into(),
        other if other.starts_with("Vec<") => {
            let inner = &other[4..other.len()-1];
            format!("List<{}>", dart_type(inner))
        }
        other if other.starts_with("HashMap<") => {
            let parts: Vec<&str> = other[8..other.len()-1].split(',').collect();
            if parts.len() == 2 {
                format!("Map<{}, {}>", dart_type(parts[0].trim()), dart_type(parts[1].trim()))
            } else { "Map".into() }
        }
        other => other.into(),
    }
}
```

- [ ] **Step 3: Run all logic codegen tests**

```bash
cd omni-compiler && cargo test --test logic_codegen_tests 2>&1
```

Expected: 4 tests pass (python/js/rust/dart empty program).

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/rust.rs omni-compiler/src/codegen/dart.rs
git commit -m "feat(rust-codegen): Rust and Dart pure-logic codegen"
```

---

### Task 2.4: Wire codegen into compile pipeline

**Files:**
- Modify: `omni-compiler/src/lib.rs`

- [ ] **Step 1: Add `compile_to_target` and `compile_multi_to_target` functions + FFI**

Add after the existing `compile_omniview` function (~line 170):

```rust
use crate::codegen::{Target, compile as compile_codegen, CompileOutput};
use std::collections::HashMap;

/// Compile IR to a specific target, returning {path: code} map as JSON.
/// Single-file convenience wrapper.
pub fn compile_to_target_json(ir: &ir::IRProgram, target: Target, source_files: &HashMap<String, String>) -> String {
    let output = compile_codegen(ir, target, source_files);

    let files_map: HashMap<String, String> = output.files
        .into_iter()
        .map(|f| (f.path, f.content))
        .collect();

    serde_json::to_string(&serde_json::json!({
        "success": output.errors.is_empty(),
        "files": files_map,
        "errors": output.errors,
    })).unwrap_or_else(|e| format!(r#"{{"success":false,"errors":["serialization error: {}"]}}"#, e))
}

/// Compile multiple .omni source files to a specific target.
/// This is the primary multi-file entry point.
///
/// 1. Parse all sources → AST programs
/// 2. Cross-file validation via existing `validate_multi()` (imports, visibility, cycles)
/// 3. Per-file semantic analysis + IR lowering → single merged IR
/// 4. Dispatch to target codegen
///
/// Returns JSON: { success, files: {path: content}, errors: [...] }
pub fn compile_multi_to_target(
    sources: HashMap<String, String>,
    target: Target,
    path_mapping: HashMap<String, String>,
) -> String {
    let mut errors: Vec<String> = Vec::new();
    let mut programs: HashMap<String, crate::parser::ast::Program> = HashMap::new();

    // 1. Parse all sources
    for (path, source) in &sources {
        let tokens = crate::lexer::tokenize(source);
        match crate::parser::parse(&tokens) {
            Ok(program) => { programs.insert(path.clone(), program); }
            Err(e) => { errors.push(format!("{}: parse error: {}", path, e.message)); }
        }
    }

    if !errors.is_empty() {
        return serde_json::to_string(&serde_json::json!({
            "success": false, "files": {}, "errors": errors,
        })).unwrap_or_default();
    }

    // 2. Cross-file validation — reuse `validate_multi` for consistency, file checks,
    //    import resolution, logic→view boundary, and Tarjan SCC cycle detection.
    //
    //    NOTE: `validate_multi` internally parses all sources again. To avoid
    //    double-parsing, a future refactor should pass pre-parsed programs directly;
    //    for now correctness is preferred over parse-time efficiency.
    let validation = validate_multi(sources.clone(), path_mapping.clone());
    if !validation.success {
        return serde_json::to_string(&serde_json::json!({
            "success": false, "files": {}, "errors": validation.errors,
        })).unwrap_or_default();
    }

    // 3. Per-file semantic analysis + IR lowering → merged IR
    let mut merged_ir = ir::IRProgram {
        version: "1.0".into(),
        components: HashMap::new(),
        stores: HashMap::new(),
        source_map: HashMap::new(),
    };
    for (file_path, program) in &programs {
        match crate::semantic::analyze(program) {
            Ok(semantics) => {
                let file_ir = ir::lower(program, &semantics);
                merged_ir.components.extend(file_ir.components);
                merged_ir.stores.extend(file_ir.stores);
            }
            Err(diags) => {
                for d in &diags {
                    errors.push(format!("{}: semantic error: {}", file_path, d.message));
                }
            }
        }
    }

    // Abort if semantic errors were found (IR would be incomplete)
    if !errors.is_empty() {
        return serde_json::to_string(&serde_json::json!({
            "success": false, "files": {}, "errors": errors,
        })).unwrap_or_default();
    }

    // 4. Dispatch to target codegen
    compile_to_target_json(&merged_ir, target, &sources)
}

/// C FFI: compile multiple .omni source files to a specific target.
///
/// Uses ptr+len for safety (source text may contain null bytes).
///
/// `sources_json`: JSON string of {"path": "source_text", ...}
/// `target_str`: target name (python/js/rust/dart/...)
/// `path_mapping_json`: JSON string of {"import_path": "resolved_path", ...}
///
/// Returns a `*mut c_char` JSON string: { success, files: {path: content}, errors: [...] }.
/// **Caller must free with `omni_free_string`.**  (Compatible — same allocator as
/// existing `omni_compile_to_ir`, `omni_compile_to_omniview`, `omni_version`.)
#[no_mangle]
pub unsafe extern "C" fn omni_compile_multi_to_target(
    sources_json_ptr: *const u8, sources_json_len: u32,
    target_ptr: *const u8, target_len: u32,
    path_mapping_ptr: *const u8, path_mapping_len: u32,
) -> *mut c_char {
    let sources_json = std::str::from_utf8(
        std::slice::from_raw_parts(sources_json_ptr, sources_json_len as usize)
    ).unwrap_or("{}");
    let target_str = std::str::from_utf8(
        std::slice::from_raw_parts(target_ptr, target_len as usize)
    ).unwrap_or("python");
    let mapping_json = std::str::from_utf8(
        std::slice::from_raw_parts(path_mapping_ptr, path_mapping_len as usize)
    ).unwrap_or("{}");

    let sources: HashMap<String, String> = serde_json::from_str(sources_json).unwrap_or_default();
    let path_mapping: HashMap<String, String> = serde_json::from_str(mapping_json).unwrap_or_default();
    let target = Target::from_str(target_str).unwrap_or(Target::Python);

    let result = compile_multi_to_target(sources, target, path_mapping);
    rust_to_c_string(result)
}

/// C FFI: single-file convenience wrapper (backward compat).
///
/// Returns a `*mut c_char` JSON string: { success, files: {path: content}, errors: [...] }.
/// **Caller must free with `omni_free_string`.**
#[no_mangle]
pub unsafe extern "C" fn omni_compile_to_target(
    source_ptr: *const u8, source_len: u32,
    target_ptr: *const u8, target_len: u32,
) -> *mut c_char {
    let source = std::str::from_utf8(
        std::slice::from_raw_parts(source_ptr, source_len as usize)
    ).unwrap_or("");
    let sources = HashMap::from([("input.omni".to_string(), source.to_string())]);
    let path_mapping = HashMap::new();

    let result = compile_multi_to_target(sources, Target::from_str(
        std::str::from_utf8(
            std::slice::from_raw_parts(target_ptr, target_len as usize)
        ).unwrap_or("python")
    ).unwrap_or(Target::Python), path_mapping);

    rust_to_c_string(result)
}
```

- [ ] **Step 2: Update `Cargo.toml` if needed**

Check that `serde_json` is in dependencies (it already is).

- [ ] **Step 3: Verify compilation**

```bash
cd omni-compiler && cargo check 2>&1
```

Expected: compiles. Fix any issues.

- [ ] **Step 4: Run all tests**

```bash
cd omni-compiler && cargo test 2>&1
```

Expected: all existing + new tests pass.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/lib.rs
git commit -m "feat(rust-compiler): add compile_to_target + C FFI omni_compile_to_target"
```

---

### Task 2.5: Python integration — route `api.py` to Rust codegen

**Files:**
- Modify: `omniview/api.py`
- Modify: `omniview/codegen/rust_bridge.py`

- [ ] **Step 1: Add `compile_to_target` and `compile_multi` to `RustBridge`**

```python
# Add to omniview/codegen/rust_bridge.py, after compile_to_ir():

def compile_to_target(self, sources: dict, target: str, path_mapping: dict | None = None) -> dict:
    """Compile one or more .omni source files to target code via Rust DLL.
    
    Args:
        sources: {path: source_text} — at least one entry
        target: target name (python/js/rust/dart/react/vue/...)
        path_mapping: {import_path: resolved_path} — for multi-file
    
    Returns: { success: bool, files: {path: content}, errors: [...] }
    """
    try:
        import ctypes

        dll = self._load_dll()
        if dll is None:
            return {"success": False, "errors": ["Rust DLL not available"], "files": {}}

        func = dll.omni_compile_multi_to_target
        func.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32,
                         ctypes.c_void_p, ctypes.c_uint32]
        func.restype = ctypes.c_void_p

        sources_bytes = json.dumps(sources).encode('utf-8')
        target_bytes = target.encode('utf-8')
        mapping_bytes = json.dumps(path_mapping or {}).encode('utf-8')

        ptr = func(
            sources_bytes, len(sources_bytes),
            target_bytes, len(target_bytes),
            mapping_bytes, len(mapping_bytes),
        )
        if ptr is None:
            return {"success": False, "errors": ["Rust returned null"], "files": {}}

        result_json = ctypes.cast(ptr, ctypes.c_char_p).value
        if result_json:
            result_json = result_json.decode('utf-8')

        dll.omni_free_string.argtypes = [ctypes.c_void_p]
        dll.omni_free_string.restype = None
        dll.omni_free_string(ptr)

        return json.loads(result_json) if result_json else {"success": False, "errors": ["empty result"], "files": {}}
    except Exception as e:
        return {"success": False, "errors": [f"Rust compile_to_target failed: {e}"], "files": {}}
```

- [ ] **Step 2: Update `compile_omni()` in `api.py`**

The existing code at line 83-110 already routes to Rust for UI mode via `compile_to_ir`. Add pure-logic routing ONLY for targets that Rust supports:

```python
# Targets that Rust compiler implements at this point (Phase 2):
#   pure-logic: python, javascript, typescript, rust, dart
#   omniview:   omniview (already working)
# Framework targets (react, vue, dioxus, flutter) still use Python until Phase 4.
RUST_SUPPORTED_TARGETS = {'python', 'js', 'javascript', 'typescript', 'ts',
                          'rust', 'rs', 'dart', 'omniview'}

# In compile_omni(), after _detect_ui_mode() check, BEFORE the existing Rust bridge block:

if RustBridge.is_available() and target in RUST_SUPPORTED_TARGETS:
    sources = {filepath: source}  # single-file for now; multi-file once import complete
    rust_result = RustBridge.compile_to_target(sources, target)
    if rust_result.get('success'):
        return CompileResult(
            success=True,
            files=rust_result.get('files', {}),
            source=source,
        )
    else:
        for err in rust_result.get('errors', []):
            errors.append(CompileError(
                file=filepath, line=1, severity="error",
                code="RUST_CODEGEN", message=str(err),
            ))
        return CompileResult(success=False, files={}, errors=errors, source=source)

# Framework targets (react/vue/dioxus/flutter) fall through to existing Python codegen
# until their Rust implementations are ready in Phase 4.
```

- [ ] **Step 3: Test end-to-end (pure-logic targets only)**

```bash
cd D:/space/OmniLang && python -c "
from omniview.api import compile_omni
result = compile_omni('fn main() { println(\"hi\"); }', target='python')
print('success:', result.success)
print('files:', list(result.files.keys()))
if result.files:
    for name, content in result.files.items():
        print(f'--- {name} ---')
        print(content[:200])
if result.errors:
    for e in result.errors:
        print(f'Error: {e.message}')
"
# Verify framework targets still work via Python fallback:
python -c "
from omniview.api import compile_omni
result = compile_omni('component App {}', target='react')
print('react success:', result.success)
"
```

- [ ] **Step 4: Commit**

```bash
git add omniview/api.py omniview/codegen/rust_bridge.py
git commit -m "feat(python-api): route pure-logic through Rust codegen, no Python fallback"
```

---

### Task 2.6: Regression Tests — Rust vs Python Codegen

> Within Phase 2, after each pure-logic codegen target is implemented, run regression comparison against the existing Python codegen output.

**Files:**
- Create: `tests/regression/test_rust_vs_python_codegen.py`

- [ ] **Step 1: Create regression test harness**

```python
# tests/regression/test_rust_vs_python_codegen.py
import pytest
from omniview.api import compile_omni
from omniview.codegen.rust_bridge import RustBridge

# Test cases: .omni source → expected output snippet (target-agnostic patterns)
REGRESSION_CASES = [
    # (source, target, expected_snippets)
    ("fn add(a: int, b: int) -> int { return a + b; }", "python",
     ["def add(a: int, b: int) -> int:", "return a + b"]),
    ("fn add(a: int, b: int) -> int { return a + b; }", "javascript",
     ["function add(a, b)", "return a + b"]),
    ("fn greet() -> string { return \"hello\"; }", "python",
     ["def greet() -> str:", 'return "hello"']),
    ("fn greet() -> string { return \"hello\"; }", "rust",
     ["fn greet() -> String", '"hello"']),
    # Add more as codegen matures
]

@pytest.mark.parametrize("source,target,snippets", REGRESSION_CASES)
def test_rust_codegen_matches_python(source, target, snippets):
    """Rust codegen output must contain expected patterns."""
    if not RustBridge.is_available():
        pytest.skip("Rust DLL not available")
    
    result = compile_omni(source, target=target)
    assert result.success, f"Compilation failed: {result.errors}"
    
    combined = "\n".join(result.files.values())
    for snippet in snippets:
        assert snippet in combined, f"Missing '{snippet}' in:\n{combined}"
```

- [ ] **Step 2: Run regression tests**

```bash
cd D:/space/OmniLang && python -m pytest tests/regression/ -v 2>&1
```

- [ ] **Step 3: Commit**

```bash
git add tests/regression/test_rust_vs_python_codegen.py
git commit -m "test(regression): Rust vs Python codegen comparison tests"
```

---

## Phase 3: OmniView Native Codegen Refinement

> Goal: Refine `omniview.rs` output — optimize HTML/JS/CSS generation, remove old mini-signal references, prepare for WASM Worker integration.

### Task 3.1: Clean up OmniView output

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`

- [ ] **Step 1: Audit current omniview.rs**

Read the full file. Note:
- Lines 73+: `emit_signal_runtime()` embeds ~200 line reactive runtime
- Lines 200+: `generate_component()` creates JS + HTML + CSS
- Signal runtime uses `signal()`, `computed()`, `effect()` pattern

Decision per architecture: keep the signal runtime inline but mark it as "native OmniView runtime v2". No external dependencies.

- [ ] **Step 2: Add HTML document wrapper for debug-ready output**

The `OmniViewOutput` already has js/html/css. Add a helper that wraps them in a complete debug-ready HTML document:

```rust
// Add to omniview.rs:

pub fn wrap_debug_html(js: &str, css: &str, body_html: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OmniView Debug</title>
<style>
/* OmniView reset */
*, *::before, *::after {{ box-sizing: border-box; }}
body {{ margin: 0; padding: 8px; font-family: system-ui, sans-serif; }}
{css}
</style>
</head>
<body>
{body_html}
<script>
{js}
</script>
</body>
</html>"#)
}
```

- [ ] **Step 3: Run existing OmniView codegen tests**

```bash
cd omni-compiler && cargo test codegen 2>&1
```

Expected: 16+ pass.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/omniview.rs
git commit -m "refactor(omniview): add debug HTML wrapper, mark runtime as v2"
```

---

### Task 3.2: SourceMap completeness

- [ ] **Step 1: Verify SourceMap entries cover all generated JS lines**

Read `sourcemap/mod.rs` and check that each `emit()` in `omniview.rs` records a SourceMap entry.

- [ ] **Step 2: Add SourceMap JSON output method to `sourcemap/mod.rs`**

```rust
// Add to omni-compiler/src/sourcemap/mod.rs:

pub fn source_map_json(entries: &[SourceMapEntry]) -> String {
    let mappings: Vec<_> = entries.iter().map(|e| {
        serde_json::json!({
            "source": e.source_file,
            "sourceLine": e.source_line,
            "sourceCol": e.source_col,
            "generatedLine": e.generated_line,
            "generatedCol": e.generated_col,
        })
    }).collect();
    serde_json::to_string(&mappings).unwrap_or("[]".into())
}
```

- [ ] **Step 3: Commit**

```bash
git add omni-compiler/src/codegen/omniview.rs omni-compiler/src/sourcemap/mod.rs
git commit -m "feat(omniview): SourceMap JSON serialization"
```

---

## Phase 4: Framework Codegen in Rust

> Replace `omniview/codegen/{react,vue,dioxus,flutter}.py`. One file per target, each implements `Codegen` trait.

### Task 4.1: React Codegen

**Files:**
- Create: `omni-compiler/src/codegen/react.rs`

- [ ] **Step 1: Study Python ReactCodegen for patterns**

Key patterns to replicate from `omniview/codegen/react.py`:
- `generate_component(comp_ir, store_irs)` → `.tsx` + `.module.css`
- `generate_store(store_ir)` → `use<Name>.ts` hook
- Element mapping: OmniView tags → HTML tags
- Style properties → inline styles or CSS modules
- Event mapping: OmniView events → React event handlers

- [ ] **Step 2: Implement ReactCodegen skeleton**

```rust
// omni-compiler/src/codegen/react.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct ReactCodegen;

impl Codegen for ReactCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        // For each component, generate .tsx + .module.css
        for (name, comp) in &ir.components {
            // Generate .tsx
            let tsx = self.gen_component_tsx(name, comp, ir);
            files.push(CompiledFile {
                path: format!("{}.tsx", name),
                content: tsx,
            });

            // Generate .module.css
            let css = self.gen_component_css(name, comp);
            files.push(CompiledFile {
                path: format!("{}.module.css", name),
                content: css,
            });
        }

        // For each store, generate use<Name>.ts hook
        for (name, store) in &ir.stores {
            let hook = self.gen_store_hook(name, store);
            files.push(CompiledFile {
                path: format!("use{}.ts", name),
                content: hook,
            });
        }

        CompileOutput::success(files)
    }
}

impl ReactCodegen {
    fn gen_component_tsx(&self, name: &str, comp: &crate::ir::ComponentIR, _ir: &IRProgram) -> String {
        let mut w = CodeWriter::new();
        w.emit("// Generated by OmniLang — React target");
        w.emit("");
        w.emit("import React, { useState, useEffect } from 'react';");
        w.emit("");

        // Props interface
        w.emit(&format!("interface {}Props {{", name));
        for (prop_name, prop_type) in &comp.props {
            let ts_type = super::javascript::js_type(&prop_type.kind, true);
            w.emit(&format!("  {}: {};", prop_name, ts_type));
        }
        w.emit("}");
        w.emit("");

        // Component
        w.emit(&format!("export function {}({{ ", name));
        let prop_names: Vec<_> = comp.props.keys().collect();
        if !prop_names.is_empty() {
            w.emit(&format!("  {}", prop_names.join(", ")));
        }
        w.emit(&format!("}}: {}Props) {{", name));
        w.indent();

        // States
        for state in &comp.states {
            // state.initial_value is ExprIR (not Option) — always present
            let init_val = format!("{:?}", state.initial_value);
            w.emit(&format!("const [{}, set{}] = useState({});",
                state.name,
                &state.name[0..1].to_uppercase().to_string() + &state.name[1..],
                init_val
            ));
        }

        // Effects
        for effect in &comp.effects {
            w.emit(&format!("useEffect(() => {{"));
            w.indent();
            // emit effect body
            w.emit("// effect body");
            w.dedent();
            let deps: Vec<_> = effect.deps.iter().map(|d| d.as_str()).collect();
            w.emit(&format!("}}, [{}]);", deps.join(", ")));
        }

        w.emit("");
        w.emit("return (");
        w.indent();
        // View tree rendering
        if let Some(view) = &comp.view {
            self.gen_view_node(&mut w, view, 0);
        }
        w.dedent();
        w.emit(");");
        w.dedent();
        w.emit("}");
        w.emit("");

        // CSS module import
        w.emit(&format!("import styles from './{}.module.css';", name));

        w.into_string()
    }

    fn gen_view_node(&self, w: &mut CodeWriter, _node: &crate::ir::ViewTree, _depth: usize) {
        // TODO: render view tree as JSX
        w.emit("<>TODO: view tree rendering</>");
    }

    fn gen_component_css(&self, name: &str, comp: &crate::ir::ComponentIR) -> String {
        let mut w = CodeWriter::new();
        w.emit(&format!("/* Styles for {} */", name));
        for style in &comp.styles {
            w.emit(&format!(".{} {{", style.class_name));
            for (prop, value) in &style.properties {
                let css_val = expr_to_css_value(value);
                w.emit(&format!("  {}: {};", to_css_prop(prop), css_val));
            }
            w.emit("}");
        }
        w.into_string()
    }

    fn gen_store_hook(&self, name: &str, store: &crate::ir::StoreIR) -> String {
        let mut w = CodeWriter::new();
        w.emit(&format!("// Store hook: use{}", name));
        w.emit("import { useState } from 'react';");
        w.emit("");
        w.emit(&format!("export function use{}() {{", name));
        w.indent();
        for (sig_name, _sig_type) in &store.signals {
            let init = store.signal_defaults.get(sig_name)
                .map(|v| format!("{:?}", v))
                .unwrap_or_else(|| "null".into());
            w.emit(&format!("const [{}, set{}] = useState({});",
                sig_name,
                &sig_name[0..1].to_uppercase().to_string() + &sig_name[1..],
                init
            ));
        }
        w.emit(&format!("return {{ {} }};",
            store.signals.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
        w.dedent();
        w.emit("}");
        w.into_string()
    }
}
```

- [ ] **Step 2: Enable React in dispatch**

In `codegen/mod.rs`, replace the React error line:

```rust
Target::React => react::ReactCodegen.generate(ir, source_files),
```

- [ ] **Step 3: Run tests**

```bash
cd omni-compiler && cargo test 2>&1
```

Expected: all pass. React codegen compiles.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/react.rs omni-compiler/src/codegen/mod.rs
git commit -m "feat(rust-codegen): React codegen — component .tsx + .module.css + store hooks"
```

---

### Task 4.2: Vue Codegen

**Files:**
- Create: `omni-compiler/src/codegen/vue.rs`
- Modify: `omni-compiler/src/codegen/mod.rs` (add `pub mod vue;` + dispatch arm)

- [ ] **Step 1: Implement VueCodegen skeleton**

```rust
// omni-compiler/src/codegen/vue.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct VueCodegen;

impl Codegen for VueCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (name, comp) in &ir.components {
            // Vue SFC: <template> + <script setup> + <style scoped>
            let sfc = self.gen_sfc(name, comp, ir);
            files.push(CompiledFile {
                path: format!("{}.vue", name),
                content: sfc,
            });
        }

        for (name, store) in &ir.stores {
            // Vue composable: useXxx()
            let composable = self.gen_composable(name, store);
            files.push(CompiledFile {
                path: format!("use{}.ts", name),
                content: composable,
            });
        }

        CompileOutput::success(files)
    }
}

impl VueCodegen {
    fn gen_sfc(&self, name: &str, comp: &crate::ir::ComponentIR, _ir: &IRProgram) -> String {
        let mut w = CodeWriter::new();
        // <template>
        w.emit("<template>");
        w.indent();
        if let Some(view) = &comp.view {
            self.gen_template_node(&mut w, view, 0);
        }
        w.dedent();
        w.emit("</template>");
        w.emit("");

        // <script setup lang="ts">
        w.emit(r#"<script setup lang="ts">"#);
        w.emit("import { ref, watchEffect } from 'vue';");
        w.emit("");
        for state in &comp.states {
            let init_val = format!("{:?}", state.initial_value);
            w.emit(&format!("const {} = ref({});", state.name, init_val));
        }
        for effect in &comp.effects {
            w.emit("watchEffect(() => {");
            w.indent();
            w.emit("// effect body");
            w.dedent();
            w.emit("});");
        }
        w.emit("</script>");
        w.emit("");

        // <style scoped>
        w.emit(r#"<style scoped>"#);
        for style in &comp.styles {
            w.emit(&format!(".{} {{", style.class_name));
            for (prop, value) in &style.properties {
                let css_val = expr_to_css_value(value);
                w.emit(&format!("  {}: {};", to_css_prop(prop), css_val));
            }
            w.emit("}");
        }
        w.emit("</style>");

        w.into_string()
    }

    fn gen_template_node(&self, w: &mut CodeWriter, _node: &crate::ir::ViewTree, _depth: usize) {
        // Vue template: native HTML tags, v-if/v-for directives, {{ interpolation }}
        w.emit("<!-- TODO: template node → HTML with Vue directives -->");
    }

    fn gen_composable(&self, name: &str, store: &crate::ir::StoreIR) -> String {
        let mut w = CodeWriter::new();
        w.emit(&format!("// Vue composable: use{}", name));
        w.emit("import { ref } from 'vue';");
        w.emit("");
        w.emit(&format!("export function use{}() {{", name));
        w.indent();
        for (sig_name, _sig_type) in &store.signals {
            let init = store.signal_defaults.get(sig_name)
                .map(|v| format!("{:?}", v))
                .unwrap_or_else(|| "null".into());
            w.emit(&format!("const {} = ref({});", sig_name, init));
        }
        w.emit(&format!("return {{ {} }};",
            store.signals.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
        w.dedent();
        w.emit("}");
        w.into_string()
    }
}
```

- [ ] **Step 2: Enable Vue in dispatch**

In `codegen/mod.rs`:
```rust
pub mod vue;
// In compile():
Target::Vue => vue::VueCodegen.generate(ir, source_files),
```

- [ ] **Step 3: Run tests**

```bash
cd omni-compiler && cargo test 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/vue.rs omni-compiler/src/codegen/mod.rs
git commit -m "feat(rust-codegen): Vue SFC codegen — <template> + <script setup> + <style scoped>"
```

---

### Task 4.3: Dioxus Codegen

**Files:**
- Create: `omni-compiler/src/codegen/dioxus.rs`
- Modify: `omni-compiler/src/codegen/mod.rs` (add `pub mod dioxus;` + dispatch arm)

- [ ] **Step 1: Implement DioxusCodegen skeleton**

```rust
// omni-compiler/src/codegen/dioxus.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct DioxusCodegen;

impl Codegen for DioxusCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (name, comp) in &ir.components {
            let rs = self.gen_component_rs(name, comp, ir);
            files.push(CompiledFile {
                path: format!("{}.rs", name),
                content: rs,
            });
        }

        CompileOutput::success(files)
    }
}

impl DioxusCodegen {
    fn gen_component_rs(&self, name: &str, comp: &crate::ir::ComponentIR, _ir: &IRProgram) -> String {
        let mut w = CodeWriter::new();
        w.emit("// Generated by OmniLang — Dioxus target");
        w.emit("use dioxus::prelude::*;");
        w.emit("");

        // Props struct
        if !comp.props.is_empty() {
            w.emit("#[derive(Props, PartialEq, Clone)]");
            w.emit(&format!("struct {}Props {{", name));
            for (prop_name, prop_type) in &comp.props {
                let rs_t = super::rust::rs_type(&prop_type.kind);
                w.emit(&format!("    {}: {},", prop_name, rs_t));
            }
            w.emit("}");
            w.emit("");
        }

        // Component function with #[component]
        let has_props = !comp.props.is_empty();
        w.emit(&format!("#[component]"));
        if has_props {
            w.emit(&format!("pub fn {}(props: {}Props) -> Element {{", name, name));
        } else {
            w.emit(&format!("pub fn {}() -> Element {{", name));
        }
        w.indent();

        // Signals
        for state in &comp.states {
            let init_val = format!("{:?}", state.initial_value);
            w.emit(&format!("let mut {} = use_signal(|| {});", state.name, init_val));
        }

        // Effects
        for effect in &comp.effects {
            let deps: Vec<_> = effect.deps.iter().map(|d| d.as_str()).collect();
            w.emit(&format!("use_effect(move || {{"));
            w.indent();
            w.emit("// effect body");
            w.dedent();
            w.emit(&format!("}});"));
        }

        // View → rsx! macro
        w.emit("");
        w.emit("rsx! {");
        if let Some(view) = &comp.view {
            self.gen_rsx_node(&mut w, view, 1);
        }
        w.emit("}");

        w.dedent();
        w.emit("}");
        w.into_string()
    }

    fn gen_rsx_node(&self, w: &mut CodeWriter, _node: &crate::ir::ViewTree, _depth: usize) {
        w.emit("// TODO: view tree → rsx! macro elements");
    }
}
```

- [ ] **Step 2: Enable Dioxus in dispatch**

In `codegen/mod.rs`:
```rust
pub mod dioxus;
// In compile():
Target::Dioxus => dioxus::DioxusCodegen.generate(ir, source_files),
```

- [ ] **Step 3: Run tests**

```bash
cd omni-compiler && cargo test 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/dioxus.rs omni-compiler/src/codegen/mod.rs
git commit -m "feat(rust-codegen): Dioxus codegen — #[component] + use_signal + rsx! macro"
```

---

### Task 4.4: Flutter Codegen

**Files:**
- Create: `omni-compiler/src/codegen/flutter.rs`
- Modify: `omni-compiler/src/codegen/mod.rs` (add `pub mod flutter;` + dispatch arm)

- [ ] **Step 1: Implement FlutterCodegen skeleton**

```rust
// omni-compiler/src/codegen/flutter.rs

use std::collections::HashMap;
use crate::ir::IRProgram;
use super::{CodeWriter, CompileOutput, CompiledFile, Codegen};

pub struct FlutterCodegen;

impl Codegen for FlutterCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (name, comp) in &ir.components {
            let dart = self.gen_widget(name, comp, ir);
            files.push(CompiledFile {
                path: format!("{}.dart", name),
                content: dart,
            });
        }

        CompileOutput::success(files)
    }
}

impl FlutterCodegen {
    fn gen_widget(&self, name: &str, comp: &crate::ir::ComponentIR, _ir: &IRProgram) -> String {
        let mut w = CodeWriter::new();
        w.emit("// Generated by OmniLang — Flutter target");
        w.emit("import 'package:flutter/material.dart';");
        w.emit("");

        // StatefulWidget + State class pair
        w.emit(&format!("class {} extends StatefulWidget {{", name));
        w.indent();
        // Constructor with props
        if comp.props.is_empty() {
            w.emit("const {super.key});");
        } else {
            w.emit("const {");
            w.indent();
            w.emit("super.key,");
            for (prop_name, _prop_type) in &comp.props {
                w.emit(&format!("required this.{},", prop_name));
            }
            w.dedent();
            w.emit("});");
        }
        w.emit("");
        w.emit("@override");
        w.emit(&format!("State<{}> createState() => _{}State();", name, name));
        w.dedent();
        w.emit("}");
        w.emit("");

        // State class
        w.emit(&format!("class _{}State extends State<{}> {{", name, name));
        w.indent();

        // State fields
        for state in &comp.states {
            let init_val = format!("{:?}", state.initial_value);
            w.emit(&format!("var {} = {};", state.name, init_val));
        }

        // initState for effects
        if !comp.effects.is_empty() {
            w.emit("");
            w.emit("@override");
            w.emit("void initState() {");
            w.indent();
            w.emit("super.initState();");
            for effect in &comp.effects {
                w.emit("// effect init");
            }
            w.dedent();
            w.emit("}");
        }

        // build method
        w.emit("");
        w.emit("@override");
        w.emit("Widget build(BuildContext context) {");
        w.indent();
        if let Some(view) = &comp.view {
            self.gen_widget_tree(&mut w, view, 1);
        } else {
            w.emit("return const SizedBox.shrink();");
        }
        w.dedent();
        w.emit("}");
        w.dedent();
        w.emit("}");

        w.into_string()
    }

    fn gen_widget_tree(&self, w: &mut CodeWriter, _node: &crate::ir::ViewTree, _depth: usize) {
        // Flutter: OmniView elements → Material/Cupertino widgets
        w.emit("// TODO: view tree → Flutter widget tree");
    }
}
```

- [ ] **Step 2: Enable Flutter in dispatch**

In `codegen/mod.rs`:
```rust
pub mod flutter;
// In compile():
Target::Flutter => flutter::FlutterCodegen.generate(ir, source_files),
```

- [ ] **Step 3: Run tests**

```bash
cd omni-compiler && cargo test 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/flutter.rs omni-compiler/src/codegen/mod.rs
git commit -m "feat(rust-codegen): Flutter codegen — StatefulWidget + setState + Widget build"
```

---

### Task 4.5: Framework Codegen Regression Tests

> After each framework codegen target, add regression comparison against Python output (same pattern as Task 2.6).

**Files:**
- Modify: `tests/regression/test_rust_vs_python_codegen.py`

- [ ] **Step 1: Add framework test cases**

```python
# Add to REGRESSION_CASES in tests/regression/test_rust_vs_python_codegen.py:
FRAMEWORK_CASES = [
    ("component App { view { <div>Hello</div> } }", "react",
     ["export function App(", "return (", "<div>Hello</div>"]),
    ("component App { view { <div>Hello</div> } }", "vue",
     ["<template>", "<div>Hello</div>", "<script setup"]),
    ("component App { view { <div>Hello</div> } }", "dioxus",
     ["#[component]", "rsx! {", "<div>Hello</div>"]),
    ("component App { view { <div>Hello</div> } }", "flutter",
     ["class App extends StatefulWidget", "Widget build"]),
]

@pytest.mark.parametrize("source,target,snippets", FRAMEWORK_CASES)
def test_framework_codegen_matches_python(source, target, snippets):
    if not RustBridge.is_available():
        pytest.skip("Rust DLL not available")
    result = compile_omni(source, target=target)
    assert result.success, f"Compilation failed: {result.errors}"
    combined = "\n".join(result.files.values())
    for snippet in snippets:
        assert snippet in combined, f"Missing '{snippet}' in:\n{combined}"
```

- [ ] **Step 2: Run framework regression tests**

```bash
cd D:/space/OmniLang && python -m pytest tests/regression/ -v -k "framework" 2>&1
```

- [ ] **Step 3: Commit**

```bash
git add tests/regression/test_rust_vs_python_codegen.py
git commit -m "test(regression): framework codegen regression tests (React/Vue/Dioxus/Flutter)"
```

---

## Phase 5: Cleanup & Polish

> Delete Python parser/codegen, merge CLI, migrate preprocessor, write grammar spec.

### Task 5.1: Merge CLI

**Files:**
- Modify: `omnilang/cli.py` (surviving CLI)
- Delete: `omniview/cli.py`

- [ ] **Step 1: Merge command sets**

`omnilang/cli.py` has: `build`, `check`, `fix`, `run`, `init`, `fmt`
`omniview/cli.py` has: `serve`, `init`, `build`, `check`, `watch`, `fmt`, `gen-extern`

Merged CLI adds `serve`, `watch`, `gen-extern` from omniview. Both `build` commands redirect to `compile_omni()`.

- [ ] **Step 2: All build routes through Rust compiler**

Update `cmd_build()` to call `omniview.api.compile_omni()` which in turn calls `RustBridge.compile_to_target()`.

- [ ] **Step 3: Delete `omniview/cli.py`**

```bash
git rm omniview/cli.py
```

- [ ] **Step 4: Update entry points**

Update `pyproject.toml` `[project.scripts]` to point to single CLI:
```toml
[project.scripts]
omnic = "omnilang.cli:main"
```

- [ ] **Step 5: Verify CLI works**

```bash
cd D:/space/OmniLang && python -m omnilang.cli --help
```

Expected: shows merged command list.

- [ ] **Step 6: Commit**

```bash
git add omnilang/cli.py pyproject.toml && git rm omniview/cli.py
git commit -m "refactor(cli): merge omnilang+omniview CLI, all routes through Rust"
```

---

### Task 5.2: Preprocessor Migration to Rust Parser

- [ ] **Step 1: Analyze preprocessor logic**

Read `omniview/preprocessor.py` (346 lines). The preprocessor:
1. Strips comments
2. Finds statement blocks (`fn body`, `action body`, event handlers, lifecycle hooks)
3. Extracts balanced `{...}` content
4. Replaces blocks with `__OMNI_BLOCK_N__` placeholders

- [ ] **Step 2: Verify Rust parser already handles inline blocks — then delete preprocessor**

The Rust parser (`omni-compiler/src/parser/`) uses recursive descent and already handles balanced `{...}` braces natively — no preprocessing step needed. The Python preprocessor was a workaround for Lark's limitations.

Verification steps:
1. Confirm `parser/top_level.rs` `parse_component_def()` can parse component bodies with inline statement blocks directly
2. Confirm `parser/view.rs` parses `@if`/`@for` blocks and event handler `{...}` bodies inline
3. Confirm `parser/stmt.rs` handles nested `if`/`while`/`for` blocks with balanced brace parsing
4. Run the full parser test suite to verify no regressions

If any preprocessor-only edge case is found (e.g., inline code in view expressions), add a targeted fix to the Rust parser rather than reintroducing a preprocessor pass.

- [ ] **Step 2b: Implement any missing inline block handling (if needed)**

For each edge case found in step 2, modify the relevant parser function. Example — if view event handlers with `{...}` aren't parsed:

In `omni-compiler/src/parser/view.rs`, the `parse_event_handler` function should handle inline statement blocks directly:
- After parsing the event name and `=>`, enter block parsing mode
- Use the existing `parse_block()` from `stmt.rs` which already handles balanced `{}`

- [ ] **Step 3: Run tests**

```bash
cd omni-compiler && cargo test 2>&1
cd D:/space/OmniLang && python -m pytest omniview/tests/test_preprocessor.py 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/parser/
git commit -m "feat(rust-parser): handle inline blocks directly, no preprocessor needed"
```

---

### Task 5.3: Delete Python Parser/Codegen

> ⚠️ **Prerequisite: Task 5.2 (preprocessor migration) must be complete and all tests passing before deleting preprocessor.py.**

- [ ] **Step 1: Verify Rust compiler covers all targets**

Checklist:
- [ ] `omni-compiler` passes all tests
- [ ] `compile_to_target` FFI works for all 10 targets (pure-logic + OmniView + framework)
- [ ] Python integration tests pass via Rust FFI

- [ ] **Step 2: Delete pure-logic codegen (omnilang/codegen/)**

```bash
git rm omnilang/codegen/__init__.py
git rm omnilang/codegen/base.py
git rm omnilang/codegen/python.py
git rm omnilang/codegen/javascript.py
git rm omnilang/codegen/rust.py
git rm omnilang/codegen/dart.py
git rm -r omnilang/codegen/helpers/
```

- [ ] **Step 3: Delete UI codegen (omniview/codegen/)**

```bash
git rm omniview/codegen/__init__.py
git rm omniview/codegen/base.py
git rm omniview/codegen/react.py
git rm omniview/codegen/vue.py
git rm omniview/codegen/dioxus.py
git rm omniview/codegen/flutter.py
git rm omniview/codegen/omniview.py
git rm omniview/codegen/css_mapping.py
git rm omniview/codegen/ir_reader.py
git rm omniview/codegen/source.py
```

- [ ] **Step 4: Delete Python parser**

```bash
git rm omnilang/parser.py
git rm omnilang/parser_expressions.py
git rm omnilang/parser_helpers.py
git rm omniview/parser.py
```

- [ ] **Step 5: Delete grammar files**

```bash
git rm omnilang/grammar/omnilang.lark
git rm omniview/grammar.lark
```

- [ ] **Step 6: Delete preprocessor (Task 5.2 must be done first!)**

```bash
git rm omniview/preprocessor.py
```

- [ ] **Step 7: Delete lowering + semantic**

```bash
git rm omniview/lowering.py
git rm omniview/semantic/analyzer.py
```

- [ ] **Step 8: Delete `omniview/ast_nodes.py`**

```bash
git rm omniview/ast_nodes.py
git rm omnilang/ast_nodes.py
```

- [ ] **Step 9: Clean up `omniview/api.py`**

Remove `_compile_ui_mode()`, `_compile_pure_logic()`, and all Python-native compilation code. Keep only:
- `compile_omni()` → routes to `RustBridge.compile_to_target()`
- `CompileError`, `CompileResult` data classes
- `parse_view_tree()`, `parse_code_blocks()` (editor utilities, use Rust via FFI)
- `_compute_diff()` helper

- [ ] **Step 10: Run full test suite**

```bash
cd omni-compiler && cargo test 2>&1
cd D:/space/OmniLang && python -m pytest omniview/tests/ -x 2>&1 | tail -5
```

Fix any breakage.

- [ ] **Step 11: Commit**

```bash
git add -A && git commit -m "refactor!: delete Python parser/codegen/preprocessor/semantic — Rust compiler is sole implementation"
```

---

### Task 5.4: Write Unified Grammar Spec

**Files:**
- Create: `docs/superpowers/specs/2026-06-16-omni-grammar-spec.md`

- [ ] **Step 1: Write grammar spec document**

Document the complete `.omni` language grammar in a single specification. Sections:
1. Lexical structure (tokens, keywords, identifiers, literals)
2. Program structure (imports → declarations)
3. Declarations (fn, struct, type, component, store, theme)
4. Statements (let, if/else, while, for-in, return, expression)
5. Expressions (literals, operators, precedence, function calls, method calls)
6. View syntax (elements, props, directives @if/@for/@slot, interpolation)
7. Import/export syntax (named, namespace, type-only, re-export, library)
8. Visibility (`pub`, auto-public for component/store/theme)

Derived from: Rust parser implementation + current Lark grammar files.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-06-16-omni-grammar-spec.md
git commit -m "docs: unified OmniLang grammar specification"
```

---

### Task 5.5: Lint Rules Migration to Rust

> Decision 22: 所有 40+ lint 规则在 Rust 重写。Current Python lints in `omnilang/lints/` and `omniview/lints/`.

- [ ] **Step 1: Audit existing Python lint rules**

List all rules in the Python lint modules:
```bash
ls D:/space/OmniLang/omnilang/lints/ 2>&1
ls D:/space/OmniLang/omniview/lints/ 2>&1
```
Document each rule: name, severity, check logic, what AST/IR node it inspects.

- [ ] **Step 2: Implement lint framework in Rust**

In `omni-compiler/src/semantic/lints.rs`:
```rust
use std::collections::HashMap;

use crate::ir::IRProgram;
use crate::parser::ast::Program;

pub struct LintDiagnostic {
    pub rule: String,
    pub severity: LintSeverity,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

pub enum LintSeverity { Error, Warning, Info }

/// A lint rule can inspect both AST and IR.
/// - AST-level rules: unused-import, naming-convention, dead-code (imports lowered away in IR)
/// - IR-level rules: type-mismatch, missing-key, overly-nested components
pub trait LintRule {
    fn name(&self) -> &str;
    fn severity(&self) -> LintSeverity;

    /// Check against the AST (called before lowering). Default: no-op.
    fn check_ast(&self, _program: &Program, _file_path: &str) -> Vec<LintDiagnostic> {
        vec![]
    }

    /// Check against the IR (called after lowering). Default: no-op.
    fn check_ir(&self, _ir: &IRProgram) -> Vec<LintDiagnostic> {
        vec![]
    }
}

/// Run all lint rules against ASTs and the merged IR.
pub fn run_all(
    programs: &HashMap<String, Program>,
    ir: &IRProgram,
) -> Vec<LintDiagnostic> {
    let rules: Vec<Box<dyn LintRule>> = vec![
        // Each rule added here as it's ported
    ];
    let mut diags = Vec::new();

    // AST-level checks
    for (file_path, program) in programs {
        for rule in &rules {
            diags.extend(rule.check_ast(program, file_path));
        }
    }

    // IR-level checks
    for rule in &rules {
        diags.extend(rule.check_ir(ir));
    }

    diags
}
```

- [ ] **Step 3: Port rules in batches (priority order)**

| Batch | Rules | Count |
|-------|-------|-------|
| 1 | Critical: undefined-ref, type-mismatch, missing-return | ~5 |
| 2 | Style: naming-convention, dead-code, unused-import | ~10 |
| 3 | Best-practice: overly-nested, large-component, missing-key | ~10 |
| 4 | Security: no-eval, sanitize-html, hardcoded-secret | ~5 |
| 5 | Remaining: all others | ~10+ |

Each batch: implement rules → run against test .omni files → compare with Python lint output.

- [ ] **Step 4: Wire lints into compile pipeline**

In `lib.rs`, `compile_multi_to_target()` — after semantic analysis + IR lowering, call `run_all(&programs, &merged_ir)` and append diagnostics to output JSON's `errors`/`warnings` fields.

- [ ] **Step 5: Delete Python lint files**

```bash
git rm omnilang/lints/*.py  # after all rules ported
git rm omniview/lints/*.py
```

- [ ] **Step 6: Commit**

```bash
git add omni-compiler/src/semantic/lints.rs
git commit -m "feat(rust-lints): lint rule framework + first batch of rules"
```
---

## Summary

| Phase | Duration | Tasks | Key Deliverable |
|-------|----------|-------|-----------------|
| 1 | 0.5 week | 1.1 | Import system audit, decisions on what to skip |
| 2 | 3-4 weeks | 2.0-2.6 | Rust pure-logic codegen (4 targets), FFI, Python integration, regression tests |
| 3 | 1-2 weeks | 3.1-3.2 | OmniView debug HTML wrapper, SourceMap completeness |
| 4 | 3-4 weeks | 4.1-4.5 | React/Vue/Dioxus/Flutter codegen in Rust + framework regression tests |
| 5 | 2-3 weeks | 5.1-5.5 | CLI merge, preprocessor→Rust, delete Python codegen/parser, grammar spec, lint rules |

**Total:** ~10-14 weeks. Phases 2-4 can partially overlap (different targets are independent).
