# Poiesis 阶段 v1 M1 里程碑验收记录

> 实机验证剧本（计划 Task 8），Victauri REST API 驱动（sessionless fallback）。
> 日期：2026-08-12 · 项目：demo2（D:\space\omni\demo2，含 fn/绑定/信号）

## 结论

**✅ M1 验收通过**（全部剧本步骤通过；发现 2 个缺陷，均已修复并提交）。

## 验证矩阵

| 剧本步骤 | 验证内容 | 结果 | 证据 |
|---|---|---|---|
| Step 1 | Victauri 连接 app.identifier = com.omnilang.poiesis；画布渲染 | ✅ | get_plugin_info；[data-renderer="omniview"] 存在 |
| Step 2 | 呼吸点睡/醒双向 | ✅ | 睡态 title"醒来"→ 点击 → 逻辑层 opacity 1 + 10 端口 + 2 fn 节点；再点 → opacity 0 / visibility hidden / pointer-events none / 端口层卸载（干净画布） |
| Step 3 | 醒态 fn 节点浮现 + 视口同步 | ✅ | fn/assign 节点可见；Ctrl+wheel 缩放 1.12×：设计层 `scale(1.12) translate(-58.6, 27.3)` ↔ ReactFlow `matrix(1.12, 0, 0, 1.12, -65.64, 30.6)`——pan/zoom 数值精确一致（-65.64 = -58.61×1.12） |
| Step 4 | 元素端口 | ✅ | 10 端口 = 10 元素；DOM 顺序索引匹配全部 dx=dy=0（±0px，容差 ±4px；zoom 1.12 下 ÷zoom 补偿验证生效） |
| Step 5 | 语境面板 | ✅ | 睡态单选 → 主面板三标签 [结构][内容][行为]，逻辑层保持睡（单选不醒 ✓）；点 [内容] → 自动醒（opacity 1）+ ContentTab 槽位渲染（button→文本内容）；bind:value 元素 → "已绑定"徽章（slot-bound-value）；点画布空白 → 面板消失 + 浮标出现 + selCount 0；Shift 多选两元素 → 双面板（主三标签 + 副仅[内容]） + 醒区自动醒；面板锚定 gap=12px（右落位）+ y 钳制正确 |
| Step 6 | 不倒退 | ✅ | 醒态 [结构] 属性编辑：改 id → store props.id 更新 + WASM 重编译 → 画布 data-omni-el-id 当场变（醒着也能改样式 ✓）；Store 抽屉工具栏 📦 开合（storePanelVisible 翻转 + 信号显示）；右栏消失（grid = "280px 1fr" 两列，context-inspector 类零残留）；保存 → isDirty false + main.omni mtime 更新（往返闭环）；切 tab 再切回 → 元素保留 |

## 实机发现并已修复的缺陷

| 缺陷 | 修复提交 |
|---|---|
| **呼吸点未挂载**（计划"挂载留给 T4/T6"未落实——实机 UI 无呼吸点） | `f527d483` — DesignPage 组件 tab 分支挂载 `<BreathingPoint />` |
| **clearSelection 零 UI 调用者**（用户无法点画布空白取消选中 → "取消选中睡回"状态规则不可达） | `4ccec5c8` — OmniViewRenderer 根容器空白点击清空选中（元素 onClick 已 stopPropagation，仅真空白触发） |

- **画布视口外渲染（M1 引入，M2 T8 实机发现）**：zAxisRef 缺 display:flex → InfiniteCanvas 高度塌陷 → Artboard 推出视口（y≈-426）。M1 验证用 eval_js 程序化操作掩盖。修复：zAxisRef 加 display:flex + flexDirection:column。

## 记录的边界（非缺陷，M1 范围内如实声明）

1. **文本插值 `{expr}` 的来源印记 M1 未覆盖**：ContentTab 徽章仅检测 `bindings` 字段（bind:value 类）；`{newSignal}` 文本插值存于 props.label，无徽章。计划 ContentTab 实现如此，M3 来源印记深化时补齐。
2. **预存红门（非本里程碑引入）**：vitest 2 失败（ConditionReturnNode.test.tsx:112 / SyntaxNode.test.tsx:160——createPortal 08-07 引入，早于测试文件）；tsc 1 错误（tokenizer-compound-assign.test.ts:61 TS18048，批次 2 backlog）。
3. **醒态下 WASM 重编译移动元素不重新测量端口**（container box 不变，ResizeObserver 不触发）——计划自带策略，M3 关注。
4. 面板拖拽偏移两面板共享（计划自带行为）；rAF 测量保留上次位置（元素删除后不归位）——均计划参考实现自带，M3 再议。

## 本里程碑提交清单（main 分支，17 提交）

```
748b51f8  T1 视口同步工具
1409f23a  T2 睡/醒状态机
092a1eb9  T3 呼吸点组件
f28485fc  T3.5 Shift 多选
aac3d112  T4 画布叠加改造
4d222c2c  T4 复审修复
a4c822ba  T5 空态文案
3a1b505b  T5 元素端口层
8796cdce  T5 复审修复
1f574074  T6 语境面板
02fddb8e  T6 复审修复
f8ea9aa5  T7 删右栏
fb3c8028  T7 复审修复
f527d483  呼吸点挂载收口（T8 发现）
4ccec5c8  画布空白清空选中（T8 发现）
```

（注：main 上另有并行会话的 batch-4 提交穿插，与本里程碑无关。）

## 后续

- M1 验收通过 → 按路线图推进 M2（睡态工具：底栏两件套 + 源节点 + 改值调试）
- 建议修预存红门：tokenizer-compound-assign tsc 错误 + 2 个 createPortal 测试（批次 2/3 遗留）
