# OmniLang Rust Workspace 重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 OmniLang 项目重构为 Rust workspace（omnilang compiler + Poiesis Tauri 编辑器），一步到位清理所有废弃代码和分支。

**Architecture:** 大爆炸式重构 — 先清理后创建。omnilang crate（cdylib + rlib + wasm）作为编译器核心，Poiesis（Tauri 2.x + React/Vite）作为桌面编辑器。Python CLI（omnic）通过 FFI 调用 omnilang DLL。

**Tech Stack:** Rust 2021 edition, Tauri 2.x, React 19 + Vite 8 + TypeScript 6, Python 3.11+, Cargo workspace

## Global Constraints

- omnilang crate: cdylib + rlib + wasm（feature-gated）
- Poiesis: Tauri 2.x，前端 React/Vite/TS，后端 Rust 轻量（Tauri plugins + 核心 commands）
- Python CLI: pip install -e omnic/，FFI 加载 omnilang.dll
- 所有现有测试删除，重构后重写
- 删除所有 worktree 分支，仅保留 main
- CI 支持 Windows/macOS/Linux + WASM + Python + 前端四线

---

## 前置：备份当前状态

### Task 0: Git 状态确认与备份

**Files:**
- 无文件变更

- [ ] **Step 1: 查看当前 git 状态**

```bash
git status
```

- [ ] **Step 2: 创建备份 tag**

```bash
git tag pre-workspace-refactor
```

- [ ] **Step 3: 记录当前分支结构**

```bash
git branch -a > /tmp/branches-before-refactor.txt
git worktree list > /tmp/worktrees-before-refactor.txt
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: snapshot before workspace refactoring"
```

---

## Phase 1: Git 分支与 Worktree 清理

### Task 1: 删除 Worktree 分支

**Files:**
- 删除 `.claude/worktrees/editor-debt-cleanup/`
- 删除 `.claude/worktrees/fix-store-getter-scoping/`
- 删除 `.claude/worktrees/learn-omniview-phase-1/`
- 删除 `.claude/worktrees/omnic-cli-redesign/`

- [ ] **Step 1: 删除 worktree 目录并移除分支**

```bash
# 逐一删除 worktree
git worktree remove .claude/worktrees/editor-debt-cleanup --force 2>/dev/null || true
git worktree remove .claude/worktrees/fix-store-getter-scoping --force 2>/dev/null || true
git worktree remove .claude/worktrees/learn-omniview-phase-1 --force 2>/dev/null || true
git worktree remove .claude/worktrees/omnic-cli-redesign --force 2>/dev/null || true
```

- [ ] **Step 2: 清理残留 worktree 目录**

```bash
rm -rf .claude/worktrees/editor-debt-cleanup 2>/dev/null || true
rm -rf .claude/worktrees/fix-store-getter-scoping 2>/dev/null || true
rm -rf .claude/worktrees/learn-omniview-phase-1 2>/dev/null || true
rm -rf .claude/worktrees/omnic-cli-redesign 2>/dev/null || true
```

- [ ] **Step 3: 删除非 worktree 的本地分支**

```bash
# 切换回 main（确保在 main 上）
git checkout main

# 逐分支删除
git branch -D feat/omniview-motion-phase1 2>/dev/null || true
git branch -D worktree-editor-architecture-refactor 2>/dev/null || true
git branch -D worktree-editor-debt-cleanup 2>/dev/null || true
git branch -D worktree-feat-editor-omni-format-redesign 2>/dev/null || true
git branch -D worktree-fix-store-getter-scoping 2>/dev/null || true
git branch -D worktree-learn-omniview-phase-1 2>/dev/null || true
git branch -D worktree-omnic-cli-redesign 2>/dev/null || true
```

- [ ] **Step 4: 验证结果**

```bash
# 应只显示 main 和可能的远程跟踪分支
git branch -a
# 应只显示 main 的 worktree
git worktree list
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: remove all worktree branches, keep only main"
```

---

## Phase 2: 根目录清理

### Task 2: 删除根目录杂物文件

- [ ] **Step 1: 删除 screenshots（PNG）**

```bash
rm -f ./*.png
```

- [ ] **Step 2: 删除 snapshots 和临时文本文件**

```bash
# Markdown snapshots
rm -f ./*-initial-page
rm -f ./*after-*.md
rm -f ./*snapshot*.md
rm -f ./*snapshot*.txt
rm -f ./*snapshot*.yml
rm -f ./*-page-snapshot*.txt
rm -f ./*step-*.txt
rm -f ./canvas-state
rm -f ./component-tree
rm -f ./editor-*
rm -f ./welcome-screen
rm -f ./welcome-screen-2
rm -f ./button_props.md
rm -f ./editor_state.md
rm -f ./property_panel.md
rm -f ./editor-snapshot.md
rm -f ./initial-snapshot.md
rm -f ./learn-omniview-snapshot.md
rm -f ./playwright-snapshot.md
rm -f ./test-page-snapshot.md
rm -f ./todo-section.yml
rm -f ./learn-omniview-initial.yml
rm -f ./snapshot-initial.yml
rm -f ./snapshot-motion-tab.txt
rm -f ./page-snapshot-initial.txt
rm -f ./latest-errors.txt
rm -f ./console-errors-full.txt
rm -f ./console-after-submit.txt
rm -f ./console-after-label-change.log
rm -f ./hmr-log.txt
```

- [ ] **Step 3: 删除临时脚本和 JS 文件**

```bash
rm -f ./check_bal.js
rm -f ./check_pos.js
rm -f ./check_syntax.js
rm -f ./extract_js.js
rm -f ./generated.js
```

- [ ] **Step 4: 删除废弃配置和临时目录**

```bash
rm -f ./.flutter-skill.yaml
rm -f ./package.json
rm -f ./package-lock.json
rm -f ./pyproject.toml
rm -rf ./.tours/
rm -rf ./.superpowers/
```

- [ ] **Step 5: 删除临时文本文件（GitHub/Gitee 占位）**

```bash
rm -f ./GitHub
rm -f ./Gitee
```

- [ ] **Step 6: 删除根目录遗留的 playwright snapshot**

```bash
rm -rf ./.playwright-mcp/
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: remove root-level clutter files (screenshots, snapshots, temp scripts)"
```

### Task 3: 删除废弃目录

- [ ] **Step 1: 删除编译产物和构建目录**

```bash
rm -rf ./build/
rm -rf ./build_output/
rm -rf ./node_modules/
```

- [ ] **Step 2: 删除废弃应用目录**

```bash
rm -rf ./vscode-extension/
rm -rf ./dashboard/
rm -rf ./tools/
```

- [ ] **Step 3: 删除测试目录**

```bash
rm -rf ./test/
rm -rf ./test-proj/
rm -rf ./tests/
rm -rf ./.e2e-test/
```

- [ ] **Step 4: 删除 Python/Rust 缓存**

```bash
rm -rf ./.pytest_cache/
rm -rf ./.ruff_cache/
rm -rf ./omnilang.egg-info/
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: remove deprecated directories (vscode-extension, dashboard, tools, tests)"
```

> ⚠️ `editor/` 暂不删除 — Phase 7 需要从中拷贝前端代码。迁移完成后在 Task 8 末尾删除。

---

## Phase 3: Workspace 骨架

### Task 4: 创建 Rust Workspace

**Files:**
- Create: `Cargo.toml`（workspace root）
- Modify: `.gitignore`

- [ ] **Step 1: 创建 workspace 根 Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "omnilang",
    "poiesis/src-tauri",
]
```

- [ ] **Step 2: 更新 .gitignore**

当前 `.gitignore` 内容：
```gitignore
.playwright-mcp/
build_output/
docs/screenshots/
__pycache__/
```

合并为（删除旧的 build_output/，加上新规则）：
```gitignore
# Rust
target/
Cargo.lock

# Node / Tauri
poiesis/node_modules/
poiesis/dist/

# Python
__pycache__/
omnic/*.egg-info/

# WASM output
poiesis/public/wasm/

# IDE
.vscode/
.idea/

# Docs
docs/screenshots/
```

> 删除了 `.playwright-mcp/` 和 `build_output/`（这些目录本身会被删除）。

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml .gitignore
git commit -m "feat: create Rust workspace skeleton with .gitignore"
```

---

## Phase 4: omnic Python CLI（先迁移，释放 omnilang/ 目录名给 Rust crate）

### Task 5: 迁移 Python CLI 到 omnic/

**Files:**
- Create: `omnic/pyproject.toml`
- Move: `omnilang/*.py` → `omnic/omnilang/`
- Modify: `omnic/omnilang/rust_bridge.py`（DLL 名变更）
- 删除: `omnilang/lsp/`（Python LSP），然后删除整个旧 `omnilang/` 目录（释放目录名）

- [ ] **Step 1: 创建 omnic 目录结构**

```bash
mkdir -p omnic/omnilang
```

- [ ] **Step 2: 迁移 Python 源文件（排除 lsp 和编译产物）**

```bash
cp omnilang/__init__.py omnic/omnilang/
cp omnilang/api.py omnic/omnilang/
cp omnilang/cli.py omnic/omnilang/
cp omnilang/config.py omnic/omnilang/
cp omnilang/errors.py omnic/omnilang/
cp omnilang/result.py omnic/omnilang/
cp omnilang/rust_bridge.py omnic/omnilang/
cp omnilang/sourcemap.py omnic/omnilang/
```

- [ ] **Step 3: 更新 rust_bridge.py 中的 DLL 名称**

在 `omnic/omnilang/rust_bridge.py` 中，将所有 `omni_compiler` 替换为 `omnilang`（DLL 名称随 crate 名变更）：

```python
# 替换后代码：
# _LIB = ctypes.cdll.LoadLibrary("omnilang.dll")      # Windows
# _LIB = ctypes.cdll.LoadLibrary("libomnilang.so")     # Linux
# _LIB = ctypes.cdll.LoadLibrary("libomnilang.dylib")  # macOS
```

- [ ] **Step 4: 创建 omnic/pyproject.toml**

```toml
[project]
name = "omnic"
version = "0.2.0"
description = "OmniLang CLI — compile once, run everywhere"
requires-python = ">=3.10"
dependencies = [
    "lark>=1.1",
    "tomli>=2.0; python_version < '3.11'",
]

[project.optional-dependencies]
dev = [
    "pytest>=7",
]

[project.scripts]
omnic = "omnilang.cli:main"

[tool.setuptools.packages.find]
include = ["omnilang*"]

[build-system]
requires = ["setuptools>=64"]
build-backend = "setuptools.build_meta"
```

- [ ] **Step 5: 删除旧的 Python omnilang 目录（释放目录名，之后给 Rust crate 用）**

```bash
rm -rf omnilang/
```

- [ ] **Step 6: 验证 pip 安装**

```bash
pip install -e omnic/
python -c "from omnilang import cli; print('omnic import OK')"
```

预期：import 成功（DLL 可能不在路径中导致功能不可用，但包结构正确）。

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: migrate Python CLI to omnic/ package"
```

---

## Phase 5: omnilang crate（编译器）

### Task 6: 迁移 Rust compiler 到 omnilang/

**Files:**
- Create: `omnilang/Cargo.toml`
- Move: `omni-compiler/src/` → `omnilang/src/`
- Delete: `omni-compiler/`

- [ ] **Step 1: 创建 omnilang Rust crate 目录并迁移源码**

> 此时旧的 Python `omnilang/` 已被 Task 5 删除，目录名可用。

```bash
mkdir -p omnilang
cp -r omni-compiler/src omnilang/src
```

- [ ] **Step 2: 创建 omnilang/Cargo.toml（更新包名和版本）**

```toml
[package]
name = "omnilang"
version = "0.2.0"
edition = "2021"
description = "OmniLang compiler — .omni -> Intent IR -> target code"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
thiserror = "1"

[features]
default = []
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]

[package.metadata.wasm-pack.profile.release]
wasm-opt = false

[profile.release]
lto = true
opt-level = "s"
strip = true
```

- [ ] **Step 3: 删除旧 omni-compiler 目录**

```bash
rm -rf omni-compiler/
```

- [ ] **Step 4: 验证编译**

```bash
cargo build -p omnilang
```

预期：编译成功（仅编译 omnilang 包）。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: migrate omni-compiler -> omnilang crate (cdylib + rlib)"
```

---

## Phase 6: Poiesis Tauri 应用

### Task 7: 创建 Poiesis Tauri 2.x 应用骨架

**Files:**
- Create: `poiesis/package.json`
- Create: `poiesis/vite.config.ts`
- Create: `poiesis/tsconfig.json`
- Create: `poiesis/tsconfig.node.json`
- Create: `poiesis/index.html`
- Create: `poiesis/src-tauri/Cargo.toml`
- Create: `poiesis/src-tauri/tauri.conf.json`
- Create: `poiesis/src-tauri/build.rs`
- Create: `poiesis/src-tauri/src/main.rs`
- Create: `poiesis/src-tauri/src/lib.rs`
- Create: `poiesis/src-tauri/icons/`（占位）

- [ ] **Step 1: 创建 poiesis 目录结构**

```bash
mkdir -p poiesis/src-tauri/src
mkdir -p poiesis/src-tauri/icons
mkdir -p poiesis/public
mkdir -p poiesis/src
```

- [ ] **Step 2: 创建 poiesis/package.json**

```json
{
  "name": "poiesis",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "lint": "eslint ."
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-fs": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "@tauri-apps/plugin-store": "^2",
    "@dnd-kit/core": "^6.3.1",
    "@dnd-kit/sortable": "^10.0.0",
    "@dnd-kit/utilities": "^3.2.2",
    "@monaco-editor/react": "^4.7.0",
    "@xyflow/react": "^12.11.0",
    "dagre": "^0.8.5",
    "idb": "^8.0.3",
    "mitt": "^3.0.1",
    "monaco-editor": "^0.55.1",
    "pako": "^2.1.0",
    "react": "^19.2.6",
    "react-dom": "^19.2.6",
    "sonner": "^2.0.7",
    "zustand": "^5.0.14"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@eslint/js": "^10.0.1",
    "@testing-library/react": "^16.3.2",
    "@types/node": "^24.12.3",
    "@types/pako": "^2.0.4",
    "@types/react": "^19.2.14",
    "@types/react-dom": "^19.2.3",
    "@vitejs/plugin-react": "^6.0.1",
    "eslint": "^10.3.0",
    "eslint-plugin-react-hooks": "^7.1.1",
    "eslint-plugin-react-refresh": "^0.5.2",
    "globals": "^17.6.0",
    "jsdom": "^29.1.1",
    "typescript": "~6.0.2",
    "typescript-eslint": "^8.59.2",
    "vite": "^8.0.12",
    "vitest": "^4.1.8"
  }
}
```

- [ ] **Step 3: 创建 poiesis/vite.config.ts**

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
```

- [ ] **Step 4: 创建 poiesis/tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedSideEffectImports": true
  },
  "include": ["src"]
}
```

- [ ] **Step 5: 创建 poiesis/index.html**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Poiesis</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 6: 创建 poiesis/src-tauri/Cargo.toml**

```toml
[package]
name = "poiesis"
version = "0.1.0"
edition = "2021"
description = "Poiesis — OmniLang visual editor"

[lib]
name = "poiesis_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"
tauri-plugin-shell = "2"
tauri-plugin-window-state = "2"
tauri-plugin-store = "2"
omnilang = { path = "../../omnilang" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 7: 创建 poiesis/src-tauri/build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 8: 创建 poiesis/src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/nicegui-org/nicegui/main/nicegui/static/tauri/config2.schema.json",
  "productName": "Poiesis",
  "version": "0.1.0",
  "identifier": "com.omnilang.poiesis",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "Poiesis",
        "width": 1400,
        "height": 900
      }
    ],
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "fs": {
      "scope": ["**"]
    },
    "shell": {
      "open": true
    }
  }
}
```

- [ ] **Step 9: 创建 poiesis/src-tauri/src/lib.rs（Tauri commands）**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<String>,
}

#[tauri::command]
fn compile_project(project_path: String, target: String) -> CompileResult {
    // TODO: 调用 omnilang crate 编译
    // 后续开发阶段实现具体逻辑
    CompileResult {
        success: true,
        output: format!("compiled {} -> {}", project_path, target),
        errors: vec![],
    }
}

#[tauri::command]
fn project_export(project_path: String, output_dir: String) -> String {
    format!("export {} -> {}", project_path, output_dir)
}

#[tauri::command]
fn file_watch(path: String) -> String {
    format!("watching {}", path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            compile_project,
            project_export,
            file_watch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Poiesis");
}
```

- [ ] **Step 10: 创建 poiesis/src-tauri/src/main.rs**

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    poiesis_lib::run()
}
```

- [ ] **Step 11: 验证 Tauri 骨架编译**

```bash
cargo build
```

预期：workspace 中 omnilang 和 poiesis 均编译成功。

- [ ] **Step 12: Commit**

```bash
git add poiesis/ Cargo.lock
git commit -m "feat: create Poiesis Tauri 2.x application skeleton"
```

---

## Phase 7: 前端代码迁移

### Task 8: 迁移 editor/client → poiesis/src/（重组结构）

**Files:**
- Move & reorganize: `editor/client/src/` → `poiesis/src/`（按新结构）

- [ ] **Step 1: 创建新目录结构**

```bash
mkdir -p poiesis/src/commands
mkdir -p poiesis/src/services/codegen
mkdir -p poiesis/src/services/compiler
mkdir -p poiesis/src/services/project
mkdir -p poiesis/src/services/parser
mkdir -p poiesis/src/services/runtime
mkdir -p poiesis/src/features/code-mode
mkdir -p poiesis/src/features/design-mode
mkdir -p poiesis/src/features/compiler
mkdir -p poiesis/src/features/debug
mkdir -p poiesis/src/driver
mkdir -p poiesis/src/store
mkdir -p poiesis/src/core/commands
mkdir -p poiesis/src/core/state
mkdir -p poiesis/src/core/types
mkdir -p poiesis/src/core/events
mkdir -p poiesis/src/core/utils
mkdir -p poiesis/src/core/constants
mkdir -p poiesis/src/components/canvas/code/nodes
mkdir -p poiesis/src/components/canvas/design
mkdir -p poiesis/src/components/canvas/shared/edges
mkdir -p poiesis/src/components/panels
mkdir -p poiesis/src/components/editor
mkdir -p poiesis/src/components/inspector/sections
mkdir -p poiesis/src/components/shell
mkdir -p poiesis/src/components/preview
mkdir -p poiesis/src/components/shared
mkdir -p poiesis/src/lib/constants
mkdir -p poiesis/src/lib/types
mkdir -p poiesis/src/lib/utils
mkdir -p poiesis/src/snippets
mkdir -p poiesis/src/a11y
mkdir -p poiesis/src/data
```

- [ ] **Step 2: 迁移 services（纯 TS 逻辑，不变，排除 __tests__）**

```bash
# codegen（排除 __tests__）
cp editor/client/src/services/codegen/*.ts poiesis/src/services/codegen/

# compiler
cp editor/client/src/services/compiler/*.ts poiesis/src/services/compiler/

# project
cp editor/client/src/services/project/*.ts poiesis/src/services/project/

# parser
cp editor/client/src/services/parser/*.ts poiesis/src/services/parser/

# runtime
cp editor/client/src/services/runtime/*.ts poiesis/src/services/runtime/

# codegraph（遗漏的目录）
mkdir -p poiesis/src/services/codegraph
cp editor/client/src/codegraph/*.ts poiesis/src/services/codegraph/
```

- [ ] **Step 3: 迁移 features**

```bash
cp editor/client/src/features/code-mode/*.tsx poiesis/src/features/code-mode/
cp editor/client/src/features/design-mode/*.tsx poiesis/src/features/design-mode/
cp editor/client/src/features/compiler/*.ts poiesis/src/features/compiler/
cp editor/client/src/features/debug/*.ts poiesis/src/features/debug/
# project init
cp editor/client/src/features/project-explorer/*.ts poiesis/src/features/ 2>/dev/null || true
```

- [ ] **Step 4: 迁移 driver**

```bash
cp editor/client/src/driver/*.ts poiesis/src/driver/
cp editor/client/src/driver/*.d.ts poiesis/src/driver/ 2>/dev/null || true
```

- [ ] **Step 5: 迁移 store（排除 __tests__）**

```bash
cp editor/client/src/store/*.ts poiesis/src/store/
```

- [ ] **Step 6: 迁移 core（排除 __tests__）**

```bash
cp editor/client/src/core/commands/*.ts poiesis/src/core/commands/
cp editor/client/src/core/state/*.ts poiesis/src/core/state/
cp editor/client/src/core/types/*.ts poiesis/src/core/types/
cp editor/client/src/core/events/*.ts poiesis/src/core/events/
cp editor/client/src/core/utils/*.ts poiesis/src/core/utils/
cp editor/client/src/core/constants/*.ts poiesis/src/core/constants/
```

- [ ] **Step 7: 迁移 components（ui/ → components/）**

```bash
# canvas（排除 __tests__）
cp editor/client/src/ui/canvas/*.tsx poiesis/src/components/canvas/ 2>/dev/null || true
cp editor/client/src/ui/canvas/*.ts poiesis/src/components/canvas/ 2>/dev/null || true
cp editor/client/src/ui/canvas/code/*.tsx poiesis/src/components/canvas/code/ 2>/dev/null || true
cp editor/client/src/ui/canvas/code/nodes/*.tsx poiesis/src/components/canvas/code/nodes/ 2>/dev/null || true
cp editor/client/src/ui/canvas/design/*.tsx poiesis/src/components/canvas/design/ 2>/dev/null || true
cp editor/client/src/ui/canvas/shared/*.ts poiesis/src/components/canvas/shared/ 2>/dev/null || true
cp editor/client/src/ui/canvas/shared/*.tsx poiesis/src/components/canvas/shared/ 2>/dev/null || true
cp editor/client/src/ui/canvas/shared/edges/*.tsx poiesis/src/components/canvas/shared/edges/ 2>/dev/null || true

# panels
cp editor/client/src/ui/panels/*.tsx poiesis/src/components/panels/
cp editor/client/src/ui/panels/*.ts poiesis/src/components/panels/

# editor (Monaco)
cp editor/client/src/ui/editor/*.tsx poiesis/src/components/editor/
cp editor/client/src/ui/editor/*.ts poiesis/src/components/editor/

# inspector
cp editor/client/src/ui/inspector/sections/*.tsx poiesis/src/components/inspector/sections/

# shell
cp editor/client/src/ui/shell/*.tsx poiesis/src/components/shell/
cp editor/client/src/ui/shell/*.ts poiesis/src/components/shell/

# preview
cp editor/client/src/ui/preview/*.tsx poiesis/src/components/preview/

# shared
cp editor/client/src/ui/shared/*.tsx poiesis/src/components/shared/
```

- [ ] **Step 8: 迁移 snippets, a11y, data**

```bash
cp editor/client/src/snippets/*.ts poiesis/src/snippets/
cp editor/client/src/a11y/*.ts poiesis/src/a11y/
cp editor/client/src/data/*.ts poiesis/src/data/ 2>/dev/null || true
```

- [ ] **Step 9: 迁移 lib（utils + constants + notifications/toast）**

```bash
cp editor/client/src/utils/*.ts poiesis/src/lib/utils/
cp editor/client/src/constants/*.ts poiesis/src/lib/constants/
cp editor/client/src/notifications/*.ts poiesis/src/lib/ 2>/dev/null || true
# 类型移到 lib/types
cp editor/client/src/annotation/types.ts poiesis/src/lib/types/ 2>/dev/null || true
```

- [ ] **Step 10: 迁移入口文件**

```bash
cp editor/client/src/main.tsx poiesis/src/main.tsx
cp editor/client/src/app/App.tsx poiesis/src/App.tsx
```

- [ ] **Step 11: 迁移并创建 commands/ 层（api/ + services/api/ → commands/）**

先拷贝旧的 API service 文件作为参考，再创建新的 Tauri invoke 封装：

```bash
# 旧 HTTP API 实现（保留作为参考，后续移除 HTTP 调用）
cp editor/client/src/api/*.ts poiesis/src/commands/
cp editor/client/src/services/api/*.ts poiesis/src/commands/
```

创建 `poiesis/src/commands/index.ts`：

```typescript
import { invoke } from "@tauri-apps/api/core";

export interface CompileResult {
  success: boolean;
  output: string;
  errors: string[];
}

export async function compileProject(projectPath: string, target: string): Promise<CompileResult> {
  return invoke<CompileResult>("compile_project", { projectPath, target });
}

export async function projectExport(projectPath: string, outputDir: string): Promise<string> {
  return invoke<string>("project_export", { projectPath, outputDir });
}

export async function fileWatch(path: string): Promise<string> {
  return invoke<string>("file_watch", { path });
}
```

- [ ] **Step 12: 迁移 editor/runtime → services/runtime**

```bash
cp editor/runtime/src/*.ts poiesis/src/services/runtime/
```

- [ ] **Step 14: 删除旧 editor/ 目录**

```bash
rm -rf editor/
```

- [ ] **Step 15: Commit**

```bash
git add poiesis/src/
git commit -m "feat: migrate editor frontend to poiesis/src/ with reorganized structure; remove old editor/"
```

### Task 9: 更新前端 import 路径

**Files:**
- Modify: 所有 `poiesis/src/` 下的 .ts/.tsx 文件（import 路径更新）

- [ ] **Step 1: 用 grep 定位所有需要替换的 import 路径**

> 已确认：项目使用相对路径 import（`../`、`../../`），不用 `@/` alias。

```bash
cd poiesis/src

# 统计所有需要修改的 import 模式
echo "=== ui/ → components/ ==="
grep -rn "from '.*/ui/" --include="*.ts" --include="*.tsx" | wc -l
echo "=== services/api → commands ==="
grep -rn "from '.*/services/api" --include="*.ts" --include="*.tsx" | wc -l
echo "=== api/ → commands/ ==="
grep -rn "from '.*/api/" --include="*.ts" --include="*.tsx" | grep -v "services/api" | wc -l
echo "=== utils/ → lib/utils/ ==="
grep -rn "from '.*/utils/" --include="*.ts" --include="*.tsx" | wc -l
echo "=== constants/ → lib/constants/ ==="
grep -rn "from '.*/constants/" --include="*.ts" --include="*.tsx" | wc -l
echo "=== notifications/ → lib/ ==="
grep -rn "from '.*/notifications" --include="*.ts" --include="*.tsx" | wc -l
echo "=== annotation/ → lib/types/ ==="
grep -rn "from '.*/annotation" --include="*.ts" --include="*.tsx" | wc -l
echo "=== codegraph/ → services/codegraph/ ==="
grep -rn "from '.*/codegraph/" --include="*.ts" --include="*.tsx" | wc -l
echo "=== features/project-explorer/ → features/ ==="
grep -rn "from '.*/features/project-explorer/" --include="*.ts" --include="*.tsx" | wc -l
```

- [ ] **Step 2: 执行路径替换**

项目使用相对路径，用 sed 逐类替换：

```bash
cd poiesis/src

# 1. ui/ → components/（最大量）
find . -name "*.ts" -o -name "*.tsx" | xargs sed -i \
  -e "s|/ui/canvas/|/components/canvas/|g" \
  -e "s|/ui/panels/|/components/panels/|g" \
  -e "s|/ui/editor/|/components/editor/|g" \
  -e "s|/ui/shell/|/components/shell/|g" \
  -e "s|/ui/shared/|/components/shared/|g" \
  -e "s|/ui/preview/|/components/preview/|g" \
  -e "s|/ui/inspector/|/components/inspector/|g"

# 2. services/api → commands
find . -name "*.ts" -o -name "*.tsx" | xargs sed -i \
  -e "s|/services/api|/commands|g" \
  -e "s|/api/|/commands/|g"

# 3. utils/ → lib/utils/, constants/ → lib/constants/
find . -name "*.ts" -o -name "*.tsx" | xargs sed -i \
  -e "s|/utils/|/lib/utils/|g" \
  -e "s|/constants/|/lib/constants/|g"

# 4. notifications/ → lib/
find . -name "*.ts" -o -name "*.tsx" | xargs sed -i \
  -e "s|/notifications/|/lib/|g"

# 5. annotation/ → lib/types/
find . -name "*.ts" -o -name "*.tsx" | xargs sed -i \
  -e "s|/annotation/types|/lib/types/types|g"

# 6. codegraph/ → services/codegraph/
find . -name "*.ts" -o -name "*.tsx" | xargs sed -i \
  -e "s|/codegraph/|/services/codegraph/|g"

# 7. features/project-explorer/ → features/ (useProjectInit moved up)
find . -name "*.ts" -o -name "*.tsx" | xargs sed -i \
  -e "s|features/project-explorer/useProjectInit|features/useProjectInit|g"
```

> ⚠️ 每步替换后建议 `grep -rn "old_pattern" --include="*.ts" --include="*.tsx" | wc -l` 确认已无残留。

- [ ] **Step 3: 检查并修正残留的旧路径**

```bash
cd poiesis/src

# 确认所有旧路径都已替换
echo "残留 ui/ import:"
grep -rn "from '.*/ui/" --include="*.ts" --include="*.tsx" | grep -v node_modules | wc -l
echo "残留 services/api import:"
grep -rn "from '.*/services/api" --include="*.ts" --include="*.tsx" | wc -l
echo "残留 codegraph/ import:"
grep -rn "from '.*/codegraph/" --include="*.ts" --include="*.tsx" | wc -l

# 手动修复任何残留（sed 可能漏掉的边缘情况）
```

- [ ] **Step 4: 处理 App.tsx 的 import**

检查 `poiesis/src/App.tsx` 和 `poiesis/src/main.tsx`，确保顶层 import 路径正确。

- [ ] **Step 5: 验证 TypeScript 编译**

```bash
cd poiesis && npm install
cd poiesis && npx tsc --noEmit
```

修复所有类型错误。

- [ ] **Step 6: Commit**

```bash
git add poiesis/
git commit -m "refactor: update frontend import paths for new poiesis structure"
```

---

## Phase 8: 脚本与 CI 更新

### Task 10: 更新构建脚本

**Files:**
- Modify: `scripts/build-wasm.sh`
- Delete: `scripts/start-editor.sh`
- Delete: `scripts/test-kanban-compare.mjs`

- [ ] **Step 1: 更新 scripts/build-wasm.sh**

将 `omni-compiler` 替换为 `omnilang`，输出目录改为 `../poiesis/public/wasm`：

```bash
#!/bin/bash
set -euo pipefail

echo "==> Building omnilang WASM..."
wasm-pack build omnilang \
  --out-dir ../poiesis/public/wasm \
  --target web \
  --features wasm

echo "==> WASM built to poiesis/public/wasm/"
```

- [ ] **Step 2: 删除废弃脚本**

```bash
rm -f scripts/start-editor.sh
rm -f scripts/test-kanban-compare.mjs
```

- [ ] **Step 3: Commit**

```bash
git add scripts/
git commit -m "chore: update build-wasm.sh paths, remove deprecated scripts"
```

### Task 11: 更新 CI 配置

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: 重写 .github/workflows/ci.yml**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rust:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
      - name: Build workspace
        run: cargo build --release
      - name: Run Rust tests
        run: cargo test --workspace
      - name: Upload DLL artifact
        uses: actions/upload-artifact@v4
        with:
          name: omnilang-${{ matrix.os }}
          path: |
            target/release/omnilang.dll
            target/release/libomnilang.so
            target/release/libomnilang.dylib

  wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      - name: Install wasm-pack
        run: cargo install wasm-pack
      - name: Build WASM
        run: bash scripts/build-wasm.sh

  python:
    runs-on: ${{ matrix.os }}
    needs: rust
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
          name: omnilang-${{ matrix.os }}
          path: target/release/
      - name: Install omnic
        run: pip install -e omnic/[dev]
      - name: Run Python tests
        run: pytest omnic/tests/ -v

  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '22'
      - name: Install dependencies
        run: cd poiesis && npm ci
      - name: Type check
        run: cd poiesis && npx tsc --noEmit
      - name: Build
        run: cd poiesis && npm run build
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: update CI pipeline for new workspace structure"
```

---

## Phase 9: 验证与收尾

### Task 12: 终验 — 全局编译与安装验证

- [ ] **Step 1: 清理后编译 workspace**

```bash
cargo clean
cargo build --release
```

预期：omnilang 和 poiesis 均编译成功。

- [ ] **Step 2: 验证 Python CLI 安装**

```bash
pip install -e omnic/
python -c "from omnilang import cli; print('omnic OK')"
```

- [ ] **Step 3: 验证前端构建**

```bash
cd poiesis && npm install && npm run build
```

- [ ] **Step 4: 验证 WASM 构建**

```bash
bash scripts/build-wasm.sh
ls poiesis/public/wasm/
```

预期：`poiesis/public/wasm/` 下有 WASM 产物。

- [ ] **Step 5: 检查 .gitignore 覆盖**

```bash
git status
```

确认 `target/`、`poiesis/node_modules/`、`poiesis/dist/`、`poiesis/public/wasm/` 等没有被 git 追踪。

- [ ] **Step 6: 最终项目结构检查**

```bash
ls -la
```

确认只剩下设计文档中指定的目录：
```
.claude/  .github/  .gitignore  .mcp.json  Cargo.toml  Cargo.lock
README.md  LICENSE  QUICKSTART.md  docs/  omnilang/  omnic/  poiesis/  scripts/
```

- [ ] **Step 7: 最终 commit**

```bash
git add -A
git commit -m "chore: final verification — workspace build, omnic install, frontend build all pass"
```

- [ ] **Step 8: 推送（确认后执行）**

```bash
git push origin main --force-with-lease
```

⚠️ 推送前确认所有验证通过。

---

## 附录：import 路径替换参考

项目使用相对路径 import。下面是 Task 9 中的路径映射速查表：

| 旧 import 模式 | 新 import 模式 | 说明 |
|---------------|---------------|------|
| `../ui/canvas/...` | `../components/canvas/...` | ui/ → components/ |
| `../../ui/canvas/...` | `../../components/canvas/...` | 嵌套层级同样替换 |
| `../ui/panels/...` | `../components/panels/...` | |
| `../ui/editor/...` | `../components/editor/...` | |
| `../ui/shell/...` | `../components/shell/...` | |
| `../ui/shared/...` | `../components/shared/...` | |
| `../ui/preview/...` | `../components/preview/...` | |
| `../ui/inspector/...` | `../components/inspector/...` | |
| `../services/api` | `../commands` | HTTP → Tauri invoke |
| `../../services/api` | `../../commands` | 嵌套层级同样替换 |
| `../api/...` | `../commands/...` | 旧 api/ 目录合并 |
| `../utils/...` | `../lib/utils/...` | utils 并入 lib |
| `../../utils/...` | `../../lib/utils/...` | |
| `../constants/...` | `../lib/constants/...` | constants 并入 lib |
| `../../constants/...` | `../../lib/constants/...` | |
| `../notifications/...` | `../lib/...` | notifications 并入 lib |
| `../annotation/types` | `../lib/types/types` | annotation 并入 lib |
| `../codegraph/...` | `../services/codegraph/...` | codegraph 移入 services |
| `../../codegraph/...` | `../../services/codegraph/...` | |
| `../features/project-explorer/useProjectInit` | `../features/useProjectInit` | project-explorer 文件上提 |

**不变路径**（文件夹未改名）：
- `../store/...`、`../core/...`、`../features/code-mode/...`、`../features/design-mode/...`、
- `../features/compiler/...`、`../features/debug/...`、`../services/codegen/...`、
- `../services/compiler/...`、`../services/project/...`、`../services/parser/...`、
- `../services/runtime/...`、`../driver/...`、`../snippets/...`、`../a11y/...`、`../data/...`
