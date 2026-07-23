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
| 统一注解格式 | `.omni` 中只使用 `// @pid:xxx`（独占一行，在声明之上）。旧 `// @token/@class/@variant` 格式已废弃 |
| 手动 omni | fn/action body 内用 `// @<type>` 标注节点类型（@if/@elif/@else/@return/@code-block/@for/@while），保存后自动替换为 `// @pid` |
| project.json v3 | 新增 `nodeMeta.nodeType`、`nodeMeta.blockName`、`codeFlowGraphs`；全部 `Record<string, T>` 结构 |

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
| 9 | `poiesis/src/components/canvas/code/card-editor/CodeCardEditor.tsx` | ★ 逐行代码卡片编辑器（双击 code-block 节点） |
| 10 | `poiesis/src-tauri/src/lib.rs` | Rust 后端全部 Tauri 命令 |

## 开发铁律

1. **改磁盘 .omni 后不要点保存**——编辑器会用内存覆盖磁盘。正确做法：关 Tab → 重开 Tab
2. **往返闭环**——改 parser 必检查 emitter，改 emitter 必检查 parser
3. **Variant 分支属性必须显式定义**——漏了某个分支的属性 → 浏览器默认值，不是"继承上一级"
4. **kebab-case（磁盘） ↔ camelCase（内存）**——两个方向都有转换
5. **Victauri > 猜**——`eval_js` 读 computed style，`dom_snapshot` 看结构

<!-- APPEND -->

## CodeCard 编辑器（card-editor/）

2026-07-23 重写。逐行编辑器模型，不依赖 Lexical。

### 架构

```
双击 code-block 节点 → CodeCardEditor 弹出
│
├─ RowList（行列表容器）
│   ├─ InputRow     ← <input> 文本输入行，打字触发补全
│   ├─ CardRow      ← 卡片分发器 → DeclareVarCard / SetVarCard / ...
│   ├─ CloseBrace   ← 只读 } 闭合行
│   └─ (空行)       ← 光标落点
│
├─ AutocompleteMenu ← 浮动菜单，定位当前行下方，前缀过滤
│
└─ Toolbar          ← Save / Cancel / 文本模式 → CodeEditorModal
```

### 数据模型

```typescript
type EditorRow =
  | { kind: "input";  id: string; text: string }   // 文本输入行
  | { kind: "card";   op: CodeOp; indent: number }  // 卡片行
  | { kind: "close-brace"; indent: number }          // } 闭合
  | { kind: "empty";  id: string }                   // 空行

// 加载: opsToRows(CodeOp[]) → EditorRow[]
// 保存: rowsToOps(EditorRow[]) → CodeOp[] → emitOps() → source
```

### 左侧颜色条

| op.kind | 颜色 | 含义 |
|---------|------|------|
| declare_var | 🟣 紫 #7c3aed | 变量声明 |
| set_var | 🟠 琥珀 #f59e0b | 赋值 |
| if_block | 🔵 青 #06b6d4 | 条件块 |
| while | 🔵 青 #06b6d4 | 循环 |
| for_in | 🔵 青 #06b6d4 | 遍历 |
| return | 🔴 红 #ef4444 | 返回 |
| 其他 | 各色 | call_fn/await/break/continue/comment/raw |

### 键盘

| 按键 | 行为 |
|------|------|
| 打字 | InputRow 累积文本 → 前缀过滤补全 |
| ↑↓ | 行间移动焦点 |
| Enter | 选中补全第一项 → 替换为卡片 |
| Tab | 卡片内跳下一个槽位 |
| Backspace | 空卡片 → 退化为 InputRow |

### 注意事项

- **CodeOp 数据模型不变**：`core/types/code-op.ts` 的 12 种 CodeOp 类型是稳定的
- **emitOps/parseOps 不变**：`services/codegen/` 的发射器和解析器是稳定的
- **双模式**：Monaco CodeEditorModal 是文本编辑 fallback
- **不再依赖 Lexical**：完全移除 lexical + @lexical/react
<!-- INSERT_AFTER -->
