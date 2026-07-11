---
name: poiesis-dev
description: Poiesis 开发指南——当编写、调试、理解 Poiesis 可视化编辑器代码时使用。触发词："poiesis", "编辑器", "editor", "design system", "variant", "token", "class", "画布", "canvas"。
---

# Poiesis 开发指南

> Poiesis = OmniLang 语言的**架构优先、双向可视化编辑器**。Tauri v2 + React 19 + Zustand 5。

## 一句话定位

**图谱是地图，代码是目的地，画布是预览。**

## 核心事实（必须先知道）

| 事实 | 说明 |
|------|------|
| 磁盘 ≠ 内存 | 直接改 `.omni` 文件不会被编辑器感知——编辑器用 Zustand Store 做主内存，保存时才写磁盘 |
| 两大页面 | ArchitecturePage（泡泡关系图谱，默认首页）+ DesignPage（标签页编辑器） |
| 往返闭环 | `.omni 源码 → parser → Store → codegen emitter → .omni 源码`（PID 注释保证稳定性） |
| 编译双路径 | WASM（编辑器画布预览）+ Rust 后端（项目导出到 React/Flutter 等） |
| 调试用 Victauri | `dom_snapshot` + `eval_js` + `screenshot`，不猜状态 |

## 目录结构

```
poiesis-dev/
├── SKILL.md                       # 本文件 —— 框架概览
└── references/
    ├── architecture.md            # 架构详解：数据流、Store、编译路径、Rust 后端
    ├── design-system.md           # Design System：Token / Class / Variant 完整机制
    ├── confusions.md              # 常见混淆与陷阱（必读！）
    ├── workflow.md                # 开发工作流、Victauri 调试、修改流程
    └── file-map.md               # 关键文件职责地图
```

## 速查：我要做什么 → 看哪个文件

| 要做的事 | 参考文件 |
|----------|---------|
| 理解整体架构和数据流 | `references/architecture.md` |
| 修改/新增 Token/Class/Variant 机制 | `references/design-system.md` |
| 排查"为什么改了没用/为什么值不对" | `references/confusions.md` |
| 调试 UI 问题、学习开发流程 | `references/workflow.md` |
| 找某个功能在哪个文件 | `references/file-map.md` |

## 速查：关键文件（最常用 10 个）

| # | 文件 | 职责 |
|---|------|------|
| 1 | `poiesis/src/store/types.ts` | PoiesisState 完整类型（180+ 字段） |
| 2 | `poiesis/src/store/slices/design-system.ts` | Token/Class/Variant CRUD + element binding |
| 3 | `poiesis/src/core/types/design.ts` | DesignDefinition/Token/Class/Variant 类型定义 |
| 4 | `poiesis/src/services/codegen/design-emitter.ts` | Store → design.omni 源码发射 |
| 5 | `poiesis/src/services/codegen/parser.ts` | design.omni 源码 → Store 解析 |
| 6 | `poiesis/src/components/design/VariantEditor.tsx` | Variant 维度/分支/属性卡片编辑器 |
| 7 | `poiesis/src/components/design/ClassEditor.tsx` | Class 属性卡片编辑器 |
| 8 | `poiesis/src/components/panels/ContextInspector.tsx` | 元素右侧属性面板 |
| 9 | `poiesis/src-tauri/src/lib.rs` | Rust 后端全部 Tauri 命令 |
| 10 | `poiesis/src/data/componentProtocol.ts` | 内置组件协议（标签/属性/事件/推荐 variant） |

## 开发铁律

1. **改磁盘 .omni 后不要点保存**——编辑器会用内存覆盖磁盘。正确做法：关 Tab → 重开 Tab
2. **往返闭环**——改 parser 必检查 emitter，改 emitter 必检查 parser
3. **Variant 分支属性必须显式定义**——漏了某个分支的属性 → 浏览器默认值，不是"继承上一级"
4. **kebab-case（磁盘） ↔ camelCase（内存）**——两个方向都有转换
5. **Victauri > 猜**——`eval_js` 读 computed style，`dom_snapshot` 看结构

<!-- APPEND -->
<!-- 后续补充内容写在此注释之上 -->
