# OmniLang 架构重新设计

> **Status:** Draft — 基于 brainstorming 会话  
> **Date:** 2026-06-16  
> **Author:** jerghy + Claude  

## 1. 核心决策

**编译器边界：单一 `omni-compiler`（Rust）是唯一的编译器。** Python 退化为薄封装层——文件 I/O、path mapping、CLI、HTTP 服务。Python 端现有的 parser/codegen 暂时保留，渐进删除。

### 1.1 编译器职责（Rust）

```
.omni 源码（Map<FilePath, SourceText>）
    │
    ▼
omni-compiler (Rust lib, 纯逻辑, 无文件I/O)
    │
    ├── Lexer (手写, 字符级)
    ├── Parser (Pratt + 递归下降)
    ├── Semantic (跨文件验证, 循环检测, 类型检查, lint)
    ├── IR Lowering (AST → Intent IR)
    └── Codegen (ALL targets)
        ├── OmniView JS/HTML/CSS + SourceMap
        ├── React .tsx
        ├── Vue .vue
        ├── Dioxus .rs
        ├── Flutter .dart
        ├── Python .py
        ├── JavaScript .js
        ├── TypeScript .ts
        ├── Rust .rs
        └── Dart .dart
```

### 1.2 封装层职责（Python）

```
Python 薄封装
├── 文件 I/O (读 .omni, 写输出)
├── Path mapping (resolve imports, 传给 Rust)
├── CLI (omnic build/check/init/serve/fmt/run)
├── Editor HTTP 服务 (FastAPI)
└── Rust FFI 集成测试
```

### 1.3 封装层职责（Editor / WASM）

```
Editor / WASM 层
├── Web Worker: omni-compiler.wasm (Rust→WASM, 浏览器内编译)
├── Editor 前端 (React SPA)
└── 反向 codegen (editor state → .omni 源码)
```

---

## 2. 数据流

### 2.1 Editor 调试/预览路径

```
Editor 可视化操作
    │
    ▼
.omni.json (主存储, 含画布状态 + 节点 + 项目级信息)
    │
    │  用户点击 "调试运行" 或 "编译"
    ▼
反向 codegen (editor TS 端)
    │
    ▼
.omni 源码 (输出/交换格式)
    │
    ▼
WASM Worker: omni-compiler
    │
    ▼
OmniView JS/HTML/CSS + SourceMap
    │
    ▼
预览 iframe 渲染
```

### 2.2 生产构建路径

```
.omni 源码 (磁盘文件)
    │
    ▼
omnic build (Python CLI)
    │
    ├── Python 文件发现 + import 图解析
    ├── 构建 {path: source} map
    │
    ▼
omni-compiler native DLL/so
    │
    ├── Parse + Semantic + IR
    ├── Codegen (目标框架)
    │
    ▼
输出文件 (React .tsx / Vue .vue / Flutter .dart / ...)
```

### 2.3 保存路径

```
Editor 可视化操作
    │
    ▼
.omni.json (主存储, 实时自动保存)
    │
    │  用户手动触发 "保存源码"
    ▼
反向 codegen → .omni 源码 → 写入磁盘
```

---

## 3. IR 角色

IR（Intent IR）保留作为 AST → codegen 的标准中间层。

```
AST ──► IR ──► Codegen (所有 target)
         │
         └──► IR JSON (对外 API, 供调试/测试/editor 消费)
```

- Rust 内部 AST → IR → codegen（标准管线）
- IR JSON 对外暴露，供 editor view tree 提取、调试工具、测试验证使用

---

## 4. 迁移策略

### 4.1 迁移原则

- Python parser/codegen **暂时保留，渐进删除**
- Rust codegen 写完一个 target → 用 Python codegen 做回归对比 → 验证通过后 Python 那个 target 退化为测试参考
- Rust 全部成熟后，Python parser/codegen 全部删除

### 4.2 Codegen 迁移顺序

| 阶段 | 内容 |
|------|------|
| 1 | 纯逻辑 targets：Python, JavaScript, Rust, Dart |
| 2 | OmniView 原生：JS/HTML/CSS + SourceMap |
| 3 | Framework targets：React, Vue, Dioxus, Flutter |

### 4.3 迁移阶段总览

| 阶段 | 内容 |
|------|------|
| **Phase 1** | Import 系统完成（当前正在做）→ Rust 具备多文件 parse + 跨文件验证能力 |
| **Phase 2** | Rust 纯逻辑 codegen（python/js/rust/dart）→ 替代 `omnilang/codegen/` |
| **Phase 3** | Rust OmniView 原生 codegen（JS/HTML/CSS）→ editor WASM 调试预览打通 |
| **Phase 4** | Rust framework codegen（React/Vue/Dioxus/Flutter）→ 替代 `omniview/codegen/` |
| **Phase 5** | 删除 Python parser/codegen、合并 CLI、统一 grammar spec |

---

## 5. 组件归属

| 组件 | 归属 | 说明 |
|------|------|------|
| Lexer | Rust | 手写，字符级 |
| Parser | Rust | Pratt + 递归下降 |
| Semantic | Rust | 跨文件验证、循环检测、类型检查、lint |
| IR Lowering | Rust | AST → IR |
| ALL Codegen | Rust | 所有 10 个 target |
| Preprocessor | Rust (merged into parser) | Token-level inline block handling, not a standalone phase |
| File I/O | Python | Rust 纯 lib，不碰文件系统 |
| Path Mapping | Python | 解析 import 图、alias、文件发现 |
| CLI | Python | 合并 omnilang/cli.py + omniview/cli.py |
| Editor Server | Python | FastAPI，HTTP + WebSocket |
| Editor Frontend | TS/React | SPA，含反向 codegen |
| WASM Worker | Rust→WASM | 浏览器内编译 |
| 反向 Codegen | Editor TS | visual state → .omni 源码 |
| .omni.json | Editor | 主存储格式，含画布状态 |
| LSP | 暂缓 | 未来评估 |
| Grammar Spec | 文档 | 单一 spec 文档，不再有 .lark 文件 |
| 诊断/Lint | Rust | semantic 阶段实现 |

---

## 6. 测试策略

- **Rust 单元测试：** `#[test]` 测内部模块（lexer/parser/semantic/ir/codegen）
- **Python 集成测试：** 通过 FFI 调 Rust lib，做端到端验证
- **现有 Python tests：** 暂时保留，作为 Rust 实现的回归 oracle；Rust 覆盖后删除

---

## 7. CLI 合并

合并 `omnilang/cli.py` 和 `omniview/cli.py` 为单一 `omnic` 入口，保留所有命令：

```
omnic build <file.omni> [--target ...]   编译
omnic check <file.omni>                  静态分析
omnic fmt <file.omni>                    格式化
omnic init [dir]                         新建项目
omnic serve --dir <dir>                  启动 editor
omnic run <file.omni> [--target ...]     编译并运行
```

所有命令内部调 Rust compiler（通过 FFI）。

---

## 8. 项目结构演进

### 8.1 近期（保持现有结构，精简冗余）

```
omni-compiler/     # Rust compiler (主力)
omnilang/          # 精简：config.py, modules/, 删除 parser/codegen 后
omniview/          # 精简：api.py, cli.py, modules/
editor/            # React SPA + Python FastAPI server
docs/              # spec + design docs
```

### 8.2 远期（以 Rust compiler 为中心重组）

```
omni-compiler/     # Rust lib + WASM + native builds
python/            # 合并 omnilang + omniview 的残留封装代码
  ├── cli.py
  ├── api.py
  ├── modules/
  └── server/
editor/            # 不变
docs/              # 不变
```

---

## 9. Rust Codegen 内部架构

### 9.1 分层设计

```
codegen/
├── mod.rs              # Codegen trait + target dispatch
├── logic.rs            # LogicCodegen trait (纯逻辑共享)
│   ├── python.rs       # 只覆盖语法差异 (def, None, etc.)
│   ├── javascript.rs   # 只覆盖语法差异 (function, null, etc.)
│   ├── rust.rs         # 只覆盖语法差异 (fn, Option, etc.)
│   └── dart.rs         # 只覆盖语法差异 (int, ?, etc.)
├── framework/
│   ├── react.rs        # 各自独立 — 组件模型完全不同
│   ├── vue.rs
│   ├── dioxus.rs
│   └── flutter.rs
└── omniview.rs         # 独立：原生 JS/HTML/CSS + SourceMap
```

- **LogicCodegen trait**：纯逻辑 4 个 target 共享 90% 结构，只覆盖关键字、类型标注、`pub`→visibility 映射的差异
- **Framework codegen**：React/Vue/Dioxus/Flutter 各自独立——组件模型、样式系统、事件绑定、生命周期完全不同
- **OmniView**：独立 codegen，产出原生 HTML+JS+CSS + SourceMap，WASM debug 路径用

---

## 10. Preprocessor

Preprocessor 合并入 Rust parser，**不作为独立前置阶段**。Inline block 提取、宏展开等在 parser token 流级别一步到位处理。删除 `omniview/preprocessor.py`。

---

## 11. Signal 运行时（OmniView 原生）

保留内联 signal runtime（`emit_signal_runtime()`），标记为 **OmniView Native Runtime v2**，不引入外部依赖。WASM 编译 → 原生 HTML+JS+CSS + SourceMap → iframe 加载调试。OmniView 原生输出自带 debug 能力，无需外部运行时库。

---

## 12. Formatter

Formatter（`omnic fmt`）**留在 Python**。源码→源码格式化不属于编译管道，Python 处理字符串更方便。优先级低，当前实现够用。

---

## 13. 过渡策略

- **Rust 优先，无 Python fallback。** Rust 报 unsupported target → 修 Rust，不走 Python
- Python codegen 保留作为测试 oracle，不作为生产代码路径
- Python `omniview/semantic/analyzer.py` 等其他模块：**渐进删除**，等 Rust 对齐后删

---

## 14. FFI 边界：Native DLL vs WASM 双路径

编辑器调试走 WASM（Web Worker），生产构建走 native DLL。两条路径的函数签名设计不同：

| 路径 | 导出方式 | 传输格式 |
|------|----------|----------|
| Native DLL | `#[no_mangle] pub unsafe extern "C"` | C string (JSON)，通过 ctypes |
| WASM | `#[wasm_bindgen]` | `String` (JSON)，通过 wasm-pack |

DLL 加载失败 → Python 端捕获 `OSError`/`ctypes.CDLL` 异常，返回 `CompileError`。
WASM 加载失败 → Worker `onerror` 捕获，前端显示编译错误。
两者都不走 Python fallback —— 直接报错让用户修。

---

## 15. IR 版本化

`IRProgram.version` 字段（当前 `"1.0"`）用于向前兼容。策略：

- **Minor 变化**（新增可选字段）：保持 version 不变，serde `default` 处理旧数据
- **Breaking 变化**（字段重命名/删除/类型变更）：递增 version，Python `ir_adapter.py` 按 version 做转换
- Editor 读取 IR JSON 时检查 version，不支持的版本提示升级

---

## 16. 实现状态（2026-06-16）

| 模块 | 状态 | 备注 |
|------|------|------|
| Import 语法 (Rust+Python) | ✅ ~95% | AST/parser/grammar/lexer 全部到位 |
| 跨文件验证 (Rust) | ✅ | `validate_multi()` + Tarjan SCC，58 tests pass |
| Rust codegen | 🟡 仅 OmniView | 缺 9 个 target（4 纯逻辑 + 4 框架 + TypeScript） |
| Python codegen | ✅ 全部 10 个 | 过渡期保留 |
| CLI 合并 | ❌ | 两个 CLI 各自独立，均名为 `omnic` |
| Preprocessor | ❌ | 仍在 Python，待迁入 Rust parser |
| Python tests | ✅ 360 collected | omniview/tests/ |
| Rust tests | ✅ 58 pass | lib(3) + codegen(24) + lexer(13) + parser(15) + semantic(3) |

---

## 17. 参考文档

- [Import System Design](./2026-06-16-omni-import-system-design.md) — import 语法、路径系统、跨文件验证
- [Rust Compiler Design](./2026-06-15-omni-rust-compiler-design.md) — 三层架构、crate 结构、lexer/parser/semantic 设计
- [ADR-001](../../docs/adr/001-omnilang-omniview-unification.md) — OmniLang=Dart, OmniView=Flutter 统一模型

---

## 18. 决策日志

| # | 决策 | 理由 |
|---|------|------|
| 1 | 单一 Rust compiler，Python 薄封装 | 消除双编译器边界模糊 |
| 2 | Rust compiler 纯 lib，无文件 I/O | 必须编译到 WASM；调用者负责文件 |
| 3 | 单一 CLI 入口 (`omnic`)，保留所有命令 | 统一用户体验 |
| 4 | Python parser/codegen 渐进删除 | 保护现有 300+ tests，降低风险 |
| 5 | 单一 grammar spec 文档，删除 .lark 文件 | Rust 手写 parser 是唯一实现 |
| 6 | IR 保留作为 AST→codegen 中间层 | 稳定序列化格式，外部工具可消费 |
| 7 | 反向 codegen 留在 editor (TS) | 跟 editor 状态紧密耦合 |
| 8 | .omni.json 是 editor 主存储 | 含可视化状态和项目级信息 |
| 9 | .omni 源码是输出/交换格式 | 用户手动触发生成 |
| 10 | WASM 在 Web Worker 运行 | 不阻塞 UI |
| 11 | 调试用 WASM，生产构建用 CLI | 两条路径各司其职 |
| 12 | Editor 服务端保持 Python FastAPI | 成熟稳定，无需重写 |
| 13 | Preprocessor 移到 Rust | 编译管道纯逻辑 |
| 14 | 诊断/Lint 在 Rust 实现 | 编译管道纯逻辑 |
| 15 | Codegen 迁移顺序：纯逻辑 → OmniView → Framework | 先简后繁 |
| 16 | 测试：Rust 单元 + Python FFI 集成 | Rust 纯 lib，需要 Python 框架测试 |
| 17 | LSP 暂缓 | 当前优先级不在此 |
| 18 | 项目结构先 A（保持现有）后 B（Rust 中心） | 渐进演进 |
| 19 | Rust codegen 分层：LogicCodegen trait + Framework 各独立 + OmniView 独立 | 纯逻辑 90% 共享，框架组件模型完全不同 |
| 20 | 过渡期 Rust 优先，无 Python fallback | Rust 不支持就修 Rust，不降级 |
| 21 | Preprocessor 合并入 Rust parser，不作为独立阶段 | Token 流级别一步到位 |
| 22 | 所有 40+ lint 规则在 Rust 重写 | 编译管道纯逻辑 |
| 23 | Formatter 留在 Python | 不属于编译管道 |
| 24 | Signal 运行时保留为 OmniView Native Runtime v2，WASM 原生输出 HTML+JS+CSS | 内联 runtime 自带 debug 能力，无外部依赖 |
| 25 | Python semantic/analyzer 渐进删除 | 等 Rust 对齐后删 |
