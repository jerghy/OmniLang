# OmniLang 代码审查修复实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 2026-07-06 全面代码审查报告的发现，按优先级修复 9 个 CRITICAL、19 个 HIGH、27 个 MEDIUM 和部分 LOW 问题。

**Architecture:** 分 4 个 Phase 推进：安全修复 → 错误处理 → 架构优化 → 质量提升。每个 Phase 内任务独立，可并行执行。

**Tech Stack:** Rust (omnilang crate), TypeScript/React (Poiesis), Tauri v2

**Source Report:** `docs/CODE_REVIEW_REPORT_2026-07-06.md`

## Global Constraints

- 所有修复不能破坏现有的 384 个测试
- TypeScript 类型检查必须保持零错误
- 每个 Phase 完成后运行 `cargo test --workspace` 和 `cd poiesis && npx tsc --noEmit`
- 遵循现有代码风格：Rust → clippy/fmt, TypeScript → Prettier
- 每个 Task 独立可验证，提交粒度 ≤ 3 个文件

---

## Phase 1: 安全修复 🔴（5 天，9 个 CRITICAL 问题）

### Task 1.1: 启用 Tauri CSP + 限制文件系统权限 + 关闭 shell.open

**Files:**
- Modify: `poiesis/src-tauri/tauri.conf.json:22,27-29`
- Modify: `poiesis/src-tauri/capabilities/default.json:10-12`

**Interfaces:**
- Produces: CSP header string, restricted fs:scope patterns, removed shell.open
- Side effects: 编辑器模式下的 eval 暂时被 CSP 阻止，后续 Task 1.2 解决

- [ ] **Step 1: 修改 tauri.conf.json — 启用 CSP 并关闭 shell.open**

```json
"app": {
  "windows": [...],
  "security": {
    "csp": "default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self' https:; img-src 'self' data: https:; font-src 'self'"
  }
},
"plugins": {
  "fs": {},
  "shell": {}
}
```

注意：`shell.open` 完全移除。如编辑器模式确实需要浏览器打开，后续可添加精确的 URL 白名单。

- [ ] **Step 2: 修改 capabilities/default.json — 限制 fs:scope**

```json
{
  "identifier": "fs:scope",
  "allow": [
    { "path": "$HOME/.omnilang/**" },
    { "path": "$APPDATA/omnilang/**" },
    { "path": "$HOME/Documents/**" },
    { "path": "$TEMP/omnilang/**" }
  ]
}
```

- [ ] **Step 3: 验证 — 构建 Poiesis 确认无启动错误**

```bash
cd poiesis && npx tauri build --debug 2>&1 | tail -5
```
Expected: 构建成功，应用启动后文件操作在允许目录内工作。

- [ ] **Step 4: 提交**

```bash
git add poiesis/src-tauri/tauri.conf.json poiesis/src-tauri/capabilities/default.json
git commit -m "fix: enable CSP, restrict fs:scope, remove shell.open

- Replace csp:null with strict CSP allowing 'unsafe-eval' for editor mode
- Limit fs:scope from '**' wildcard to $HOME/.omnilang/** and safe dirs
- Remove shell.open to prevent arbitrary URL/file opening
- Addresses C1, C3, H12 from code review"
```

---

### Task 1.2: 修复编辑器 postMessage 来源验证 + 移除生产 eval

**Files:**
- Modify: `omnilang/src/codegen/omniview/editor.rs:39-93`
- Modify: `omnilang/src/codegen/omniview/list_render.rs:86,131,151,302`
- Modify: `omnilang/src/codegen/omniview/runtime.rs:112`

**Interfaces:**
- Consumes: CSP 中的 `'unsafe-eval'`（Task 1.1）
- Produces: 安全的编辑器桥接代码，`eval()` 替换为 `new Function()`
- 编辑器模式继续使用 `eval()`（受 CSP + origin 验证保护），生产构建不使用 eval

- [ ] **Step 1: 修复 editor.rs — 添加 event.origin 验证 + 替换 targetOrigin '*'**

定位 `emit_editor_bridge` 方法（第 17-95 行）。修改三处：

```rust
// 第 43 行: 替换 targetOrigin '*'
// 旧:
parent.postMessage({ type: 'omni-select', nid: el.dataset.nid }, '*');
// 新:
parent.postMessage({ type: 'omni-select', nid: el.dataset.nid }, window.location.origin);

// 第 47-59 行: 添加 origin 验证
// 旧:
window.addEventListener('message', function(e) {
  switch (e.data.type) {
// 新:
window.addEventListener('message', function(e) {
  if (e.origin !== window.location.origin) return;
  switch (e.data.type) {

// 第 89 行: 替换 targetOrigin '*'
// 旧:
parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, '*');
// 新:
parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, window.location.origin);

// 第 92 行: 替换 targetOrigin '*'
// 旧:
parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, '*');
// 新:
parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, window.location.origin);
```

- [ ] **Step 2: 修复 list_render.rs — 用 `new Function()` 替代 `eval()`**

对于 `@for` 循环和动态类绑定的 `eval()`，使用 `new Function()` 包装以适应 CSP：

```rust
// 旧模式 (list_render.rs:86 等):
el.textContent = String(eval(expr));
// 新模式:
// 用 Function 构造器在独立作用域中求值，避免访问闭包变量
var _fn = new Function('return ' + expr);
el.textContent = String(_fn());

// 旧模式 (动态类绑定):
!!eval(classMap[cls])
// 新模式:
var _fn2 = new Function('return ' + classMap[cls]);
!!_fn2()
```

注：`new Function()` 仍受 CSP `'unsafe-eval'` 限制，但隔离了作用域（无法访问局部变量），比裸 `eval()` 安全。最终的彻底方案是将表达式预编译到信号系统中，但那是长期目标。

- [ ] **Step 3: 修复 runtime.rs:112 — 同样替换 eval() 模式**

- [ ] **Step 4: 运行现有测试确认无回归**

```bash
cargo test -p omnilang -- codegen::omniview
```
Expected: 所有测试通过。

- [ ] **Step 5: 提交**

```bash
git add omnilang/src/codegen/omniview/editor.rs omnilang/src/codegen/omniview/list_render.rs omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix: add postMessage origin validation, replace eval() with Function()

- Validate event.origin in editor bridge message listener
- Replace targetOrigin '*' with window.location.origin
- Replace eval() with new Function() for CSP compatibility
- Addresses C1, C2 from code review"
```

---

### Task 1.3: 修复 react/view.rs 的 unwrap() panic 风险

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:252,287,317,333`

**Interfaces:**
- Produces: 4 处 `remove().unwrap()` 替换为安全的 `if let Some()` 模式

- [ ] **Step 1: 将 4 处 `remove().unwrap()` 替换为 `if let Some()`**

```rust
// 第 251-252 行 (fn_ref 分支):
// 旧:
if has_binding_conflict {
    let setter_call = binding_events_map.remove(&react_event).unwrap();
// 新:
if let Some(setter_call) = binding_events_map.remove(&react_event) {

// 第 286-287 行 (inline_fn 分支):
// 旧:
if has_binding_conflict {
    let setter_call = binding_events_map.remove(&react_event).unwrap();
// 新:
if let Some(setter_call) = binding_events_map.remove(&react_event) {

// 第 316-317 行 (store_action 分支):
// 旧:
if has_binding_conflict {
    let setter_call = binding_events_map.remove(&react_event).unwrap();
// 新:
if let Some(setter_call) = binding_events_map.remove(&react_event) {

// 第 331-333 行 (default 分支):
// 旧:
if has_binding_conflict {
    let setter_call = binding_events_map.remove(&react_event).unwrap();
// 新:
if let Some(setter_call) = binding_events_map.remove(&react_event) {
```

同时移除不再需要的 `has_binding_conflict` 变量声明（第 243 行）：

```rust
// 旧:
let has_binding_conflict = binding_events_map.contains_key(&react_event);
// 新: 删除此行，改为在每个分支中直接调用 if let Some(...)
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p omnilang -- codegen::react
```
Expected: 所有 React codegen 测试通过。

- [ ] **Step 3: 提交**

```bash
git add omnilang/src/codegen/react/view.rs
git commit -m "fix: replace unwrap() with if-let in binding_events_map.remove()

- 4 locations in react/view.rs event handler codegen
- Prevents panic when multiple handlers map to same React event
- Addresses C4 from code review"
```

---

### Task 1.4: 修复生成的 Rust 代码中的 .unwrap()

**Files:**
- Modify: `omnilang/src/stdlib/json.rs:35,55`
- Modify: `omnilang/src/stdlib/http.rs:52`
- Modify: `omnilang/src/stdlib/storage.rs:39,61`

**Interfaces:**
- Produces: 生成的 Rust/Dioxus 代码使用 `?` 传播或 `.expect("reason")` 替代裸 `.unwrap()`

- [ ] **Step 1: 修复 json.rs — 用 expect 替代 unwrap**

```rust
// 第 35 行:
// 旧:
format!("serde_json::from_str(&{text}).unwrap()", text = text)
// 新:
format!("serde_json::from_str(&{text}).expect(\"JSON parse error\")", text = text)

// 第 55 行:
// 旧:
format!("serde_json::to_string(&{val}).unwrap()", val = val)
// 新:
format!("serde_json::to_string(&{val}).expect(\"JSON serialize error\")", val = val)
```

- [ ] **Step 2: 修复 http.rs — 用 expect 替代 unwrap**

```rust
// 第 52 行:
// 旧:
"reqwest::Client::new().request(reqwest::Method::from_str(&{req}.method).unwrap(), &{req}.url)..."
// 新:
"reqwest::Client::new().request(reqwest::Method::from_str(&{req}.method).expect(\"Invalid HTTP method\"), &{req}.url)..."
```

- [ ] **Step 3: 修复 storage.rs — 用编译时错误替代 unreachable!()**

```rust
// 第 38-39 行 (storage_get) 和 60-61 行 (storage_set):
// 旧:
_ => unreachable!("storage_get not available on {:?}", target),
// 新:
_ => format!("compile_error!(\"storage_get not available on {:?} target\", {target:?})", target = format!("{:?}", target).to_lowercase()),
```

- [ ] **Step 4: 运行生成的 Rust 目标测试**

```bash
cargo test -p omnilang -- codegen::rust
cargo test -p omnilang -- codegen::dioxus
```
Expected: 所有测试通过。

- [ ] **Step 5: 提交**

```bash
git add omnilang/src/stdlib/json.rs omnilang/src/stdlib/http.rs omnilang/src/stdlib/storage.rs
git commit -m "fix: replace .unwrap() with .expect() in generated Rust code

- json_parse/json_stringify for Rust/Dioxus targets
- http_request for Rust/Dioxus targets
- storage_get/set unreachable!() → compile_error!()
- Addresses C5 from code review"
```

---

### Task 1.5: 修复路径遍历 — 符号链接防护 + 规范化

**Files:**
- Modify: `poiesis/src-tauri/src/lib.rs:516-527` (sanitize_path)
- Modify: `poiesis/src-tauri/src/lib.rs:458,473` (open_project, save_project)

**Interfaces:**
- Consumes: 项目基础目录 `dir`
- Produces: `fn sanitize_path(base: &Path, user_path: &str) -> Result<PathBuf, String>` 返回安全的规范化路径

- [ ] **Step 1: 重写 sanitize_path 函数**

```rust
/// Validate and canonicalize a user-supplied relative path against a base directory.
/// Returns the canonicalized path if it is within the base directory.
fn sanitize_path(base: &std::path::Path, user_path: &str) -> Result<std::path::PathBuf, String> {
    // Reject empty paths
    if user_path.is_empty() {
        return Err("Empty path is not allowed".to_string());
    }
    
    // Reject paths that are already absolute
    let p = std::path::Path::new(user_path);
    if p.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }
    
    // Join with base and canonicalize (resolves symlinks, .., etc.)
    let full = base.join(p);
    let canonical = full.canonicalize().map_err(|e| {
        format!("Invalid path: {}", e)
    })?;
    
    // Canonicalize the base for comparison
    let base_canonical = base.canonicalize().map_err(|e| {
        format!("Base directory error: {}", e)
    })?;
    
    // Verify the resolved path is within the base directory
    if !canonical.starts_with(&base_canonical) {
        return Err("Path traversal detected".to_string());
    }
    
    Ok(canonical)
}
```

- [ ] **Step 2: 更新所有调用点** — `read_file`, `write_file`, `open_project`, `save_project`

将 `sanitize_path(&file)?` 替换为 `sanitize_path(&base_dir, &file)?`，其中 `base_dir` 从 `ProjectState` 获取。

- [ ] **Step 3: 修复 file_stem/file_name 的 unwrap()（第 166-167 行）**

```rust
// 旧:
let _file_stem = project_dir.file_stem().unwrap().to_string_lossy().to_string();
let name = project_dir.file_name().unwrap().to_string_lossy().to_string();
// 新:
let _file_stem = project_dir.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
let name = project_dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
if name.is_empty() {
    return CompileResult::from_error("Invalid file path: no filename", "E_IO");
}
```

- [ ] **Step 4: 运行 Poiesis 测试**

```bash
cargo test -p poiesis
```
Expected: 所有 Tauri 测试通过。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src-tauri/src/lib.rs
git commit -m "fix: harden path traversal protection with canonicalize

- Rewrite sanitize_path to use canonicalize + prefix check
- Now detects symlink traversal to outside directories
- Apply sanitize_path to all file operation commands
- Replace file_stem().unwrap() with safe fallback
- Addresses C7 from code review"
```

---

### Task 1.6: 修复 TypeScript 端静默 I/O 错误（空 catch 块）

**Files:**
- Modify: `poiesis/src/commands/annotation-api.ts:21,36,105,145,203`
- Modify: `poiesis/src/commands/project-api.ts:32,183,194,211,223`
- Modify: `poiesis/src/features/useProjectInit.ts:32,44`
- Modify: `poiesis/src/features/compiler/useCompile.ts:144`
- Modify: `poiesis/src/services/persistence/load-manager.ts:25`

**Interfaces:**
- Consumes: 现有的函数签名
- Produces: 所有空 catch 块至少包含 `console.warn("[Module] description:", e)`，关键路径添加 toast 通知

- [ ] **Step 1: 修复 annotation-api.ts — 5 个空 catch 块**

```typescript
// 第 21 行:
// 旧: catch { return null; }
// 新: catch (e) { console.warn("[annotation] Failed to resolve project dir:", e); return null; }

// 第 36 行:
// 旧: catch {}
// 新: catch (e) { console.warn("[annotation] Failed to create .poiesis dir:", e); }

// 第 105 行:
// 旧: catch { return { definitions: {} }; }
// 新: catch (e) { console.warn("[annotation] Failed to load component definitions:", e); return { definitions: {}, error: String(e) }; }

// 第 145 行:
// 旧: catch { return { entries: {}, lineToPid: {} }; }
// 新: catch (e) { console.warn("[annotation] Failed to read PID map:", e); return { entries: {}, lineToPid: {}, error: String(e) }; }

// 第 203 行:
// 旧: catch { return { state: null }; }
// 新: catch (e) { console.warn("[annotation] Failed to load editor.toml:", e); return { state: null, error: String(e) }; }
```

- [ ] **Step 2: 修复 project-api.ts — 5 个空 catch 块**

```typescript
// 第 32 行:
// 旧: catch { return null; }
// 新: catch (e) { console.warn("[project] Failed to resolve project dir:", e); return null; }

// 第 183 行:
// 旧: catch { return null; }
// 新: catch (e) { console.warn("[project] Failed to open folder dialog:", e); return null; }

// 第 194 行:
// 旧: catch { return []; }
// 新: catch (e) { console.warn("[project] Failed to get recent projects:", e); return []; }

// 第 211 行:
// 旧: catch {}
// 新: catch (e) { console.warn("[project] Failed to add recent project:", e); }

// 第 223 行:
// 旧: catch {}
// 新: catch (e) { console.warn("[project] Failed to remove recent project:", e); }
```

- [ ] **Step 3: 修复 useProjectInit.ts, useCompile.ts, load-manager.ts 中的空 .catch()**

```typescript
// useProjectInit.ts:32
// 旧: saveComponentDefinitionsToServer(...).catch(() => {});
// 新: saveComponentDefinitionsToServer(...).catch((e) => console.warn("[init] Failed to save component defs:", e));

// useProjectInit.ts:44
// 旧: api.compile.stopRun().catch(() => {});
// 新: api.compile.stopRun().catch((e) => console.warn("[init] Failed to stop run:", e));

// useCompile.ts:144
// 旧: api.compile.writeRunFiles(...).catch(() => {});
// 新: api.compile.writeRunFiles(...).catch((e) => console.warn("[compile] Failed to write run files:", e));

// load-manager.ts:25
// 旧: catch { projectJson = createEmptyProjectJson(); }
// 新: catch (e) { console.warn("[persistence] Failed to load project.json, using empty:", e); projectJson = createEmptyProjectJson(); }
```

- [ ] **Step 4: 类型检查**

```bash
cd poiesis && npx tsc --noEmit
```
Expected: 零错误。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/commands/annotation-api.ts poiesis/src/commands/project-api.ts poiesis/src/features/useProjectInit.ts poiesis/src/features/compiler/useCompile.ts poiesis/src/services/persistence/load-manager.ts
git commit -m "fix: replace empty catch blocks with console.warn logging

- 10+ empty catch {} blocks now log errors with module prefix
- annotation-api.ts: 5 catches → console.warn
- project-api.ts: 5 catches → console.warn
- useProjectInit/useCompile/load-manager: .catch(()→{}) → .catch(e→warn)
- Addresses C8 from code review"
```

---

### Task 1.7: 修复 Rust 端 I/O 错误静默吞没

**Files:**
- Modify: `omnic/src/main.rs:667-671` (watch 模式)
- Modify: `omnic/src/main.rs:291,387,670` (content.as_str().unwrap_or(""))
- Modify: `omnic/src/main.rs:200` (analyse 默认值)

**Interfaces:**
- Produces: I/O 错误 -> 用户可见的错误消息；缺失 JSON 字段 -> 生成错误而非静默空值

- [ ] **Step 1: 修复 watch 模式静默 I/O 错误**

```rust
// 第 667-671 行:
// 旧:
let _ = std::fs::create_dir_all(parent);
let _ = std::fs::write(&out_path, content.as_str().unwrap_or(""));
// 新:
if let Err(e) = std::fs::create_dir_all(parent) {
    eprintln!("[{timestamp}] #{build_count} Create dir error: {e}");
    continue;
}
if let Err(e) = std::fs::write(&out_path, content.as_str().unwrap_or("")) {
    eprintln!("[{timestamp}] #{build_count} Write error: {e}");
    continue;
}
```

- [ ] **Step 2: 修复 content 字段缺失时的静默空文件**

```rust
// 第 291, 387, 670 行:
// 旧:
let content = content.as_str().unwrap_or("");
// 新:
let content = match content.as_str() {
    Some(s) => s,
    None => {
        eprintln!("Warning: null or non-string content for file '{}', skipping", filename);
        continue;
    }
};
```

- [ ] **Step 3: 修复 analyse 命令默认值不一致**

```rust
// 第 200 行:
// 旧:
if !result["success"].as_bool().unwrap_or(true) {
// 新:
if !result["success"].as_bool().unwrap_or(false) {
    // 缺失 success 字段 → 视为失败，与 build/run 一致
```

- [ ] **Step 4: 验证 CLI 行为**

```bash
cargo run -p omnic -- analyse src/main.omni --format json 2>&1
cargo run -p omnic -- build src/main.omni --target omniview 2>&1
```
Expected: 缺失 JSON 字段时输出可见错误。

- [ ] **Step 5: 提交**

```bash
git add omnic/src/main.rs
git commit -m "fix: surface I/O errors in watch mode, consistent defaults

- watch mode: replace let _ = with error logging for write/create_dir
- content.as_str().unwrap_or(\"\") → skip with warning on missing content
- analyse: change unwrap_or(true) → unwrap_or(false) for consistency
- Addresses C8, H16, H17 from code review"
```

---

### Task 1.8: 修复解析器静默将无效字面量转为 0

**Files:**
- Modify: `omnilang/src/lexer/mod.rs:553,572,591,661,669`

**Interfaces:**
- Consumes: 现有 token 类型
- Produces: 无效数字字面量 → ParseError 包含位置信息，而非静默 0

- [ ] **Step 1: 修改 5 处 unwrap_or(0/0.0) 为错误处理**

lexer 需要在这些位置返回 `ParseError` 而非静默回退。由于 lexer 结构每个 token 处理一个字符，最佳做法是将这些 `.unwrap_or()` 替换为检查 + 返回错误：

```rust
// 第 553 行 (十六进制):
// 旧:
let value = u64::from_str_radix(&hex, 16).unwrap_or(0);
// 新: 使用 match 并在错误时产生诊断
let value = match u64::from_str_radix(&hex, 16) {
    Ok(v) => v,
    Err(_) => {
        return Err(ParseError {
            message: format!("Invalid hex literal: 0x{}", hex),
            span: Span { line: self.line, col: start_col..self.col, byte_offset: start_byte..self.byte_of(self.pos) },
        });
    }
};

// 同理处理 八进制(572), 二进制(591), 浮点数(661,669)
```

- [ ] **Step 2: 添加解析器测试**

在 `omnilang/src/parser/mod.rs` 的测试模块中添加：

```rust
#[test]
fn test_reject_invalid_hex() {
    let source = "let x = 0xFFG;";
    let tokens = lexer::tokenize(source);
    let result = parse(&tokens, source);
    assert!(result.is_err(), "Invalid hex '0xFFG' should produce parse error");
}

#[test]
fn test_reject_invalid_float() {
    let source = "let x = 1.2.3;";
    let tokens = lexer::tokenize(source);
    let result = parse(&tokens, source);
    assert!(result.is_err(), "Invalid float '1.2.3' should produce parse error");
}
```

- [ ] **Step 3: 运行测试**

```bash
cargo test -p omnilang -- lexer
cargo test -p omnilang -- parser
```
Expected: 新测试 PASS，现有测试 PASS。

- [ ] **Step 4: 提交**

```bash
git add omnilang/src/lexer/mod.rs omnilang/src/parser/mod.rs
git commit -m "fix: reject invalid numeric literals instead of silently converting to 0

- Hex/octal/binary/float parse failures now return ParseError
- Adds tests for invalid hex (0xFFG) and invalid float (1.2.3)
- Addresses C9 from code review"
```

---

### Task 1.9: 修复 WASM API 的 unwrap_or_default() 静默配置忽略

**Files:**
- Modify: `omnilang/src/wasm.rs:21,53,103,132,207,219`

**Interfaces:**
- Produces: 格式错误的 options_json → 返回错误 JSON（包含解析失败信息），而非静默使用默认值

- [ ] **Step 1: 将所有 unwrap_or_default() 替换为返回错误 JSON**

在 wasm.rs 顶部添加辅助函数：

```rust
fn parse_options_or_error(options_json: &str) -> Result<CompileOptions, String> {
    serde_json::from_str::<CompileOptions>(options_json).map_err(|e| {
        serde_json::json!({
            "success": false,
            "files": null,
            "errors": [{
                "severity": "error",
                "code": "E_INVALID_OPTIONS",
                "message": format!("Failed to parse options JSON: {}", e),
                "line": 0, "col": 0, "end_line": 0, "end_col": 0, "suggestion": null
            }]
        }).to_string()
    })
}
```

然后更新每个调用点：

```rust
// 旧:
let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
// 新:
let opts = match parse_options_or_error(options_json) {
    Ok(o) => o,
    Err(error_json) => return error_json,
};
```

涉及位置：第 21、53、103、132、207、219 行。

- [ ] **Step 2: 运行 WASM 测试**

```bash
cargo test -p omnilang -- wasm --features wasm
```
Expected: 所有 WASM 测试通过。

- [ ] **Step 3: 提交**

```bash
git add omnilang/src/wasm.rs
git commit -m "fix: return error on invalid WASM options JSON instead of silent default

- Add parse_options_or_error() helper
- 6 call sites now return structured error JSON on parse failure
- Addresses H11 from code review"
```

---

## Phase 2: 错误处理加固 🟠（4 天）

### Task 2.1: 修复时间线/动画参数静默默认值

**Files:**
- Modify: `omnilang/src/codegen/omniview/timeline.rs:76,130,233`
- Modify: `omnilang/src/codegen/react/motion.rs:374,419,420`

- [ ] **Step 1: timeline.rs — 替换 unwrap_or 为编译警告生成**

```rust
// 第 76, 130, 233 行: 将 .unwrap_or(default) 替换为 match + 自文档化
// 例如第 76 行:
let duration = match config.get("duration").and_then(|v| v.as_f64()) {
    Some(d) if d > 0.0 => d,
    Some(_) => { self.warnings.push("timeline duration must be positive".into()); 0.3 }
    None => { self.warnings.push("timeline duration missing, using default 0.3s".into()); 0.3 }
};
```

- [ ] **Step 2: react/motion.rs — 记录被丢弃的动画参数**

```rust
// 第 374, 419, 420 行:
// 旧: let _ = velocity;
// 新: self.warnings.push(format!("Spring velocity {:?} discarded in React motion conversion", velocity));
```

- [ ] **Step 3: 运行 codegen 测试**

```bash
cargo test -p omnilang -- codegen
```

- [ ] **Step 4: 提交**

```bash
git add omnilang/src/codegen/omniview/timeline.rs omnilang/src/codegen/react/motion.rs
git commit -m "fix: surface animation parameter defaults as warnings

- Replace silent .unwrap_or() defaults with diagnostic warnings
- Log discarded spring/elastic animation parameters
- Addresses M22, L4 from code review"
```

---

### Task 2.2: 修复 walkdir 和语义分析中的静默错误

**Files:**
- Modify: `poiesis/src-tauri/src/lib.rs:144,194` (walkdir filter_map)
- Modify: `omnilang/src/semantic/resolve.rs:140,398,435` (declare 结果)

- [ ] **Step 1: walkdir — 记录跳过的文件**

```rust
// 第 144, 194 行:
// 旧:
.filter_map(|e| e.ok())
// 新:
.filter_map(|e| match e {
    Ok(entry) => Some(entry),
    Err(err) => {
        eprintln!("[omni] warning: skipping inaccessible entry: {}", err);
        None
    }
})
```

- [ ] **Step 2: resolve.rs — 检查 declare 返回值**

```rust
// 第 140, 398, 435 行:
// 旧:
let _ = self.symbols.declare(Symbol { ... });
// 新:
if let Err(conflict) = self.symbols.declare(Symbol { ... }) {
    self.diagnostics.push(Diagnostic::warning(
        format!("Symbol '{}' redeclared (shadowing '{}')", name, conflict.name),
        span,
    ));
}
```

- [ ] **Step 3: 运行测试确认无回归**

```bash
cargo test --workspace
```

- [ ] **Step 4: 提交**

```bash
git add poiesis/src-tauri/src/lib.rs omnilang/src/semantic/resolve.rs
git commit -m "fix: log walkdir errors, warn on symbol redeclaration

- walkdir: log skipped entries instead of silently ignoring errors
- semantic/resolve: emit warning on symbol redeclaration (shadowing)
- Addresses M16, M21 from code review"
```

---

### Task 2.3: 修复 ffi.rs 非 UTF-8 输入静默回退

**Files:**
- Modify: `omnilang/src/ffi.rs:71,215,226,236,285`

- [ ] **Step 1: 替换 unwrap_or 为错误返回**

```rust
// 第 71 行:
// 旧: std::str::from_utf8(slice).unwrap_or("{}")
// 新: 返回包含 "Invalid UTF-8 input" 的错误 JSON
fn safe_str_or_error<'a>(slice: &'a [u8], fallback: &'a str) -> Result<&'a str, String> {
    std::str::from_utf8(slice).map_err(|e| format!("Invalid UTF-8 input: {}", e))
}

// 调用点更新:
let source = match safe_str_or_error(source_slice, "{}") {
    Ok(s) => s,
    Err(e) => return CString::new(format!(r#"{{"success":false,"errors":["{}"]}}"#, e)).unwrap().into_raw(),
};
```

- [ ] **Step 2: 同样处理 path_mapping 解析失败（ffi.rs:271）**

```rust
// 旧: .unwrap_or_else(|e| { eprintln!(...); HashMap::new() })
// 新: 在返回的 JSON 中包含诊断警告
```

- [ ] **Step 3: 运行 FFI 测试**

```bash
cargo test -p omnilang -- ffi
```

- [ ] **Step 4: 提交**

```bash
git add omnilang/src/ffi.rs
git commit -m "fix: return error on invalid UTF-8 FFI input instead of silent fallback

- ffi.rs: replace unwrap_or({}) with error return for invalid UTF-8
- path_mapping parse failure now included in response diagnostics
- Addresses M20 from code review"
```

---

### Task 2.4: 修复 TypeScript 数据完整性 — readPidMap 和 lineToPid

**Files:**
- Modify: `poiesis/src/commands/annotation-api.ts:145`
- Modify: `poiesis/src/services/codegen/omni-parser.ts:56-58`
- Modify: `poiesis/src/store/index.ts:505-512`

- [ ] **Step 1: readPidMap — 区分"文件不存在"和"读取失败"**

```typescript
// 旧: catch { return { entries: {}, lineToPid: {} }; }
// 新: return { entries: {}, lineToPid: {}, error: String(e) };
// 调用者检查 error 字段决定是否触发自动重新生成
```

- [ ] **Step 2: lineToPid — 添加重复行检测**

```typescript
// 在 omni-parser.ts parsePidAnnotations 中:
for (let line = entry.startLine; line <= entry.endLine; line++) {
  if (pidMap.lineToPid[line] && pidMap.lineToPid[line] !== entry.pid) {
    console.warn(`[parser] Line ${line} has conflicting PID annotations: ${pidMap.lineToPid[line]} vs ${entry.pid}`);
  }
  pidMap.lineToPid[line] = entry.pid;
}
```

- [ ] **Step 3: removeComponent — 使用 set() 内部获取新鲜状态**

```typescript
// store/index.ts removeComponent:
// 修改为在 set() 回调内获取 tabs，避免陈旧闭包
removeComponent: (name) => set((s) => {
  const { [name]: _, ...rest } = s.componentDefs;
  const tabs = s.designPage.openTabs;
  const tab = tabs.find((t) => t.componentName === name);
  if (tab) {
    // closeTab 逻辑内联以确保一致性
    // ...
  }
  return { componentDefs: rest };
}),
```

- [ ] **Step 4: 类型检查**

```bash
cd poiesis && npx tsc --noEmit
```

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/commands/annotation-api.ts poiesis/src/services/codegen/omni-parser.ts poiesis/src/store/index.ts
git commit -m "fix: improve data integrity — PID collision detection, stale state

- readPidMap: return error field so callers distinguish file-not-found vs read-error
- omni-parser: warn on conflicting PID annotations on same line
- removeComponent: use set() callback to avoid stale closure
- Addresses M17, M18, M19 from code review"
```

---

## Phase 3: 架构优化 🟠（5 天）

### Task 3.1: 拆分 lower_ast.rs (2654 行)

**Files:**
- Create: `omnilang/src/ir/lower/mod.rs`
- Create: `omnilang/src/ir/lower/component.rs`
- Create: `omnilang/src/ir/lower/view.rs`
- Create: `omnilang/src/ir/lower/expr.rs`
- Create: `omnilang/src/ir/lower/stmt.rs`
- Create: `omnilang/src/ir/lower/motion.rs`
- Modify: `omnilang/src/ir/mod.rs` (添加 `pub mod lower;`)
- Remove: `omnilang/src/ir/lower_ast.rs` (分解后删除)

- [ ] **Step 1: 创建 lower/mod.rs — 入口点**

```rust
// omnilang/src/ir/lower/mod.rs
mod component;
mod view;
mod expr;
mod stmt;
mod motion;

pub(crate) use component::*;
pub(crate) use view::*;
pub(crate) use expr::*;
pub(crate) use stmt::*;
pub(crate) use motion::*;

// Re-export lower function
pub use super::lower_ast::lower;  // 临时 — 逐步迁移
```

- [ ] **Step 2: 按类别提取函数到对应文件**

| 目标文件 | 提取函数 | 估计行数 |
|----------|---------|---------|
| `lower/component.rs` | lower_component, lower_store, lower_component_state | ~500 行 |
| `lower/view.rs` | lower_view_block, lower_view_node, lower_view_tree | ~570 行 |
| `lower/expr.rs` | lower_expr, lower_literal, lower_binary, lower_call, 所有表达式相关 | ~300 行 |
| `lower/stmt.rs` | lower_stmt, lower_block, lower_if, lower_for, lower_return | ~300 行 |
| `lower/motion.rs` | lower_motion_def, lower_timeline_def, lower_animation_config | ~300 行 |

- [ ] **Step 3: 更新 ir/mod.rs**

```rust
// 添加:
pub mod lower;
// 保留旧模块入口直到测试通过:
mod lower_ast;
pub use lower_ast::lower;
```

- [ ] **Step 4: 运行全量测试确认无回归**

```bash
cargo test --workspace
```
Expected: 384 测试全部通过。

- [ ] **Step 5: Phase 3 完成后最终切换到新模块**

```rust
// ir/mod.rs:
// 删除 mod lower_ast; 行
// 更新 pub use lower::lower;
```

- [ ] **Step 6: 提交（分多个 commit，每个子模块一个）**

```bash
git add omnilang/src/ir/lower/ omnilang/src/ir/mod.rs
git commit -m "refactor: split lower_ast.rs (2654 lines) into sub-modules

- Extract lower/component.rs, view.rs, expr.rs, stmt.rs, motion.rs
- Each sub-module <600 lines
- All 384 tests pass
- Addresses H1 from code review"
```

---

### Task 3.2: 提取共享 build_path_mapping

**Files:**
- Create: `omnilang/src/path_mapping.rs`
- Modify: `omnilang/src/lib.rs` (pub mod path_mapping)
- Modify: `omnic/src/main.rs:122-173` (使用 omni crate 函数)
- Modify: `poiesis/src-tauri/src/lib.rs:367-420` (使用 omni crate 函数)

- [ ] **Step 1: 创建 omnilang/src/path_mapping.rs**

```rust
//! Path mapping for cross-file import resolution.
//! Shared between the omnic CLI, Poiesis Tauri backend, and WASM API.
use std::collections::HashMap;
use std::path::Path;

/// Derive import path mapping from source file paths.
pub fn derive_path_mapping(sources: &HashMap<String, String>) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    for filepath in sources.keys() {
        mapping.insert(filepath.clone(), filepath.clone());
        if let Some(basename) = Path::new(filepath).file_name() {
            let basename_str = basename.to_string_lossy().to_string();
            mapping.entry(basename_str.clone()).or_insert_with(|| filepath.clone());
            if let Some(stem) = Path::new(&basename_str).file_stem() {
                let stem_str = stem.to_string_lossy().to_string();
                mapping.entry(stem_str).or_insert_with(|| filepath.clone());
            }
        }
    }
    let paths: Vec<&String> = sources.keys().collect();
    for import_path in &paths {
        let importer_dir = Path::new(import_path).parent().unwrap_or(Path::new("."));
        for target_path in &paths {
            if *import_path == *target_path { continue; }
            if let Some(rel_simple) = pathdiff::diff_paths(Path::new(target_path), importer_dir) {
                let rel_path = rel_simple.to_string_lossy().replace('\\', "/");
                mapping.insert(format!("./{rel_path}"), (*target_path).clone());
                mapping.insert(rel_path.clone(), (*target_path).clone());
                if rel_path.ends_with(".omni") {
                    let no_ext = &rel_path[..rel_path.len() - 5];
                    mapping.insert(format!("./{no_ext}"), (*target_path).clone());
                    mapping.insert(no_ext.to_string(), (*target_path).clone());
                }
            }
        }
    }
    mapping
}
```

- [ ] **Step 2: 更新 lib.rs 添加模块导出**

```rust
// omnilang/src/lib.rs:
pub mod path_mapping;
```

- [ ] **Step 3: 更新 omnic/src/main.rs 和 poiesis/src-tauri/src/lib.rs**

```rust
// 旧: derive_path_mapping(&sources) (本文件定义)
// 新: omnilang::path_mapping::derive_path_mapping(&sources)
```

删除旧实现函数。

- [ ] **Step 4: 运行全量测试**

```bash
cargo test --workspace
```

- [ ] **Step 5: 提交**

```bash
git add omnilang/src/path_mapping.rs omnilang/src/lib.rs omnic/src/main.rs poiesis/src-tauri/src/lib.rs
git commit -m "refactor: extract shared path_mapping into omnilang crate

- New omnilang::path_mapping::derive_path_mapping()
- Deduplicate from omnic/main.rs and poiesis lib.rs
- Addresses H3 from code review"
```

---

### Task 3.3: 统一分析管道（消除 H4 重复）

**Files:**
- Create: `omnilang/src/analyse.rs`
- Modify: `omnilang/src/lib.rs`
- Modify: `omnilang/src/ffi.rs:99-173`
- Modify: `omnic/src/api.rs:105-160`

- [ ] **Step 1: 创建 omnilang/src/analyse.rs — 共享分析管道**

```rust
//! Shared multi-file analysis pipeline.
use std::collections::HashMap;
use crate::error::Diagnostic;
use crate::parser;
use crate::semantic;
use crate::ir;

/// Result of multi-file analysis.
pub struct AnalyseOutput {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub ir: Option<ir::IRProgram>,
    pub warnings: Vec<Diagnostic>,
}

/// Parse + semantic analysis + lint for multiple source files.
pub fn analyse_multi(
    sources: &HashMap<String, String>,
    path_mapping: &HashMap<String, String>,
) -> AnalyseOutput {
    // Phase 1: Parse all sources
    let mut diags = Vec::new();
    let mut programs = HashMap::new();
    for (path, source) in sources {
        let tokens = crate::lexer::tokenize(source);
        match parser::parse(&tokens, source) {
            Ok(p) => { programs.insert(path.clone(), p); }
            Err(e) => { diags.push(Diagnostic::from(e).with_file(path)); }
        }
    }
    if !diags.is_empty() {
        return AnalyseOutput { success: false, diagnostics: diags, ir: None, warnings: vec![] };
    }
    
    // Phase 2: Cross-file validation
    let vr = crate::validate::validate_multi_ref(&programs, path_mapping);
    diags.extend(vr.errors);
    
    // Phase 3: Per-file semantic + IR lowering
    let mut merged_ir = ir::IRProgram { version: "1.0".into(), modules: HashMap::new(), source_map: HashMap::new(), stdlib_imports: vec![] };
    for (path, program) in &programs {
        let sem = semantic::analyze(program);
        for mut d in sem.diagnostics {
            if d.file.is_none() { d.file = Some(path.clone()); }
            diags.push(d);
        }
        let ir_prog = ir::lower(program, &sem);
        // merge modules...
    }
    
    // Phase 4: Lint
    let lint_diags = crate::semantic::lints::run_all(&programs, &merged_ir);
    let warnings: Vec<Diagnostic> = lint_diags.iter().filter(|d| d.severity != crate::semantic::lints::LintSeverity::Error).map(...).collect();
    
    let success = !diags.iter().any(|d| d.severity == crate::error::Severity::Error);
    AnalyseOutput { success, diagnostics: diags, ir: Some(merged_ir), warnings }
}
```

- [ ] **Step 2: 更新 lib.rs 的 compile_multi_to_target 使用 analyse_multi**

```rust
// compile_multi_to_target 中 Phase 1-4 替换为:
let analysis = crate::analyse::analyse_multi(&sources, &path_mapping);
// 然后仅执行 Phase 5 (codegen)
```

- [ ] **Step 3: 更新 ffi.rs 和 api.rs 使用同一个函数**

- [ ] **Step 4: 运行全量测试**

```bash
cargo test --workspace
```

- [ ] **Step 5: 提交**

```bash
git add omnilang/src/analyse.rs omnilang/src/lib.rs omnilang/src/ffi.rs omnic/src/api.rs
git commit -m "refactor: unify analysis pipeline into analyse::analyse_multi()

- Extract shared parse→semantic→lint pipeline
- ffi.rs, api.rs, lib.rs now use single implementation
- Eliminates ~90 lines of duplicate code
- Addresses H4 from code review"
```

---

### Task 3.4: 优化 compile_multi_to_target 避免重复解析 (H5)

**Files:**
- Modify: `omnilang/src/validate.rs`

- [ ] **Step 1: 添加 validate_multi_ref 函数**

```rust
/// Cross-file validation using already-parsed programs.
pub fn validate_multi_ref(
    programs: &HashMap<String, crate::parser::ast::Program>,
    path_mapping: &HashMap<String, String>,
) -> ValidationResult {
    // 与 validate_multi 相同，但不重新解析
    // 构建 CrossFileIndex 时直接从 programs 提取符号
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p omnilang
```

- [ ] **Step 3: 提交**

```bash
git add omnilang/src/validate.rs
git commit -m "perf: avoid double-parsing in compile_multi_to_target

- Add validate_multi_ref() accepting pre-parsed programs
- Eliminates second parse pass in multi-file compilation
- Addresses H5 from code review"
```

---

### Task 3.5: 拆分 Poiesis 超大文件 (H8)

由于 Poiesis 拆分需保持功能完整，分两个子任务推进。

**Task 3.5a: 拆分 DesignTabBar.tsx (755 行)**

- Create: `poiesis/src/components/design/FileTreePanel.tsx` (~200 行)
- Create: `poiesis/src/components/design/NewFileModal.tsx` (~100 行)
- Modify: `poiesis/src/components/design/DesignTabBar.tsx` (缩减到 ~300 行)

**Task 3.5b: 拆分 FlowCanvas.tsx (662 行)**

- Create: `poiesis/src/components/canvas/code/CanvasContextMenu.tsx` (~100 行)
- Create: `poiesis/src/components/canvas/code/flowConstants.ts` (~50 行)
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx` (缩减到 ~350 行)

**Task 3.5c: 拆分 DesignPage.tsx (478 行)**

- Create: `poiesis/src/hooks/useSaveProject.ts` (~80 行)
- Create: `poiesis/src/hooks/useDragHandlers.ts` (~80 行)
- Modify: `poiesis/src/pages/DesignPage.tsx` (缩减到 ~250 行)

**Task 3.5d: 拆分 pipeline.ts (766 行)**

- Create: `poiesis/src/services/codegen/component-builder.ts` (~250 行)
- Create: `poiesis/src/services/codegen/style-parser.ts` (~100 行)
- Modify: `poiesis/src/services/codegen/pipeline.ts` (缩减到 ~350 行)

- [ ] **每步后运行 TypeScript 类型检查: `cd poiesis && npx tsc --noEmit`**
- [ ] **每步后验证构建: `cd poiesis && npm run build`**
- [ ] **每个子任务独立提交**

---

## Phase 4: 质量提升 🟡/🟢（5 天）

### Task 4.1: TypeScript 类型修复（@ts-nocheck 迁移，优先级排序）

**Files (按优先级):**
1. `poiesis/src/driver/store.ts` — 移除 `as any`，添加正确类型
2. `poiesis/src/services/codegraph/store.ts` — 消除 `usePoiesisStore as any`
3. `poiesis/src/store/componentRegistry.ts` — 移除 `as any`
4. `poiesis/src/core/state/styleRegistry.ts` — 修复 `(set as any)`
5. `poiesis/src/driver/OmniDriver.ts` — 逐步移除 `@ts-nocheck`
6. 其余 `@ts-nocheck` 文件

- [ ] **Step 1: 创建类型化的 store 访问器**

```typescript
// poiesis/src/driver/store.ts 重构:
import { usePoiesisStore } from "../../store";
import type { ElementData, WireData } from "../../core/types";

// 类型化的 active store 接口
interface ActiveDesignStore {
  get elements(): ElementData[];
  get wires(): WireData[];
  addElement(el: ElementData): void;
  removeElement(id: string): void;
  // ... 所有使用的方法
}

// 直接从 zustand store 构建，不做 as any
export function getActiveStore(): ActiveDesignStore {
  const state = usePoiesisStore.getState();
  return {
    get elements() { return state.elements; },
    get wires() { return state.wires; },
    addElement: (el) => state.addElement(el),
    removeElement: (id) => state.removeElement(id),
    // ...
  };
}
```

- [ ] **Step 2-6: 逐文件迁移，每次运行 tsc 确认零错误**

- [ ] **提交（每文件一个 commit）**

---

### Task 4.2: 补充测试覆盖

**Sub-tasks:**
- **4.2a**: `semantic::analyze()` 单元测试  → `omnilang/src/semantic/mod.rs` 的 `#[cfg(test)]` 模块
- **4.2b**: `lower_ast.rs` / `ir::lower::*` 单元测试
- **4.2c**: `validate.rs` 多文件验证测试
- **4.2d**: `omnic/src/` CLI 测试（derive_path_mapping, extract_error_messages, cmd_*）
- **4.2e**: 端到端编译管道测试（parse → semantic → IR → codegen → 验证输出）

- [ ] **每个子任务必须包含具体测试用例和实施代码**
- [ ] **每个子任务后运行 `cargo test --workspace` 并确认 80%+ 覆盖率**
- [ ] **每个子任务独立提交**

---

### Task 4.3: Clippy 清理 (231 条建议)

**策略**: 分 3 批处理，从高影响低风险开始。

**Batch 1** (~90 条，1 天): 机械性修复
- 冗余闭包: `.map(|x| f(x))` → `.map(f)`
- 无用 format!: `format!("{}", x)` → `x.to_string()`
- 迭代器: `for (_k, v) in &map` → `for v in map.values()`

**Batch 2** (~50 条，0.5 天): 可简化 map_or
- `.map_or(false, |e| e == x)` → `.is_some_and(|e| e == x)`

**Batch 3** (~90 条，0.5 天): 嵌套 if-let 合并 + 其余

- [ ] **每个 Batch 运行 `cargo clippy --workspace -- -D warnings` 确认通过**
- [ ] **每个 Batch 独立提交**

---

### Task 4.4: 低优先级清理

**Sub-tasks:**
- **4.4a**: 添加 `.gitignore` 条目 (`*.log`, `.env`, `*.pdb`)
- **4.4b**: 添加 `#[must_use]` 到 `compile()`, `compile_omniview()`, `sanitize_css_value()`
- **4.4c**: 移除 `omnic/Cargo.toml` 中未使用依赖 `open`, `toml`
- **4.4d**: 为 `Codegen` trait 添加 `Send + Sync` 约束
- **4.4e**: 修复 README 中过时的目录结构描述
- **4.4f**: 清理过时注释 (`// editor/client/src/...` → `// poiesis/src/...`)

- [ ] **每个子任务独立提交**

---

### Task 4.5: CI 集成 cargo-audit 和 cargo-deny

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: 添加 cargo-audit job**

```yaml
audit:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions-rust-lang/setup-rust-toolchain@v1
    - run: cargo install cargo-audit
    - run: cargo audit
```

- [ ] **Step 2: 添加 clippy 严格模式 job**

```yaml
clippy:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions-rust-lang/setup-rust-toolchain@v1
    - run: cargo clippy --workspace -- -D warnings
```

- [ ] **Step 3: 提交**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add cargo-audit and strict clippy checks"
```

---

## 执行顺序与依赖

```
Phase 1 (安全): Task 1.1 ──→ Task 1.2 ──→ [1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9 可并行]
                      │
Phase 2 (错误): Task 2.1, 2.2, 2.3, 2.4 可并行
                      │
Phase 3 (架构): Task 3.1 → Task 3.2 → Task 3.3 → Task 3.4 (顺序依赖)
                Task 3.5 可并行
                      │
Phase 4 (质量): Task 4.1, 4.2, 4.3, 4.4, 4.5 可并行
```

## 验收标准

每个 Phase 完成后验证：

| Phase | 验收标准 |
|-------|---------|
| Phase 1 | `csp != null`; `fs:scope != "**"`; 无生产代码 unwrap(); 空 catch 被替换 |
| Phase 2 | `cargo clippy` 无静默失败警告; TS 无空 catch |
| Phase 3 | `lower_ast.rs` 已删除; path_mapping 单一实现; 超大文件 <800 行 |
| Phase 4 | `@ts-nocheck` 文件数减半; 测试覆盖 +5%; clippy 0 错误; CI 含 audit |

**全局**: `cargo test --workspace` 全部通过; `cd poiesis && npx tsc --noEmit` 零错误。

---

## 估算工时

| Phase | 任务数 | 估算 |
|-------|--------|------|
| Phase 1 | 9 | 5 天 |
| Phase 2 | 4 | 4 天 |
| Phase 3 | 5 | 5 天 |
| Phase 4 | 5 | 5 天 |
| **总计** | **23** | **19 天** |
