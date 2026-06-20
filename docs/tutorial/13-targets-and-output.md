# 13. 编译目标与输出

> **目标**：理解全部 10 个编译目标 —— 何时选择哪个、各产出什么、如何运行结果。

OmniLang 将 `.omni` 编译到 **10 个目标** —— 5 个逻辑目标（纯计算）和 5 个 UI 目标（可视化界面）。

---

## 目标总览

### 逻辑目标

| 目标 | 扩展名 | 运行时 | 成熟度 | 适合场景 |
|------|--------|--------|--------|---------|
| `python` | `.py` | Python 3.10+ | ✅ 完成 | 脚本、数据处理、AI/ML |
| `js` | `.js` | Node.js / 浏览器 | ✅ 完成 | Serverless、Web 后端 |
| `ts` | `.ts` | TypeScript（`ts-node`） | ✅ 完成 | 类型安全的 JS 项目 |
| `rust` | `.rs` | Rust（Cargo） | ✅ 完成 | 系统编程、性能关键场景 |
| `dart` | `.dart` | Dart SDK | ✅ 完成 | Flutter 后端、CLI 工具 |

### UI 目标

| 目标 | 扩展名 | 框架 | 成熟度 | 适合场景 |
|------|--------|------|--------|---------|
| `omniview` | `.html` + `.js` + `.css` | 无（原生） | ✅ 生产 | 快速原型、独立应用 |
| `react` | `.tsx` | React + Zustand | ✅ L2 | 生产级 React 应用 |
| `vue` | `.vue` | Vue 3（SFC） | ✅ L2 | 生产级 Vue 应用 |
| `dioxus` | `.rs` | Dioxus（Rust） | ✅ L2 | Rust Web 应用 |
| `flutter` | `.dart` | Flutter | L1（基础） | 移动/桌面应用 |

---

## 逻辑目标详解

### Python

```bash
omnic build --target python
omnic run src/main.omni --target python
```

输出：`build/main.py`

生成 Python 代码，包含：
- 结构体编译为 dataclass
- 类型注解（Python 3.10+ 使用 `str | int`）
- 模式匹配使用 `match`/`case`（Python 3.10+）

### JavaScript

```bash
omnic build --target js
omnic run src/main.omni --target js
```

输出：`build/main.js`

生成 ES 模块，包含：
- 结构体编译为 class
- 私有字段使用 `#` 语法（`private_fields = true` 时）
- 闭包编译为箭头函数

### TypeScript

```bash
omnic build --target ts
```

输出：`build/main.ts`

与 JavaScript 相同，但加上完整的类型注解。

### Rust

```bash
omnic build --target rust
omnic run src/main.omni --target rust
```

输出：`build/main.rs`（运行时会创建 Cargo 项目于 `build/debug/rust_proj/`）

生成惯用 Rust 代码，包含：
- 结构体 → `struct`
- 带标签联合体 → `enum`
- 异步函数使用 `async`/`await`，配合选定的运行时（tokio/async-std）

### Dart

```bash
omnic build --target dart
omnic run src/main.omni --target dart
```

输出：`build/main.dart`

生成 Dart 代码，包含：
- 结构体 → class
- 带标签联合体 → sealed class
- 异步函数使用 Future/async

---

## UI 目标详解

### OmniView（原生）

```bash
omnic build --target omniview
omnic run src/main.omni    # 自动检测并打开浏览器
```

输出：`build/body.html` + `build/index.js` + `build/index.css`
运行输出：`build/debug/index.html`（独立自包含）

OmniView 是**最成熟**的后端。生成独立 HTML 文件，包含：
- 内联信号响应式运行时（约 3KB）
- 通过 `data-nid` 属性直接操作 DOM
- `@for` 使用 `<template>` 克隆渲染
- `@if` 使用 `display: block/none` 切换
- 调试用源码映射
- 无框架依赖——单文件即可运行

**最适合**：快速原型、独立演示、嵌入现有页面。

### React

```bash
omnic build --target react
```

输出：`build/*.tsx`

生成 React 组件，使用 Zustand 管理状态。需要 React 项目环境（Vite、Next.js 等）。

### Vue

```bash
omnic build --target vue
```

输出：`build/*.vue`（单文件组件）

生成 Vue 3 SFC，使用 `<script setup>` 和组合式 API。

### Dioxus

```bash
omnic build --target dioxus
```

输出：`build/*.rs`

生成 Dioxus 组件，使用 RSX 宏。需要 Rust/Dioxus 项目。

### Flutter

```bash
omnic build --target flutter
```

输出：`build/*.dart`

生成 Flutter 组件。L1 成熟度——基本组件结构已支持。

---

## 目标自动检测

省略 `--target` 时，`omnic` 按以下逻辑判断：

```
1. 检查 --target CLI 参数           → 如果提供则使用
2. 检查 .omnilang.toml 中的 ui_target → 用于视图文件
3. 检查源码内容：
   - 包含 "import omniview"         → react（UI）
   - 以 "component" 开头             → react（UI）
   - 否则                           → python（逻辑）
```

`omnic run` 会将 UI 目标（`react`、`vue`、`dioxus`、`flutter`）重定向到 `omniview` 以便直接在浏览器中执行。

---

## 如何选择目标

### 我在构建 CLI 工具
→ 选 `python`、`js` 或 `rust`，取决于你的技术栈

### 我在构建 Web 应用
→ 先用 `omniview` 做原型，再用 `react` 或 `vue` 投入生产

### 我在构建移动应用
→ `flutter`（L1 级别——查看当前状态）

### 我在编写供他人使用的库
→ 编译到多个逻辑目标：`python`、`js`、`rust`、`dart`

### 我只是在学 OmniLang
→ 使用 `omnic run`——自动检测并在浏览器（视图）或终端（逻辑）中展示结果

---

## 下一步

- **[11. CLI 参考手册](11-cli-reference.md)** — 所有 `omnic` 命令
- **[12. 配置指南](12-configuration.md)** — `.omnilang.toml` 中的目标专属配置
- **[14. VS Code 与编辑器](14-vscode-and-editor.md)** — IDE 集成
