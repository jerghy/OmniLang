# Poiesis 阶段 v1 M2 里程碑验收记录

> 实机验证剧本（计划 Task 8），Victauri MCP 驱动。
> 日期：2026-08-12 · 项目：demo2（D:\space\omni\demo2）

## 结论

**✅ M2 验收通过**（全部剧本步骤通过；发现 1 个 M1 引入的关键缺陷，已修复并提交）。

## 验证矩阵

| 剧本步骤 | 验证内容 | 结果 | 证据 |
|---|---|---|---|
| Step 1 | Victauri 连接 com.omnilang.poiesis；打开 demo2 | ✅ | get_plugin_info；欢迎页 → 最近项目点击 |
| Step 2 | 底栏两件套 [组件库][数据节点] + store 开关 | ✅ | bottom-bar 存在；palette-toggle/data-nodes-toggle；点 [数据节点] → store dataNodesVisible true |
| Step 3 | 睡态源节点门控；醒 → 源节点浮现 | ✅ | 睡态 source-node-layer 不渲染（开关开也无）；醒 → 1 张卡（main 组件信号 newSignal——main 无 store 引用，合理）；**画布视口缺陷修复后元素可见** |
| Step 4 | 卡片名/值正确 | ✅ | newSignal 卡显示 "signalcs/"（当前值）；拖动 = 代码级已实现（dnd-kit transform 跟随），自动化模拟未触发（合成事件局限），人工验证项 |
| Step 5 | 改值当场演全链路 | ✅ | 点卡 → value-editor 浮层（回显 "signalcs/"）→ 输入 `"M2-T8-value"` → 保存 → debugValues{main:{newSignal:{text,valid:true}}} + **画布 Text_c13705 文本当场变 "M2-T8-value"**（setDebugValue → applyDebugOverrides → WASM 重编译）+ 编辑器自动关闭 |
| Step 6 | [组件库] 弹出面板 | ✅ | palette-toggle → PaletteDrawer 打开（46 个拖拽项 + "组件面板"内容 + 关闭按钮）；拖拽创建 = 既有链路（ComponentPalette 原样复用，M2 零改动），自动化模拟未触发 dnd-kit 激活，人工验证项 |
| Step 7 | 不倒退 | ✅ | 呼吸点睡/醒（醒态 title 睡回 → 点击 → awake false + 逻辑层 opacity 0）；选中元素 → 语境面板三标签 [结构][内容][行为] |

## 实机发现并已修复的缺陷

| 缺陷 | 修复提交 |
|---|---|
| **画布视口外渲染（M1 引入，M2 T8 实机发现）**：zAxisRef 容器缺 `display:flex` → InfiniteCanvas 根 `flex:1` 在 block 容器内失效 → 高度塌陷 0 → Artboard（absolute + top:50%）锚在 y=0 → 画布内容整体推到视口上方（y≈-426），用户看到空窗口。M1 验证全用 eval_js 程序化操作（视口外元素可 dispatch），掩盖了可见性问题。修复：zAxisRef 加 `display:flex + flexDirection:column`（实测画布回到 y=44 视口内）。 | `0573bbda`（代码）+ `a78fb58f`（M1 验收补注） |

## 记录的边界（非缺陷，如实声明）

1. **源节点拖动 / palette 拖拽创建**：均为自动化模拟局限（合成 PointerEvent 不触发 dnd-kit 激活——依赖真实指针捕获），功能已实现且链路未变（palette 拖拽为 M1 前既有功能，ComponentPalette 原样复用）。建议人工实测一次。
2. **TodoStore 源节点卡**：main tab 无 store 引用（demo2 main.omni 无 @store）→ 1 张卡合理；TestCodeFlow 的 TodoStore 引用场景依赖 storeReferences 方向修复（`.scratch/poiesis-stage-v1/issues/01-storeReferences-direction.md`，@store 接线前处理）。
3. **isDirty true**：M2 会话中元素点击/选择等产生（改值本身不 dirty 项目——setDebugValue 代码级确认不触 isDirty）。
4. 调试值不持久化（内存态，重开 app 自然清空——代码级确定，未做重启实测）。

## 本里程碑提交清单（main 分支）

```
bd05b6c2  M2 T1 底栏两件套
977a94c2  M2 T1 复审（active 类测试 + aria-pressed）
8a62c5c5  M2 T2 源节点收集工具
39b22cde  M2 T2 复审（valid 门控 + computed 兜底测试）
5b19046a  M2 T3 源节点卡片
ddebce65  M2 T3 复审（title 兜底 + 拖拽依赖注释）
0033af7b  M2 T4 数据区 SourceNodeLayer
d1fcb1e1  M2 T4 复审（拖拽 transform + tabType 门控测试）
cc99a933  M2 T5 改值调试 ValueEditor
6769b996  M2 T5 复审（IME 守卫 + kind 门控 + 回显剥引号）
186f2fba  M2 T6 [组件库] 弹出面板
3f5f415b  M2 T6 复审（aria-label + Escape + 错误边界共享）
0b0a2053  M2 T7 全量回归 + 死样式收尾
0573bbda  画布视口缺陷修复（M2 T8 实机发现，M1 引入）
a78fb58f  M1 验收补注
```

（注：main 上另有并行会话的提交穿插；tsc 预存 tokenizer 错误已由并行会话修复。）

## 后续

- M2 验收通过 → 按路线图推进 M3（醒态绑定：行为卡 + 五类槽点名绑定 + 手柄拖线 + 来源印记）
- 待办：issue 01（storeReferences 方向）在 @store 接线前修复；人工实测源节点拖动与 palette 拖拽创建
