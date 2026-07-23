# Consolidate OmniView Compilation Utilities into omnilang

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate 4 categories of duplicated code between `omnic` CLI and `poiesis` editor by moving shared utilities into the `omnilang` crate, using the omnic/omnilang versions as authoritative.

**Architecture:** Consolidate HTML document wrapping, source file discovery, path mapping construction, and target string parsing into `omnilang` public APIs. Both `omnic` CLI and `poiesis` Tauri backend call the same functions. Poiesis editor/debug bridge code (generated via `editor_mode`/`debug_mode` flags in `codegen::omniview::generate()`) is preserved unchanged.

**Tech Stack:** Rust (`omnilang`, `omnic`, `poiesis/src-tauri`), TypeScript (`poiesis/src`)

## Global Constraints

- Poiesis `editor_mode` and `debug_mode` codegen paths MUST remain unchanged — these generate the editor bridge and debug bridge JS
- Use `omnilang` versions as authoritative when there is a conflict
- `omnilang::codegen::omniview::utils::wrap_debug_html()` and `build_file_output()` are already `pub` and are the canonical implementations
- `omnilang::path_mapping::derive_path_mapping()` is the canonical path mapping implementation
- `omnilang::codegen::Target::from_str()` is the canonical target parser
- No changes to Poiesis TypeScript source-generator pipeline (`pipeline.ts`, `source-emitter.ts`, etc.) — that is a separate concern

---

### Task 1: Add `discover_omni_sources()` to `omnilang`

**Files:**
- Create: `omnilang/src/discovery.rs`
- Modify: `omnilang/src/lib.rs` (add `pub mod discovery;`)
- Modify: `omnilang/Cargo.toml` (add `walkdir` dependency)

**Interfaces:**
- Produces: `pub fn discover_omni_sources(root: &Path, mode: DiscoveryMode) -> Result<FxHashMap<String, String>, String>`
- Produces: `pub enum DiscoveryMode { Directory, SingleFileWithSiblings }`

- [ ] **Step 1: Create `omnilang/src/discovery.rs`**

```rust
//! OmniLang source file discovery.
//! Shared between omnic CLI, Poiesis Tauri backend, and WASM API.

use rustc_hash::FxHashMap;
use std::path::Path;

/// How to discover .omni source files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryMode {
    /// Walk the entire directory tree for `**/*.omni`.
    Directory,
    /// Single file + sibling .omni files in the same tree (for import resolution).
    SingleFileWithSiblings,
}

/// Discover .omni source files and return them as (relative_path → content).
///
/// For Directory mode, paths are relative to `root`.
/// For SingleFileWithSiblings mode, the target file is keyed by its basename,
/// and sibling files are keyed by paths relative to the target's parent directory.
///
/// # Errors
///
/// Returns `Err(String)` if the root path does not exist or no .omni files are found.
#[must_use]
pub fn discover_omni_sources(
    root: &Path,
    mode: DiscoveryMode,
) -> Result<FxHashMap<String, String>, String> {
    match mode {
        DiscoveryMode::Directory => discover_directory(root),
        DiscoveryMode::SingleFileWithSiblings => discover_single_with_siblings(root),
    }
}

fn discover_directory(project_dir: &Path) -> Result<FxHashMap<String, String>, String> {
    if !project_dir.is_dir() {
        return Err(format!("Project directory not found: {}", project_dir.display()));
    }

    let mut sources: FxHashMap<String, String> = FxHashMap::default();
    let mut errors: Vec<String> = Vec::new();

    for entry in walkdir::WalkDir::new(project_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("[omni] warning: skipping inaccessible entry: {}", err);
                None
            }
        })
        .filter(|e| e.file_name().to_string_lossy().ends_with(".omni"))
    {
        let path = entry.path().to_owned();
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let rel_path = path
                    .strip_prefix(project_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                sources.insert(rel_path, content);
            }
            Err(e) => {
                errors.push(format!("Cannot read {:?}: {}", path, e));
            }
        }
    }

    if sources.is_empty() {
        let mut msg = String::from("No .omni source files found.");
        for err in &errors {
            msg.push_str(&format!("\n  - {err}"));
        }
        return Err(msg);
    }

    Ok(sources)
}

fn discover_single_with_siblings(file_path: &Path) -> Result<FxHashMap<String, String>, String> {
    if !file_path.is_file() {
        return Err(format!("File not found: {}", file_path.display()));
    }

    if file_path.extension().map_or(true, |e| e != "omni") {
        return Err(format!("Not a .omni file: {}", file_path.display()));
    }

    let mut sources: FxHashMap<String, String> = FxHashMap::default();

    // Add the target file itself
    let name = file_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| "Invalid file path: no filename".to_string())?;
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Cannot read {:?}: {}", file_path, e))?;
    sources.insert(name.clone(), content);

    // Walk sibling directories for import resolution
    let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
    for entry in walkdir::WalkDir::new(parent)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("[omni] warning: skipping inaccessible entry: {}", err);
                None
            }
        })
        .filter(|e| {
            e.file_name().to_string_lossy().ends_with(".omni") && e.path() != file_path
        })
    {
        let path = entry.path().to_owned();
        if let Ok(content) = std::fs::read_to_string(&path) {
            let rel_path = path
                .strip_prefix(parent)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            sources.insert(rel_path, content);
        }
    }

    Ok(sources)
}
```

- [ ] **Step 2: Register the module in `omnilang/src/lib.rs`**

Add after `pub mod codegen;` (around line 11):
```rust
pub mod discovery;
```

- [ ] **Step 3: Add `walkdir` dependency to `omnilang/Cargo.toml`**

Add `walkdir = "2"` in the `[dependencies]` section.

- [ ] **Step 4: Build**

```bash
cargo build -p omnilang --features codegen-omniview
```
Expected: Compiles successfully.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/discovery.rs omnilang/src/lib.rs omnilang/Cargo.toml
git commit -m "feat(omnilang): add discover_omni_sources() for shared file discovery"
```

---

### Task 2: Make omnic CLI use `discover_omni_sources()`

**Files:**
- Modify: `omnic/src/api.rs` (`collect_sources` function, lines 15-84)
- Modify: `omnic/Cargo.toml` (remove `glob` if unused elsewhere)

**Interfaces:**
- Consumes: `omnilang::discovery::discover_omni_sources(path, DiscoveryMode::Directory)`
- Produces: Same `collect_sources` return type

- [ ] **Step 1: Rewrite `collect_sources()` in `omnic/src/api.rs`**

Replace lines 15-84 with:

```rust
/// Collect .omni source files from the given paths.
///
/// - Empty list: auto-discover `src/**/*.omni` then `./*.omni`
/// - Directories: discover `.omni` files inside them
/// - Files: added directly with sibling discovery for import resolution
pub fn collect_sources(paths: &[String]) -> anyhow::Result<FxHashMap<String, String>> {
    use omnilang::discovery::{discover_omni_sources, DiscoveryMode};
    use std::path::Path;

    if paths.is_empty() {
        // Auto-discover: try src/ first, then .
        for dir in &["src", "."] {
            let p = Path::new(dir);
            if p.is_dir() {
                match discover_omni_sources(p, DiscoveryMode::Directory) {
                    Ok(sources) => return Ok(sources),
                    Err(_) => continue,
                }
            }
        }
        anyhow::bail!("No .omni files found. Specify file(s) or run from a project directory.");
    }

    let mut sources: FxHashMap<String, String> = FxHashMap::default();
    for p in paths {
        let path = Path::new(p);
        if path.is_dir() {
            let discovered = discover_omni_sources(path, DiscoveryMode::Directory)
                .map_err(|e| anyhow::anyhow!("{p}: {e}"))?;
            sources.extend(discovered);
        } else if path.exists() {
            let discovered = discover_omni_sources(path, DiscoveryMode::SingleFileWithSiblings)
                .map_err(|e| anyhow::anyhow!("{p}: {e}"))?;
            sources.extend(discovered);
        } else {
            anyhow::bail!("File not found: {p}");
        }
    }

    if sources.is_empty() {
        anyhow::bail!("No .omni files found. Specify file(s) or run from a project directory.");
    }

    Ok(sources)
}
```

Remove the now-unused `use std::path::Path;` at the top of `api.rs` if it's only used in `collect_sources`.

- [ ] **Step 2: Remove `glob` from `omnic/Cargo.toml` if unused**

```bash
grep -r "glob::" omnic/src/
```
If only in `collect_sources` (now removed), delete `glob = "..."` from `[dependencies]`.

- [ ] **Step 3: Build and test**

```bash
cargo build -p omnic
cargo test -p omnic
```
Expected: Compiles and tests pass.

- [ ] **Step 4: Commit**

```bash
git add omnic/src/api.rs omnic/Cargo.toml
git commit -m "refactor(omnic): use omnilang::discovery::discover_omni_sources()"
```

---

### Task 3: Make Poiesis Tauri use shared discovery + `Target::from_str()`

**Files:**
- Modify: `poiesis/src-tauri/src/lib.rs` (`compile_project` function, lines 190-354)
- Modify: `poiesis/src-tauri/Cargo.toml` (remove `walkdir` if unused)

**Interfaces:**
- Consumes: `omnilang::discovery::discover_omni_sources()`
- Consumes: `target_str.parse::<omnilang::codegen::Target>()`

- [ ] **Step 1: Replace target parsing (lines 190-218)**

Replace the `match target.as_str()` block with:

```rust
let codegen_target: omnilang::codegen::Target = match target.parse() {
    Ok(t) => t,
    Err(e) => {
        return CompileResult::from_error(
            &format!("Unknown target: {} ({})", target, e),
            "E_TARGET",
        );
    }
};
```

- [ ] **Step 2: Replace directory discovery (lines 226-275)**

Replace the `if project_dir.is_dir()` block with:

```rust
if project_dir.is_dir() {
    // Check file list cache
    let root_mtime = std::fs::metadata(project_dir)
        .ok()
        .and_then(|m| m.modified().ok());
    if let Ok(cache) = state.file_list_cache.lock() {
        if let Some(ref cached) = *cache {
            if cached.root_mtime == root_mtime {
                let mut compile_cache = state
                    .compile_cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                return do_compile(
                    cached.sources.clone(),
                    codegen_target,
                    Some(&mut *compile_cache),
                );
            }
        }
    }

    match omnilang::discovery::discover_omni_sources(
        project_dir,
        omnilang::discovery::DiscoveryMode::Directory,
    ) {
        Ok(s) => sources = s,
        Err(e) => {
            return CompileResult::from_error(&e, "E_DISCOVERY");
        }
    }

    // Update file list cache
    if let Ok(mut cache) = state.file_list_cache.lock() {
        *cache = Some(CachedFileList {
            sources: sources.clone(),
            root_mtime,
        });
    }
}
```

- [ ] **Step 3: Replace single-file discovery (lines 281-336)**

Replace the `else if project_dir.is_file()` block with:

```rust
} else if project_dir.is_file()
    && project_dir.extension().is_some_and(|e| e == "omni")
{
    match omnilang::discovery::discover_omni_sources(
        project_dir,
        omnilang::discovery::DiscoveryMode::SingleFileWithSiblings,
    ) {
        Ok(s) => sources = s,
        Err(e) => {
            return CompileResult::from_error(&e, "E_DISCOVERY");
        }
    }
}
```

Delete the now-unused `let mut discover_errors: Vec<String> = Vec::new();` and related code.

- [ ] **Step 4: Remove `walkdir` from `poiesis/src-tauri/Cargo.toml` if unused**

```bash
grep -r "walkdir" poiesis/src-tauri/src/
```
If only in `compile_project`, remove from `[dependencies]`.

- [ ] **Step 5: Build**

```bash
cargo build -p poiesis
```
Expected: Compiles successfully.

- [ ] **Step 6: Commit**

```bash
git add poiesis/src-tauri/src/lib.rs poiesis/src-tauri/Cargo.toml
git commit -m "refactor(poiesis): use omnilang discovery + Target::from_str in compile_project"
```

---

### Task 4: Fix `omnic run` to use `wrap_debug_html()`

**Files:**
- Modify: `omnic/src/commands/run.rs` (`assemble_omniview_html` function, lines 26-28, 110-166)

**Interfaces:**
- Consumes: `omnilang::codegen::omniview::utils::wrap_debug_html(js, css, body_html) -> String`

- [ ] **Step 1: Rewrite `assemble_omniview_html()` (lines 111-166)**

Replace the entire function with:

```rust
/// Assemble a standalone OmniView HTML file using the shared utility.
fn assemble_omniview_html(output_dir: &Path) -> anyhow::Result<()> {
    use omnilang::codegen::omniview::utils::wrap_debug_html;

    let body_html = read_file_opt(output_dir, "body.html")
        .or_else(|| read_file_opt(output_dir, "index.html"))
        .unwrap_or_default();
    let js = read_file_opt(output_dir, "index.js").unwrap_or_default();
    let css = read_file_opt(output_dir, "index.css").unwrap_or_default();

    let html = wrap_debug_html(&js, &css, &body_html);

    let index_path = output_dir.join("index.html");
    std::fs::write(&index_path, &html)?;

    let abs_path = std::fs::canonicalize(&index_path)?;
    println!("Open {} in your browser", abs_path.display());
    Ok(())
}

fn read_file_opt(output_dir: &Path, name: &str) -> Option<String> {
    let path = output_dir.join(name);
    match std::fs::read_to_string(&path) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            eprintln!("warning: failed to read {}: {}", path.display(), e);
            None
        }
    }
}
```

Remove the old `read_file` closure and the `requestAnimationFrame` workaround (it belongs in the runtime, not the CLI wrapper).

- [ ] **Step 2: Build and test**

```bash
cargo build -p omnic
```
Expected: Compiles successfully.

- [ ] **Step 3: Smoke test**

```bash
cargo run -p omnic -- run --target omniview
```
Expected: Produces valid standalone HTML.

- [ ] **Step 4: Commit**

```bash
git add omnic/src/commands/run.rs
git commit -m "refactor(omnic): use wrap_debug_html() instead of manual HTML assembly"
```

---

### Task 5: Expose `wrap_debug_html` via WASM, use in Poiesis TS

**Files:**
- Modify: `omnilang/src/wasm.rs` (add WASM binding)
- Modify: `poiesis/src/commands/rustCompiler.ts` (add TS wrapper + type)
- Modify: `poiesis/src/features/compiler/useCompile.ts:147-167` (use it)

- [ ] **Step 1: Add WASM binding in `omnilang/src/wasm.rs`**

Add after `resolve_styles` (after line 123):

```rust
/// Wrap generated OmniView output in a standalone debug-ready HTML document.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn wrap_debug_html(js: &str, css: &str, body_html: &str) -> String {
    crate::codegen::omniview::utils::wrap_debug_html(js, css, body_html)
}
```

- [ ] **Step 2: Add to TS `RustCompiler` class in `rustCompiler.ts`**

Add to the `wasm` interface type (around line 125):
```typescript
wrap_debug_html?: (js: string, css: string, body_html: string) => string;
```

Add to `init()` method wasm binding (around line 173):
```typescript
wrap_debug_html: wasmModule.wrap_debug_html as unknown as (js: string, css: string, body_html: string) => string,
```

Add method after `resolveStylesMulti` (after line 437):
```typescript
wrapDebugHtml(js: string, css: string, bodyHtml: string): string {
  if (!this.wasm?.wrap_debug_html) {
    // Fallback for older WASM modules
    return `<!DOCTYPE html>
<html lang="en"><head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OmniView Debug</title><style>
*,*::before,*::after{box-sizing:border-box}
body{margin:0;padding:8px;font-family:system-ui,sans-serif}
${css}</style></head><body>
${bodyHtml}<script>${js}</script></body></html>`;
  }
  return this.wasm.wrap_debug_html(js, css, bodyHtml);
}
```

- [ ] **Step 3: Update `useCompile.ts` (lines 147-167)**

Replace the inline HTML template literal with:
```typescript
const srcdoc = rustCompiler.wrapDebugHtml(
  files['index.js'] || '',
  files['index.css'] || '',
  files['index.html'] || '',
);
```

- [ ] **Step 4: Rebuild WASM**

```bash
cd omnilang && wasm-pack build --target web --features wasm,codegen-omniview
# Copy output to poiesis/src/wasm/
```

- [ ] **Step 5: Build Poiesis frontend**

```bash
cd poiesis && npx tsc --noEmit
```
Expected: No new type errors.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/wasm.rs poiesis/src/commands/rustCompiler.ts poiesis/src/features/compiler/useCompile.ts poiesis/src/wasm/
git commit -m "feat: expose wrap_debug_html via WASM, deduplicate HTML wrapping in Poiesis"
```

---

### Task 6: Clean up path mapping duplication

**Files:**
- Modify: `omnilang/src/wasm.rs` (`compile_multi_to_omniview`, `resolve_styles_multi`)
- Modify: `poiesis/src/commands/rustCompiler.ts` (`compileForDebugWindow`)
- Modify: `poiesis/src/components/design/useWasmCompile.ts`

- [ ] **Step 1: Fix `wasm.rs::compile_multi_to_omniview()` (lines 195-209)**

Replace the manual path_mapping construction with:
```rust
let path_mapping = crate::path_mapping::derive_path_mapping(&sources_map);
```

- [ ] **Step 2: Fix `wasm.rs::resolve_styles_multi()` — merge auto-derived mapping**

After deserializing `path_map`, add:
```rust
// Merge auto-derived path mapping from source keys
let derived = crate::path_mapping::derive_path_mapping(&sources_map);
for (k, v) in derived {
    path_map.entry(k).or_insert(v);
}
```
Then pass `path_map` to `crate::resolve_styles_multi()` instead of the original `path_mapping`.

- [ ] **Step 3: Simplify TS-side path mapping in `rustCompiler.ts::compileForDebugWindow()`**

Replace lines 315-327 with:
```typescript
// Path mapping is auto-derived in WASM from source keys
const pathMapping: Record<string, string> = {};
```

- [ ] **Step 4: Simplify TS-side path mapping in `useWasmCompile.ts`**

Replace lines 115-123 with:
```typescript
// Path mapping is auto-derived by WASM from source keys
const pathMapping: Record<string, string> = {};
```

- [ ] **Step 5: Rebuild WASM + type check**

Same as Task 5 steps 4-5.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/wasm.rs poiesis/src/commands/rustCompiler.ts poiesis/src/components/design/useWasmCompile.ts
git commit -m "refactor: use derive_path_mapping() in WASM, remove TS-side path mapping"
```

---

### Task 7: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Build all workspace crates**

```bash
cargo build --workspace --features codegen-omniview
```
Expected: All crates compile without errors.

- [ ] **Step 2: Run omnilang tests**

```bash
cargo test -p omnilang --features codegen-omniview
```
Expected: All tests pass.

- [ ] **Step 3: Run omnic tests**

```bash
cargo test -p omnic
```
Expected: All tests pass.

- [ ] **Step 4: Run Poiesis frontend type check**

```bash
cd poiesis && npx tsc --noEmit
```
Expected: No type errors.

- [ ] **Step 5: Manual integration test — `omnic run`**

```bash
cargo run -p omnic -- run --target omniview
```
Expected: Produces `build/debug/index.html`, opens correctly in browser.

- [ ] **Step 6: Manual integration test — Poiesis canvas + debug window**

Start Poiesis, verify canvas preview renders and debug window compiles.

- [ ] **Step 7: Commit**

```bash
git commit -m "chore: final verification — all tests pass after consolidation"
```

---

## Summary of Changes

| Category | Before | After |
|----------|--------|-------|
| Source discovery | 2 impls (glob in omnic, walkdir in poiesis) | 1 impl in `omnilang::discovery` |
| Target parsing | Manual `match` in poiesis, `FromStr` in omnic | Both use `Target::from_str()` |
| HTML wrapping | 4 implementations | 1 impl `wrap_debug_html()` in omnilang |
| Path mapping | 3 manual constructions | 1 auto-derived via `derive_path_mapping()` |
| ~150 lines duplicated | Consolidated into omnilang |
