# CodeFlowCanvas 分组标注 + 组级折叠 设计

- 日期：2026-08-08
- 状态：已获用户批准（brainstorming 全流程）
- 前置：ADR-057（语法节点）、ADR-058（拖拽回写）、ADR-063/064/065（布局与连线终态）

## 1. 背景与需求

用户在设计 CodeFlowCanvas 优化时明确三个目标：**编写功能快、用起来舒服、整理代码逻辑不扰乱思维**。

痛点澄清（用户拍板）：**"逻辑一长就乱，找不到节点/看不到全局"**——一个 fn 几十个节点平铺在画布上时思维被打断。

经多轮澄清收敛（用户逐条否决了 fn 子图折叠大纲、Ctrl+F 功能搜索/画布搜索后），最终确定核心方向：

> **节点注释/分组标注：可以把多个节点框起来，整成一个组合（相当于注释）。圈起来的部分可以 Fold 或 Unfold。**

即：**分组框（Blueprint Comments 精神）+ 组名即注释 + 组级折叠**。搜索定位明确不做（YAGNI）。

## 2. 关键决策

| # | 决策 | 理由 |
|---|------|------|
| D1 | 分组创建 = **Shift+点击多选** → 右键「组成分组」 | 用户明确否决拖拽框选：CodeFlow 拖拽=平移画布，是既有手势，绝不可冲突 |
| D2 | 分组数据存 **project.json 独立顶层字段** `codeFlowGroups`（键=fn 名，与 codeFlowGraphs 对齐） | 项目先例：`persistedPositions` 独立字段"不动 codeFlowGraphs 的 v3 形状"；用户指示"数据保存到 JSON 里就行" |
| D3 | 分组**不参与 .omni 解析恢复**；源码注释 `// ── 组名 ──` 只是生成物 | 与 persistedPositions 同哲学：源码管语义、JSON 管编辑态。避免改 parser 注释识别，零 round-trip 风险 |
| D4 | 折叠**仅是画布视觉**，代码始终输出完整语句 + 注释 | 折叠若影响代码则改变语义（注释掉语句不执行）——错误 |
| D5 | 成员约束：仅语句级节点（let/assign/call/store-call/condition/return 等），**不含 fn 节点**；组内成员必须在同一 fn 的 wire 连通域内 | fn 是容器不是语句；分组语义=语句级注释分区 |
| D6 | 不做嵌套分组 | YAGNI；Blueprint 嵌套分组低频 |
| D7 | 删除分组只删分组与注释，**节点保留** | 分组是注释性结构，不承载删除语义 |
| D8 | 折叠态**持久化**（collapsed 进 JSON） | 重开画布应保持用户整理过的视图 |
| D9 | 分组不参与自动布局排布，仅成员排布后**重算包围盒**；折叠组卡以估算尺寸参与布局 | 布局算法（computeLayeredLayout）不动，只加跟随层 |

## 3. 数据模型

### 3.1 project.json（v3 扩展，新增顶层字段）

```typescript
// core/types/project-json.ts 新增
/** Code flow 分组（节点注释组合），键 = fn 名（与 codeFlowGraphs 对齐）。
 *  独立顶层字段——不动 codeFlowGraphs 的 v3 形状（persistedPositions 先例）。 */
codeFlowGroups: Record<string, CodeFlowGroup[]>;

interface CodeFlowGroup {
  id: string;            // uid("group")
  name: string;          // 组名（= 生成的注释文本）
  memberIds: string[];   // 成员节点 id（PID）
  color: string;         // 8 色调色板之一
  collapsed: boolean;    // 折叠态（持久化）
}
```

### 3.2 保存链路（改动点）

`store/slices/serialization.ts` 的 `toProjectJson()`：

```typescript
// 新增，仿 collectPersistedPositions 同构（活动 tab 覆盖优先，tabData 槽先扫）
codeFlowGroups: collectCodeFlowGroups(s.elements, s.tabData, s.codeFlowGraphs),
```

- 收集：遍历 tabData 槽 + 活动 tab elements，对 codeblock 元素按所属 fn（wire 拓扑 findOwnerFnId）归类
- 折叠态/颜色/成员以活动 tab 最新编辑为准
- 仅收集有分组的 fn（空数组不写入）

### 3.3 加载链路（改动点）

`loadFromProjectJson()`：缓存 `codeFlowGroups: json.codeFlowGroups || {}`（仿 persistedPositions 缓存语义）。

物化路径（materializeFnGraphs / graph-parser 物化时）：
- 按 fn 名从缓存取 groups，校验 memberIds 存在于该 fn 元素集（**非法成员过滤兜底**）
- 挂载到运行时图，供 FlowCanvas 渲染

### 3.4 运行时 Store

`store/types.ts` 新增：

```typescript
/** Codeflow 分组缓存：fn 名 → 分组数组（从 project.json 读回，物化时消费）。 */
codeFlowGroups: Record<string, CodeFlowGroup[]>;
```

## 4. 交互设计（完全不动拖拽手势）

| 操作 | 手势 |
|------|------|
| 多选 | **Shift+点击**节点（逐个累积；再次点击取消该节点；点击空白清除全部） |
| 创建分组 | 多选 ≥2 节点 → **右键任一选中节点** → 「组成分组」→ 行内输入组名（默认"新分组"）→ Enter 确认 |
| 折叠/展开 | 分组框标题栏 ▸/▾ 按钮（或双击标题栏） |
| 改名 | 双击标题栏文字 → 行内编辑（NodeTextField 草稿提交语义复用） |
| 删除分组 | 标题栏 ×（仅删分组与注释，节点保留） |
| 改色 | 标题栏色板（8 色） |

**折叠连线语义（关键技术点）**：组内节点隐藏，但**进出连线保留**——连线自动改接到分组框边缘（进线汇入左侧、出线发自右侧），展开时还原到原节点/端口。折叠态组内成员不可被单独选中/编辑。

**折叠态组卡**：`▸ [组名] · N 条语句 · 首条语句预览（formatSyntaxNode 复用）`；组卡可整体拖动（拖动内部节点的手势不变，折叠态改拖组卡）。

## 5. 视觉设计

**展开态分组框**：

```
┌─ 🎨 [组名]  ▸▾ · · · · · · · · · · · · · × ─┐
│  (半透明彩色底色，框住成员节点)              │
│   [let]   [assign]   [condition]             │
│   [call]  [store-call]                      │
└─────────────────────────────────────────────┘
```

- 半透明底色（8 色调色板，默认靛蓝系）+ 1px 同色描边 + 圆角
- **不遮挡节点交互**：节点仍可直接点击/编辑/拖动（分组框 pointer-events 透传到节点）
- 框随成员节点实时自适应（成员移动/增删重算包围盒，padding 24px）

**折叠态组卡**：紧凑卡片（组名 + 成员数 + 首条语句预览）。

## 6. 代码生成（graph-compiler 改动）

```omni
// ── 初始化 ──            ← 组名注释，组内 wire 拓扑首条语句前
let count = 0;
set_status("ready");
```

- **插入位置**：组内成员按 fn 内 wire 拓扑排序后的第一条语句前（复用 graph-compiler 既有 emit 顺序）
- **折叠态不影响生成**：折叠只是画布视觉，代码始终输出完整语句 + 注释
- 组名改名 → 注释同步更新；组名空 → 原样输出 `// ──  ──`（注释纯文本不校验）
- **解析侧不识别该注释**：分组以 JSON 为唯一权威

## 7. 自动布局配合

- 「自动整理布局」重排：成员节点按 computeLayeredLayout 既有算法排布，**分组框在成员排布后重算包围盒**（分组不参与排布，只跟随）
- 折叠态组卡参与布局：占位尺寸 = 组卡估算尺寸（宽度 = 组内最大节点宽，高度 = 组卡高 + 内部间距）
- 布局按钮与分组不冲突：现有算法把每个节点当独立元素，分组只是视觉层

## 8. 边界与错误处理

| 场景 | 行为 |
|------|------|
| 删除分组内节点 | 节点删除，分组框自适应；成员 <2 时分组仍保留（不自动删） |
| 删除全部成员 | 分组保留（空框），可手动删组 |
| 把组内节点连出组外 | 允许（跨组连线正常渲染），分组语义=注释不约束拓扑 |
| 创建时选中含 fn 节点 | 阻止并提示（fn 不能入组） |
| 加载时非法成员（如 fn/不存在 id） | 过滤兜底 |
| 展开时组内节点与组外重叠 | 不处理（画布自由布局） |
| 跨 fn 的多选 | 不允许创建分组（成员必须同一 fn） |
| 分组键与 codeFlowGraphs 键不一致 | 收集时以 findOwnerFnId 推导 fn 归属，键始终与 codeFlowGraphs 对齐 |

## 9. 测试策略

**单元测试（数据层——本设计重点）**
- `collectCodeFlowGroups`：从 elements + tabData 收集（活动 tab 覆盖优先）；空分组不写入
- **JSON round-trip**：保存 → serialize → deserialize → 分组（id/name/color/collapsed/memberIds）完全一致
- graph-compiler emit：`// ── 组名 ──` 插入位置 = 组内 wire 拓扑首条语句前；折叠态不影响生成
- 校验：成员同一 fn、不含 fn 节点；加载时非法成员过滤兜底
- 物化消费：project.json 读回 → 按 fn 名挂载

**组件测试（交互层）**
- Shift+点击多选（累积/取消/清除）→ 右键「组成分组」→ 行内命名
- 分组框：标题栏/改名/改色/删除；折叠/展开切换
- **折叠连线语义**：组内节点隐藏、进出线改接分组框边缘、展开还原
- 折叠态组卡拖动、自动布局后分组框重算

## 10. 实施范围（8 任务，全部走 TDD）

| # | 任务 | 涉及文件 |
|---|------|---------|
| 1 | 数据层：`codeFlowGroups` 类型 + 序列化（save/load/collect） | `core/types/project-json.ts`、`store/slices/serialization.ts` |
| 2 | 运行时挂载：store 新字段 + 物化消费 + 非法成员过滤 | `store/types.ts`、物化路径 |
| 3 | Shift 多选 + 右键「组成分组」+ 行内命名 | `FlowCanvas.tsx`、`NodeContextMenu.tsx` |
| 4 | 分组框组件（标题栏/改名/改色/删除/折叠按钮） | 新 `GroupBox.tsx` + css |
| 5 | 折叠态组卡 + 连线边界重路由 | 新 `GroupCard` 逻辑、edges 构建 |
| 6 | 代码生成注释插入（组内首条语句前） | `graph-compiler.ts` |
| 7 | 自动布局配合（分组框重算 + 折叠卡占位） | `flow-layout.ts`、`FlowCanvas.tsx` |
| 8 | 全量回归 + 真机验证（Victauri：建组/折叠/保存重开/注释落盘） | 测试 + 实测 |

## 11. 明确不做（YAGNI）

- fn 子图折叠/大纲视图（用户否决）
- Ctrl+F 功能搜索/画布搜索（用户否决："找节点/找功能不是很重要"）
- 拖拽框选创建（用户否决：与画布平移冲突）
- 嵌套分组
- 分组含 fn 节点
- 折叠影响代码输出
