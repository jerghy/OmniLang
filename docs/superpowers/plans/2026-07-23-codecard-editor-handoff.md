实现 CodeCard Editor，按以下计划逐任务执行。

## 项目背景

Poiesis 是 OmniLang 的可视化编辑器（Tauri v2 + React 19 + Zustand 5）。CodeFlow 画布有 fn/condition/code-block/return 四种节点，code-block 节点目前用 Monaco 文本编辑器编辑自由代码。问题：代码块内部变量声明、状态变更是黑箱，编辑器无法感知。

目标：用结构化卡片编辑器替换 code-block 的文本编辑器。用户在卡片编辑器中打字 → 自动补全 → 卡片插入，用写代码的速度搭积木。code-block 内部不写入 PID 注释，干净 Omni 源码。

## 核心文档

- 设计文档：`docs/superpowers/specs/2026-07-23-codecard-editor-design.md`
- 实现计划：`docs/superpowers/plans/2026-07-23-codecard-editor-plan.md`

## 执行方式

使用 subagent-driven-development。16 个任务，按 Phase 1→4 顺序执行。每个任务派一个独立子代理，完成后审查再继续下一个。

## 关键约束

- 不改 CodeFlow 画布节点+连线体系
- 不改 .omni 语言语法
- code-block 内部不写入 PID 注释
- 向后兼容：旧 code-block source 字段必须正常编译
- 键盘交互优先于鼠标

## Poiesis 关键文件速查

| 文件 | 作用 |
|------|------|
| `poiesis/src/core/types/element.ts` | CodeblockElementData 定义 |
| `poiesis/src/core/types/code-op.ts` | CodeOp 类型（新增） |
| `poiesis/src/services/codegen/graph-compiler.ts` | 节点→Omni 源码，需要加 emitOps |
| `poiesis/src/services/codegen/graph-parser.ts` | Omni 源码→节点，需要加 parseOps |
| `poiesis/src/components/canvas/code/FlowCanvas.tsx` | CodeFlow 画布，双击事件 |
| `poiesis/src/components/canvas/code/CodeEditorModal.tsx` | 旧 Monaco 编辑器（将被替换） |
| `poiesis/src/store/types.ts` | PoiesisState 接口 |
| `poiesis/src/store/slices/element-crud.ts` | 元素 CRUD actions |
| `poiesis/src/core/utils/id.ts` | uid() 生成唯一 ID |

## WASM 相关（Phase 4）

| 文件 | 作用 |
|------|------|
| `omnilang/src/parser/ast/mod.rs` | AST 类型，需加 Serialize derive |
| `omnilang/src/parser/stmt.rs` | parse_stmt/parse_block 解析器 |
| `omnilang/src/lib.rs` | 需新增 parse_stmts 函数 |
| `omnilang-wasm/src/lib.rs` | 需新增 parse_code_block WASM 绑定 |
| `poiesis/src/services/monaco/diagnostics.ts` | 现有 WASM diagnostics 集成参考 |

## 注意事项

- `poiesis/src/core/utils/id.ts` 有 `uid(prefix?)` 函数，用 `uid("op")` 生成 CodeOp.pid
- 编译器已有 `emitPidComment(pid)` 给顶层节点用，但 code-block 内部操作不 emit PID
- 解析器已有 `splitByPid`/`computeLineDepths`/`assignBranchContext` 给顶层节点用，code-op-parser 自己实现 JS 正则降级
- 任务 5 的 parseOps 是 JS 正则降级方案（WASM 不可用时）；Phase 2 的 WASM parse_code_block 完成后切换到 WASM parser

## 开始

先读 `docs/superpowers/plans/2026-07-23-codecard-editor-plan.md`，然后从 Task 1 开始逐任务执行。
