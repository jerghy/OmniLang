# OmniLang Rust Workspace 重构设计

**日期**：2026-06-28
**状态**：已确认

## 目标

将 OmniLang 项目重构为 Rust workspace，包含两个核心板块：

1. **omnilang** — OmniLang 编译器（Rust crate：cdylib + rlib + wasm）
2. **Poiesis** — 可视化编辑器（Tauri 2.x 桌面应用）

删除所有杂项，清理 git 分支。

## 架构总览

```
OmniLang/                          # git repo root
├── Cargo.toml                     # [workspace] manifest
├── Cargo.lock
├── .github/workflows/ci.yml       # 更新后的 CI
├── .gitignore                     # 更新
├── .mcp.json                      # MCP 配置
├── .claude/                       # Claude Code 配置
├── README.md                      # 更新
├── LICENSE
├── QUICKSTART.md
│
├── omnilang/                      # Rust compiler crate（原 omni-compiler）
│   ├── Cargo.toml                 # package name: "omnilang", crate-type: cdylib + rlib
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── lsp.rs                 # Rust LSP server（保留，Python LSP 删除）
│       ├── stdlib.rs
│       ├── lexer/
│       ├── parser/
│       ├── ir/
│       ├── semantic/
│       ├── codegen/
│       ├── runtime/               # 动画运行时常量，编译时嵌入产物
│       └── sourcemap/
│
├── omnic/                         # Python CLI 包（原 omnilang/）
│   ├── pyproject.toml
│   └── omnilang/
│       ├── __init__.py
│       ├── cli.py
│       ├── api.py
│       ├── config.py
│       ├── errors.py
│       ├── result.py
│       ├── rust_bridge.py         # 更新 DLL 名: omni_compiler.dll → omnilang.dll
│       └── sourcemap.py
│
├── poiesis/                       # Tauri 2.x 桌面应用（原 editor）
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   ├── public/
│   │   └── wasm/                  # wasm-pack build 输出（gitignore）
│   ├── src-tauri/                 # Rust 后端
│   │   ├── Cargo.toml             # 依赖 omnilang（路径）+ tauri 2.x + plugins
│   │   ├── tauri.conf.json
│   │   ├── icons/
│   │   └── src/
│   │       └── main.rs            # Tauri commands: compile_project, project_export, file_watch
│   └── src/                       # React/Vite 前端（重新组织）
│       ├── main.tsx
│       ├── App.tsx
│       ├── commands/              # Tauri invoke 封装（替代原 services/api/ HTTP 调用）
│       ├── services/              # 纯 TS 逻辑（codegen, compiler, project, runtime, parser）
│       ├── features/              # 功能模块（code-mode, design-mode, compiler, debug）
│       ├── driver/                # Canvas 驱动层
│       ├── store/                 # Zustand 状态管理
│       ├── core/                  # 核心领域模型（commands/state/types/events/utils）
│       ├── components/            # UI 组件（原 ui/）
│       ├── lib/                   # 工具常量（constants, types, utils, toast）
│       ├── snippets/              # 内置代码片段
│       ├── a11y/                  # 无障碍
│       └── data/                  # 静态数据/协议定义
│
├── docs/                          # 文档
│   ├── README.md
│   ├── stdlib.md
│   ├── omni-state-management-reference.md
│   ├── omniview-syntax-reference.md
│   ├── adr/
│   ├── cookbook/
│   ├── design/
│   ├── rete-docs/                 # 保留
│   ├── superpowers/
│   └── tutorial/
│
└── scripts/                       # 构建脚本
    ├── build-wasm.sh              # 更新路径: omnilang → poiesis/public/wasm
    ├── ci_smoke.sh                # 保留，更新路径
    └── omnilang-deploy.sh         # 保留，更新路径
```

## Cargo Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "omnilang",
    "poiesis/src-tauri",
]
```

### omnilang crate

```toml
[package]
name = "omnilang"
version = "0.2.0"
edition = "2021"

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
```

- **cdylib** → Python CLI（omnic）通过 FFI 调用，DLL 名 `omnilang.dll`
- **rlib** → Poiesis Tauri 后端直接依赖
- **wasm** → Poiesis 前端浏览器侧即时解析/校验

### poiesis src-tauri

```toml
[package]
name = "poiesis"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"
tauri-plugin-shell = "2"
tauri-plugin-window-state = "2"
tauri-plugin-store = "2"
omnilang = { path = "../../omnilang" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[build-dependencies]
tauri-build = { version = "2" }
```

**自定义 Tauri Commands**（核心，后续按需扩展）：
- `compile_project` — 调用 omnilang 完整编译 .omni → 目标代码
- `project_export` — 导出编译产物到指定目录
- `file_watch` — 文件变更监听（热重载）

## 数据流

```
用户编辑 .omni
  ├─→ 浏览器 WASM → 即时解析/校验（轻量，快速反馈）
  └─→ Tauri command → omnilang rlib → 完整编译 → 目标代码
```

## 前端结构调整（editor/client → poiesis/src）

保持功能和 UI 不变，仅重组文件结构以适应 Tauri：

| 原路径 | 新路径 | 说明 |
|--------|--------|------|
| `services/api/` | `commands/` | HTTP 调用 → Tauri invoke 封装 |
| `services/codegen/` | `services/codegen/` | 不变（纯 TS） |
| `services/compiler/` | `services/compiler/` | 不变 |
| `services/project/` | `services/project/` | 不变 |
| `services/parser/` | `services/parser/` | 不变 |
| `services/runtime/` | `services/runtime/` | 不变 |
| `editor/runtime/src/` | `services/runtime/` | TS runtime 迁移 |
| `ui/` | `components/` | 符合 Tauri/Vite 惯例 |
| `store/` | `store/` | 不变 |
| `driver/` | `driver/` | 不变 |
| `core/` | `core/` | 不变 |
| `features/` | `features/` | 不变 |
| `snippets/` | `snippets/` | 不变 |
| `a11y/` | `a11y/` | 不变 |
| `utils/` + `notifications/` + `constants/` | `lib/` | 合并 |
| `data/` | `data/` | 不变 |

## 命名变更汇总

| 项目 | 旧名 | 新名 |
|------|------|------|
| Rust crate | `omni-compiler` | `omnilang` |
| DLL（Windows） | `omni_compiler.dll` | `omnilang.dll` |
| Python 包目录 | `omnilang/` | `omnic/omnilang/` |
| 编辑器 | `editor` | `poiesis` |
| 启动脚本 | `start-editor.sh` | **删除**（用 `cd poiesis && npm run tauri dev`） |

## 删除清单

### 目录
- `editor/` → 迁移到 `poiesis/`
- `omni-compiler/` → 迁移到 `omnilang/`
- `omnilang/`（旧 Python 包位置）→ 迁移到 `omnic/`
- `vscode-extension/` — 删除
- `tools/` — 删除
- `dashboard/` — 删除
- `test/` — 删除
- `test-proj/` — 删除
- `tests/` — 删除
- `build/` — 删除
- `build_output/` — 删除
- `.e2e-test/` — 删除
- `node_modules/` — 删除
- `.pytest_cache/` — 删除
- `.ruff_cache/` — 删除
- `.playwright-mcp/` — 删除
- `omnilang.egg-info/` — 删除

### 根目录零散文件
所有 screenshots（*.png）、snapshots（*.md, *.txt, *.yml）、临时脚本（check_*.js, extract_js.js, generated.js）、日志文件（console-*.txt, hmr-log.txt）、状态 dump（canvas-state, component-tree, editor-*）、废弃配置（.flutter-skill.yaml, package.json, package-lock.json, pyproject.toml）、临时目录（.tours/, .superpowers/）。

### scripts/ 清理
- `start-editor.sh` — 删除
- `test-kanban-compare.mjs` — 删除
- `build-wasm.sh` — 保留，更新路径
- `ci_smoke.sh` — 保留，更新路径
- `omnilang-deploy.sh` — 保留，更新路径

### Git 分支
删除所有本地和 worktree 分支，仅保留 `main`：
- `feat/omniview-motion-phase1`
- `worktree-editor-architecture-refactor`
- `worktree-editor-debt-cleanup`
- `worktree-feat-editor-omni-format-redesign`
- `worktree-fix-store-getter-scoping`
- `worktree-learn-omniview-phase-1`
- `worktree-omnic-cli-redesign`
- 清理所有 worktree 目录（`.claude/worktrees/*`）

### Python LSP
- `omnilang/lsp/` — 删除（Rust LSP 保留在 `omnilang/src/lsp.rs`，通过 FFI 调用）

### 测试
所有现有测试删除，重构后按需重写。

## 保留清单

- `.claude/` — Claude Code 配置
- `.mcp.json` — MCP 服务配置
- `.github/workflows/ci.yml` — CI（更新）
- `.gitignore` — 更新
- `docs/` — 全部保留（含 rete-docs/）
- `scripts/build-wasm.sh` — 更新路径
- `scripts/ci_smoke.sh` — 更新路径
- `scripts/omnilang-deploy.sh` — 更新路径
- `README.md`、`LICENSE`、`QUICKSTART.md`

## .gitignore 更新

在现有规则基础上新增：

```gitignore
# Rust
target/
Cargo.lock

# Node / Tauri
poiesis/node_modules/
poiesis/dist/

# Python
omnic/__pycache__/
omnic/*.egg-info/

# WASM output
poiesis/public/wasm/

# IDE
.vscode/
.idea/
```

## CI 流水线

| Job | 内容 |
|-----|------|
| `rust` | `cargo build --release` + `cargo test --workspace`（三平台） |
| `wasm` | `wasm-pack build omnilang --out-dir ../poiesis/public/wasm --target web --features wasm` |
| `python` | `pip install -e omnic/[dev]` + `pytest`（依赖 rust job 的 DLL artifact） |
| `frontend` | `cd poiesis && npm ci && npm run build && npm test` |

## 实现步骤

1. 清理根目录杂物（零散文件）
2. 删除所有 worktree 和分支
3. 删除废弃目录
4. 清理缓存目录
5. 创建 workspace `Cargo.toml`
6. 迁移 `omni-compiler/` → `omnilang/`（调整 Cargo.toml：name = "omnilang"）
7. 迁移 `omnilang/`（Python 包）→ `omnic/omnilang/`，更新 `rust_bridge.py` DLL 名
8. 创建 `omnic/pyproject.toml`
9. 删除 `omnilang/lsp/`（Python LSP）
10. 创建 `poiesis/` Tauri 2.x 应用骨架
11. 迁移 `editor/client/` → `poiesis/src/`（按新结构重组）
12. 迁移 `editor/runtime/src/` → `poiesis/src/services/runtime/`
13. 配置 `poiesis/src-tauri/Cargo.toml` 依赖 omnilang + Tauri plugins
14. 实现核心 Tauri commands（compile_project, project_export, file_watch）
15. 更新 `scripts/build-wasm.sh` 路径
16. 删除 `scripts/start-editor.sh`、`scripts/test-kanban-compare.mjs`
17. 更新 `.github/workflows/ci.yml`
18. 更新 `.gitignore`
19. 验证 `cargo build --workspace` 通过
20. 验证 `pip install -e omnic/` 可用
