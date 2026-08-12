# Poiesis 阶段 v1 M3 里程碑验收记录（阶段 v1 收官）

> 实机验证剧本（计划 Task 8），Victauri MCP 驱动。
> 日期：2026-08-12 · 项目：demo2（D:\space\omni\demo2）

## 结论

**✅ M3 验收通过（阶段 v1 收官）**：行为卡 + 五类槽点名 + 自动 signal + 手柄拖线 + 来源印记全链路验证；实机发现 3 个缺陷（1 个 T4、2 个 T5），均已修复并提交。

## 验证矩阵

| 剧本步骤 | 验证内容 | 结果 | 证据 |
|---|---|---|---|
| Step 1 | 画布可见性（M2 教训） | ✅ | renderer y=85 视口内；元素 y=96（display:flex 修复持续有效） |
| Step 2 | 行为卡 | ✅ | behavior-card 表单（7 事件 × 10 动作，空表单校验 disabled）；当[点击]→[加]→[newSignal] 提交 → 自动 fn `onPressAuto` 生成 + test-btn-ghost onPress 指向 + body 动作模板 `newSignal = newSignal + (1);` + fn→body wire；**test-btn-danger 已有用户 fn 绑定被正确拒绝覆盖**（T2 复审守卫 ✓） |
| Step 3 | 五类槽点名绑定 | ✅ | BindingPicker 三类来源（信号 newSignal / fn 出口 newFn+onPressAuto / 画布元素）；点选 newSignal → label 插值 `{newSignal}` 落盘 + **徽章"来自 newSignal"** + 画布当场变 |
| Step 4 | 自动 signal 生命周期 | ✅ | connectElementLink → `input_f1bd5d_value` 创建（componentDefs.signals）；同槽复用（2 条 wire 共享 1 signal）；断开一保留（有引用）→ 断开二删除（无引用 → 回收）；目标重置；数据区"自动连线"分组 + 卡片 |
| Step 5 | 手柄拖线 | ✅ | PortHandleLayer 渲染（10 端口节点 × 2 Handle 双向）；拖拽交互 = 合成事件局限（人工验证项，同 M2） |
| Step 6 | 来源印记 + 亮线 | ✅ | 徽章"来自 X"（T6 深化）；选中 input_f1bd5d → 相关边 `port-edge-active` + stroke-width 2.5（edge 渲染依赖 1×1 节点修复） |
| Step 7 | 不倒退 + 落盘闭环 | ✅ | 保存 → main.omni 落盘：`signal input_f1bd5d_value: String = "";` + `{input_f1bd5d_value}` + `bind:value={input_f1bd5d_value}` 三件套语法正确（mtime 更新）；呼吸点/面板/数据节点正常 |

## 实机发现并已修复的缺陷

| 缺陷 | 修复提交 |
|---|---|
| **自动 signal 落盘语法错误**：connectElementLink 创建的 DefEntry 缺 defaultValue + type 小写 "string" → emitter 生成 `signal x: string;`（期望 `=` 遇 `;`）→ WASM 编译错误 → 画布字面量 + 渲染破裂。根因：`formatDefaultValue` 空串跳过（source-emitter.ts:27）+ 仅大写 "String" 补引号（:30）。修复：`{ type: "String", defaultValue: '""' }`（含引号字面量） | `f12b8b34`（补 defaultValue）+ `edf2d3de`（类型/引号修正 + 测试锁契约） |
| **端口节点 0×0 不初始化 ReactFlow edges 层**：PortHandleNode 空 div 无尺寸 → 实机 `.react-flow__edges` 容器无 svg → 自动连线不渲染（jsdom mock 尺寸掩盖）。修复：节点 1×1 最小尺寸 + 回归测试 | `4f4fac15` |

## 记录的边界（非缺陷，如实声明）

1. **拖拽交互（手柄拖线 / palette 拖拽）**：自动化合成事件不触发 dnd-kit/ReactFlow 激活（真实指针捕获依赖）——手柄层渲染 + 拖线落盘链路（connectElementLink）已验证，真实拖拽建议人工实测一次。
2. **行为卡保存往返**：行为卡提交验证在 store 层（自动 fn 三件套 + 事件指向 + emitter 结构）；完整"保存 → 重开 → 行为仍在"未做（行为卡改动在重启中回滚——未保存场景）；auto signal 的保存往返已验证（Step 7 落盘闭环）。
3. **isAutoSignal 大写 id 边界**（T4 spec 审查 LOW）：画布 uid 小写，理论边界。
4. **目标端断开重置**：断开时目标文本清空（预绑定值不恢复——代码注释声明接受，M3 范围）。
5. **手柄仅醒态**（T5 质量审查 M5）：多选（醒区）时 ElementPortLayer 显示点但手柄不渲染——T8 剧本未覆盖此细分，记录。

## 本里程碑提交清单（main 分支）

```
66fe11b7  M3 前置：issue 01 storeReferences 方向
38d12aa4  M3 T1 行为卡 UI
52392332  M3 T1 复审
bebd3ebb  M3 T2 行为卡落盘
6b6aba59  M3 T2 复审（undo/清理/落位）
50819f1f  M3 T3 五类槽点名绑定
3db674f4  M3 T3 复审（badge 键对齐）
6ad9ad75  M3 T4 自动 signal 生命周期
2969c2ef  M3 T4 复审（信号 CRUD undo）
a98a710f  M3 T5 手柄拖线
79da5ee9  M3 T5 复审（C1 拖放槽解析）
eade9162  M3 T6 来源印记 + 亮线
8c2fafa9  M3 T6 复审（元素悬停亮线）
f12b8b34  T8 实机：auto signal defaultValue
edf2d3de  T8 实机：类型 String + 引号默认值
4f4fac15  T8 实机：端口节点 1×1 edges 层
```

## 阶段 v1 总结

M1（睡/醒地基）+ M2（睡态工具）+ M3（醒态绑定）三里程碑完成。阶段 v1 交付：统一画布睡/醒状态机 + 语境面板 + 底栏两件套 + 源节点/改值调试 + 行为卡/五类槽点名/手柄拖线/自动 signal/来源印记——全链路实机验证（demo2），语法零变更（全部落盘现有 .omni 语法）。

**实机验证关键教训（跨里程碑）**：① 真实视口可见性断言必须做（M1 eval_js 掩盖画布视口缺陷）；② 合成事件对真实指针交互（dnd-kit/ReactFlow 激活）不可靠——代码级 + 人工验证；③ ReactFlow 渲染层对 0 尺寸节点有隐藏初始化条件（jsdom mock 掩盖）；④ emitter 的空串/类型大小写有隐藏语义（formatDefaultValue）。

## 后续（阶段 v2 候选）

- 拖拽交互人工实测收口；行为卡保存往返实机
- spec 中后置项：元素当线源、账本/小灯形象、变量/数据页源节点、骨架视图、果园
- 预存 backlog：tsc 已 0（并行会话修）；剩余 vitest 全绿
